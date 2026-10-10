//! Surgical removal of a `[plugins.<name>]` declaration.

use std::fs;
use std::path::{Path, PathBuf};

use toml_edit::{DocumentMut, Item, Table};

use crate::plugin::cache::{DeletePath, PluginCacheRoot, read_path};
use crate::plugin::error::PluginError;
use crate::plugin::types::PluginName;

/// What the plan decided to do with the plugin's cached artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactDecision {
    /// Delete the cached artifact at `path`; no remaining entry uses `source`.
    Delete { path: PathBuf, source: String },
    /// Keep the artifact because another declared plugin still declares `source`.
    KeepShared {
        path: PathBuf,
        source: String,
        shared_with: Vec<PluginName>,
    },
    /// Keep the artifact because `--keep-artifact` asked us to.
    KeepRequested { path: PathBuf, source: String },
    /// Keep `source` because it is the user's own existing local file.
    KeepLocal { path: PathBuf, source: String },
    /// Keep the artifact because its path escapes the cache root.
    KeepOutsideRoot { path: PathBuf, source: String },
    /// The runtime has no cached artifact (or the declaration has no source).
    NotApplicable,
}

/// Fully computed, in-memory removal plan (Pillar I.2 phase 1).
#[derive(Debug, Clone)]
pub struct RemovePlan {
    pub name: PluginName,
    pub config_path: PathBuf,
    pub new_text: String,
    pub artifact: ArtifactDecision,
}

/// What actually happened to the artifact during apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactOutcome {
    Deleted(PathBuf),
    NotPresent(PathBuf),
    KeptShared {
        path: PathBuf,
        shared_with: Vec<PluginName>,
    },
    KeptRequested(PathBuf),
    KeptLocal(PathBuf),
    KeptOutsideRoot(PathBuf),
    NotApplicable,
}

/// Result of applying a [`RemovePlan`].
#[derive(Debug, Clone)]
pub struct RemoveOutcome {
    pub name: PluginName,
    pub config_path: PathBuf,
    pub artifact: ArtifactOutcome,
}

/// Phase 1: parse, locate the table, decide the artifact and render the exact new TOML.
pub fn plan_remove(
    config_path: &Path,
    name: &PluginName,
    keep_artifact: bool,
    cache: &PluginCacheRoot,
) -> Result<RemovePlan, PluginError> {
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

    let Some(plugins) = doc.get_mut("plugins").and_then(|item| item.as_table_mut()) else {
        return Err(not_configured(name, &label));
    };

    let Some(removed) = plugins.remove(name.as_str()) else {
        return Err(not_configured(name, &label));
    };

    let source = table_str(&removed, "source");
    let runtime = table_str(&removed, "runtime");
    let artifact = decide_artifact(plugins, source, runtime.as_deref(), keep_artifact, cache);

    Ok(RemovePlan {
        name: name.clone(),
        config_path: config_path.to_path_buf(),
        new_text: doc.to_string(),
        artifact,
    })
}

/// Phase 2: write the config first, then act on the artifact.
///
/// If artifact deletion fails the config is already truthful, so the error names
/// the path without pretending the config write did not happen (Pillar III.4).
pub fn execute_remove(plan: &RemovePlan) -> Result<RemoveOutcome, PluginError> {
    crate::atomic::write_atomic(&plan.config_path, plan.new_text.as_bytes()).map_err(|reason| {
        PluginError::PluginConfigWrite {
            path: plan.config_path.display().to_string(),
            reason: reason.to_string(),
        }
    })?;

    Ok(RemoveOutcome {
        name: plan.name.clone(),
        config_path: plan.config_path.clone(),
        artifact: apply_artifact(&plan.artifact)?,
    })
}

fn not_configured(name: &PluginName, config_path: &str) -> PluginError {
    PluginError::PluginNotConfigured {
        name: name.clone(),
        config_path: config_path.to_string(),
    }
}

fn table_str(item: &Item, key: &str) -> Option<String> {
    item.as_table()
        .and_then(|table| table.get(key))
        .and_then(|value| value.as_str())
        .map(str::to_string)
}

fn decide_artifact(
    remaining: &Table,
    source: Option<String>,
    runtime: Option<&str>,
    keep_artifact: bool,
    cache: &PluginCacheRoot,
) -> ArtifactDecision {
    let Some(source) = source else {
        return ArtifactDecision::NotApplicable;
    };

    if runtime != Some("wasm") {
        return ArtifactDecision::NotApplicable;
    }

    let path = read_target(cache, &source);
    if keep_artifact {
        return ArtifactDecision::KeepRequested { path, source };
    }

    let shared_with = sharing_sources(remaining, &source);
    if !shared_with.is_empty() {
        return ArtifactDecision::KeepShared {
            path,
            source,
            shared_with,
        };
    }

    match cache.delete_path(&source) {
        DeletePath::Deletable(path) => ArtifactDecision::Delete { path, source },
        DeletePath::LocalFile(path) => ArtifactDecision::KeepLocal { path, source },
        DeletePath::OutsideRoot(path) => ArtifactDecision::KeepOutsideRoot { path, source },
    }
}

/// The path `list` and a load would READ for `source`; the cache is already
/// resolved, so the injected resolver cannot fail.
fn read_target(cache: &PluginCacheRoot, source: &str) -> PathBuf {
    read_path(source, || Ok(cache.clone())).unwrap_or_else(|_| cache.artifact_path(source))
}

