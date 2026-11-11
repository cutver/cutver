//! Domain logic for `cutver plugin restore`.
//!
//! Hydrates missing plugin artifacts into `PluginCacheRoot` by inspecting
//! declared `[plugins.<name>]` tables in `cutver.toml` and fetching verified
//! artifacts from the registry index.
//!
//! **Invariant**: Never mutates `cutver.toml`.

use std::fs;
use std::path::Path;

use crate::config::Config;
use crate::plugin::cache::PluginCacheRoot;
use crate::plugin::driver::wasm::verify_sha256;
use crate::plugin::error::PluginError;
use crate::plugin::registry::{RegistryEntry, RegistryIndexUrl, RegistryRelease, Transport, resolve_entry_with};
use crate::plugin::types::{PluginConfig, PluginName, RuntimeKind};

/// Maximum artifact download size (32 MiB, matching `apply.rs`).
const MAX_ARTIFACT_BYTES: u64 = 32 * 1024 * 1024;

/// Configuration options driving `cutver plugin restore`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RestoreOptions {
    /// In simulation mode, resolve and verify without downloading to disk.
    pub dry_run: bool,
}

/// Outcome of restoring one plugin artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginRestoreOutcome {
    /// Artifact was already present in cache or local filesystem with a matching digest.
    AlreadyPresent { name: PluginName, source: String },
    /// Artifact was successfully downloaded, verified, and placed into cache.
    Restored {
        name: PluginName,
        artifact: String,
        digest: String,
    },
    /// In `--dry-run` mode, artifact would be downloaded.
    Planned {
        name: PluginName,
        artifact: String,
        url: String,
        digest: String,
    },
    /// The plugin is not a WASM plugin or has no declared source.
    Skipped { name: PluginName, reason: String },
    /// Artifact restoration failed.
    Failed { name: PluginName, reason: String },
}

/// Aggregate report returned by `restore_plugins`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RestoreReport {
    pub outcomes: Vec<PluginRestoreOutcome>,
}

impl RestoreReport {
    pub fn restored_count(&self) -> usize {
        self.outcomes
            .iter()
            .filter(|o| {
                matches!(
                    o,
                    PluginRestoreOutcome::Restored { .. } | PluginRestoreOutcome::Planned { .. }
                )
            })
            .count()
    }

    pub fn failed_count(&self) -> usize {
        self.outcomes
            .iter()
            .filter(|o| matches!(o, PluginRestoreOutcome::Failed { .. }))
            .count()
    }
}

/// Restores missing declared plugin artifacts from `config` into `cache`.
pub fn restore_plugins(
    config: &Config,
    options: &RestoreOptions,
    index: &RegistryIndexUrl,
    cache: &PluginCacheRoot,
    transport: &dyn Transport,
) -> Result<RestoreReport, PluginError> {
    let mut outcomes = Vec::new();

    // Sort plugin names for deterministic restoration order
    let mut plugin_names: Vec<&PluginName> = config.plugins.keys().collect();
    plugin_names.sort();

    for name in plugin_names {
        let plugin_cfg = &config.plugins[name];
        let outcome = restore_one(name, plugin_cfg, options, index, cache, transport)?;
        outcomes.push(outcome);
    }

    Ok(RestoreReport { outcomes })
}

