//! The one supervised spawn authority: the child handle, the process group, the deadline poll and
//! the tree kill, shared by every caller that has to bound a child process (S5, audit F-16).
//!
//! Two callers, two shapes, one machinery:
//!
//! - `crate::preflight::run_supervised` runs a *shell command* in a named directory and keeps no
//!   output: `[preflight]` steps, the `[hooks]` commands and `[publish].commands`.
//! - `crate::plugin::driver::process` spawns a *program* directly, with its own argv, its granted
//!   environment and one JSON envelope exchanged over the pipes: the process plugin runtime.
//!
//! The second one used to carry its own half-copy, and the half it was missing is the defect in
//! #316: it spawned outside any process group and called `Child::kill` on the single handle it
//! held, so a process the plugin started itself survived `timeout_seconds`, kept the inherited
//! stdout/stderr pipes open and was never accounted for.
//!
//! ## Why a descendant cannot outlive what started it (S6)
//!
//! [`spawn_group`] puts the child in a process group of its own, and [`kill_tree`] signals that
//! group rather than the one handle `spawn` returned: `killpg` on unix, `taskkill /F /T` on
//! Windows, which walks the same tree by parentage. A descendant is created inside the group it
//! inherits, so every process the child started is inside the thing the deadline reaches. That is a
//! property of how the child is started and how the deadline ends it — enforced in those two
//! functions, not a comment excusing a survivor.

use std::io::{self, Read, Write};
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

/// How long a supervised wait sleeps between two `try_wait` calls.
const POLL: Duration = Duration::from_millis(50);

/// How a supervised command ended.
pub(crate) enum Ended {
    /// The child exited on its own, inside the deadline or with no deadline at all.
    Exited(ExitStatus),
    /// The deadline passed first. The whole process tree has been killed by then.
    TimedOut { timeout: Duration, elapsed: Duration },
}

/// Why a supervised command could not be completed. A failed spawn or wait kills the tree before
/// this is returned, so it cannot leave a live child behind either.
pub(crate) enum SupervisionError {
    Spawn(io::Error),
    Wait(io::Error),
    /// The budget ran out before an exchange that was already under way finished — [`run_capture`]
    /// only, and never with the direct child still running: that case is [`Ended::TimedOut`].
    TimedOut {
        timeout: Duration,
    },
}

impl SupervisionError {
    pub(crate) fn into_io(self) -> io::Error {
        match self {
            Self::Spawn(e) | Self::Wait(e) => e,
            Self::TimedOut { timeout } => io::Error::new(
                io::ErrorKind::TimedOut,
                format!("the exchange did not finish inside the {timeout:?} budget"),
            ),
        }
    }
}

/// Everything one completed exchange produced.
pub(crate) struct Capture {
    pub(crate) status: ExitStatus,
    pub(crate) stdout: Vec<u8>,
    pub(crate) stderr: Vec<u8>,
    /// What the stdin writer reported: `Err` when the payload could not be written, which is what a
    /// child that stopped reading stdin — or a descendant still holding that pipe — looks like here.
    pub(crate) stdin: io::Result<()>,
}

/// The platform shell and the flag that hands it one command string.
///
/// Only the shell callers need this; the process plugin driver spawns its program directly, so no
/// shell is allowed to reinterpret its argv.
pub(crate) fn shell() -> (&'static str, &'static str) {
    #[cfg(windows)]
    {
        ("cmd", "/C")
    }
    #[cfg(not(windows))]
    {
        ("sh", "-c")
    }
}

