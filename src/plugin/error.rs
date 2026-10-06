use super::types::PluginName;
use thiserror::Error;

/// Rich, actionable errors for the Cutver Plugin Subsystem.
/// Follows CONTRACT.md: What failed, Where did it fail, How to fix it.
#[derive(Debug, Error)]
pub enum PluginError {
    #[error(
        "plugin '{name}' was not found in registered plugins.\n  \
        Where: plugin lookup in PluginManager\n  \
        Fix: ensure '{name}' is declared under [plugins.{name}] in cutver.toml."
    )]
    PluginNotFound { name: PluginName },

    #[error(
        "plugin '{name}' is missing required configuration: {reason}.\n  \
        Where: configuring plugin driver for '{name}'\n  \
        Fix: provide the missing field under [plugins.{name}] in cutver.toml."
    )]
    MissingConfiguration { name: PluginName, reason: String },

    #[error(
        "WASM runtime is currently not supported for plugin '{name}'.\n  \
        Where: initializing plugin driver for '{name}'\n  \
        Fix: set 'runtime = \"process\"' in [plugins.{name}] in cutver.toml."
    )]
    WasmNotSupported { name: PluginName },

    #[error(
        "plugin '{name}' does not declare or support capability '{capability}'.\n  \
        Where: plugin definition in cutver.toml\n  \
        Fix: add '{capability}' to the plugin's 'capabilities' list or check the plugin documentation."
    )]
    UnsupportedCapability { name: PluginName, capability: String },

    #[error(
        "failed to spawn plugin '{name}' process: {source}\n  \
        Where: executing command '{command}'\n  \
        Fix: verify the executable exists, has execute permissions, and is in PATH."
    )]
    SpawnFailed {
        name: PluginName,
        command: String,
        #[source]
        source: std::io::Error,
    },

    #[error(
        "plugin '{name}' timed out after {timeout_secs}s during capability '{capability}'.\n  \
        Where: invocation of '{capability}'\n  \
        Fix: increase 'timeout_seconds' in cutver.toml or optimize plugin execution."
    )]
    Timeout {
        name: PluginName,
        capability: String,
        timeout_secs: u64,
    },

    #[error(
        "plugin '{name}' invocation failed with exit status {exit_code}: {stderr}\n  \
        Where: capability '{capability}'\n  \
        Fix: check the plugin error output above and inspect plugin logs."
    )]
    ExecutionFailed {
        name: PluginName,
        capability: String,
        exit_code: i32,
        stderr: String,
    },

    #[error(
        "plugin '{name}' returned invalid IPC JSON payload for capability '{capability}': {source}\n  \
        Where: decoding plugin output from capability '{capability}'\n  \
        Fix: verify the plugin implementation adheres to the Cutver v1 JSON schema specification."
    )]
    InvalidPayload {
        name: PluginName,
        capability: String,
        #[source]
        source: serde_json::Error,
    },

    #[error(
        "plugin '{name}' failed integrity check: expected hash '{expected}', got '{actual}'.\n  \
        Where: verifying downloaded plugin artifact\n  \
        Fix: ensure the plugin source has not been modified or update the hash in cutver.toml."
    )]
    IntegrityMismatch {
        name: PluginName,
        expected: String,
        actual: String,
    },

    #[error(
        "failed to initialize WASM runtime for plugin '{name}': {reason}\n  \
        Where: initializing sandboxed WASM driver\n  \
        Fix: ensure the WASM binary is valid and compiled for wasm32-wasip1 or wasm32-unknown-unknown."
    )]
    WasmInitFailed { name: PluginName, reason: String },

    #[error(
        "plugin '{name}' rejected permission request for '{resource}'.\n  \
        Where: capability sandbox execution\n  \
        Fix: grant '{resource}' in plugin permissions under [plugins.{name}.permissions] in cutver.toml."
    )]
    PermissionDenied { name: PluginName, resource: String },
}