fn restore_one(
    name: &PluginName,
    plugin: &PluginConfig,
    options: &RestoreOptions,
    index: &RegistryIndexUrl,
    cache: &PluginCacheRoot,
    transport: &dyn Transport,
) -> Result<PluginRestoreOutcome, PluginError> {
    if plugin.runtime != RuntimeKind::Wasm {
        return Ok(PluginRestoreOutcome::Skipped {
            name: name.clone(),
            reason: format!("runtime is {}", plugin.runtime.as_str()),
        });
    }

    let source = match &plugin.source {
        Some(s) if !s.trim().is_empty() => s.trim(),
        _ => {
            return Ok(PluginRestoreOutcome::Skipped {
                name: name.clone(),
                reason: "no source declared".to_string(),
            });
        }
    };

    // Check if source exists locally or in cache
    let is_local_file = Path::new(source).is_file();
    let cache_path = cache.artifact_path(source);

    if is_local_file {
        return Ok(PluginRestoreOutcome::AlreadyPresent {
            name: name.clone(),
            source: source.to_string(),
        });
    }

    if cache_path.is_file()
        && let Ok(bytes) = fs::read(&cache_path)
    {
        let matched = match &plugin.hash {
            Some(expected) => verify_sha256(name, &bytes, Some(expected)).is_ok(),
            None => true,
        };
        if matched {
            return Ok(PluginRestoreOutcome::AlreadyPresent {
                name: name.clone(),
                source: source.to_string(),
            });
        }
    }

    // Artifact is missing or mismatched; resolve from registry
    let entry = match resolve_entry_with(index, name, transport) {
        Ok(e) => e,
        Err(err) => {
            return Ok(PluginRestoreOutcome::Failed {
                name: name.clone(),
                reason: format!("failed to resolve registry entry: {err}"),
            });
        }
    };

    let release = match match_release(&entry, source, plugin.hash.as_deref()) {
        Some(r) => r,
        None => {
            return Ok(PluginRestoreOutcome::Failed {
                name: name.clone(),
                reason: format!("no matching release found in registry for source '{source}'"),
            });
        }
    };

    let artifact_name = format!("{name}-{}.wasm", release.version);

    if options.dry_run {
        return Ok(PluginRestoreOutcome::Planned {
            name: name.clone(),
            artifact: artifact_name,
            url: release.url.as_str().to_string(),
            digest: release.sha256.as_str().to_string(),
        });
    }

    // Download, verify, and write atomically to cache
    let bytes = match transport.fetch(release.url.as_str(), MAX_ARTIFACT_BYTES) {
        Ok(b) => b,
        Err(err) => {
            return Ok(PluginRestoreOutcome::Failed {
                name: name.clone(),
                reason: format!("failed to download artifact: {err}"),
            });
        }
    };

    if let Err(err) = verify_sha256(name, &bytes, Some(release.sha256.as_str())) {
        return Ok(PluginRestoreOutcome::Failed {
            name: name.clone(),
            reason: format!("downloaded artifact failed checksum verification: {err}"),
        });
    }

    let target_path = cache.artifact_path(&artifact_name);
    if let Err(err) = write_artifact_atomic(&target_path, &bytes) {
        return Ok(PluginRestoreOutcome::Failed {
            name: name.clone(),
            reason: format!("failed to write artifact to cache: {err}"),
        });
    }

    Ok(PluginRestoreOutcome::Restored {
        name: name.clone(),
        artifact: artifact_name,
        digest: release.sha256.as_str().to_string(),
    })
}

/// Matches a release in `entry` either by declared SHA-256 hash or by version parsed from filename.
fn match_release<'a>(entry: &'a RegistryEntry, source: &str, hash: Option<&str>) -> Option<&'a RegistryRelease> {
    if let Some(expected_hash) = hash {
        let clean_expected = expected_hash.trim().trim_start_matches("sha256:");
        if let Some(found) = entry
            .releases
            .iter()
            .find(|r| r.sha256.as_str().eq_ignore_ascii_case(clean_expected))
        {
            return Some(found);
        }
    }

    // Fallback: extract version from source filename like "github-publish-0.1.2.wasm"
    let file_stem = Path::new(source).file_stem()?.to_str()?;
    if let Some((_name_part, version_str)) = file_stem.rsplit_once('-')
        && let Ok(ver) = version_str.parse::<semver::Version>()
    {
        return entry.releases.iter().find(|r| r.version == ver);
    }

    // Final fallback: use highest stable release
    entry.releases.iter().rfind(|r| r.version.pre.is_empty())
}

