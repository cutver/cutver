//! The plugin half of the diagnosis: what `cutver.toml` declares, why the manager could not be
//! built, and the dashboard rows for both (#268 S5, #300 S1).
//!
//! These live here rather than inside `doctor.rs` because that file sits under the 400-line ratchet in
//! `tests/source_line_budget.rs`, whose table "can shrink but never grow". The parent re-exports every
//! path that was public there, so no caller of `crate::cli::doctor` had to move.

use std::path::Path;
use std::sync::Arc;

use serde::Serialize;

use super::render_grid_row;
use crate::config::Config;
use crate::plugin::PluginManager;
use crate::plugin::PluginName;
use crate::plugin::driver::DeclaredEnvGrants;
use crate::plugin::types::FilesystemGrant;

/// One declared plugin: its name, its capability set, the filesystem grants resolution recorded for
/// it (#268 S5) and its declared environment grants (#300 S1).
#[derive(Debug, Serialize)]
pub struct PluginJson {
    pub name: String,
    pub capabilities: Vec<String>,
    pub grants: Vec<GrantJson>,
    pub env_grants: Vec<EnvGrantJson>,
}

/// One `permissions.env` grant of a declared plugin: the key `cutver.toml` declared and whether the
/// host can produce it.
///
/// The value never travels — a diagnosis prints declarations, not secrets — and `set` is asked of the
/// same [`DeclaredEnvGrants`] authority the driver resolves through, so the row and the invocation
/// that follows it cannot disagree (#300 S1).
#[derive(Debug, Serialize)]
pub struct EnvGrantJson {
    /// The key `cutver.toml` declared, exactly as written.
    pub key: String,
    /// Whether the environment this command runs in can produce that key.
    pub set: bool,
}

/// One `permissions.filesystem` grant of a declared plugin, carrying the three values resolution kept
/// (#268 S3, S6) and where the grant landed.
#[derive(Debug, Serialize)]
pub struct GrantJson {
    /// The spelling `cutver.toml` declared.
    pub declared: String,
    /// The guest path the plugin mounts.
    pub resolved: String,
    /// The canonical host directory behind `resolved`; `null` when it does not exist yet.
    pub canonical: Option<String>,
    /// `inside`, `outside` or `unresolved`, derived from `canonical` against the workspace root.
    pub location: &'static str,
}

impl GrantJson {
    /// Projects one resolved grant and derives where its host directory really sits.
    ///
    /// `outside` is unreachable from a loaded `cutver.toml` — `config::discovery` refuses a grant that
    /// escapes the workspace with `ConfigError::GrantOutsideWorkspace` — but it is *derived* here
    /// rather than assumed, so the report states the comparison it made. A directory that does not
    /// exist yet has no canonical path, and that is a third answer, not "outside" (#268 S6).
    pub fn from_grant(grant: &FilesystemGrant, root_dir: &Path) -> Self {
        let location = match &grant.canonical {
            Some(canonical) if Path::new(canonical).starts_with(root_dir) => "inside",
            Some(_) => "outside",
            None => "unresolved",
        };
        Self {
            declared: grant.declared.clone(),
            resolved: grant.resolved.clone(),
            canonical: grant.canonical.clone(),
            location,
        }
    }

    /// The human sentence for [`Self::location`], so the row states the comparison instead of a word.
    fn location_note(&self) -> &'static str {
        match self.location {
            "inside" => "inside the workspace",
            "outside" => "OUTSIDE the workspace",
            _ => "unresolved (the directory does not exist yet)",
        }
    }
}

/// The plugin half of the diagnosis as data: what `cutver.toml` declares and, when it could not be
/// built, why (D1).
pub fn plugin_section(config: &Config) -> (Vec<PluginJson>, Option<String>) {
    (plugin_records(config), build_manager(config).err())
}

