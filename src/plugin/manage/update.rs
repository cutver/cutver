//! Domain logic for `cutver plugin update`.
//!
//! Updates declared plugin versions in `cutver.toml` to the latest compatible release
//! from the registry, downloads and verifies the new artifact, and surgically modifies
//! only `source` and `hash` while preserving all comments, indentation, `.permissions`,
//! and `.config` subtables (Pillar I.2 & Pillar II.1).

use std::fs;
use std::path::Path;

use toml_edit::DocumentMut;

use crate::plugin::cache::PluginCacheRoot;
use crate::plugin::driver::wasm::verify_sha256;
use crate::plugin::error::PluginError;
use crate::plugin::registry::{RegistryIndexUrl, Transport, resolve_entry_with, select_release};
use crate::plugin::types::PluginName;

/// Maximum artifact download size (32 MiB).
const MAX_ARTIFACT_BYTES: u64 = 32 * 1024 * 1024;

/// Configuration options driving `cutver plugin update`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UpdateOptions {
    /// Target plugin name. If `None`, updates all declared plugins.
    pub name: Option<PluginName>,
    /// Optional SemVer requirement (e.g. `"^0.2"`).
    pub version: Option<String>,
    /// In simulation mode, evaluate updates without writing to disk or cache.
    pub dry_run: bool,
}

/// Outcome of attempting to update one plugin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginUpdateOutcome {
    /// Plugin was updated to a newer release.
    Updated {
        name: PluginName,
        from_version: String,
        to_version: String,
        artifact: String,
    },
    /// Plugin is already at the latest matching release.
    UpToDate { name: PluginName, version: String },
    /// The plugin is not a WASM plugin or has no declared source.
    Skipped { name: PluginName, reason: String },
    /// Updating the plugin failed.
    Failed { name: PluginName, reason: String },
}

/// Aggregate report of all updated plugins.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UpdateReport {
    pub outcomes: Vec<PluginUpdateOutcome>,
}

impl UpdateReport {
    pub fn updated_count(&self) -> usize {
        self.outcomes
            .iter()
            .filter(|o| matches!(o, PluginUpdateOutcome::Updated { .. }))
            .count()
    }

    pub fn failed_count(&self) -> usize {
        self.outcomes
            .iter()
            .filter(|o| matches!(o, PluginUpdateOutcome::Failed { .. }))
            .count()
    }
}

/// Updates declared plugins in `config_path` according to `options`.
pub fn update_plugins(
    config_path: &Path,
    options: &UpdateOptions,
    index: &RegistryIndexUrl,
    cache: &PluginCacheRoot,
    transport: &dyn Transport,
) -> Result<UpdateReport, PluginError> {
    let label = config_path.display().to_string();
    let text = fs::read_to_string(config_path).map_err(|reason| PluginError::PluginConfigRead {
        path: label.clone(),
        reason: reason.to_string(),
    })?;

    let mut doc = text
        .parse::<DocumentMut>()
        .map_err(|reason| PluginError::PluginConfigEdit {
            path: label.clone(),
            reason: reason.to_string(),
        })?;

    let plugins_table = match doc.get("plugins").and_then(|i| i.as_table()) {
        Some(t) => t.clone(),
        None => {
            return Ok(UpdateReport::default());
        }
    };

    let mut plugin_names: Vec<String> = plugins_table
        .iter()
        .filter_map(|(k, item)| if item.is_table() { Some(k.to_string()) } else { None })
        .collect();
    plugin_names.sort();

    let mut outcomes = Vec::new();
    let mut modified = false;

    for name_str in plugin_names {
        let plugin_name = match PluginName::new(&name_str) {
            Ok(n) => n,
            Err(_) => continue,
        };

        if let Some(target) = &options.name
            && target != &plugin_name
        {
            continue;
        }

        let outcome = update_one(&plugin_name, &mut doc, options, index, cache, transport, &mut modified)?;
        outcomes.push(outcome);
    }

    if modified && !options.dry_run {
        crate::atomic::write_atomic(config_path, doc.to_string().as_bytes()).map_err(|err| {
            PluginError::PluginConfigWrite {
                path: label,
                reason: err.to_string(),
            }
        })?;
    }

    Ok(UpdateReport { outcomes })
}

