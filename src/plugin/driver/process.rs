use std::collections::{BTreeMap, HashSet};
use std::io::{Read, Write};
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::plugin::driver::PluginDriver;
use crate::plugin::dto::PluginInvocation;
use crate::plugin::error::PluginError;
use crate::plugin::types::{Capability, PluginName};

/// Process-based plugin driver communicating via JSON IPC over stdin/stdout.
#[derive(Debug, Clone)]
pub struct ProcessDriver {
    name: PluginName,
    command: String,
    args: Vec<String>,
    env: BTreeMap<String, String>,
    timeout_seconds: u64,
    capabilities: HashSet<Capability>,
}

impl ProcessDriver {
    /// Creates a new `ProcessDriver` instance.
    pub fn new(
        name: PluginName,
        command: impl Into<String>,
        args: Vec<String>,
        env: BTreeMap<String, String>,
        timeout_seconds: u64,
        capabilities: HashSet<Capability>,
    ) -> Self {
        Self {
            name,
            command: command.into(),
            args,
            env,
            timeout_seconds,
            capabilities,
        }
    }

    /// Returns the configured executable command.
    pub fn command(&self) -> &str {
        &self.command
    }

    /// Returns the command arguments.
    pub fn args(&self) -> &[String] {
        &self.args
    }

    /// Returns the environment variables configured for the sub-process.
    pub fn env(&self) -> &BTreeMap<String, String> {
        &self.env
    }

    /// Returns the timeout in seconds.
    pub fn timeout_seconds(&self) -> u64 {
        self.timeout_seconds
    }

    /// Returns the declared capabilities.
    pub fn capabilities(&self) -> &HashSet<Capability> {
        &self.capabilities
    }

    /// Spawns the child process configured with piped stdio and only the granted environment.
    ///
    /// `env_clear` runs first: a native plugin inherits no ambient authority, so an undeclared key
    /// in the parent environment (a token in CI, say) never reaches the child (Pillar VI.2).
    fn spawn_child(&self) -> Result<Child, PluginError> {
        Command::new(&self.command)
            .args(&self.args)
            .env_clear()
            .envs(&self.env)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|source| PluginError::SpawnFailed {
                name: self.name.clone(),
                command: self.command.clone(),
                source,
            })
    }
}

impl PluginDriver for ProcessDriver {
    fn name(&self) -> &PluginName {
        &self.name
    }

    fn invoke(&self, invocation: &PluginInvocation) -> Result<Vec<u8>, PluginError> {
        let capability = invocation.capability;
        if !self.capabilities.contains(&capability) {
            return Err(PluginError::UnsupportedCapability {
                name: self.name.clone(),
                capability: capability.as_str().to_string(),
            });
        }

        let envelope = serde_json::to_vec(invocation).map_err(|source| PluginError::InvalidPayload {
            name: self.name.clone(),
            capability: capability.as_str().to_string(),
            source,
        })?;

        let mut child = self.spawn_child()?;
        let writer = spawn_writer_thread(child.stdin.take(), envelope);

        let timeout = Duration::from_secs(self.timeout_seconds);
        wait_with_timeout(child, writer, timeout, &self.name, capability.as_str())
    }
}

/// Streams the payload into the child's stdin from its own thread, reporting write failures.
///
/// The handle is owned by this thread, so the write runs concurrently with `poll_or_kill`: a
/// plugin that never reads stdin blocks only this thread, and the kill that `poll_or_kill` issues
/// on timeout closes the pipe, which releases the write with a broken pipe. The result travels on
/// a channel rather than a join handle, because joining would let a descendant that inherited the
/// pipe hold the caller (issue #264).
fn spawn_writer_thread(stdin: Option<ChildStdin>, payload: Vec<u8>) -> mpsc::Receiver<std::io::Result<()>> {
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

/// Spawns background thread to drain a stream to completion.
fn spawn_drain_thread<R: Read + Send + 'static>(reader: Option<R>) -> thread::JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut buffer = Vec::new();
        if let Some(mut r) = reader {
            let _ = r.read_to_end(&mut buffer);
        }
        buffer
    })
}

