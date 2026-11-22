use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use sha2::{Digest, Sha256};

use crate::plugin::cache::PluginCacheRoot;
use crate::plugin::driver::{DeclaredEnvGrants, EnvLookup, PluginDriver, process_env_lookup};
use crate::plugin::dto::PluginInvocation;
use crate::plugin::error::PluginError;
use crate::plugin::types::{Capability, PermissionsConfig, PluginName};

/// Stable Extism export every WASM plugin must expose to receive invocations.
pub const INVOKE_EXPORT: &str = "invoke";

/// WebAssembly plugin driver executing guest plugins inside an Extism sandbox.
///
/// The sandbox is built on the first invocation, not when the driver is constructed (#300): loading a
/// configuration hands the guest nothing, so a command that only inspects the configuration stays
/// runnable on a host that does not export the declared `permissions.env` grants.
pub struct WasmDriver {
    name: PluginName,
    capabilities: HashSet<Capability>,
    source: WasmSource,
    plugin: Mutex<Option<extism::Plugin>>,
}

/// Everything instantiation needs, unresolved: the granted environment is resolved at instantiation.
struct WasmSource {
    bytes: Vec<u8>,
    permissions: PermissionsConfig,
    timeout_secs: Option<u64>,
    grants: DeclaredEnvGrants,
}

impl WasmSource {
    /// Builds the manifest and the sandbox, resolving the declared grants at this boundary.
    fn instantiate(&self, name: &PluginName) -> Result<extism::Plugin, PluginError> {
        let manifest = build_manifest(name, &self.bytes, &self.permissions, self.timeout_secs, &self.grants)?;
        build_plugin(name, manifest)
    }
}

impl std::fmt::Debug for WasmDriver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmDriver")
            .field("name", &self.name)
            .field("capabilities", &self.capabilities)
            .finish()
    }
}

impl WasmDriver {
    /// Creates a new sandboxed `WasmDriver`.
    ///
    /// Boundary constructor binding the grant resolution to the process environment; a caller that
    /// already has a lookup (the plugin factory, and every test) uses [`Self::from_env_lookup`].
    pub fn new(
        name: PluginName,
        wasm_bytes: &[u8],
        permissions: &PermissionsConfig,
        timeout_secs: Option<u64>,
        capabilities: HashSet<Capability>,
        expected_hash: Option<&str>,
    ) -> Result<Self, PluginError> {
        Self::from_env_lookup(
            name,
            wasm_bytes,
            permissions,
            timeout_secs,
            capabilities,
            expected_hash,
            process_env_lookup(),
        )
    }

    /// Creates a new sandboxed `WasmDriver` resolving its grants through `lookup`.
    ///
    /// Construction verifies the artifact's digest and keeps the bytes; the module is compiled and
    /// instantiated on the first invocation, which is also where the declared grants are resolved
    /// (#300). An unset declared grant therefore fails the invocation — naming the plugin and the
    /// key — instead of failing the command that only loaded the configuration.
    #[allow(clippy::too_many_arguments)]
    pub fn from_env_lookup(
        name: PluginName,
        wasm_bytes: &[u8],
        permissions: &PermissionsConfig,
        timeout_secs: Option<u64>,
        capabilities: HashSet<Capability>,
        expected_hash: Option<&str>,
        lookup: EnvLookup,
    ) -> Result<Self, PluginError> {
        verify_sha256(&name, wasm_bytes, expected_hash)?;
        let grants = DeclaredEnvGrants::with_lookup(permissions.env.clone(), lookup);

        Ok(Self {
            name,
            capabilities,
            source: WasmSource {
                bytes: wasm_bytes.to_vec(),
                permissions: permissions.clone(),
                timeout_secs,
                grants,
            },
            plugin: Mutex::new(None),
        })
    }

    /// Returns the declared capabilities for this driver.
    pub fn capabilities(&self) -> &HashSet<Capability> {
        &self.capabilities
    }
}

impl PluginDriver for WasmDriver {
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

        let mut slot = self.plugin.lock().map_err(|_| PluginError::WasmInitFailed {
            name: self.name.clone(),
            reason: "failed to acquire plugin lock (mutex poisoned)".to_string(),
        })?;

        if slot.is_none() {
            *slot = Some(self.source.instantiate(&self.name)?);
        }
        let plugin = slot.as_mut().expect("instantiated just above");

        let payload = serde_json::to_vec(invocation).map_err(|source| PluginError::InvalidPayload {
            name: self.name.clone(),
            capability: capability.as_str().to_string(),
            source,
        })?;

        plugin
            .call::<&[u8], Vec<u8>>(INVOKE_EXPORT, &payload)
            .map_err(|err| map_extism_error(&self.name, capability.as_str(), err))
    }
}

/// Verifies SHA-256 integrity when `expected_hash` is specified.
pub fn verify_sha256(name: &PluginName, bytes: &[u8], expected_hash: Option<&str>) -> Result<(), PluginError> {
    let Some(expected) = expected_hash else {
        return Ok(());
    };

    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let actual_hex = format!("{:x}", hasher.finalize());

    let normalized_expected = expected.strip_prefix("sha256:").unwrap_or(expected);

    if !actual_hex.eq_ignore_ascii_case(normalized_expected) {
        return Err(PluginError::IntegrityMismatch {
            name: name.clone(),
            expected: expected.to_string(),
            actual: format!("sha256:{actual_hex}"),
        });
    }

    Ok(())
}