fn update_one(
    name: &PluginName,
    doc: &mut DocumentMut,
    options: &UpdateOptions,
    index: &RegistryIndexUrl,
    cache: &PluginCacheRoot,
    transport: &dyn Transport,
    modified: &mut bool,
) -> Result<PluginUpdateOutcome, PluginError> {
    let plugin_table = match doc
        .get_mut("plugins")
        .and_then(|p| p.as_table_mut())
        .and_then(|t| t.get_mut(name.as_str()))
        .and_then(|i| i.as_table_mut())
    {
        Some(t) => t,
        None => {
            return Ok(PluginUpdateOutcome::Skipped {
                name: name.clone(),
                reason: "not a table".to_string(),
            });
        }
    };

    let runtime = plugin_table.get("runtime").and_then(|v| v.as_str()).unwrap_or("wasm");
    if runtime != "wasm" {
        return Ok(PluginUpdateOutcome::Skipped {
            name: name.clone(),
            reason: format!("runtime is {runtime}"),
        });
    }

    let current_source = plugin_table
        .get("source")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    let current_version = extract_version_from_source(&current_source);

    let entry = match resolve_entry_with(index, name, transport) {
        Ok(e) => e,
        Err(err) => {
            return Ok(PluginUpdateOutcome::Failed {
                name: name.clone(),
                reason: format!("failed to resolve registry entry: {err}"),
            });
        }
    };

    let target_release = match select_release(&entry, options.version.as_deref()) {
        Ok(r) => r,
        Err(err) => {
            return Ok(PluginUpdateOutcome::Failed {
                name: name.clone(),
                reason: format!("no matching release: {err}"),
            });
        }
    };

    let new_version = target_release.version.to_string();
    let new_artifact = format!("{name}-{new_version}.wasm");
    let new_hash = target_release.sha256.as_str();

    // Check if current version and hash already match the resolved release
    let current_hash = plugin_table.get("hash").and_then(|v| v.as_str()).unwrap_or_default();
    let clean_current_hash = current_hash.trim().trim_start_matches("sha256:");

    if current_version == new_version && clean_current_hash.eq_ignore_ascii_case(new_hash) {
        return Ok(PluginUpdateOutcome::UpToDate {
            name: name.clone(),
            version: current_version,
        });
    }

    if options.dry_run {
        return Ok(PluginUpdateOutcome::Updated {
            name: name.clone(),
            from_version: current_version,
            to_version: new_version,
            artifact: new_artifact,
        });
    }

    // Download, verify, and write new artifact to cache
    let bytes = match transport.fetch(target_release.url.as_str(), MAX_ARTIFACT_BYTES) {
        Ok(b) => b,
        Err(err) => {
            return Ok(PluginUpdateOutcome::Failed {
                name: name.clone(),
                reason: format!("failed to download artifact: {err}"),
            });
        }
    };

    if let Err(err) = verify_sha256(name, &bytes, Some(new_hash)) {
        return Ok(PluginUpdateOutcome::Failed {
            name: name.clone(),
            reason: format!("checksum verification failed: {err}"),
        });
    }

    let target_path = cache.artifact_path(&new_artifact);
    if let Err(err) = write_artifact_atomic(&target_path, &bytes) {
        return Ok(PluginUpdateOutcome::Failed {
            name: name.clone(),
            reason: format!("failed to write artifact to cache: {err}"),
        });
    }

    // Surgically update only source and hash in cutver.toml
    plugin_table.insert("source", toml_edit::value(&new_artifact));
    plugin_table.insert("hash", toml_edit::value(new_hash));
    *modified = true;

    Ok(PluginUpdateOutcome::Updated {
        name: name.clone(),
        from_version: current_version,
        to_version: new_version,
        artifact: new_artifact,
    })
}

