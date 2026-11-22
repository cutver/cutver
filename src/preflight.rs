use crate::config::PreflightCommand;
use std::collections::HashSet;
use std::fmt;
use std::io;
use std::path::Path;
use std::process::{self, Command};
use std::time::{Duration, Instant};
use thiserror::Error;

pub(crate) use crate::proc::{Ended, SupervisionError};

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

pub fn run(root: &Path, steps: &[Step]) -> Result<(), Error> {
    for step in steps.iter().filter(|s| !s.skipped) {
        run_step(root, step)?;
    }
    Ok(())
}

fn run_step(root: &Path, step: &Step) -> Result<(), Error> {
    let timeout = step.timeout.map(Duration::from_secs);
    // A preflight step runs in the repository root, like every other stage of the pipeline. The
    // process working directory is ambient authority — it is whatever launched cutver — so a step
    // that named a relative path used to write into the caller's directory instead of the
    // repository's (#295).
    match run_supervised(&step.command, root, timeout) {
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
// Supervised shell commands: one caller of the shared supervision
//
// "Run this shell command, and if it outlives its timeout kill it together with its whole process
// tree" lived twice, once here and once in `bump::exec::command`, and the `[hooks]` commands had
// no copy at all, so a hook that never exits held the release open forever. The three call sites
// now share one implementation: `[preflight]` steps, `[publish].commands` and the `[hooks]`
// commands. Since #316 the machinery itself lives in `crate::proc`, which the process plugin driver
// calls too, so the process group, the deadline poll and the tree kill exist once (S5).
//
// It lives in a crate-root module because `bump::exec` is private to `bump`, so a helper owned
// there is unreachable from here; keeping a second copy for that reason is exactly the defect.
// --------------------------------------------------------------------------------------------

/// Runs `command` through the platform shell in `dir`, killing the tree at `timeout`.
///
/// The thin shell wrapper over the one supervision: a command line, the directory it runs in, and
/// no output to capture. The process plugin driver calls `crate::proc::run_capture` instead, because
/// it spawns a program rather than a line for the shell to interpret.
pub(crate) fn run_supervised(command: &str, dir: &Path, timeout: Option<Duration>) -> Result<Ended, SupervisionError> {
    let (shell, flag) = crate::proc::shell();
    let mut cmd = Command::new(shell);
    cmd.arg(flag).arg(command).current_dir(dir);
    // The budget starts before the spawn, the same way `run_capture` takes it: spawn latency is part
    // of the invocation, and reading the clock after the spawn would hand it to the command for free.
    let start = Instant::now();
    let mut child = crate::proc::spawn_group(&mut cmd).map_err(SupervisionError::Spawn)?;
    crate::proc::supervise(&mut child, start, timeout).map_err(SupervisionError::Wait)
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
        assert!(run(Path::new("."), &plan).is_ok());
    }

    #[test]
    #[cfg_attr(windows, ignore)]
    fn run_executes_successful_and_failing_commands() {
        assert!(run(Path::new("."), &plan(&[("ok".into(), cmd("true"))], &[])).is_ok());
        match run(Path::new("."), &plan(&[("fail".into(), cmd("false"))], &[])) {
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
        assert!(matches!(run(Path::new("."), &plan), Err(Error::StepFailed { name, .. }) if name == "first"));
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
        match run(Path::new("."), &plan) {
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