fn write_artifact_atomic(target: &Path, bytes: &[u8]) -> Result<(), std::io::Error> {
    let parent = target.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;

    let temp_name = format!(
        ".restore-tmp-{}-{}",
        std::process::id(),
        target.file_name().and_then(|n| n.to_str()).unwrap_or("plugin.wasm")
    );
    let temp_path = parent.join(temp_name);

    fs::write(&temp_path, bytes)?;

    if let Err(err) = fs::rename(&temp_path, target) {
        let _ = fs::remove_file(&temp_path);
        return Err(err);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::error::TransportError;
    use std::collections::HashMap;
    use std::sync::Mutex;
    use tempfile::TempDir;

    const PAYLOAD: &[u8] = b"hello";
    const PAYLOAD_HASH: &str = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";

    struct MockTransport {
        responses: HashMap<String, Vec<u8>>,
        calls: Mutex<Vec<String>>,
    }

    impl MockTransport {
        fn new() -> Self {
            Self {
                responses: HashMap::new(),
                calls: Mutex::new(Vec::new()),
            }
        }

        fn with(mut self, url: impl Into<String>, body: impl Into<Vec<u8>>) -> Self {
            self.responses.insert(url.into(), body.into());
            self
        }
    }

    impl Transport for MockTransport {
        fn fetch(&self, url: &str, _cap_bytes: u64) -> Result<Vec<u8>, TransportError> {
            self.calls.lock().unwrap().push(url.to_string());
            self.responses
                .get(url)
                .cloned()
                .ok_or_else(|| TransportError::TransportHttpStatus {
                    url: url.to_string(),
                    status: 404,
                })
        }
    }

    fn sample_entry(name: &str, version: &str, hash: &str, asset_url: &str) -> String {
        format!(
            "name = \"{name}\"\n\
             description = \"Sample plugin\"\n\
             author = \"cutver\"\n\
             tier = \"official\"\n\
             repository = \"https://github.com/cutver/plugins\"\n\
             capabilities = [\"lifecycle.v1\"]\n\n\
             [[release]]\n\
             version = \"{version}\"\n\
             url = \"{asset_url}\"\n\
             sha256 = \"{hash}\"\n"
        )
    }

    #[test]
    fn restores_missing_artifact_into_cache() {
        let temp = TempDir::new().unwrap();
        let cache_dir = temp.path().join("cache");
        let cache = PluginCacheRoot::from_env_lookup(|_| Some(cache_dir.to_str().unwrap().to_string())).unwrap();

        let name = PluginName::new("demo").unwrap();
        let mut config = Config::default();
        config.plugins.insert(
            name.clone(),
            PluginConfig {
                runtime: RuntimeKind::Wasm,
                source: Some("demo-0.1.0.wasm".to_string()),
                hash: Some(PAYLOAD_HASH.to_string()),
                command: None,
                capabilities: vec![],
                events: vec![],
                manifest_match: vec![],
                permissions: Default::default(),
                timeout_seconds: None,
                config: None,
            },
        );

        let entry_toml = sample_entry("demo", "0.1.0", PAYLOAD_HASH, "https://example.com/demo-0.1.0.wasm");
        let transport = MockTransport::new()
            .with(
                "https://raw.githubusercontent.com/cutver/registry/main/plugins/demo.toml",
                entry_toml,
            )
            .with("https://example.com/demo-0.1.0.wasm", PAYLOAD);

        let index = RegistryIndexUrl::default_index();
        let report = restore_plugins(&config, &RestoreOptions::default(), &index, &cache, &transport).unwrap();

        assert_eq!(report.restored_count(), 1);
        assert_eq!(report.failed_count(), 0);
        assert!(cache.artifact_path("demo-0.1.0.wasm").is_file());

        // Running restore again reports already present with zero new downloads
        let report2 = restore_plugins(&config, &RestoreOptions::default(), &index, &cache, &transport).unwrap();
        assert_eq!(report2.restored_count(), 0);
        assert!(matches!(
            report2.outcomes[0],
            PluginRestoreOutcome::AlreadyPresent { .. }
        ));
    }

    #[test]
    fn dry_run_plans_without_writing_to_disk() {
        let temp = TempDir::new().unwrap();
        let cache_dir = temp.path().join("cache");
        let cache = PluginCacheRoot::from_env_lookup(|_| Some(cache_dir.to_str().unwrap().to_string())).unwrap();

        let name = PluginName::new("demo").unwrap();
        let mut config = Config::default();
        config.plugins.insert(
            name.clone(),
            PluginConfig {
                runtime: RuntimeKind::Wasm,
                source: Some("demo-0.1.0.wasm".to_string()),
                hash: Some(PAYLOAD_HASH.to_string()),
                command: None,
                capabilities: vec![],
                events: vec![],
                manifest_match: vec![],
                permissions: Default::default(),
                timeout_seconds: None,
                config: None,
            },
        );

        let entry_toml = sample_entry("demo", "0.1.0", PAYLOAD_HASH, "https://example.com/demo-0.1.0.wasm");
        let transport = MockTransport::new().with(
            "https://raw.githubusercontent.com/cutver/registry/main/plugins/demo.toml",
            entry_toml,
        );

        let index = RegistryIndexUrl::default_index();
        let report = restore_plugins(&config, &RestoreOptions { dry_run: true }, &index, &cache, &transport).unwrap();

        assert_eq!(report.restored_count(), 1);
        assert!(matches!(report.outcomes[0], PluginRestoreOutcome::Planned { .. }));
        assert!(!cache.artifact_path("demo-0.1.0.wasm").is_file());
    }
}
