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
//! [`spawn_group`] puts the child in a container of its own, and [`Grouped::kill_tree`] ends that
//! container in one act rather than visiting what is inside it: `killpg` on unix, and a Windows job
//! object ended by `TerminateJobObject`. A descendant is created inside the container it inherits, so
//! every process the child started is inside the thing the deadline reaches. That is a property of
//! how the child is started and how the deadline ends it — enforced in those two places, not a
//! comment excusing a survivor.
//!
//! The container is the point, and #384 is what a walk costs instead: `taskkill /F /T` reaches the
//! same tree, but it reaches it one process at a time and from the leaves up, so a shell between a
//! killed leaf and the root resumes its own command list in that gap. Measured: a plugin whose
//! descendant was due three seconds in wrote its marker 1.4 s into a one-second invocation, and the
//! kill itself took 89–397 ms of the caller's time. Ending the container instead is one kernel call —
//! `TerminateJobObject` on Windows, `killpg` on unix — measured at about 1 ms, so a descendant never
//! gets the window the walk left it. One call over the members, not atomicity: a descendant created
//! between `CreateProcess` and the job assignment is outside the container ([`spawn_group`]), and a
//! host that refused a job is back on the `taskkill` walk.

use std::io;
use std::process::{Child, Command, ExitStatus};
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

mod capture;

pub(crate) use capture::{Capture, Limits, run_capture};

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

/// A spawned child inside the container a deadline can end in one act.
///
/// On unix the container is the process group [`spawn_group`] created; on Windows it is a job object
/// the child is assigned to as soon as it exists. Both are inherited by everything the child starts,
/// so [`Grouped::kill_tree`] ends the whole tree without enumerating it.
pub(crate) struct Grouped {
    pub(crate) child: Child,
    /// The job holding the child and everything it starts. `None` when Windows refused to make or
    /// assign one, which leaves [`Grouped::kill_tree`] on its `taskkill` fallback.
    #[cfg(windows)]
    job: Option<Win32Job>,
}

/// Spawns `cmd` inside a container of its own.
///
/// The container is the reason no descendant survives the deadline (S6): every process the child
/// starts inherits it, and [`Grouped::kill_tree`] ends the container instead of the single handle
/// returned here. A spawn that skipped this step would leave the caller holding one `Child` and
/// nothing to aim a tree kill at — which is exactly how a plugin's own helpers used to outlive it.
pub(crate) fn spawn_group(cmd: &mut Command) -> io::Result<Grouped> {
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
    let child = cmd.spawn()?;
    // Assigned before this function returns, so the child cannot start a descendant outside the job
    // unless it does so in the microseconds between `CreateProcess` and this call.
    #[cfg(windows)]
    let job = Win32Job::new_for(&child);
    Ok(Grouped {
        child,
        #[cfg(windows)]
        job,
    })
}

