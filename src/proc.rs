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
    /// The output could not be read to the end — a failed read, or a drain thread that panicked.
    /// A partial buffer would look like a short-but-valid payload, so it is an error instead.
    Drain(io::Error),
    /// The budget ran out before an exchange that was already under way finished — [`run_capture`]
    /// only, and never with the direct child still running: that case is [`Ended::TimedOut`].
    TimedOut {
        timeout: Duration,
    },
    /// A stream that is a *payload* produced more than the exchange's limit for it, so there is no
    /// document for the caller. The drain read the excess and dropped it instead of blocking the child.
    Overrun {
        limit_bytes: usize,
    },
}

impl SupervisionError {
    pub(crate) fn into_io(self) -> io::Error {
        match self {
            Self::Spawn(e) | Self::Wait(e) | Self::Drain(e) => e,
            Self::TimedOut { timeout } => io::Error::new(
                io::ErrorKind::TimedOut,
                format!("the exchange did not finish inside the {timeout:?} budget"),
            ),
            Self::Overrun { limit_bytes } => io::Error::new(
                io::ErrorKind::InvalidData,
                format!("the exchange produced more than the {limit_bytes}-byte output limit"),
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

/// The byte limits one [`run_capture`] exchange applies to the child's two output streams.
///
/// The streams are bounded differently because they mean different things: `stdout` carries the
/// **payload**, so a run past its limit leaves no document and the exchange fails with
/// [`SupervisionError::Overrun`]; `stderr` carries **diagnostics**, so a run past its limit keeps the
/// first bytes and records the cut in them. The numbers are the caller's: a plugin's are
/// `MAX_PLUGIN_*_BYTES` in `crate::plugin::driver::process`.
pub(crate) struct Limits {
    pub(crate) stdout: usize,
    pub(crate) stderr: usize,
}

/// What one drained stream produced, and what had to be dropped to fit its limit.
#[derive(Debug, Default)]
struct Drained {
    bytes: Vec<u8>,
    /// Bytes the child wrote past the limit; zero when the stream stayed inside it.
    dropped: u64,
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
pub(crate) fn run_capture(
    cmd: &mut Command,
    payload: &[u8],
    timeout: Duration,
    limits: Limits,
) -> Result<Capture, SupervisionError> {
    let start = Instant::now();
    cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = spawn_group(cmd).map_err(SupervisionError::Spawn)?;
    let writer = spawn_writer_thread(child.stdin.take(), payload.to_vec());
    let stdout_drain = spawn_drain_thread(child.stdout.take(), limits.stdout);
    let stderr_drain = spawn_drain_thread(child.stderr.take(), limits.stderr);

    let status = match supervise(&mut child, start, Some(timeout)).map_err(SupervisionError::Wait)? {
        Ended::Exited(status) => status,
        // The tree is dead but the pipes may not be yet: a descendant can still hold what it
        // inherited until the kill lands. Waiting to find out would hand it the caller's thread, so
        // the exchange ends here and the writer and drain threads are left detached — they end on
        // their own once the OS closes the pipes.
        Ended::TimedOut { timeout, .. } => return Err(SupervisionError::TimedOut { timeout }),
    };

    // Every join below is bounded by what is left of the same budget (L3), so the caller never waits
    // past it. The answer is settled first because an overrun outranks the exit status: a payload the
    // host stopped reading holds no document for a non-zero exit to have explained.
    let stdin = recv_written(writer, left(start, timeout));
    let answer = recv_drained(stdout_drain, start, timeout)?;
    if answer.dropped > 0 {
        return Err(SupervisionError::Overrun {
            limit_bytes: limits.stdout,
        });
    }
    let Drained { mut bytes, dropped } = recv_drained(stderr_drain, start, timeout)?;
    if dropped > 0 {
        let marker = format!(
            "\n[cutver] stderr truncated at {} bytes; {dropped} more were dropped.\n",
            limits.stderr
        );
        bytes.extend_from_slice(marker.as_bytes());
    }
    Ok(Capture {
        status,
        stdout: answer.bytes,
        stderr: bytes,
        stdin,
    })
}

/// What remains of the budget `timeout`, measured from `start`; zero once it has passed.
fn left(start: Instant, timeout: Duration) -> Duration {
    timeout.saturating_sub(start.elapsed())
}

/// The payload stream and the diagnostics stream differ in what a cut means (`Limits`), so the
/// excess is reported here rather than decided: `dropped` is the caller's to interpret.
fn recv_drained(
    drained: mpsc::Receiver<io::Result<Drained>>,
    start: Instant,
    timeout: Duration,
) -> Result<Drained, SupervisionError> {
    match drained.recv_timeout(left(start, timeout)) {
        // A read that failed is a read that failed. Returning the bytes collected so far would hand
        // the caller a truncated payload it cannot tell from a complete one (#266).
        Ok(result) => result.map_err(SupervisionError::Drain),
        // A descendant still holds this pipe: the output never reached EOF inside the budget.
        Err(RecvTimeoutError::Timeout) => Err(SupervisionError::TimedOut { timeout }),
        // The thread ended without reporting, which means it panicked. That is a bug, not an empty
        // output, and naming it as one is the whole point of this arm.
        Err(RecvTimeoutError::Disconnected) => Err(SupervisionError::Drain(io::Error::other(
            "the output drain thread panicked",
        ))),
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
fn spawn_drain_thread<R: Read + Send + 'static>(
    reader: Option<R>,
    limit: usize,
) -> mpsc::Receiver<io::Result<Drained>> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut drained = Drained::default();
        let result = match reader {
            Some(mut reader) => read_capped(&mut reader, limit, &mut drained).map(|()| drained),
            None => Ok(drained),
        };
        // A send error only means the caller already returned; the thread ends here either way.
        let _ = sender.send(result);
    });
    receiver
}

/// Reads `reader` to EOF, keeping at most `limit` bytes of it and throwing the rest away.
///
/// The read **keeps going after the limit is reached**, and the excess is read rather than left in
/// the pipe. That sequencing is the reason this is two copies and not a bounded `read_to_end`: a
/// drain that stopped reading would leave the child blocked on a full pipe, so [`supervise`] would
/// reach the deadline first and the caller would report a *timeout* — the wrong diagnosis for a
/// plugin that is merely loud, with a remedy ("raise `timeout_seconds`") that cannot fix it.
///
/// The bytes kept are the **first** ones, never the last, and `dropped` says how many passed by:
/// for a payload the limit means "this is not a document", for diagnostics the head is the useful half.
fn read_capped<R: Read>(reader: &mut R, limit: usize, drained: &mut Drained) -> io::Result<()> {
    io::copy(&mut reader.by_ref().take(limit as u64), &mut drained.bytes)?;
    drained.dropped = io::copy(reader, &mut io::sink())?;
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A reader that fails the way a pipe fails: the child exited zero, the read did not finish.
    struct FailingReader;

    impl Read for FailingReader {
        fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("the pipe broke mid-read"))
        }
    }

    /// #266: "A drain-thread panic or a short read on a child that exited 0 returns `Ok` with
    /// truncated stdout, and the caller reports an opaque payload error with no cause." An empty
    /// buffer is exactly what makes that opaque, so the read failure has to reach the caller.
    #[test]
    fn a_failed_drain_is_reported_instead_of_truncating_the_output() {
        let drained = spawn_drain_thread(Some(FailingReader), 1024);
        let error = recv_drained(drained, Instant::now(), Duration::from_secs(1))
            .expect_err("a read that failed must not look like an empty output");
        assert!(
            error.into_io().to_string().contains("the pipe broke mid-read"),
            "the read failure must reach the caller with its cause attached"
        );
    }
}