fn sharing_sources(remaining: &Table, source: &str) -> Vec<PluginName> {
    remaining
        .iter()
        .filter(|(_, item)| {
            item.as_table()
                .and_then(|table| table.get("source"))
                .and_then(|value| value.as_str())
                == Some(source)
        })
        .filter_map(|(key, _)| PluginName::new(key).ok())
        .collect()
}

fn apply_artifact(decision: &ArtifactDecision) -> Result<ArtifactOutcome, PluginError> {
    match decision {
        ArtifactDecision::Delete { path, .. } => delete_artifact(path),
        ArtifactDecision::KeepShared { path, shared_with, .. } => Ok(ArtifactOutcome::KeptShared {
            path: path.clone(),
            shared_with: shared_with.clone(),
        }),
        ArtifactDecision::KeepRequested { path, .. } => Ok(ArtifactOutcome::KeptRequested(path.clone())),
        ArtifactDecision::KeepLocal { path, .. } => Ok(ArtifactOutcome::KeptLocal(path.clone())),
        ArtifactDecision::KeepOutsideRoot { path, .. } => Ok(ArtifactOutcome::KeptOutsideRoot(path.clone())),
        ArtifactDecision::NotApplicable => Ok(ArtifactOutcome::NotApplicable),
    }
}

fn delete_artifact(path: &Path) -> Result<ArtifactOutcome, PluginError> {
    if !path.exists() {
        return Ok(ArtifactOutcome::NotPresent(path.to_path_buf()));
    }

    fs::remove_file(path).map_err(|source| PluginError::PluginArtifactDelete {
        path: path.display().to_string(),
        source,
    })?;

    Ok(ArtifactOutcome::Deleted(path.to_path_buf()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root(cache_dir: &Path) -> PluginCacheRoot {
        PluginCacheRoot::from_env_lookup(|key| {
            (key == "CUTVER_PLUGIN_CACHE_DIR").then(|| cache_dir.to_string_lossy().to_string())
        })
        .unwrap()
    }

    fn name() -> PluginName {
        PluginName::new("demo").unwrap()
    }

    fn table(source: &str) -> Table {
        format!("[other]\nruntime = \"wasm\"\nsource = \"{source}\"\n")
            .parse::<DocumentMut>()
            .unwrap()
            .as_table()
            .clone()
    }

    #[test]
    fn artifact_is_deleted_when_no_remaining_entry_shares_the_source() {
        let dir = tempfile::tempdir().unwrap();
        let cache = root(dir.path());
        let remaining = table("other.wasm");

        let decision = decide_artifact(&remaining, Some("demo.wasm".to_string()), Some("wasm"), false, &cache);

        assert!(matches!(decision, ArtifactDecision::Delete { .. }));
    }

    #[test]
    fn artifact_is_kept_when_another_entry_shares_the_source() {
        let dir = tempfile::tempdir().unwrap();
        let cache = root(dir.path());
        let remaining = table("shared.wasm");

        let decision = decide_artifact(&remaining, Some("shared.wasm".to_string()), Some("wasm"), false, &cache);

        match decision {
            ArtifactDecision::KeepShared { shared_with, .. } => {
                assert_eq!(shared_with, vec![PluginName::new("other").unwrap()]);
            }
            other => panic!("expected KeepShared, got {other:?}"),
        }
    }

    #[test]
    fn keep_artifact_flag_skips_deletion_even_without_sharing() {
        let dir = tempfile::tempdir().unwrap();
        let cache = root(dir.path());

        let decision = decide_artifact(&Table::new(), Some("demo.wasm".to_string()), Some("wasm"), true, &cache);

        assert!(matches!(decision, ArtifactDecision::KeepRequested { .. }));
    }

    #[test]
    fn absolute_source_is_kept_because_it_escapes_the_cache_root() {
        let dir = tempfile::tempdir().unwrap();
        let cache = root(dir.path());

        let decision = decide_artifact(
            &Table::new(),
            Some("/etc/passwd".to_string()),
            Some("wasm"),
            false,
            &cache,
        );

        assert!(matches!(decision, ArtifactDecision::KeepOutsideRoot { .. }));
    }

    #[test]
    fn local_source_file_is_kept_because_it_is_the_users_file() {
        let dir = tempfile::tempdir().unwrap();
        let cache = root(dir.path());
        let local = dir.path().join("rel.wasm");
        fs::write(&local, b"wasm").unwrap();

        let decision = decide_artifact(
            &Table::new(),
            Some(local.to_string_lossy().to_string()),
            Some("wasm"),
            false,
            &cache,
        );

        assert!(matches!(decision, ArtifactDecision::KeepLocal { .. }));
    }

    #[test]
    fn plan_delete_removes_the_table_and_keeps_unrelated_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let cache = root(dir.path());
        let config = dir.path().join("cutver.toml");
        fs::write(
            &config,
            "# top comment\n[version]\ncurrent_source = \"Cargo.toml\"\n\n[[manifest]]\npath = \"Cargo.toml\"\nkind = \"cargo-package\"\n\n[plugins.demo]\nruntime = \"wasm\"\nsource = \"demo.wasm\"\nhash = \"abc\"\n",
        )
        .unwrap();

        let plan = plan_remove(&config, &name(), false, &cache).unwrap();
        let text = &plan.new_text;

        assert!(!text.contains("[plugins.demo]"));
        assert!(text.contains("# top comment"));
        assert!(text.contains("current_source = \"Cargo.toml\""));
        assert!(matches!(plan.artifact, ArtifactDecision::Delete { .. }));
    }
}
