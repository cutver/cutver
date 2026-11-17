use crate::config::PreflightCommand;
use std::collections::HashSet;
use std::fmt;
use std::io;
use std::path::Path;
use std::process::{self, Child, Command, ExitStatus};
use std::thread;
use std::time::{Duration, Instant};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub name: String,
    pub command: String,
    pub timeout: Option<u64>,
    pub skipped: bool,
}

impl fmt::Display for Step {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.skipped {
            write!(f, "{} (skipped)", self.name)
        } else {
            write!(f, "{}: {}", self.name, self.command)
        }
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error(
        "preflight step '{name}' ({command}) failed with status {status}.\n  The release was aborted safely before modifying any files or creating commits.\n  To bypass this step if non-critical, run:\n    cutver bump auto --skip-preflight {name}"
    )]
    StepFailed {
        name: String,
        command: String,
        status: process::ExitStatus,
    },
    #[error("preflight step '{name}' timed out after {elapsed_ms}ms (limit {timeout}s)")]
    Timeout {
        name: String,
        timeout: u64,
        elapsed_ms: u128,
    },
    #[error("failed to run preflight step '{name}': {source}")]
    Spawn {
        name: String,
        #[source]
        source: io::Error,
    },
}

pub fn plan(steps: &[(String, PreflightCommand)], skip: &[String]) -> Vec<Step> {
    let skip_set: HashSet<&str> = skip.iter().map(String::as_str).collect();
    steps
        .iter()
        .map(|(name, pc)| Step {
            name: name.clone(),
            command: pc.command.clone(),
            timeout: pc.timeout,
            skipped: skip_set.contains(name.as_str()),
        })
        .collect()
}

pub fn run(steps: &[Step]) -> Result<(), Error> {
    for step in steps.iter().filter(|s| !s.skipped) {
        run_step(step)?;
    }
    Ok(())
}

fn run_step(step: &Step) -> Result<(), Error> {
    let timeout = step.timeout.map(Duration::from_secs);
    // A preflight step runs in the process working directory, which is what it always did: the
    // bounded spawn only adds the deadline and the process-group kill.
    match run_supervised(&step.command, None, timeout) {
        Ok(Ended::Exited(status)) => {
            if status.success() {
                return Ok(());
            }
            Err(Error::StepFailed {
                name: step.name.clone(),
                command: step.command.clone(),
                status,
            })
        }
        Ok(Ended::TimedOut { timeout, elapsed }) => Err(Error::Timeout {
            name: step.name.clone(),
            timeout: timeout.as_secs(),
            elapsed_ms: elapsed.as_millis(),
        }),
        Err(e) => Err(Error::Spawn {
            name: step.name.clone(),
            source: e.into_io(),
        }),
    }
}

// --------------------------------------------------------------------------------------------
// Supervised shell commands: the one copy (audit F-16)
//
// "Run this shell command, and if it outlives its timeout kill it together with its whole process
// tree" lived twice, once here and once in `bump::exec::command`, and the `[hooks]` commands had
// no copy at all, so a hook that never exits held the release open forever. The three call sites
// now share this one implementation: `[preflight]` steps, `[publish].commands` and the `[hooks]`
// commands.
//
// It lives in this crate-root module because `bump::exec` is private to `bump`, so a helper owned
// there is unreachable from here; keeping a second copy for that reason is exactly the defect.
// --------------------------------------------------------------------------------------------

/// How a supervised command ended.
pub(crate) enum Ended {
    /// The shell exited on its own, inside the deadline or with no deadline at all.
    Exited(ExitStatus),
    /// The deadline passed first. The whole process tree has been killed by then.
    TimedOut { timeout: Duration, elapsed: Duration },
}

/// Why a supervised command could not be waited for. The tree is killed before this is returned, so
/// a failed wait cannot leak a live child either.
pub(crate) enum SupervisionError {
    Spawn(io::Error),
    Wait(io::Error),
}

impl SupervisionError {
    pub(crate) fn into_io(self) -> io::Error {
        match self {
            Self::Spawn(e) | Self::Wait(e) => e,
        }
    }
}

const POLL: Duration = Duration::from_millis(50);

/// Runs `command` through the platform shell in `dir`, killing the tree at `timeout`.
pub(crate) fn run_supervised(
    command: &str,
    dir: Option<&Path>,
    timeout: Option<Duration>,
) -> Result<Ended, SupervisionError> {
    let start = Instant::now();
    let mut child = spawn(command, dir).map_err(SupervisionError::Spawn)?;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(Ended::Exited(status)),
            Ok(None) => {
                let elapsed = start.elapsed();
                if let Some(limit) = timeout {
                    if elapsed >= limit {
                        kill_tree(&mut child);
                        return Ok(Ended::TimedOut {
                            timeout: limit,
                            elapsed,
                        });
                    }
                    thread::sleep(POLL.min(limit - elapsed));
                } else {
                    thread::sleep(POLL);
                }
            }
            Err(e) => {
                kill_tree(&mut child);
                return Err(SupervisionError::Wait(e));
            }
        }
    }
}

fn spawn(command: &str, dir: Option<&Path>) -> Result<Child, io::Error> {
    let (shell, flag) = shell();
    let mut cmd = Command::new(shell);
    cmd.arg(flag).arg(command);
    if let Some(dir) = dir {
        cmd.current_dir(dir);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        unsafe {
            cmd.pre_exec(|| {
                let _ = setpgid(0, 0);
                Ok(())
            });
        }
    }
    cmd.spawn()
}

