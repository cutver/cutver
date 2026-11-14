use std::collections::{BTreeMap, HashSet};
use std::io::{Read, Write};
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, Stdio};
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
/// on timeout closes the pipe, which releases the write with a broken pipe.
fn spawn_writer_thread(stdin: Option<ChildStdin>, payload: Vec<u8>) -> thread::JoinHandle<std::io::Result<()>> {
    thread::spawn(move || {
        let Some(mut stdin) = stdin else {
            return Ok(());
        };
        stdin.write_all(&payload)?;
        stdin.flush()
    })
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
    writer: thread::JoinHandle<std::io::Result<()>>,
    timeout: Duration,
    name: &PluginName,
    capability: &str,
) -> Result<Vec<u8>, PluginError> {
    let child_shared = Arc::new(Mutex::new(child));
    let stdout_reader = child_shared
        .lock()
        .map_err(|_| make_poison_err(name, capability))?
        .stdout
        .take();
    let stderr_reader = child_shared
        .lock()
        .map_err(|_| make_poison_err(name, capability))?
        .stderr
        .take();

    let stdout_thread = spawn_drain_thread::<ChildStdout>(stdout_reader);
    let stderr_thread = spawn_drain_thread::<ChildStderr>(stderr_reader);

    // The writer joins only after the child ends, so write, wait and drain share one budget.
    let outcome = poll_or_kill(&child_shared, timeout, name, capability);
    let written = writer
        .join()
        .unwrap_or_else(|_| Err(std::io::Error::other("plugin stdin writer thread panicked")));
    let status = outcome?;

    let stdout_bytes = stdout_thread.join().unwrap_or_default();
    let stderr_bytes = stderr_thread.join().unwrap_or_default();

    if !status.success() {
        // The child's own failure carries its stderr, which is the better diagnosis of the two.
        return Err(PluginError::ExecutionFailed {
            name: name.clone(),
            capability: capability.to_string(),
            exit_code: status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&stderr_bytes).trim().to_string(),
        });
    }

    written.map_err(|source| PluginError::StdinWriteFailed {
        name: name.clone(),
        capability: capability.to_string(),
        source,
    })?;

    Ok(stdout_bytes)
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
