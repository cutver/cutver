use std::collections::{BTreeMap, HashSet};
use std::io;
use std::process::{Command, ExitStatus};
use std::time::Duration;

use crate::plugin::driver::PluginDriver;
use crate::plugin::dto::PluginInvocation;
use crate::plugin::error::PluginError;
use crate::plugin::types::{Capability, PluginName};
use crate::proc::{self, Capture, SupervisionError};

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

    /// Builds the command for one invocation: the program, its argv, and only the granted keys.
    ///
    /// `env_clear` runs first: a native plugin inherits no ambient authority, so an undeclared key
    /// in the parent environment (a token in CI, say) never reaches the child (Pillar VI.2). The
    /// stdio pipes, the process group and the deadline are not set here: they belong to the shared
    /// supervision in [`crate::proc`], which is the one copy of that machinery (S5).
    fn child_command(&self) -> Command {
        let mut cmd = Command::new(&self.command);
        cmd.args(&self.args).env_clear().envs(&self.env);
        cmd
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

        // The plugin is spawned directly -- no shell, so its argv means what it says -- and its
        // whole exchange (the envelope write, the wait and the output drain) runs under the one
        // `timeout_seconds` budget (S8). The process group, the deadline poll and the tree kill all
        // come from `crate::proc`, which is why a process the plugin starts cannot outlive it (S6).
        let timeout = Duration::from_secs(self.timeout_seconds);
        match proc::run_capture(&mut self.child_command(), &envelope, timeout) {
            Ok(Capture {
                status,
                stdout,
                stderr,
                stdin,
            }) => {
                finish_outcome(status, &stderr, stdin, &self.name, capability.as_str())?;
                Ok(stdout)
            }
            Err(SupervisionError::TimedOut { timeout, .. }) => Err(PluginError::Timeout {
                name: self.name.clone(),
                capability: capability.as_str().to_string(),
                timeout_secs: timeout.as_secs(),
            }),
            Err(source) => Err(PluginError::SpawnFailed {
                name: self.name.clone(),
                command: self.command.clone(),
                source: source.into_io(),
            }),
        }
    }
}

/// Applies the exit-status-over-write-error precedence: the child's own stderr is the better
/// diagnosis, so it wins over a broken stdin pipe.
fn finish_outcome(
    status: ExitStatus,
    stderr_bytes: &[u8],
    written: io::Result<()>,
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

#[cfg(test)]
mod tests;
