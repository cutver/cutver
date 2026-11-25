use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::time::Duration;

use crate::plugin::diag;
use crate::plugin::driver::baseline::system_env_baseline;
use crate::plugin::driver::{DeclaredEnvGrants, PluginDriver};
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
    grants: DeclaredEnvGrants,
    timeout_seconds: u64,
    capabilities: HashSet<Capability>,
    root_dir: PathBuf,
}

impl ProcessDriver {
    /// Creates a new `ProcessDriver` instance.
    ///
    /// `root_dir` is the resolved workspace root, and it is the directory the child runs in (#367):
    /// there is no root-less form, so no process driver can inherit the directory of the process that
    /// built it (`CONTRACT.md:154-155`). Every caller is on the `Config` that resolution happened
    /// for — the factory receives it from [`crate::plugin::PluginManager::from_config`] — and a
    /// fixture names its own directory explicitly.
    ///
    /// `grants` carries the declared `permissions.env` keys, unresolved (#300): the child environment
    /// is built when the plugin is invoked, because a configuration that is only loaded hands out
    /// nothing. A caller with a map it already resolved passes it directly.
    pub fn new(
        name: PluginName,
        root_dir: impl Into<PathBuf>,
        command: impl Into<String>,
        args: Vec<String>,
        grants: impl Into<DeclaredEnvGrants>,
        timeout_seconds: u64,
        capabilities: HashSet<Capability>,
    ) -> Self {
        Self {
            name,
            command: command.into(),
            args,
            grants: grants.into(),
            timeout_seconds,
            capabilities,
            root_dir: root_dir.into(),
        }
    }

    /// Returns the resolved workspace root the child runs in.
    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    /// Returns the configured executable command.
    pub fn command(&self) -> &str {
        &self.command
    }

    /// Returns the command arguments.
    pub fn args(&self) -> &[String] {
        &self.args
    }

    /// Returns the declared environment grants, unresolved.
    pub fn grants(&self) -> &DeclaredEnvGrants {
        &self.grants
    }

    /// Returns the timeout in seconds.
    pub fn timeout_seconds(&self) -> u64 {
        self.timeout_seconds
    }

    /// Returns the declared capabilities.
    pub fn capabilities(&self) -> &HashSet<Capability> {
        &self.capabilities
    }

    /// Builds the command for one invocation: the program, its argv, its working directory, the
    /// system baseline and the granted keys.
    ///
    /// The working directory is named, never inherited (#367 S1): the child runs in the resolved
    /// workspace root, so a plugin that opens a relative template resolves it against the release the
    /// user is cutting rather than against the directory they happened to run `cutver` from
    /// (`CONTRACT.md:154-155`).
    ///
    /// The declared grants are resolved *here*, not when the driver was constructed (#300): a
    /// configuration that is loaded, a `cutver doctor` that inspects it, and a `plugin restore` that
    /// never invokes the plugin must all run on a host without the secret. Fail-closed keeps its
    /// ground at this boundary — an unset declared key is [`PluginError::UnsetEnvGrant`] and no child
    /// is spawned at all (Pillar VI.2).
    ///
    /// `env_clear` runs first: a native plugin inherits no ambient *authority*, so an undeclared key
    /// in the parent environment (a token in CI, say) never reaches the child (Pillar VI.2). What the
    /// child does get is [`crate::plugin::driver::SYSTEM_ENV_BASELINE`], the named and documented set
    /// of system keys a non-Rust plugin cannot start without (`PATH`, `HOME`, `TMPDIR`, Windows'
    /// `SystemRoot`, and the rest of that list) — none of them secret, and none of them authority the
    /// child did not already have (#369 S1). A key the host does not set is omitted, never emptied.
    ///
    /// The declared grants are applied last, so a `permissions.env` key is the author's answer for
    /// that key and replaces the baseline value instead of colliding with it.
    ///
    /// The stdio pipes, the container (a process group on unix, a job object on Windows) and the
    /// deadline are not set here: they belong to the shared supervision in [`crate::proc`], which is
    /// the one copy of that machinery (S5).
    fn child_command(&self) -> Result<Command, PluginError> {
        let granted = self.grants.resolve(&self.name)?;
        let mut cmd = Command::new(&self.command);
        cmd.args(&self.args)
            .env_clear()
            .envs(system_env_baseline())
            .envs(&granted)
            .current_dir(&self.root_dir);
        Ok(cmd)
    }
}

