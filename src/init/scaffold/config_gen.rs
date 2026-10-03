//! TOML configuration generation and updating logic for `cutver init`.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use super::runner::InitError;
use crate::init::discovery::{DiscoveredKind, DiscoveredManifest, DiscoveryResult};

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
    }

    out.push_str("\n[git]\n");
    out.push_str("tag_prefix = \"v\"\n");
    out.push_str("require_clean_tree = true\n");
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

    out
}

/// Appends newly discovered manifests to an existing `cutver.toml` configuration.
pub fn update_existing_config(
    config_path: &Path,
    discovered: &[DiscoveredManifest],
) -> Result<Vec<DiscoveredManifest>, InitError> {
    let text = fs::read_to_string(config_path).map_err(|e| InitError::ReadConfig {
        path: config_path.display().to_string(),
        source: e,
    })?;

    let doc: toml_edit::DocumentMut = text.parse().map_err(|e| InitError::ParseConfig {
        path: config_path.display().to_string(),
        source: e,
    })?;

    let mut existing_paths = HashSet::new();
    if let Some(manifests) = doc.get("manifest").and_then(|m| m.as_array_of_tables()) {
        for m in manifests.iter() {
            if let Some(p) = m.get("path").and_then(|p| p.as_str()) {
                existing_paths.insert(p.to_string());
            }
        }
    }
    if let Some(arr) = doc.get("manifest").and_then(|m| m.as_array()) {
        for v in arr.iter() {
            if let Some(tbl) = v.as_inline_table()
                && let Some(p) = tbl.get("path").and_then(|p| p.as_str())
            {
                existing_paths.insert(p.to_string());
            }
        }
    }

    let new_manifests: Vec<DiscoveredManifest> = discovered
        .iter()
        .filter(|m| !existing_paths.contains(&m.path))
        .cloned()
        .collect();

    if new_manifests.is_empty() {
        return Ok(Vec::new());
    }

    let mut updated_text = text;
    if !updated_text.ends_with('\n') {
        updated_text.push('\n');
    }
    for m in &new_manifests {
        updated_text.push('\n');
        updated_text.push_str(&format_manifest_entry(m));
    }

    let _ = updated_text
        .parse::<toml_edit::DocumentMut>()
        .map_err(|e| InitError::ParseConfig {
            path: config_path.display().to_string(),
            source: e,
        })?;

    crate::atomic::write_atomic(config_path, updated_text.as_bytes()).map_err(|e| InitError::WriteFile {
        path: config_path.display().to_string(),
        source: e,
    })?;

    Ok(new_manifests)
}