/// Builds an Extism manifest configured with sandbox permissions and timeout.
fn build_manifest(
    name: &PluginName,
    wasm_bytes: &[u8],
    permissions: &PermissionsConfig,
    timeout_secs: Option<u64>,
    grants: &DeclaredEnvGrants,
) -> Result<extism::Manifest, PluginError> {
    let mut manifest = extism::Manifest::new([extism::Wasm::data(wasm_bytes.to_vec())]);

    for host in &permissions.network {
        manifest = manifest.with_allowed_host(host);
    }

    for (host, guest) in filesystem_mounts(permissions) {
        manifest = manifest.with_allowed_path(host, guest);
    }

    // The grant authority is shared with the process runtime, so an unset declared key fails here
    // instead of reaching the guest as an empty string. This is the boundary (#300): the map exists
    // only while the guest is handed it, and the command that merely loaded the configuration never
    // gets this far.
    for (key, value) in grants.resolve(name)? {
        manifest = manifest.with_config_key(key, value);
    }

    if let Some(secs) = timeout_secs {
        manifest = manifest.with_timeout(Duration::from_secs(secs));
    }

    Ok(manifest)
}

/// Returns the guest-visible mount name for a granted directory.
///
/// Extism reads the manifest key as the host directory and the value as the name the guest
/// sees, so the two cannot be the same string. The key is canonicalized at the driver, because
/// that is the directory the sandbox really opens; the guest keeps the declared absolute path
/// with forward slashes, which is the one the lifecycle payload advertises as `root_dir`.
fn guest_name(host_path: &str) -> String {
    host_path.replace('\\', "/")
}

/// The `(host directory, guest name)` pair each granted directory mounts, in declaration order.
///
/// `filesystem` is the authority: it names the grants the caller supplied and it carries the spelling
/// the guest sees, so the guest name is always that entry, forward-slashed (#268 S2). The records
/// `config::discovery` fills are a lookup table, never a substitute for it — a configuration loaded
/// from `cutver.toml` mounts exactly the canonical host `cutver doctor` reports instead of
/// canonicalizing a second time and dropping the answer, while a `PermissionsConfig` built by hand (a
/// driver test, a plugin manager handed a raw `PluginConfig`) still mounts exactly the entries it
/// asked for and has them canonicalized here. A directory that does not exist yet is mounted as
/// resolved: the grant is confined to the workspace by `config::discovery`, which refuses one that
/// escapes it.
fn filesystem_mounts(permissions: &PermissionsConfig) -> Vec<(String, String)> {
    permissions
        .filesystem
        .iter()
        .map(|resolved| {
            let canonical = permissions
                .filesystem_grants
                .iter()
                .find(|grant| &grant.resolved == resolved)
                .and_then(|grant| grant.canonical.clone());
            let host = canonical.unwrap_or_else(|| {
                std::fs::canonicalize(resolved)
                    .map(|path| path.to_string_lossy().into_owned())
                    .unwrap_or_else(|_| resolved.clone())
            });
            (host, guest_name(resolved))
        })
        .collect()
}

/// Instantiates an Extism Plugin from a Manifest.
fn build_plugin(name: &PluginName, manifest: extism::Manifest) -> Result<extism::Plugin, PluginError> {
    extism::PluginBuilder::new(manifest)
        .with_wasi(true)
        .build()
        .map_err(|err| PluginError::WasmInitFailed {
            name: name.clone(),
            reason: err.to_string(),
        })
}

/// Maps Extism execution errors to typed `PluginError` variants.
fn map_extism_error(name: &PluginName, capability: &str, err: extism::Error) -> PluginError {
    let err_str = err.to_string();
    if err_str.contains("timeout") || err_str.contains("deadline") {
        PluginError::Timeout {
            name: name.clone(),
            capability: capability.to_string(),
            timeout_secs: 0,
        }
    } else {
        PluginError::ExecutionFailed {
            name: name.clone(),
            capability: capability.to_string(),
            exit_code: 1,
            stderr: err_str,
        }
    }
}

/// Resolves or loads WASM artifact bytes from a local file path or cache.
pub fn load_wasm_bytes(name: &PluginName, source: &str, expected_hash: Option<&str>) -> Result<Vec<u8>, PluginError> {
    let path = crate::plugin::cache::read_path(source, PluginCacheRoot::resolve)?;
    let bytes = fs::read(&path).map_err(|e| PluginError::MissingConfiguration {
        name: name.clone(),
        reason: format!("failed to read WASM artifact '{source}': {e}"),
    })?;

    if let Some(expected) = expected_hash {
        verify_sha256(name, &bytes, Some(expected))?;
    }

    Ok(bytes)
}

/// Helper caching a WASM artifact to the local plugin cache.
pub fn cache_wasm_artifact(
    name: &PluginName,
    source: &str,
    expected_hash: Option<&str>,
) -> Result<PathBuf, PluginError> {
    let src_path = Path::new(source);
    if src_path.is_file() {
        return Ok(src_path.to_path_buf());
    }

    let cache = PluginCacheRoot::resolve()?;
    let cache_dir = cache.path();
    fs::create_dir_all(cache_dir).map_err(|e| PluginError::MissingConfiguration {
        name: name.clone(),
        reason: format!("failed to create plugin cache directory '{cache_dir:?}': {e}"),
    })?;

    let file_name = Path::new(source)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("plugin.wasm");
    let dest_path = cache.artifact_path(file_name);

    if dest_path.is_file() {
        let bytes = fs::read(&dest_path).map_err(|e| PluginError::MissingConfiguration {
            name: name.clone(),
            reason: format!("failed to read cached artifact at '{dest_path:?}': {e}"),
        })?;
        if let Some(expected) = expected_hash {
            verify_sha256(name, &bytes, Some(expected))?;
        }
        return Ok(dest_path);
    }

    Err(PluginError::MissingConfiguration {
        name: name.clone(),
        reason: format!("WASM artifact '{source}' was not found locally or in cache"),
    })
}

#[cfg(test)]
mod mount_tests;

#[cfg(test)]
mod tests;
