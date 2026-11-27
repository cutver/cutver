//! TOML configuration generation and updating logic for `cutver init`.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use super::runner::InitError;
use crate::init::discovery::{DiscoveredKind, DiscoveredManifest, DiscoveryResult};
use toml_edit::TableLike;

/// Formats a single discovered manifest entry as TOML table syntax.
pub fn format_manifest_entry(m: &DiscoveredManifest) -> String {
    let mut s = String::new();
    s.push_str("[[manifest]]\n");
    s.push_str(&format!("path = \"{}\"\n", m.path));
    match &m.kind {
        DiscoveredKind::CargoPackage => {
            s.push_str("kind = \"cargo-package\"\n");
        }
        DiscoveredKind::Json { field } => {
            s.push_str("kind = \"json\"\n");
            s.push_str(&format!("field = \"{field}\"\n"));
        }
        DiscoveredKind::Gradle {
            version_name_field,
            version_code_field,
        } => {
            s.push_str("kind = \"gradle\"\n");
            s.push_str(&format!("version_name_field = \"{version_name_field}\"\n"));
            s.push_str(&format!("version_code_field = \"{version_code_field}\"\n"));
        }
        DiscoveredKind::Pyproject => {
            s.push_str("kind = \"pyproject\"\n");
        }
    }
    s
}

/// Generates a fresh `cutver.toml` configuration content based on project discovery.
pub fn generate_fresh_config(discovery: &DiscoveryResult, no_template: bool) -> String {
    let mut out = String::new();
    out.push_str("# cutver.toml - release orchestration configuration\n");
    out.push_str("# For full documentation, see https://github.com/cutver/cutver\n");

    if discovery.manifests.is_empty() {
        out.push_str("\n# No manifests were automatically detected during init.\n");
        out.push_str("# Declare your project manifests below (the first is the source of truth):\n");
        out.push_str("# [[manifest]]\n");
        out.push_str("# path = \"Cargo.toml\"\n");
        out.push_str("# kind = \"cargo-package\"\n");
    } else {
        out.push_str("\n# The first declared manifest serves as the primary source of truth for versions\n");
        for m in &discovery.manifests {
            out.push('\n');
            out.push_str(&format_manifest_entry(m));
        }
    }

    let has_preflight = discovery.hints.has_rust || discovery.hints.has_node;
    if has_preflight {
        out.push_str("\n[preflight]\n");
        if discovery.hints.has_rust {
            out.push_str("check = \"cargo check --workspace\"\n");
        }
        if discovery.hints.has_node {
            out.push_str("test = \"npm test\"\n");
        }
        out.push_str("default_timeout = 300\n");
    }

    out.push_str("\n[changelog]\n");
    out.push_str("path = \"CHANGELOG.md\"\n");
    out.push_str("format = \"keep-a-changelog\"\n");
    if !no_template {
        out.push_str("mode = \"template\"\n");
        out.push_str("template_file = \".github/templates/cutver/RELEASE.md\"\n");
    } else {
        out.push_str("mode = \"conventional\"\n");
        // The built-in renderer omits first-time contributors unless asked. A scaffolded project
        // has no previous output to preserve, so it starts with the section on.
        out.push_str("# Surface the authors making their first contribution, at ### level.\n");
        out.push_str("new_contributors = true\n");
    }

    out.push_str("\n[git]\n");
    out.push_str("tag_prefix = \"v\"\n");
    out.push_str("require_clean_tree = true\n");
    // Left commented: it maintains a floating major tag (v1, v2) for consumers such as Docker
    // images and GitHub Actions, which is only correct once a project actually publishes those.
    out.push_str("# floating_major_tag = true\n");
    out.push_str("commit_message = \"chore(release): v{version} [skip ci]\"\n");
    out.push_str("require_branch = \"main\"\n");

    let has_hooks = discovery.hints.has_rust || discovery.hints.has_uv_lock;
    if has_hooks {
        out.push_str("\n[hooks]\n");
        if discovery.hints.has_rust && discovery.hints.has_uv_lock {
            out.push_str("post_bump = \"cargo check --workspace && uv lock\"\n");
        } else if discovery.hints.has_rust {
            out.push_str("post_bump = \"cargo check --workspace\"\n");
        } else if discovery.hints.has_uv_lock {
            out.push_str("post_bump = \"uv lock\"\n");
        }
    }

    out.push_str("\n[publish]\n");
    out.push_str("push = true\n");

    // A generated file (a version constant, a header, a badge block) is a sink: it is rendered from
    // a template instead of reverse-matched by a regex. Shown commented so the shape is discoverable
    // without generating anything the project did not ask for.
    out.push_str("\n# Generated from a template instead of edited in place:\n");
    out.push_str("# [[manifest]]\n");
    out.push_str("# path = \"src/version.ts\"\n");
    out.push_str("# kind = \"template\"\n");
    out.push_str("# template = \"src/version.ts.jinja\"  # optional; defaults to <path>.jinja\n");

    out
}

