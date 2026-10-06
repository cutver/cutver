use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use sha2::{Digest, Sha256};

use crate::plugin::driver::PluginDriver;
use crate::plugin::error::PluginError;
use crate::plugin::types::{Capability, PermissionsConfig, PluginName};

/// WebAssembly plugin driver executing guest plugins inside an Extism sandbox.
pub struct WasmDriver {
    name: PluginName,
    plugin: Mutex<extism::Plugin>,
    capabilities: HashSet<Capability>,
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
    pub fn new(
        name: PluginName,
        wasm_bytes: &[u8],
        permissions: &PermissionsConfig,
        timeout_secs: Option<u64>,
        capabilities: HashSet<Capability>,
        expected_hash: Option<&str>,
    ) -> Result<Self, PluginError> {
        verify_sha256(&name, wasm_bytes, expected_hash)?;
        let manifest = build_manifest(wasm_bytes, permissions, timeout_secs);
        let plugin = build_plugin(&name, manifest)?;

        Ok(Self {
            name,
            plugin: Mutex::new(plugin),
            capabilities,
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

    fn invoke(&self, capability: &str, payload: &[u8]) -> Result<Vec<u8>, PluginError> {
        let is_supported = self.capabilities.iter().any(|cap| cap.as_str() == capability);
        if !is_supported {
            return Err(PluginError::UnsupportedCapability {
                name: self.name.clone(),
                capability: capability.to_string(),
            });
        }

        let mut plugin = self.plugin.lock().map_err(|_| PluginError::WasmInitFailed {
            name: self.name.clone(),
            reason: "failed to acquire plugin lock (mutex poisoned)".to_string(),
        })?;

        plugin
            .call::<&[u8], Vec<u8>>(capability, payload)
            .map_err(|err| map_extism_error(&self.name, capability, err))
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
fn build_manifest(wasm_bytes: &[u8], permissions: &PermissionsConfig, timeout_secs: Option<u64>) -> extism::Manifest {
    let mut manifest = extism::Manifest::new([extism::Wasm::data(wasm_bytes.to_vec())]);

    for host in &permissions.network {
        manifest = manifest.with_allowed_host(host);
    }

    for path_str in &permissions.filesystem {
        let path = PathBuf::from(path_str);
        manifest = manifest.with_allowed_path(path_str.clone(), path);
    }

    for env_var in &permissions.env {
        let val = std::env::var(env_var).unwrap_or_default();
        manifest = manifest.with_config_key(env_var, val);
    }

    if let Some(secs) = timeout_secs {
        manifest = manifest.with_timeout(Duration::from_secs(secs));
    }

    manifest
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

/// Resolves the Cutver plugin cache directory: `~/.cache/cutver/plugins/`.
pub fn plugin_cache_dir() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".cache").join("cutver").join("plugins")
}

/// Resolves or loads WASM artifact bytes from a local file path or cache.
pub fn load_wasm_bytes(name: &PluginName, source: &str, expected_hash: Option<&str>) -> Result<Vec<u8>, PluginError> {
    let path = Path::new(source);
    let bytes = if path.is_file() {
        fs::read(path).map_err(|e| PluginError::MissingConfiguration {
            name: name.clone(),
            reason: format!("failed to read WASM file from '{source}': {e}"),
        })?
    } else {
        let cached = plugin_cache_dir().join(source);
        fs::read(&cached).map_err(|e| PluginError::MissingConfiguration {
            name: name.clone(),
            reason: format!("failed to read cached WASM artifact from '{source}': {e}"),
        })?
    };

    if let Some(expected) = expected_hash {
        verify_sha256(name, &bytes, Some(expected))?;
    }

    Ok(bytes)
}

/// Helper caching a WASM artifact to the local plugin cache.
pub fn cache_wasm_artifact(name: &PluginName, source: &str, expected_hash: Option<&str>) -> Result<PathBuf, PluginError> {
    let src_path = Path::new(source);
    if src_path.is_file() {
        return Ok(src_path.to_path_buf());
    }

    let cache_dir = plugin_cache_dir();
    fs::create_dir_all(&cache_dir).map_err(|e| PluginError::MissingConfiguration {
        name: name.clone(),
        reason: format!("failed to create plugin cache directory '{cache_dir:?}': {e}"),
    })?;

    let file_name = Path::new(source)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("plugin.wasm");
    let dest_path = cache_dir.join(file_name);

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
mod tests {
    use super::*;

    const MINIMAL_WASM: &[u8] = &[0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];

    #[test]
    fn test_integrity_verification_success_plain_and_prefix() {
        let name = PluginName::new("test-plugin").unwrap();
        let mut hasher = Sha256::new();
        hasher.update(MINIMAL_WASM);
        let hex = format!("{:x}", hasher.finalize());

        assert!(verify_sha256(&name, MINIMAL_WASM, Some(&hex)).is_ok());
        let prefixed = format!("sha256:{hex}");
        assert!(verify_sha256(&name, MINIMAL_WASM, Some(&prefixed)).is_ok());
    }

    #[test]
    fn test_integrity_mismatch_returns_error() {
        let name = PluginName::new("test-plugin").unwrap();
        let wrong_hash = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let err = verify_sha256(&name, MINIMAL_WASM, Some(wrong_hash)).unwrap_err();

        match err {
            PluginError::IntegrityMismatch {
                name: err_name,
                expected,
                actual,
            } => {
                assert_eq!(err_name, name);
                assert_eq!(expected, wrong_hash);
                assert!(actual.starts_with("sha256:"));
            }
            other => panic!("expected IntegrityMismatch, got {other:?}"),
        }
    }

    #[test]
    fn test_capability_check_fails_when_undeclared() {
        let name = PluginName::new("test-wasm").unwrap();
        let perms = PermissionsConfig::default();
        let caps = HashSet::from([Capability::ManifestV1]);

        let driver = WasmDriver::new(name.clone(), MINIMAL_WASM, &perms, None, caps, None).unwrap();

        let err = driver.invoke("lifecycle.v1", b"{}").unwrap_err();
        match err {
            PluginError::UnsupportedCapability {
                name: err_name,
                capability,
            } => {
                assert_eq!(err_name, name);
                assert_eq!(capability, "lifecycle.v1");
            }
            other => panic!("expected UnsupportedCapability, got {other:?}"),
        }
    }

    #[test]
    fn test_wasm_driver_creation_with_valid_minimal_wasm() {
        let name = PluginName::new("valid-driver").unwrap();
        let perms = PermissionsConfig {
            network: vec!["api.example.com".to_string()],
            env: vec!["PATH".to_string()],
            filesystem: vec![".".to_string()],
        };
        let caps = HashSet::from([Capability::ManifestV1]);

        let driver = WasmDriver::new(name.clone(), MINIMAL_WASM, &perms, Some(5), caps, None);
        assert!(driver.is_ok());
        let driver = driver.unwrap();
        assert_eq!(driver.name(), &name);
        assert!(driver.capabilities().contains(&Capability::ManifestV1));
    }

    #[test]
    fn test_load_wasm_bytes_local_file() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("cutver_test_minimal.wasm");
        fs::write(&file_path, MINIMAL_WASM).unwrap();

        let name = PluginName::new("test-load").unwrap();
        let loaded = load_wasm_bytes(&name, file_path.to_str().unwrap(), None).unwrap();
        assert_eq!(loaded, MINIMAL_WASM);

        let _ = fs::remove_file(file_path);
    }
}
