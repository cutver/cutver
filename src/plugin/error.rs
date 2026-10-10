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
        "this build of cutver was compiled without the 'plugins' feature, so it cannot load WASM plugin '{name}'.\n  \
        Where: initializing plugin driver for '{name}'\n  \
        Fix: install or build a plugin-enabled cutver: 'cargo install cutver --features plugins' (or 'cargo build --features plugins')."
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

    #[error(
        "multiple plugins matched manifest '{path}': {}.\n  \
        Where: manifest plugin resolution\n  \
        Fix: explicitly specify 'plugin = \"<name>\"' in [[manifest]] or disambiguate 'manifest_match' patterns in cutver.toml.",
        matches.iter().map(|p| p.as_str()).collect::<Vec<_>>().join(", ")
    )]
    AmbiguousManifestPlugin { path: String, matches: Vec<PluginName> },

    #[error(
        "no registered plugin with capability 'manifest.v1' matches manifest '{path}'.\n  \
        Where: manifest plugin resolution\n  \
        Fix: configure a plugin with capability 'manifest.v1' and matching 'manifest_match' pattern in cutver.toml, or explicitly set 'plugin = \"<name>\"'."
    )]
    NoPluginForManifest { path: String },

    #[error(
        "no registered plugin declares capability 'changelog.v1'.\n  \
        Where: changelog plugin resolution\n  \
        Fix: configure a plugin declaring 'capabilities = [\"changelog.v1\"]' under [plugins.<name>] in cutver.toml, or explicitly specify 'plugin = \"<name>\"' under [changelog]."
    )]
    NoChangelogPlugin,

    #[error(
        "multiple plugins declare capability 'changelog.v1': {}.\n  \
        Where: changelog plugin resolution\n  \
        Fix: explicitly specify 'plugin = \"<name>\"' under [changelog] in cutver.toml.",
        matches.iter().map(|p| p.as_str()).collect::<Vec<_>>().join(", ")
    )]
    AmbiguousChangelogPlugin { matches: Vec<PluginName> },

    #[error(
        "no registered plugin declares capability 'versioning.v1'.\n  \
        Where: versioning plugin resolution\n  \
        Fix: configure a plugin declaring 'capabilities = [\"versioning.v1\"]' under [plugins.<name>] in cutver.toml, or explicitly specify 'plugin = \"<name>\"' under [version]."
    )]
    NoVersioningPlugin,

    #[error(
        "multiple plugins declare capability 'versioning.v1': {}.\n  \
        Where: versioning plugin resolution\n  \
        Fix: explicitly specify 'plugin = \"<name>\"' under [version] in cutver.toml.",
        matches.iter().map(|p| p.as_str()).collect::<Vec<_>>().join(", ")
    )]
    AmbiguousVersioningPlugin { matches: Vec<PluginName> },

    #[error(
        "could not resolve the Cutver plugin cache directory: neither '{variable}' nor '{fallback}' (or 'USERPROFILE' on Windows) is set.\n  \
        Where: resolving the plugin cache root before loading a WASM plugin artifact\n  \
        Fix: set '{variable}' to the directory that should hold installed plugins (for example, {variable}=~/.cache/cutver/plugins), or set '{fallback}' to the user's home directory."
    )]
    UnresolvableCacheRoot { variable: String, fallback: String },

    #[error(
        "registry entry for plugin '{name}' could not be fetched from '{url}': {reason}.\n  \
        Where: downloading the registry index entry for '{name}'\n  \
        Fix: check your network connection or proxy, then retry; the entry URL is built from the Cutver registry index."
    )]
    RegistryUnreachable { name: String, url: String, reason: String },

    #[error(
        "registry server returned HTTP {status} for plugin '{name}' at '{url}'.\n  \
        Where: downloading the registry index entry for '{name}'\n  \
        Fix: confirm the plugin name is published in the Cutver registry index; an HTTP 404 means it is not."
    )]
    RegistryHttpStatus { name: String, url: String, status: u16 },

    #[error(
        "registry entry for plugin '{name}' at '{url}' exceeds the {cap_bytes}-byte response size cap.\n  \
        Where: reading the registry index response body for '{name}'\n  \
        Fix: the entry is malformed or oversized; report it to the Cutver registry maintainers."
    )]
    RegistryResponseTooLarge { name: String, url: String, cap_bytes: u64 },

    #[error(
        "registry entry for plugin '{name}' is malformed: {reason}.\n  \
        Where: parsing the registry index entry for '{name}'\n  \
        Fix: correct the named field in the Cutver registry index entry to match the registry schema."
    )]
    RegistryMalformedEntry { name: String, reason: String },

    #[error(
        "registry entry file '{requested}' declares name '{declared}', which does not match the requested plugin.\n  \
        Where: validating the registry index entry requested as '{requested}'\n  \
        Fix: rename the entry file to match its declared 'name', or fix the 'name' field in the Cutver registry index."
    )]
    RegistryNameMismatch { requested: String, declared: String },

    #[error(
        "registry entry for plugin '{name}' declares unsupported tier '{tier}'.\n  \
        Where: validating the 'tier' field of the registry index entry for '{name}'\n  \
        Fix: set 'tier' to one of: official, verified, community in the Cutver registry index."
    )]
    RegistryUnsupportedTier { name: String, tier: String },

    #[error(
        "registry entry for plugin '{name}' declares unknown capability '{capability}'.\n  \
        Where: validating the 'capabilities' field of the registry index entry for '{name}'\n  \
        Fix: use one of the Cutver capabilities: manifest.v1, lifecycle.v1, changelog.v1, versioning.v1."
    )]
    RegistryUnknownCapability { name: String, capability: String },

    #[error(
        "registry release '{version}' for plugin '{name}' has an invalid artifact URL '{url}': {reason}.\n  \
        Where: validating a release URL in the registry index entry for '{name}'\n  \
        Fix: use an absolute https:// URL whose path ends in '.wasm' in the Cutver registry index."
    )]
    RegistryReleaseUrl {
        name: String,
        version: String,
        url: String,
        reason: String,
    },

    #[error(
        "registry entry for plugin '{name}' has an invalid sha256 digest '{value}': expected exactly 64 lowercase hexadecimal characters.\n  \
        Where: validating a release digest in the registry index entry for '{name}'\n  \
        Fix: set 'sha256' to the artifact's real lower-case hex SHA-256 in the Cutver registry index."
    )]
    RegistryDigestShape { name: String, value: String },

    #[error(
        "'{requirement}' is not a valid version requirement for plugin '{name}': {reason}.\n  \
        Where: parsing the requested version before release selection for '{name}'\n  \
        Fix: pass a semver requirement such as '1.2.3' or '^1.0' to --version."
    )]
    RegistryInvalidVersionRequirement {
        name: String,
        requirement: String,
        reason: String,
    },

    #[error(
        "no release of plugin '{name}' satisfies '{requirement}': available versions are {available}.\n  \
        Where: selecting a release from the registry index entry for '{name}'\n  \
        Fix: pass one of the available versions to --version, or publish a matching release."
    )]
    RegistryNoMatchingRelease {
        name: String,
        requirement: String,
        available: String,
    },

    #[error(
        "registry entry for plugin '{name}' has no stable release: available versions are {available}.\n  \
        Where: selecting the latest stable release from the registry index entry for '{name}'\n  \
        Fix: pass --version with a pre-release requirement to opt in, or publish a stable release."
    )]
    RegistryNoStableRelease { name: String, available: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pins the lean-binary advice so it cannot drift back to a remedy that cannot work.
    ///
    /// `WasmNotSupported` is reachable only from the `#[cfg(not(feature = "plugins"))]`
    /// branch of `create_wasm_runtime_driver`, so it always means the binary was built
    /// without the `plugins` feature. The fix must name that feature and the
    /// plugin-enabled build, never a `cutver.toml` key.
    #[test]
    fn wasm_not_supported_names_the_plugins_feature_and_the_enabled_build() {
        let name = PluginName::new("demo").unwrap();
        let message = PluginError::WasmNotSupported { name }.to_string();

        assert!(
            message.contains("plugins"),
            "message must name the 'plugins' feature token: {message}"
        );
        assert!(
            message.contains("--features plugins"),
            "message must show the plugin-enabled build command: {message}"
        );
        assert!(
            !message.contains("runtime = \"process\""),
            "message must not recommend a cutver.toml key that cannot help: {message}"
        );
    }
}