fn extract_version_from_source(source: &str) -> String {
    let stem = Path::new(source).file_stem().and_then(|s| s.to_str()).unwrap_or(source);

    if let Some((_name, ver)) = stem.rsplit_once('-') {
        ver.to_string()
    } else {
        "unknown".to_string()
    }
}

fn write_artifact_atomic(target: &Path, bytes: &[u8]) -> Result<(), std::io::Error> {
    let parent = target.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;

    let temp_name = format!(
        ".update-tmp-{}-{}",
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

    const PAYLOAD_V1_HASH: &str = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
    const PAYLOAD_V2: &[u8] = b"wasm-bytes-v2";
    const PAYLOAD_V2_HASH: &str = "d810405b0a70899ab80f4303e141373584f8974e37a59596879010cbcb974eec";

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

    fn sample_multi_entry(name: &str) -> String {
        format!(
            "name = \"{name}\"\n\
             description = \"Sample plugin\"\n\
             author = \"cutver\"\n\
             tier = \"official\"\n\
             repository = \"https://github.com/cutver/plugins\"\n\
             capabilities = [\"lifecycle.v1\"]\n\n\
             [[release]]\n\
             version = \"0.1.0\"\n\
             url = \"https://example.com/demo-0.1.0.wasm\"\n\
             sha256 = \"{PAYLOAD_V1_HASH}\"\n\n\
             [[release]]\n\
             version = \"0.2.0\"\n\
             url = \"https://example.com/demo-0.2.0.wasm\"\n\
             sha256 = \"{PAYLOAD_V2_HASH}\"\n"
        )
    }

    #[test]
    fn updates_plugin_in_place_preserving_permissions_and_config() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("cutver.toml");
        let cache_dir = temp.path().join("cache");
        let cache = PluginCacheRoot::from_env_lookup(|_| Some(cache_dir.to_str().unwrap().to_string())).unwrap();

        let initial_toml = format!(
            "# Header comment\n\
             [version]\n\
             current_source = \"package.json\"\n\n\
             [plugins.demo]\n\
             runtime = \"wasm\"\n\
             source = \"demo-0.1.0.wasm\"\n\
             hash = \"{PAYLOAD_V1_HASH}\"\n\
             capabilities = [\"lifecycle.v1\"]\n\
             events = [\"on_pre_bump\"]\n\n\
             [plugins.demo.permissions]\n\
             network = [\"api.github.com\"]\n\n\
             [plugins.demo.config]\n\
             repository = \"cutver/sandbox\"\n"
        );
        fs::write(&config_path, &initial_toml).unwrap();

        let transport = MockTransport::new()
            .with(
                "https://raw.githubusercontent.com/cutver/registry/main/plugins/demo.toml",
                sample_multi_entry("demo"),
            )
            .with("https://example.com/demo-0.2.0.wasm", PAYLOAD_V2);

        let index = RegistryIndexUrl::default_index();
        let report = update_plugins(&config_path, &UpdateOptions::default(), &index, &cache, &transport).unwrap();

        assert_eq!(report.updated_count(), 1);
        assert_eq!(report.failed_count(), 0);

        let updated_toml = fs::read_to_string(&config_path).unwrap();
        assert!(updated_toml.contains("# Header comment"));
        assert!(updated_toml.contains("source = \"demo-0.2.0.wasm\""));
        assert!(updated_toml.contains(&format!("hash = \"{PAYLOAD_V2_HASH}\"")));
        // Preserves events, permissions, and config!
        assert!(updated_toml.contains("events = [\"on_pre_bump\"]"));
        assert!(updated_toml.contains("[plugins.demo.permissions]"));
        assert!(updated_toml.contains("network = [\"api.github.com\"]"));
        assert!(updated_toml.contains("[plugins.demo.config]"));
        assert!(updated_toml.contains("repository = \"cutver/sandbox\""));

        assert!(cache.artifact_path("demo-0.2.0.wasm").is_file());
    }
}
