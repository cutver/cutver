//! Phase 1 of `cutver plugin add`: pure in-memory planning (Pillar I.2).
//!
//! Reads `cutver.toml`, rejects an existing `[plugins.<name>]` declaration
//! *before* any network call (decision D14), resolves and validates the registry
//! release, enforces the capability flag requirements (decision D16) and renders
//! the surgical `toml_edit` document and its human-readable diff. No file is
//! written and the artifact is not downloaded here.

use std::fs;
use std::path::{Path, PathBuf};

use semver::Version;
use toml_edit::{Array, DocumentMut, Item, Table, Value};

use crate::plugin::cache::PluginCacheRoot;
use crate::plugin::error::{InstallError, PluginError};
use crate::plugin::registry::{RegistryIndexUrl, Transport, resolve_entry_with, select_release};
use crate::plugin::types::{Capability, PluginName};

/// User-supplied `cutver plugin add` options.
#[derive(Debug, Clone, Default)]
pub struct AddOptions {
    /// Version requirement passed to release selection.
    pub version: Option<String>,
    /// Events the plugin handles; required by `lifecycle.v1`.
    pub events: Vec<String>,
    /// Manifest globs the plugin matches; required by `manifest.v1`.
    pub manifest_match: Vec<String>,
    /// Replace an existing `[plugins.<name>]` declaration.
    pub force: bool,
}

/// Fully computed, in-memory installation plan (phase 1 output).
#[derive(Debug, Clone)]
pub struct InstallPlan {
    pub name: PluginName,
    pub config_path: PathBuf,
    pub artifact_path: PathBuf,
    pub artifact_url: String,
    pub version: Version,
    pub digest: String,
    pub index: String,
    pub new_text: String,
    pub diff: String,
}

/// Phase 1: resolve the release and compute the exact config edit without mutating anything.
pub fn plan_add(
    config_path: &Path,
    name: &PluginName,
    options: &AddOptions,
    index: &RegistryIndexUrl,
    cache: &PluginCacheRoot,
    transport: &dyn Transport,
) -> Result<InstallPlan, PluginError> {
    let (label, mut doc) = read_document(config_path)?;
    let existing = existing_item(&doc, name);
    guard_existing(name, &label, existing, options.force)?;

    let entry = resolve_entry_with(index, name, transport)?;
    let release = select_release(&entry, options.version.as_deref())?;
    require_flags(name, &entry.capabilities, options)?;

    let artifact_name = format!("{name}-{}.wasm", release.version);
    let table = build_table(&entry.capabilities, options, &artifact_name, release.sha256.as_str());
    let diff = render_diff(name, existing, &table);
    insert_plugins_table(&mut doc, name, table, &label)?;

    Ok(InstallPlan {
        name: name.clone(),
        config_path: config_path.to_path_buf(),
        artifact_path: cache.artifact_path(&artifact_name),
        artifact_url: release.url.as_str().to_string(),
        version: release.version.clone(),
        digest: release.sha256.as_str().to_string(),
        index: index.as_str().to_string(),
        new_text: doc.to_string(),
        diff,
    })
}

/// Reads and parses the config document, returning its label and typed document.
fn read_document(config_path: &Path) -> Result<(String, DocumentMut), PluginError> {
    let label = config_path.display().to_string();
    let text = fs::read_to_string(config_path).map_err(|reason| PluginError::PluginConfigRead {
        path: label.clone(),
        reason: reason.to_string(),
    })?;
    let doc = text
        .parse::<DocumentMut>()
        .map_err(|reason| PluginError::PluginConfigEdit {
            path: label.clone(),
            reason: reason.to_string(),
        })?;

    Ok((label, doc))
}

/// The existing `[plugins.<name>]` item, when present.
fn existing_item<'a>(doc: &'a DocumentMut, name: &PluginName) -> Option<&'a Item> {
    doc.get("plugins")
        .and_then(|item| item.as_table())
        .and_then(|table| table.get(name.as_str()))
}

/// Fails closed on an existing declaration unless `--force` was passed (decision D14).
fn guard_existing(name: &PluginName, label: &str, existing: Option<&Item>, force: bool) -> Result<(), InstallError> {
    if existing.is_some() && !force {
        return Err(InstallError::PluginAlreadyConfigured {
            name: name.clone(),
            config_path: label.to_string(),
        });
    }

    Ok(())
}