/// Waits for `child` to exit, killing its tree once `timeout` has elapsed since `start`.
///
/// The one deadline poll in the crate, so a timeout means the same thing to a preflight step, a
/// hook, a publish command and a plugin. `timeout: None` means no bound at all, which those callers
/// spell as an absent `timeout` / `default_timeout` key.
pub(crate) fn supervise(child: &mut Grouped, start: Instant, timeout: Option<Duration>) -> io::Result<Ended> {
    loop {
        match child.child.try_wait() {
            Ok(Some(status)) => return Ok(Ended::Exited(status)),
            Ok(None) => {
                let elapsed = start.elapsed();
                if let Some(limit) = timeout {
                    if elapsed >= limit {
                        child.kill_tree();
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
                child.kill_tree();
                return Err(e);
            }
        }
    }
}

impl Grouped {
    /// Ends the child and every process it started, then reaps the child.
    ///
    /// The kill is one act over a container, not a walk over a tree: `TerminateJobObject` on Windows
    /// and `killpg` on unix end every member of it at once. The difference is what #384 measured: a
    /// walk reaches the leaves first, and the shell between a killed leaf and the root runs the next
    /// command in its list in the gap before the walk returns for it — so a descendant acted after
    /// the deadline even though every one of them was eventually killed. `taskkill` stays here as the
    /// fallback for a host that refused a job object at spawn, where a walk is all there is.
    ///
    /// The direct `kill` and `wait` stay as the fallback for a host with neither container and to
    /// reap the child on every path.
    pub(crate) fn kill_tree(&mut self) {
        #[cfg(unix)]
        unsafe {
            let _ = killpg(self.child.id() as i32, SIGKILL);
        }
        #[cfg(windows)]
        {
            match &self.job {
                Some(job) => job.terminate(),
                None => {
                    let pid = self.child.id().to_string();
                    let _ = Command::new("taskkill")
                        .args(["/F", "/T", "/PID", pid.as_str()])
                        .output();
                }
            }
            let _ = self.child.kill();
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
    }
}

/// A Windows job object holding one supervised child and everything it starts.
///
/// Created without `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` on purpose: closing the handle when the
/// invocation ends must not turn a plugin that exited inside its budget into a kill of the
/// descendants it deliberately left running, which `run_capture` documents as preserved. Only
/// [`Win32Job::terminate`] ends the tree; `Drop` just closes the handle.
#[cfg(windows)]
struct Win32Job(win32::Handle);

#[cfg(windows)]
impl Win32Job {
    /// Puts `child` in a fresh job, or reports that Windows refused.
    ///
    /// `None` is not an error: a host that will not nest a job under the one it already runs us in
    /// still gets a supervised spawn, with `taskkill` as its tree kill.
    fn new_for(child: &Child) -> Option<Self> {
        use std::os::windows::io::AsRawHandle;
        unsafe {
            let job = win32::CreateJobObjectW(std::ptr::null_mut(), std::ptr::null());
            if job.is_null() {
                return None;
            }
            if win32::AssignProcessToJobObject(job, child.as_raw_handle() as win32::Handle) == 0 {
                win32::CloseHandle(job);
                return None;
            }
            Some(Self(job))
        }
    }

    /// Ends every process the job holds in one kernel call, measured at about 1 ms against the 89–397
    /// ms a `taskkill /F /T` walk took — the window in which a shell between a killed leaf and its
    /// root ran its next command after the deadline.
    ///
    /// One call over the members, not atomicity: the deaths are not simultaneous, and a descendant
    /// created before the assignment reached this job is not one of its members, as [`spawn_group`]
    /// says.
    fn terminate(&self) {
        unsafe {
            let _ = win32::TerminateJobObject(self.0, 1);
        }
    }
}

#[cfg(windows)]
impl Drop for Win32Job {
    fn drop(&mut self) {
        unsafe {
            let _ = win32::CloseHandle(self.0);
        }
    }
}

#[cfg(windows)]
mod win32 {
    use std::ffi::c_void;

    pub(super) type Handle = *mut c_void;

    // Declared here rather than pulled from a crate: `kernel32` is the only dependency this needs,
    // and the crate already declares the unix calls it needs the same way.
    #[link(name = "kernel32")]
    unsafe extern "system" {
        pub(super) fn CreateJobObjectW(attrs: *mut c_void, name: *const u16) -> Handle;
        pub(super) fn AssignProcessToJobObject(job: Handle, process: Handle) -> i32;
        pub(super) fn TerminateJobObject(job: Handle, exit_code: u32) -> i32;
        pub(super) fn CloseHandle(handle: Handle) -> i32;
    }
}

#[cfg(unix)]
const SIGKILL: i32 = 9;
#[cfg(unix)]
unsafe extern "C" {
    fn setpgid(pid: i32, pgid: i32) -> i32;
    fn killpg(pgrp: i32, sig: i32) -> i32;
}

#[cfg(test)]
mod tests;