/// Appends newly discovered manifests to an existing `cutver.toml` configuration.
///
/// Answers with the manifests it appended and the primary source of truth the updated document
/// declares, so the run can report both without reading the document a second time (#404 S4).
pub fn update_existing_config(
    config_path: &Path,
    discovered: &[DiscoveredManifest],
) -> Result<(Vec<DiscoveredManifest>, Option<String>), InitError> {
    let text = fs::read_to_string(config_path).map_err(|e| InitError::ReadConfig {
        path: config_path.display().to_string(),
        source: e,
    })?;

    let doc: toml_edit::DocumentMut = text.parse().map_err(|e| InitError::ParseConfig {
        path: config_path.display().to_string(),
        source: e,
    })?;

    // `CONTRACT.md` names `init --update` as a migration-warning surface, and the document is
    // already parsed here, so the scan costs no extra read and cannot fail twice (#272).
    crate::warnings::deprecations(&crate::config::deprecations_in_document(&doc));

    let existing_paths: HashSet<String> = declared_manifests(&doc).into_iter().map(|(path, _)| path).collect();

    let new_manifests: Vec<DiscoveredManifest> = discovered
        .iter()
        .filter(|m| !existing_paths.contains(&m.path))
        .cloned()
        .collect();

    if new_manifests.is_empty() {
        return Ok((Vec::new(), primary_source(&doc)));
    }

    let mut updated_text = text;
    if !updated_text.ends_with('\n') {
        updated_text.push('\n');
    }
    for m in &new_manifests {
        updated_text.push('\n');
        updated_text.push_str(&format_manifest_entry(m));
    }

    // Re-parsed after appending, so the primary source of truth is the one the document it wrote
    // declares — including the case where the configuration declared no manifest at all.
    let updated = updated_text
        .parse::<toml_edit::DocumentMut>()
        .map_err(|e| InitError::ParseConfig {
            path: config_path.display().to_string(),
            source: e,
        })?;

    crate::atomic::write_atomic(config_path, updated_text.as_bytes()).map_err(|e| InitError::WriteFile {
        path: config_path.display().to_string(),
        source: e,
    })?;

    Ok((new_manifests, primary_source(&updated)))
}

/// Every declared manifest as a table, in declaration order, in both spellings the loader accepts: an
/// array of tables (`[[manifest]]`) and an array of inline tables (`manifest = [{ … }]`).
fn declared_entries(doc: &toml_edit::DocumentMut) -> Vec<&dyn TableLike> {
    let Some(item) = doc.get("manifest") else {
        return Vec::new();
    };
    let mut entries: Vec<&dyn TableLike> = Vec::new();
    if let Some(tables) = item.as_array_of_tables() {
        entries.extend(tables.iter().map(|table| table as &dyn TableLike));
    } else if let Some(array) = item.as_array() {
        entries.extend(
            array
                .iter()
                .filter_map(|value| value.as_inline_table())
                .map(|table| table as &dyn TableLike),
        );
    }
    entries
}

/// Every declared manifest's path and whether it is flagged `primary = true`, in declaration order.
fn declared_manifests(doc: &toml_edit::DocumentMut) -> Vec<(String, bool)> {
    declared_entries(doc)
        .into_iter()
        .filter_map(|entry| {
            let path = entry.get("path")?.as_str()?.to_string();
            let primary = entry.get("primary").and_then(|primary| primary.as_bool()) == Some(true);
            Some((path, primary))
        })
        .collect()
}

/// The one kind that is a version sink: `ManifestKind::Template` is the only variant
/// `#[serde(tag = "kind", rename_all = "kebab-case")]` spells `template` (`src/config/types.rs`), so
/// this string is the whole sink set.
const TEMPLATE_SINK_KIND: &str = "template";

/// The primary source of truth an existing document declares, read from the document itself.
///
/// `init --update` must not start failing because a configuration it was asked to append to does not
/// load, so this mirrors `deduce_current_source` (`src/config/validation.rs:13`) at the document
/// level, in its order: `[version] current_source` (or its deprecated alias `source`), then the
/// manifest flagged `primary = true`, then the first declared manifest that is not a template sink
/// (`src/config/validation.rs:33-39`) — a sink generates its target from the version instead of
/// holding one, so it can never supply it (#410 D2).
///
/// A configuration that does not load still gets an answer rather than a refusal: the loader's own
/// refusals — more than one `primary = true`, a `current_source` contradicting it, every entry a sink,
/// no entries at all — are exactly the cases where `--update` must still report something.
pub(super) fn primary_source(doc: &toml_edit::DocumentMut) -> Option<String> {
    let declared = doc
        .get("version")
        .and_then(|version| version.get("current_source").or_else(|| version.get("source")))
        .and_then(|item| item.as_str())
        .filter(|path| !path.is_empty());
    if let Some(path) = declared {
        return Some(path.to_string());
    }
    // Read here instead of in `declared_manifests`: that reader answers which paths `--update` must
    // not append, and a sink it skipped would be appended again on every run (#410 D3). One pass over
    // the entries, matched by position rather than by path: two entries may declare the same path, and
    // a sink declared first must not hide the manifest declared under the same path after it.
    let entries: Vec<(&str, bool, bool)> = declared_entries(doc)
        .into_iter()
        .filter_map(|entry| {
            let path = entry.get("path")?.as_str()?;
            let primary = entry.get("primary").and_then(|primary| primary.as_bool()) == Some(true);
            let sink = entry.get("kind").and_then(|kind| kind.as_str()) == Some(TEMPLATE_SINK_KIND);
            Some((path, primary, sink))
        })
        .collect();
    entries
        .iter()
        .find(|(_, primary, _)| *primary)
        .or_else(|| entries.iter().find(|(_, _, sink)| !sink))
        .map(|(path, _, _)| (*path).to_string())
}