/// Spawns `cmd` inside a process group of its own.
///
/// The group is the reason no descendant survives the deadline (S6): every process the child
/// starts inherits it, and [`kill_tree`] signals the group instead of the single handle returned
/// here. A spawn that skipped this step would leave the caller holding one `Child` and nothing to
/// aim a tree kill at — which is exactly how a plugin's own helpers used to outlive it.
pub(crate) fn spawn_group(cmd: &mut Command) -> io::Result<Child> {
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

/// Waits for `child` to exit, killing its tree once `timeout` has elapsed since `start`.
///
/// The one deadline poll in the crate, so a timeout means the same thing to a preflight step, a
/// hook, a publish command and a plugin. `timeout: None` means no bound at all, which those callers
/// spell as an absent `timeout` / `default_timeout` key.
pub(crate) fn supervise(child: &mut Child, start: Instant, timeout: Option<Duration>) -> io::Result<Ended> {
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(Ended::Exited(status)),
            Ok(None) => {
                let elapsed = start.elapsed();
                if let Some(limit) = timeout {
                    if elapsed >= limit {
                        kill_tree(child);
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
                kill_tree(child);
                return Err(e);
            }
        }
    }
}

/// Runs `cmd` as one bounded exchange: write `payload` to its stdin, wait for it, drain its output.
///
/// `cmd` comes from the caller because what a plugin is — a program, its argv, and `env_clear()`
/// followed by exactly the granted keys — belongs to the driver. This module owns everything the
/// exchange needs around it: the piped stdio, the process group, the single deadline, and the
/// write/drain threads that stop a full pipe from blocking the caller.
///
/// The budget covers the whole exchange — the write, the wait and the drain — because
/// `timeout_seconds` names one limit for one invocation, not a limit per step (S8).
///
/// On expiry with the direct child still running, [`kill_tree`] ends the tree. On expiry after it
/// has already exited, nothing is killed: an exit-zero child is not a failure, and giving it new
/// kill semantics is beyond #316. What is bounded instead is the remaining joins — L3 of
/// `odd/tasks/cutver-proc-spawn-authority.md`: closing that second hole without inventing a kill,
/// and still never handing the caller a wait past its budget.
pub(crate) fn run_capture(cmd: &mut Command, payload: &[u8], timeout: Duration) -> Result<Capture, SupervisionError> {
    let start = Instant::now();
    cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = spawn_group(cmd).map_err(SupervisionError::Spawn)?;
    let writer = spawn_writer_thread(child.stdin.take(), payload.to_vec());
    let stdout_drain = spawn_drain_thread(child.stdout.take());
    let stderr_drain = spawn_drain_thread(child.stderr.take());

    let status = match supervise(&mut child, start, Some(timeout)).map_err(SupervisionError::Wait)? {
        Ended::Exited(status) => status,
        // The tree is dead but the pipes may not be yet: a descendant can still hold what it
        // inherited until the kill lands. Waiting to find out would hand it the caller's thread, so
        // the exchange ends here and the writer and drain threads are left detached — they end on
        // their own once the OS closes the pipes.
        Ended::TimedOut { timeout, .. } => return Err(SupervisionError::TimedOut { timeout }),
    };

    // The child exited, but a descendant it started may still hold the pipes open. Every join below
    // is bounded by what is left of the same budget (L3), so the caller never waits past it.
    let stdin = recv_written(writer, left(start, timeout));
    let stdout = recv_drained(stdout_drain, start, timeout)?;
    let stderr = recv_drained(stderr_drain, start, timeout)?;
    Ok(Capture {
        status,
        stdout,
        stderr,
        stdin,
    })
}

/// What remains of the budget `timeout`, measured from `start`; zero once it has passed.
fn left(start: Instant, timeout: Duration) -> Duration {
    timeout.saturating_sub(start.elapsed())
}

fn recv_drained(
    drained: mpsc::Receiver<Vec<u8>>,
    start: Instant,
    timeout: Duration,
) -> Result<Vec<u8>, SupervisionError> {
    match drained.recv_timeout(left(start, timeout)) {
        Ok(bytes) => Ok(bytes),
        // A descendant still holds this pipe: the output never reached EOF inside the budget.
        Err(RecvTimeoutError::Timeout) => Err(SupervisionError::TimedOut { timeout }),
        // The drain thread cannot fail: it ignores read errors. Report no bytes rather than
        // inventing some.
        Err(RecvTimeoutError::Disconnected) => Ok(Vec::new()),
    }
}

/// Awaits the writer's own report inside what is left of the budget.
///
/// The child exits on its own here, so an unfinished write means the plugin — or something it
/// started — stopped draining stdin and still holds the pipe. That is a write failure, not a
/// timeout: blaming the plugin with a timeout would hide that it ran to completion.
fn recv_written(written: mpsc::Receiver<io::Result<()>>, remaining: Duration) -> io::Result<()> {
    match written.recv_timeout(remaining) {
        Ok(result) => result,
        Err(RecvTimeoutError::Timeout) => Err(io::Error::new(
            io::ErrorKind::TimedOut,
            format!("the child stopped draining stdin with {remaining:?} of the budget left"),
        )),
        Err(RecvTimeoutError::Disconnected) => Err(io::Error::other("the stdin writer thread panicked")),
    }
}

/// Streams the payload into the child's stdin from its own thread, reporting write failures.
///
/// The thread owns the handle, so the write runs concurrently with [`supervise`]: a child that
/// never reads stdin blocks only this thread, and the kill the deadline issues closes the pipe,
/// which releases the write with a broken pipe. The result travels on a channel rather than a join
/// handle, because joining would let a descendant that inherited the pipe hold the caller
/// (issue #264).
fn spawn_writer_thread(stdin: Option<ChildStdin>, payload: Vec<u8>) -> mpsc::Receiver<io::Result<()>> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let result = match stdin {
            Some(mut stdin) => stdin.write_all(&payload).and_then(|()| stdin.flush()),
            None => Ok(()),
        };
        // A send error only means the caller already returned; the thread ends here either way.
        let _ = sender.send(result);
    });
    receiver
}

/// Drains one stream to completion on its own thread, reporting the bytes on a channel.
///
/// A channel rather than a join handle for the same reason as the writer: the drain waits for an
/// EOF that a descendant holding the pipe can postpone, so the caller must be able to stop waiting
/// without stopping the read.
fn spawn_drain_thread<R: Read + Send + 'static>(reader: Option<R>) -> mpsc::Receiver<Vec<u8>> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut buffer = Vec::new();
        if let Some(mut reader) = reader {
            let _ = reader.read_to_end(&mut buffer);
        }
        let _ = sender.send(buffer);
    });
    receiver
}

/// Kills the child and every process it started, then reaps it.
///
/// The group `spawn_group` created is what makes this a tree kill: `killpg` signals every member
/// on unix, and `taskkill /F /T` walks the same tree by parentage on Windows. Signalling only the
/// single handle — `Child::kill`, which this replaces — is what let a plugin's descendant hold the
/// inherited pipes after the plugin itself was gone (#316). The direct `kill` and `wait` below stay
/// as the fallback for a host with neither, and to reap the child on every path.
pub(crate) fn kill_tree(child: &mut Child) {
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
