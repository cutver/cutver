//! Effective status of each declared plugin (read-only).

use std::fs;

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::config::Config;
use crate::plugin::cache::{PluginCacheRoot, read_path};
use crate::plugin::types::{PluginConfig, PluginName, RuntimeKind};

/// Digest comparison between the on-disk artifact and the declared `hash`.
///
/// Three states, not two: `unknown` covers "no declared hash" and "artifact not
/// present" so a missing artifact is never misreported as a mismatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DigestState {
    Matched,
    Mismatched,
    Unknown,
}

/// Effective state of one `[plugins.<name>]` declaration.
#[derive(Debug, Clone, Serialize)]
pub struct PluginStatus {
    pub name: PluginName,
    pub runtime: RuntimeKind,
    pub source: Option<String>,
    pub declared_hash: Option<String>,
    pub installed: bool,
    pub digest: DigestState,
}

/// Reports the effective state of every declared plugin, sorted by name.
pub fn list_statuses(config: &Config, cache: &PluginCacheRoot) -> Vec<PluginStatus> {
    let mut statuses: Vec<PluginStatus> = config
        .plugins
        .iter()
        .map(|(name, plugin)| status_for(name, plugin, cache))
        .collect();
    statuses.sort_by(|left, right| left.name.cmp(&right.name));
    statuses
}

fn status_for(name: &PluginName, plugin: &PluginConfig, cache: &PluginCacheRoot) -> PluginStatus {
    let (installed, digest) = match plugin.runtime {
        RuntimeKind::Wasm => inspect_wasm(plugin, cache),
        RuntimeKind::Process => (false, DigestState::Unknown),
    };

    PluginStatus {
        name: name.clone(),
        runtime: plugin.runtime,
        source: plugin.source.clone(),
        declared_hash: plugin.hash.clone(),
        installed,
        digest,
    }
}

fn inspect_wasm(plugin: &PluginConfig, cache: &PluginCacheRoot) -> (bool, DigestState) {
    let Some(source) = plugin.source.as_deref() else {
        return (false, DigestState::Unknown);
    };

    let Ok(path) = read_path(source, || Ok(cache.clone())) else {
        return (false, DigestState::Unknown);
    };

    let Ok(bytes) = fs::read(&path) else {
        return (false, DigestState::Unknown);
    };

    let Some(declared) = plugin.hash.as_deref() else {
        return (true, DigestState::Unknown);
    };

    (true, digest_state(&bytes, declared))
}

/// Compares the declared hash the way the loader's `verify_sha256` does:
/// an optional `sha256:` prefix, case-insensitive hexadecimal.
fn digest_state(bytes: &[u8], declared: &str) -> DigestState {
    let expected = declared.strip_prefix("sha256:").unwrap_or(declared);
    let actual = sha256_hex(bytes);

    if actual.eq_ignore_ascii_case(expected) {
        return DigestState::Matched;
    }

    DigestState::Mismatched
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root(cache_dir: &str) -> PluginCacheRoot {
        PluginCacheRoot::from_env_lookup(|key| (key == "CUTVER_PLUGIN_CACHE_DIR").then(|| cache_dir.to_string()))
            .unwrap()
    }

    fn config_with(source: Option<&str>, hash: Option<&str>) -> Config {
        let mut config = Config::default();
        config.plugins.insert(
            PluginName::new("demo").unwrap(),
            PluginConfig {
                runtime: RuntimeKind::Wasm,
                source: source.map(str::to_string),
                hash: hash.map(str::to_string),
                command: None,
                capabilities: Vec::new(),
                events: Vec::new(),
                manifest_match: Vec::new(),
                permissions: Default::default(),
                timeout_seconds: None,
            },
        );
        config
    }

    #[test]
    fn missing_artifact_reports_uninstalled_and_unknown() {
        let dir = tempfile::tempdir().unwrap();
        let statuses = list_statuses(
            &config_with(Some("demo.wasm"), Some(&sha256_hex(b"expected"))),
            &root(dir.path().to_str().unwrap()),
        );

        assert_eq!(statuses.len(), 1);
        assert!(!statuses[0].installed);
        assert_eq!(statuses[0].digest, DigestState::Unknown);
    }

    #[test]
    fn matching_bytes_report_installed_and_matched() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("demo.wasm"), b"expected").unwrap();

        let statuses = list_statuses(
            &config_with(Some("demo.wasm"), Some(&sha256_hex(b"expected"))),
            &root(dir.path().to_str().unwrap()),
        );

        assert!(statuses[0].installed);
        assert_eq!(statuses[0].digest, DigestState::Matched);
    }

    #[test]
    fn digest_state_distinguishes_matched_mismatched_and_unknown() {
        let hash = sha256_hex(b"payload");
        assert_eq!(digest_state(b"payload", &hash), DigestState::Matched);
        assert_eq!(
            digest_state(b"payload", &format!("sha256:{hash}")),
            DigestState::Matched
        );
        assert_eq!(digest_state(b"tampered", &hash), DigestState::Mismatched);
        assert_eq!(digest_state(b"payload", ""), DigestState::Mismatched);
    }
}