/// One record per declared plugin, in name order so two runs of the same document agree.
fn plugin_records(config: &Config) -> Vec<PluginJson> {
    let mut names: Vec<&PluginName> = config.plugins.keys().collect();
    names.sort();
    names
        .into_iter()
        .map(|name| {
            let plugin = &config.plugins[name];
            let env_grants = DeclaredEnvGrants::from_env(plugin.permissions.env.clone());
            PluginJson {
                name: name.to_string(),
                capabilities: plugin.capabilities.iter().map(|c| c.as_str().to_string()).collect(),
                grants: plugin
                    .permissions
                    .filesystem_grants
                    .iter()
                    .map(|grant| GrantJson::from_grant(grant, &config.root_dir))
                    .collect(),
                env_grants: plugin
                    .permissions
                    .env
                    .iter()
                    .map(|key| EnvGrantJson {
                        key: key.clone(),
                        set: env_grants.is_set(key),
                    })
                    .collect(),
            }
        })
        .collect()
}

/// The plugin manager `cutver.toml` describes, or the typed reason it could not be built.
///
/// D1 (`odd/tasks/cutver-doctor-grants-and-swallowed-reads.md`): an unset `permissions.env` grant used
/// to fail this construction, and `.ok()` hid it while `?` would have made `doctor` fail on exactly
/// the host that needs a diagnosis. #300 resolved that tension at its root — construction no longer
/// resolves grants — so a grant is now a row of its own ([`EnvGrantJson`]) and this error carries the
/// failures that really are construction failures: an unreadable artifact, a digest mismatch, a
/// missing `command` or `source`.
pub(super) fn build_manager(config: &Config) -> Result<Arc<PluginManager>, String> {
    PluginManager::from_config(&config.plugins)
        .map(Arc::new)
        .map_err(|error| error.to_string())
}

/// Renders the plugin rows: what was declared, one row per grant, and the load failure when the
/// manager could not be built (#268 S5, D1).
pub(super) fn render_plugin_rows(
    out: &mut String,
    config: &Config,
    load_error: Option<&str>,
    theme: &crate::cli::style::Theme,
) {
    let plugins = plugin_records(config);
    let declared = plugins.len();
    let summary = match load_error {
        Some(error) => format!("{declared} declared, could not load: {error}"),
        None => {
            let sets: Vec<String> = plugins
                .iter()
                .map(|plugin| format!("{}: {}", plugin.name, plugin.capabilities.join(", ")))
                .collect();
            if sets.is_empty() {
                "0 declared".to_string()
            } else {
                format!("{declared} declared ({})", sets.join(", "))
            }
        }
    };
    render_grid_row(out, "Plugins", &summary, theme);
    for plugin in &plugins {
        for grant in &plugin.grants {
            render_grid_row(out, "Grant", &format_grant_row(plugin, grant), theme);
        }
        for grant in &plugin.env_grants {
            render_grid_row(out, "Grant", &format_env_grant_row(plugin, grant), theme);
        }
    }
}

/// One environment grant as a dashboard row: `plugin [capabilities]: env "KEY" -> set|unset (…)`.
///
/// The row states the resolution, not only the declaration, and it names the consequence of an unset
/// key: the plugin itself will refuse to run there (#300 S1, S3).
fn format_env_grant_row(plugin: &PluginJson, grant: &EnvGrantJson) -> String {
    let state = match grant.set {
        true => "set (granted at invocation)",
        false => "unset (invoking the plugin will fail)",
    };
    format!(
        "{} [{}]: env \"{}\" -> {state}",
        plugin.name,
        plugin.capabilities.join(", "),
        grant.key
    )
}

/// One grant as a dashboard row: `plugin [capabilities]: "declared" -> resolved -> canonical (where)`.
fn format_grant_row(plugin: &PluginJson, grant: &GrantJson) -> String {
    let host = match &grant.canonical {
        Some(canonical) => format!("{canonical} ({})", grant.location_note()),
        None => grant.location_note().to_string(),
    };
    format!(
        "{} [{}]: \"{}\" -> {} -> {host}",
        plugin.name,
        plugin.capabilities.join(", "),
        grant.declared,
        grant.resolved
    )
}