/// Waits for child process with timeout isolation, failing closed on timeout.
fn wait_with_timeout(
    child: Child,
    writer: mpsc::Receiver<std::io::Result<()>>,
    timeout: Duration,
    name: &PluginName,
    capability: &str,
) -> Result<Vec<u8>, PluginError> {
    let child_shared = Arc::new(Mutex::new(child));
    let (stdout_reader, stderr_reader) = take_output_streams(&child_shared, name, capability)?;
    let stdout_thread = spawn_drain_thread::<ChildStdout>(stdout_reader);
    let stderr_thread = spawn_drain_thread::<ChildStderr>(stderr_reader);

    // One budget covers write, wait and drain. On the timeout path `poll_or_kill` kills only the
    // direct child, so a descendant that inherited the pipes can still hold them open: joining the
    // writer or the drain threads there would hand that descendant the caller's thread, which is
    // exactly what `timeout_seconds` exists to prevent. We return at once and leave those threads
    // detached; leaking one thread is bounded and rare, blocking the caller is not.
    let start = Instant::now();
    let status = poll_or_kill(&child_shared, timeout, name, capability)?;
    let written = await_writer(writer, timeout.saturating_sub(start.elapsed()), capability);

    let stdout_bytes = stdout_thread.join().unwrap_or_default();
    let stderr_bytes = stderr_thread.join().unwrap_or_default();
    finish_outcome(status, &stderr_bytes, written, name, capability)?;

    Ok(stdout_bytes)
}

/// Waits for the writer's own report inside the remaining part of the invocation budget.
///
/// The child already exited here, so an unfinished write means the plugin, or something it
/// started, stopped draining stdin and still holds the pipe. That is a write failure, not a
/// timeout: blaming the child with `Timeout` would hide that it succeeded.
fn await_writer(
    writer: mpsc::Receiver<std::io::Result<()>>,
    remaining: Duration,
    capability: &str,
) -> std::io::Result<()> {
    match writer.recv_timeout(remaining) {
        Ok(result) => result,
        Err(RecvTimeoutError::Timeout) => Err(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            format!(
                "the plugin (or a process it started) stopped draining stdin, and the {remaining:?} left of the invocation budget for capability '{capability}' expired before the write finished"
            ),
        )),
        Err(RecvTimeoutError::Disconnected) => Err(std::io::Error::other("plugin stdin writer thread panicked")),
    }
}

/// Applies the exit-status-over-write-error precedence: the child's own stderr is the better
/// diagnosis, so it wins over a broken stdin pipe.
fn finish_outcome(
    status: std::process::ExitStatus,
    stderr_bytes: &[u8],
    written: std::io::Result<()>,
    name: &PluginName,
    capability: &str,
) -> Result<(), PluginError> {
    if !status.success() {
        return Err(PluginError::ExecutionFailed {
            name: name.clone(),
            capability: capability.to_string(),
            exit_code: status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(stderr_bytes).trim().to_string(),
        });
    }

    written.map_err(|source| PluginError::StdinWriteFailed {
        name: name.clone(),
        capability: capability.to_string(),
        source,
    })
}

/// Takes both piped output streams before the wait, so a full pipe can never block the child.
fn take_output_streams(
    child_shared: &Arc<Mutex<Child>>,
    name: &PluginName,
    capability: &str,
) -> Result<(Option<ChildStdout>, Option<ChildStderr>), PluginError> {
    let mut guard = child_shared.lock().map_err(|_| make_poison_err(name, capability))?;
    Ok((guard.stdout.take(), guard.stderr.take()))
}

/// Polls child for completion or terminates it on timeout.
fn poll_or_kill(
    child_shared: &Arc<Mutex<Child>>,
    timeout: Duration,
    name: &PluginName,
    capability: &str,
) -> Result<std::process::ExitStatus, PluginError> {
    let start = Instant::now();
    let poll_interval = Duration::from_millis(10);

    loop {
        if let Some(status) = try_check_child(child_shared, name, capability)? {
            return Ok(status);
        }

        if start.elapsed() >= timeout {
            kill_child(child_shared);
            return Err(PluginError::Timeout {
                name: name.clone(),
                capability: capability.to_string(),
                timeout_secs: timeout.as_secs(),
            });
        }

        thread::sleep(poll_interval);
    }
}

/// Helper checking if child has completed without holding lock across sleeps.
fn try_check_child(
    child_shared: &Arc<Mutex<Child>>,
    name: &PluginName,
    capability: &str,
) -> Result<Option<std::process::ExitStatus>, PluginError> {
    let mut guard = child_shared.lock().map_err(|_| make_poison_err(name, capability))?;
    guard.try_wait().map_err(|err| PluginError::SpawnFailed {
        name: name.clone(),
        command: String::new(),
        source: err,
    })
}

/// Forcefully terminates child process and awaits exit status.
fn kill_child(child_shared: &Arc<Mutex<Child>>) {
    if let Ok(mut guard) = child_shared.lock() {
        let _ = guard.kill();
        let _ = guard.wait();
    }
}

fn make_poison_err(name: &PluginName, capability: &str) -> PluginError {
    PluginError::ExecutionFailed {
        name: name.clone(),
        capability: capability.to_string(),
        exit_code: -1,
        stderr: "Plugin process lock poisoned".to_string(),
    }
}

#[cfg(test)]
mod tests;