fn kill_tree(child: &mut Child) {
    #[cfg(unix)]
    unsafe {
        let _ = killpg(child.id() as i32, SIGKILL);
    }
    #[cfg(windows)]
    {
        let pid = child.id().to_string();
        let _ = Command::new("taskkill")
            .args(["/F", "/T", "/PID", pid.as_str()])
            .output();
        let _ = child.kill();
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = child.kill();
    }
    let _ = child.wait();
}

#[cfg(unix)]
const SIGKILL: i32 = 9;
#[cfg(unix)]
unsafe extern "C" {
    fn setpgid(pid: i32, pgid: i32) -> i32;
    fn killpg(pgrp: i32, sig: i32) -> i32;
}

#[cfg(unix)]
fn shell() -> (&'static str, &'static str) {
    ("sh", "-c")
}
#[cfg(windows)]
fn shell() -> (&'static str, &'static str) {
    ("cmd", "/C")
}
#[cfg(not(any(unix, windows)))]
fn shell() -> (&'static str, &'static str) {
    ("sh", "-c")
}

#[cfg(test)]
mod tests {
    static TMP_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    fn tmp_id(prefix: &str) -> String {
        let n = TMP_SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        format!("{}-{}-{}", prefix, std::process::id(), n)
    }

    use super::*;
    use crate::config::{PreflightCommand, PreflightSteps};

    fn cmd(c: &str) -> PreflightCommand {
        PreflightCommand {
            command: c.into(),
            timeout: None,
        }
    }
    fn steps() -> PreflightSteps {
        vec![
            ("a".into(), cmd("echo a")),
            ("b".into(), cmd("echo b")),
            ("c".into(), cmd("echo c")),
        ]
    }

    #[test]
    fn plan_preserves_order_and_marks_skipped() {
        let plan = plan(&steps(), &["b".into()]);
        assert_eq!(plan.len(), 3);
        assert!(!plan[0].skipped);
        assert!(plan[1].skipped);
        assert!(!plan[2].skipped);
        assert_eq!(
            plan.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(),
            vec!["a", "b", "c"]
        );
    }

    #[test]
    fn display_formats_step() {
        assert_eq!(
            Step {
                name: "tests".into(),
                command: "cargo test".into(),
                timeout: Some(5),
                skipped: false
            }
            .to_string(),
            "tests: cargo test"
        );
        assert_eq!(
            Step {
                name: "tests".into(),
                command: "cargo test".into(),
                timeout: None,
                skipped: true
            }
            .to_string(),
            "tests (skipped)"
        );
    }

    #[test]
    fn skip_list_excludes_steps_from_run() {
        let plan = plan(
            &[("fail".into(), cmd("exit 1")), ("ok".into(), cmd("true"))],
            &["fail".into()],
        );
        assert!(run(&plan).is_ok());
    }

    #[test]
    #[cfg_attr(windows, ignore)]
    fn run_executes_successful_and_failing_commands() {
        assert!(run(&plan(&[("ok".into(), cmd("true"))], &[])).is_ok());
        match run(&plan(&[("fail".into(), cmd("false"))], &[])) {
            Err(Error::StepFailed { name, status, .. }) => {
                assert_eq!(name, "fail");
                assert!(!status.success());
            }
            other => panic!("expected StepFailed, got {:?}", other),
        }
    }

    #[test]
    #[cfg_attr(windows, ignore)]
    fn run_fails_fast_on_first_error() {
        let marker = std::env::temp_dir().join(tmp_id("cutver-failfast"));
        let _ = std::fs::remove_file(&marker);
        let plan = plan(
            &[
                ("first".into(), cmd("exit 7")),
                (
                    "second".into(),
                    PreflightCommand {
                        command: format!("touch {}", marker.display()),
                        timeout: None,
                    },
                ),
            ],
            &[],
        );
        assert!(matches!(run(&plan), Err(Error::StepFailed { name, .. }) if name == "first"));
        assert!(!marker.exists(), "second step must not run after first failure");
    }

    /// The bounded spawn is shared with `[publish].commands` and `[hooks]` now (audit F-16), so this
    /// pins that the move kept what a `[preflight]` step already had: a step that outlives its
    /// timeout is killed and named. It runs on every host: the blocking command is the platform's
    /// own, because a cfg(unix) test would prove nothing on Windows.
    #[test]
    fn run_kills_long_command_at_timeout() {
        let blocking = if cfg!(windows) {
            "ping -n 11 127.0.0.1"
        } else {
            "sleep 10"
        };
        let start = Instant::now();
        let plan = plan(
            &[(
                "sleep".into(),
                PreflightCommand {
                    command: blocking.into(),
                    timeout: Some(1),
                },
            )],
            &[],
        );
        match run(&plan) {
            Err(Error::Timeout {
                name,
                timeout,
                elapsed_ms,
            }) => {
                assert_eq!(name, "sleep");
                assert_eq!(timeout, 1);
                assert!(elapsed_ms >= 1000, "the elapsed time must be measured, not assumed");
            }
            other => panic!("expected Timeout, got {:?}", other),
        }
        assert!(
            start.elapsed() < Duration::from_secs(9),
            "the timeout must fire near 1s instead of waiting for the step"
        );
    }
}