/// Enforces the `--event` / `--manifest-match` requirements of the declared capabilities.
fn require_flags(name: &PluginName, capabilities: &[Capability], options: &AddOptions) -> Result<(), InstallError> {
    capabilities
        .iter()
        .try_for_each(|capability| require_flag(name, capability, options))
}

/// The flag requirement for one capability; capabilities that need neither are no-ops.
fn require_flag(name: &PluginName, capability: &Capability, options: &AddOptions) -> Result<(), InstallError> {
    match capability {
        Capability::LifecycleV1 if options.events.is_empty() => {
            Err(flag_required(name, *capability, "events", "--event"))
        }
        Capability::ManifestV1 if options.manifest_match.is_empty() => {
            Err(flag_required(name, *capability, "manifest_match", "--manifest-match"))
        }
        Capability::LifecycleV1 | Capability::ManifestV1 | Capability::ChangelogV1 | Capability::VersioningV1 => Ok(()),
    }
}

/// Builds the requirement error naming the exact missing key and its capability.
fn flag_required(name: &PluginName, capability: Capability, key: &'static str, flag: &'static str) -> InstallError {
    InstallError::PluginFlagRequired {
        name: name.clone(),
        capability,
        key,
        flag,
    }
}

/// Builds the `[plugins.<name>]` table for the resolved release.
fn build_table(capabilities: &[Capability], options: &AddOptions, artifact: &str, digest: &str) -> Table {
    let mut table = Table::new();
    table.insert("runtime", toml_edit::value("wasm"));
    table.insert("source", toml_edit::value(artifact));
    table.insert("hash", toml_edit::value(digest));
    table.insert(
        "capabilities",
        string_array(capabilities.iter().map(Capability::as_str)),
    );

    if !options.events.is_empty() {
        table.insert("events", string_array(options.events.iter().map(String::as_str)));
    }
    if !options.manifest_match.is_empty() {
        table.insert(
            "manifest_match",
            string_array(options.manifest_match.iter().map(String::as_str)),
        );
    }

    table
}

/// A TOML array item built from string slices.
fn string_array<'a>(values: impl Iterator<Item = &'a str>) -> Item {
    let mut array = Array::new();
    for value in values {
        array.push(value);
    }

    Item::Value(Value::Array(array))
}

/// Renders the `-`/`+` diff of the replaced or inserted `[plugins.<name>]` region.
fn render_diff(name: &PluginName, existing: Option<&Item>, table: &Table) -> String {
    let mut out = String::new();
    for line in existing_rendered(name, existing).lines() {
        out.push_str("- ");
        out.push_str(line);
        out.push('\n');
    }
    for line in render_table(name, table).lines() {
        out.push_str("+ ");
        out.push_str(line);
        out.push('\n');
    }

    out
}

/// The old declaration text, including its header when it is a table.
fn existing_rendered(name: &PluginName, existing: Option<&Item>) -> String {
    match existing.and_then(Item::as_table) {
        Some(table) => render_table(name, table),
        None => existing.map(Item::to_string).unwrap_or_default(),
    }
}

/// Renders a plugin table with its `[plugins.<name>]` header.
fn render_table(name: &PluginName, table: &Table) -> String {
    format!("[plugins.{name}]\n{table}")
}

/// Inserts (or replaces) the `[plugins.<name>]` table in the document.
fn insert_plugins_table(
    doc: &mut DocumentMut,
    name: &PluginName,
    table: Table,
    label: &str,
) -> Result<(), PluginError> {
    let plugins = plugins_parent(doc);
    let Some(plugins) = plugins.as_table_mut() else {
        return Err(PluginError::PluginConfigEdit {
            path: label.to_string(),
            reason: "'plugins' must be a table to add a plugin declaration".to_string(),
        });
    };
    plugins.insert(name.as_str(), Item::Table(table));

    Ok(())
}

/// The `plugins` parent for an insertion (decision D23).
///
/// An absent key is created as an *implicit* table so toml_edit renders no
/// `[plugins]` header of its own. An existing key is returned untouched, so a
/// user's hand-written explicit table is never flipped to implicit.
fn plugins_parent(doc: &mut DocumentMut) -> &mut Item {
    doc.as_table_mut().entry("plugins").or_insert_with(|| {
        let mut table = Table::new();
        table.set_implicit(true);
        Item::Table(table)
    })
}
