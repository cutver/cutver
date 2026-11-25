//! The byte-moving half of one supervised child: the exchange over its three pipes.
//!
//! Split out of `proc.rs` so the file that decides when a tree ends stays readable next to the code
//! that does it (CONTRACT.md:174): this half decides what comes back from a tree that was started
//! and ended correctly, and what a cut in either stream means to the caller.

use std::io::{self, Read, Write};
use std::process::{ChildStdin, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use super::{Ended, SupervisionError, spawn_group, supervise};

/// Everything one completed exchange produced.
pub(crate) struct Capture {
    pub(crate) status: ExitStatus,
    pub(crate) stdout: Vec<u8>,
    pub(crate) stderr: Vec<u8>,
    /// What the stdin writer reported: `Err` when the payload could not be written, which is what a
    /// child that stopped reading stdin — or a descendant still holding that pipe — looks like here.
    pub(crate) stdin: io::Result<()>,
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
    let writer = spawn_writer_thread(child.child.stdin.take(), payload.to_vec());
    let stdout_drain = spawn_drain_thread(child.child.stdout.take(), limits.stdout);
    let stderr_drain = spawn_drain_thread(child.child.stderr.take(), limits.stderr);

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