/// The most a plugin may write to stdout while answering one invocation (1 MiB).
///
/// An answer past this is a typed [`PluginError::AnswerTooLarge`], never a silently truncated
/// payload: a document `cutver` cannot parse at this size is not an answer, it is a stray `print`
/// inside a loop, and the host must not pay for it in memory.
///
/// One mebibyte is chosen against the documents this program already parses. The largest of them is
/// a registry entry, capped at `MAX_ENTRY_BYTES` — also 1 MiB (`src/plugin/registry/fetch.rs:24`) —
/// while the largest answer a plugin legitimately returns is a rewritten manifest or a rendered
/// release entry, a few KiB at most. So the bound sits two orders of magnitude above any real answer
/// and is still reached in a fraction of a second by a runaway loop, which is the whole trade: a
/// stray log line costs a typed error instead of the machine's memory.
pub const MAX_PLUGIN_STDOUT_BYTES: usize = 1024 * 1024;

/// The most of a plugin's stderr the host keeps (64 KiB).
///
/// Diagnostics are a log, never a payload, and the host reads them in exactly one place: the failure
/// message of a non-zero exit. Past this bound the drain keeps the **first** 64 KiB and records the
/// cut in the bytes themselves, because the beginning of a message — where a traceback starts — is
/// the useful half, and because a loud plugin must never fail an otherwise valid call. 64 KiB is
/// chosen against what a human reads: a plugin whose diagnostics exceed it is writing a log file,
/// and a log file belongs under `payload.root_dir`.
pub const MAX_PLUGIN_STDERR_BYTES: usize = 64 * 1024;

/// The bounds one invocation is read under, in the shape [`crate::proc`] enforces.
fn capture_limits() -> proc::Limits {
    proc::Limits {
        stdout: MAX_PLUGIN_STDOUT_BYTES,
        stderr: MAX_PLUGIN_STDERR_BYTES,
    }
}

/// Serializes one invocation into the exact bytes a `runtime = "process"` child reads on stdin.
///
/// The plugin decodes this with whatever JSON library its language ships, so the bytes are the
/// contract and the framing is part of it: one document, nothing before `{`, no trailing newline, no
/// length prefix (#369 S5). A lenient parser in one language and a strict one in another disagree
/// about exactly the things nobody writes down, and the author discovers it as a bug in their plugin.
///
/// This is a named function rather than one expression inside [`PluginDriver::invoke`] so the golden
/// tests (`tests/plugin_request_goldens.rs`) go through the same door the driver does: appending a
/// newline here fails those tests instead of a user's Python plugin.
///
/// The error carries the plugin and capability because a request that cannot be serialized is the
/// plugin author's diagnosis, not an internal invariant.
pub fn encode_invocation(name: &PluginName, invocation: &PluginInvocation) -> Result<Vec<u8>, PluginError> {
    serde_json::to_vec(invocation).map_err(|source| PluginError::InvalidPayload {
        name: name.clone(),
        capability: invocation.capability.as_str().to_string(),
        source,
    })
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

        let envelope = encode_invocation(&self.name, invocation)?;

        // The plugin is spawned directly -- no shell, so its argv means what it says -- and its
        // whole exchange (the envelope write, the wait and the output drain) runs under the one
        // `timeout_seconds` budget (S8). The container, the deadline poll and the tree kill all come
        // from `crate::proc`, which is why a process the plugin starts cannot outlive it (S6).
        let timeout = Duration::from_secs(self.timeout_seconds);
        let mut command = self.child_command()?;
        match proc::run_capture(&mut command, &envelope, timeout, capture_limits()) {
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
            // The answer is not a document at that size, so there is nothing to decode and nothing a
            // later stage could recover: its own arm, never the catch-all. The drain read the excess
            // and threw it away instead of leaving the child blocked on a full pipe, which is why
            // this arrives as an overrun naming the limit — and not as a `Timeout` whose remedy,
            // "raise `timeout_seconds`", would fix nothing.
            Err(SupervisionError::Overrun { limit_bytes }) => Err(PluginError::AnswerTooLarge {
                name: self.name.clone(),
                capability: capability.as_str().to_string(),
                limit_bytes,
            }),
            // The exchange was under way, so this is not a spawn failure: the output could not be
            // read to the end. `exit_code: -1` is this variant's existing "no code" value, used for
            // a child killed by a signal; here there is no code because there was no clean exit to
            // read. The cause is carried as text because the drain produced no stderr to quote.
            Err(SupervisionError::Drain(source)) => Err(PluginError::ExecutionFailed {
                name: self.name.clone(),
                capability: capability.as_str().to_string(),
                exit_code: -1,
                stderr: format!("reading the plugin's output failed: {source}"),
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
///
/// With `--verbose` the same buffer is forwarded on the success path (#377 S2): the bytes are already
/// drained and capped by [`crate::proc`], so this carries them rather than reading anything again
/// (S4). The forward sits after the exit-status branch, so a failed call's stderr is still the error
/// message and only the error message — the failure path is byte identical either way (S3).
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

    diag::forward_child_stderr(stderr_bytes);

    written.map_err(|source| PluginError::StdinWriteFailed {
        name: name.clone(),
        capability: capability.to_string(),
        source,
    })
}

#[cfg(test)]
mod tests;
