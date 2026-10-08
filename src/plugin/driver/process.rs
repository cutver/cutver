use std::collections::{BTreeMap, HashSet};
use std::io::{Read, Write};
use std::process::{Child, ChildStderr, ChildStdout, Command, Stdio};
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

    /// Spawns the child process configured with piped stdio.
    fn spawn_child(&self) -> Result<Child, PluginError> {
        Command::new(&self.command)
            .args(&self.args)
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
        let capability = invocation.capability.as_str();
        let is_supported = self.capabilities.iter().any(|cap| cap.as_str() == capability);
        if !is_supported {
            return Err(PluginError::UnsupportedCapability {
                name: self.name.clone(),
                capability: capability.to_string(),
            });
        }

        let envelope = serde_json::to_vec(invocation).map_err(|source| PluginError::InvalidPayload {
            name: self.name.clone(),
            capability: capability.to_string(),
            source,
        })?;

        let mut child = self.spawn_child()?;
        write_stdin(&mut child, &envelope);

        let timeout = Duration::from_secs(self.timeout_seconds);
        wait_with_timeout(child, timeout, &self.name, capability)
    }
}

/// Writes payload into child stdin and closes the handle.
fn write_stdin(child: &mut Child, payload: &[u8]) {
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(payload);
        let _ = stdin.flush();
    }
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

    let status = poll_or_kill(&child_shared, timeout, name, capability)?;
    let stdout_bytes = stdout_thread.join().unwrap_or_default();
    let stderr_bytes = stderr_thread.join().unwrap_or_default();

    if status.success() {
        return Ok(stdout_bytes);
    }

    let exit_code = status.code().unwrap_or(-1);
    let stderr_str = String::from_utf8_lossy(&stderr_bytes).trim().to_string();
    Err(PluginError::ExecutionFailed {
        name: name.clone(),
        capability: capability.to_string(),
        exit_code,
        stderr: stderr_str,
    })
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
mod tests {
    use super::*;

    #[test]
    fn test_unsupported_capability() {
        let name = PluginName::new("echo-plugin").unwrap();
        let driver = ProcessDriver::new(name.clone(), "echo", vec![], BTreeMap::new(), 5, HashSet::new());
        let invocation = PluginInvocation::new("manifest.v1", "read", &serde_json::json!({})).unwrap();
        let err = driver.invoke(&invocation).unwrap_err();
        match err {
            PluginError::UnsupportedCapability {
                name: err_name,
                capability,
            } => {
                assert_eq!(err_name, name);
                assert_eq!(capability, "manifest.v1");
            }
            other => panic!("expected UnsupportedCapability, got {other:?}"),
        }
    }

    #[test]
    #[cfg_attr(windows, ignore)]
    fn test_successful_invocation_echo() {
        let name = PluginName::new("cat-plugin").unwrap();
        let mut caps = HashSet::new();
        caps.insert(Capability::ManifestV1);

        let (cmd, args) = if cfg!(windows) {
            (
                "cmd".to_string(),
                vec!["/C".to_string(), "findstr".to_string(), "^".to_string()],
            )
        } else {
            ("cat".to_string(), vec![])
        };

        let driver = ProcessDriver::new(name, cmd, args, BTreeMap::new(), 5, caps);
        let invocation = PluginInvocation::new(
            "manifest.v1",
            "write",
            &serde_json::json!({"action": "bump", "version": "1.0.0"}),
        )
        .unwrap();
        let output = driver.invoke(&invocation).unwrap();
        assert_eq!(output, serde_json::to_vec(&invocation).unwrap());
    }

    #[test]
    fn test_execution_failed_non_zero_exit() {
        let name = PluginName::new("fail-plugin").unwrap();
        let mut caps = HashSet::new();
        caps.insert(Capability::LifecycleV1);

        let (cmd, args) = if cfg!(windows) {
            (
                "cmd".to_string(),
                vec!["/C".to_string(), "dir non_existing_file_for_test_12345".to_string()],
            )
        } else {
            (
                "sh".to_string(),
                vec!["-c".to_string(), "echo 'failing now' >&2; exit 42".to_string()],
            )
        };

        let driver = ProcessDriver::new(name.clone(), cmd, args, BTreeMap::new(), 5, caps);
        let invocation = PluginInvocation::new("lifecycle.v1", "on_pre_bump", &serde_json::json!({})).unwrap();
        let err = driver.invoke(&invocation).unwrap_err();

        match err {
            PluginError::ExecutionFailed {
                name: err_name,
                capability,
                exit_code,
                stderr,
            } => {
                assert_eq!(err_name, name);
                assert_eq!(capability, "lifecycle.v1");
                if !cfg!(windows) {
                    assert_eq!(exit_code, 42);
                    assert!(stderr.contains("failing now"));
                } else {
                    assert_ne!(exit_code, 0);
                }
            }
            other => panic!("expected ExecutionFailed, got {other:?}"),
        }
    }

    #[test]
    fn test_timeout_isolation() {
        let name = PluginName::new("sleep-plugin").unwrap();
        let mut caps = HashSet::new();
        caps.insert(Capability::ChangelogV1);

        let (cmd, args) = if cfg!(windows) {
            (
                "powershell".to_string(),
                vec!["-Command".to_string(), "Start-Sleep -Seconds 5".to_string()],
            )
        } else {
            ("sleep".to_string(), vec!["5".to_string()])
        };

        let driver = ProcessDriver::new(name.clone(), cmd, args, BTreeMap::new(), 1, caps);
        let invocation = PluginInvocation::new("changelog.v1", "render", &serde_json::json!({})).unwrap();
        let start = Instant::now();
        let err = driver.invoke(&invocation).unwrap_err();
        let elapsed = start.elapsed();

        assert!(elapsed.as_secs() >= 1);
        assert!(elapsed.as_secs() < 4);

        match err {
            PluginError::Timeout {
                name: err_name,
                capability,
                timeout_secs,
            } => {
                assert_eq!(err_name, name);
                assert_eq!(capability, "changelog.v1");
                assert_eq!(timeout_secs, 1);
            }
            other => panic!("expected Timeout, got {other:?}"),
        }
    }

    #[test]
    fn test_getters_and_debug() {
        let name = PluginName::new("custom-plugin").unwrap();
        let mut caps = HashSet::new();
        caps.insert(Capability::VersioningV1);
        let mut env = BTreeMap::new();
        env.insert("FOO".to_string(), "BAR".to_string());

        let driver = ProcessDriver::new(
            name.clone(),
            "custom-cmd",
            vec!["--flag".to_string()],
            env.clone(),
            10,
            caps.clone(),
        );

        assert_eq!(driver.name(), &name);
        assert_eq!(driver.command(), "custom-cmd");
        assert_eq!(driver.args(), &["--flag".to_string()]);
        assert_eq!(driver.env(), &env);
        assert_eq!(driver.timeout_seconds(), 10);
        assert_eq!(driver.capabilities(), &caps);
        assert!(format!("{driver:?}").contains("custom-cmd"));
    }
}
