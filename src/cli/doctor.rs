use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use serde::Serialize;

use crate::bump::{ChangelogDrift, Drift};
use crate::cli::path::relativize_path;
use crate::config::Config;
use crate::manifest;
use crate::plugin::PluginManager;
use crate::plugin::PluginName;
use crate::plugin::types::FilesystemGrant;

/// The machine form of a diagnosis: what was checked, what it found, and nothing rendered.
///
/// `doctor` used to report drift only as coloured text on stderr, which a CI job had to scrape. This
/// object is the same run as data, and it keeps the promise of `src/cli/json.rs`: `schemaVersion`
/// first.
#[derive(Debug, Serialize)]
pub struct DoctorReport {
    /// No check found drift. A skipped check is not a failure.
    pub ok: bool,
    /// One entry per declared check, including the ones that did not run.
    pub checks: Vec<DoctorCheck>,
    /// Empty when every manifest agrees, never absent.
    pub version_drift: Vec<VersionDriftJson>,
    /// `null` when `--check-changelog` was not passed; the `changelog` check then says `skipped`.
    pub changelog_drift: Option<ChangelogDriftJson>,
    /// The deprecations this `cutver.toml` still carries, which the runner fills: `CONTRACT.md` names
    /// `doctor` as a migration-warning surface, so what the human reads is what a runner can read.
    pub warnings: Vec<String>,
    /// One entry per declared plugin in name order, carrying its capabilities and every resolved
    /// `permissions.filesystem` grant (#268 S5). Empty when none is declared, never absent.
    pub plugins: Vec<PluginJson>,
    /// Why the plugin manager could not be built, else `null` (D1): the failure is reported, never
    /// discarded and never propagated, so `doctor` keeps reporting on a host that cannot build them.
    pub plugin_load_error: Option<String>,
}

impl DoctorReport {
    /// Builds the report from what the run observed.
    pub fn new(version_drift: Vec<VersionDriftJson>, changelog_drift: Option<ChangelogDriftJson>) -> Self {
        let changelog_drifted = changelog_drift.as_ref().is_some_and(|drift| !drift.is_empty());
        let ok = version_drift.is_empty() && !changelog_drifted;
        let checks = vec![
            DoctorCheck {
                name: "versions",
                status: if version_drift.is_empty() { "ok" } else { "drift" },
            },
            DoctorCheck {
                name: "changelog",
                status: match &changelog_drift {
                    None => "skipped",
                    Some(_) if changelog_drifted => "drift",
                    Some(_) => "ok",
                },
            },
        ];
        Self {
            ok,
            checks,
            version_drift,
            changelog_drift,
            warnings: Vec::new(),
            plugins: Vec::new(),
            plugin_load_error: None,
        }
    }
}

/// One declared check and what it concluded: `ok`, `drift`, or `skipped` when it did not run.
#[derive(Debug, Serialize)]
pub struct DoctorCheck {
    pub name: &'static str,
    pub status: &'static str,
}

/// A manifest whose version disagrees with the source of truth, as data instead of a sentence.
#[derive(Debug, Serialize)]
pub struct VersionDriftJson {
    pub path: String,
    pub expected: String,
    pub actual: String,
}

impl VersionDriftJson {
    /// Projects a [`Drift`] with the path relativized exactly as the human renderer does.
    pub fn from_drift(drift: &Drift, root_dir: Option<&Path>) -> Self {
        Self {
            path: relativize_path(&drift.path, root_dir),
            expected: drift.expected.clone(),
            actual: drift.actual.clone(),
        }
    }
}

/// The changelog half of the diagnosis.
#[derive(Debug, Serialize)]
pub struct ChangelogDriftJson {
    pub missing_in_changelog: Vec<String>,
    pub orphan_sections: Vec<String>,
}

impl ChangelogDriftJson {
    /// Projects a [`ChangelogDrift`].
    pub fn from_drift(drift: &ChangelogDrift) -> Self {
        Self {
            missing_in_changelog: drift.missing_in_changelog.clone(),
            orphan_sections: drift.orphan_sections.clone(),
        }
    }

    /// True when neither half found anything.
    pub fn is_empty(&self) -> bool {
        self.missing_in_changelog.is_empty() && self.orphan_sections.is_empty()
    }
}

/// One declared plugin: its name, its capability set and the grants resolution recorded for it
/// (#268 S5).
#[derive(Debug, Serialize)]
pub struct PluginJson {
    pub name: String,
    pub capabilities: Vec<String>,
    pub grants: Vec<GrantJson>,
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
            PluginJson {
                name: name.to_string(),
                capabilities: plugin.capabilities.iter().map(|c| c.as_str().to_string()).collect(),
                grants: plugin
                    .permissions
                    .filesystem_grants
                    .iter()
                    .map(|grant| GrantJson::from_grant(grant, &config.root_dir))
                    .collect(),
            }
        })
        .collect()
}

/// The plugin manager `cutver.toml` describes, or the typed reason it could not be built.
///
/// D1 (`odd/tasks/cutver-doctor-grants-and-swallowed-reads.md`): `PluginManager::from_config` resolves
/// `permissions.env` while it constructs the drivers, so on a host with a declared grant unset (the
/// #300 scenario) it fails. Discarding that with `.ok()` hides a real condition; propagating it with
/// `?` would make `doctor` start failing on exactly the host that needs a diagnosis. So the error is
/// returned here as output — a dashboard row and the `plugin_load_error` field — and #300 stays out
/// of scope.
fn build_manager(config: &Config) -> Result<Arc<PluginManager>, String> {
    PluginManager::from_config(&config.plugins)
        .map(Arc::new)
        .map_err(|error| error.to_string())
}

/// Renders the report as the versioned object `doctor --json` prints.
pub fn render_report_json(report: &DoctorReport) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&crate::cli::json::Versioned::new(report))
}

/// Renders the complete status dashboard grid for valid doctor runs.
pub fn render_dashboard(config: &Config, check_changelog: bool, theme: &crate::cli::style::Theme) -> String {
    let mut out = String::new();
    let filename = "cutver.toml";
    let _ = writeln!(out, "{} {} is valid\n", theme.success_icon(), theme.bold(filename));

    let config_val = format!("valid ({filename})");
    // D1: one construction, and its failure is a row below instead of a `.ok()` that hides it.
    let manager = build_manager(config);
    let manifest_val = format_manifest_summary(config, theme, manager.as_ref().ok());
    let preflight_val = format_preflight_summary(config);

    render_grid_row(&mut out, "Configuration", &config_val, theme);
    render_grid_row(&mut out, "Manifests", &manifest_val, theme);
    render_grid_row(&mut out, "Preflight", &preflight_val, theme);
    render_plugin_rows(&mut out, config, manager.as_ref().err().map(String::as_str), theme);

    if check_changelog {
        render_grid_row(&mut out, "Changelog", "consistent with Git tags", theme);
    }

    out
}

/// Renders a single tabular row aligned to a fixed 16-character label width.
pub fn render_grid_row(out: &mut String, label: &str, value: &str, theme: &crate::cli::style::Theme) {
    let padded = pad_right(label, 16);
    let _ = writeln!(out, "  {}{value}", theme.muted(padded));
}

/// Formats the summary text for manifests row.
fn format_manifest_summary(
    config: &Config,
    theme: &crate::cli::style::Theme,
    plugin_manager: Option<&Arc<PluginManager>>,
) -> String {
    let total = config.manifest.len();
    let primary = config.primary_manifest();

    let (path_str, ver_str) = match primary {
        Some(m) => {
            let ver = read_manifest_version(&config.root_dir, m, plugin_manager).unwrap_or_else(|| "unknown".into());
            let rel_path = relativize_path(m.path.as_str(), Some(&config.root_dir));
            (rel_path, ver)
        }
        None => return "0 tracked".to_string(),
    };

    let styled_ver = theme.accent(&ver_str);
    if total <= 1 {
        let count_str = theme.accent(total.to_string());
        return format!("{count_str} tracked ({path_str} @ {styled_ver})");
    }

    let remaining = total - 1;
    let count_str = theme.accent(total.to_string());
    let more_str = theme.muted(format!("+{remaining} more"));
    format!("{count_str} tracked ({path_str} @ {styled_ver}, {more_str})")
}

/// Formats the summary text for preflight row.
fn format_preflight_summary(config: &Config) -> String {
    match config.preflight.as_slice() {
        [] => "0 steps defined".to_string(),
        [(name, step)] => format!("1 step defined ({name}: {})", step.command),
        _ => format!("{} steps defined", config.preflight.len()),
    }
}

/// Renders the plugin rows: what was declared, one row per grant, and the load failure when the
/// manager could not be built (#268 S5, D1).
fn render_plugin_rows(out: &mut String, config: &Config, load_error: Option<&str>, theme: &crate::cli::style::Theme) {
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
    }
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

/// Reads the version to display, or `None` when it cannot be read.
///
/// This is the display path only: `bump::doctor` reads every manifest through the same editors and
/// fails the command (exit `1`, with the error on stderr) when one cannot be read, so a `None` here
/// can only mean the run already reported that failure — the condition is tracked, not swallowed.
fn read_manifest_version(
    root_dir: &Path,
    manifest: &crate::config::Manifest,
    plugin_manager: Option<&Arc<PluginManager>>,
) -> Option<String> {
    let target = root_dir.join(&manifest.path);
    let content = fs::read_to_string(target).ok()?;
    let editor = manifest::editor_for_with_manager(manifest, plugin_manager).ok()?;
    let version = editor.read_version(&content).ok()?;
    Some(version.to_string())
}

fn pad_right(text: &str, width: usize) -> String {
    let count = width.saturating_sub(text.len());
    format!("{text}{}", " ".repeat(count))
}

#[cfg(test)]
mod report_tests {
    use super::*;

    fn drift() -> VersionDriftJson {
        VersionDriftJson {
            path: "b.json".into(),
            expected: "1.1.0".into(),
            actual: "1.0.0".into(),
        }
    }

    #[test]
    fn a_drifted_run_reports_the_path_and_marks_the_check_that_did_not_run_as_skipped() {
        let report = DoctorReport::new(vec![drift()], None);

        assert!(!report.ok);
        assert_eq!(report.checks.len(), 2);
        assert_eq!((report.checks[0].name, report.checks[0].status), ("versions", "drift"));
        assert_eq!(
            (report.checks[1].name, report.checks[1].status),
            ("changelog", "skipped"),
            "a check that did not run is not an empty result"
        );

        let json = render_report_json(&report).expect("the report serializes");
        assert!(
            json.trim_start().starts_with("{\n  \"schemaVersion\": 1,"),
            "the schema version is the first key: {json}"
        );
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("the report is valid JSON");
        assert_eq!(parsed["ok"], false, "{json}");
        assert_eq!(parsed["version_drift"][0]["path"], "b.json", "{json}");
        assert!(parsed["changelog_drift"].is_null(), "{json}");
    }

    #[test]
    fn a_clean_run_is_ok_and_every_check_that_ran_says_ok() {
        let report = DoctorReport::new(
            Vec::new(),
            Some(ChangelogDriftJson {
                missing_in_changelog: Vec::new(),
                orphan_sections: Vec::new(),
            }),
        );

        assert!(report.ok);
        assert_eq!(report.checks[1].status, "ok");

        let json = render_report_json(&report).expect("the report serializes");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("the report is valid JSON");
        assert_eq!(parsed["ok"], true, "{json}");
        assert_eq!(
            parsed["version_drift"].as_array().map(Vec::len),
            Some(0),
            "an empty result is an empty array, never absent: {json}"
        );
    }

    #[test]
    fn a_changelog_drift_marks_both_halves() {
        let report = DoctorReport::new(
            Vec::new(),
            Some(ChangelogDriftJson {
                missing_in_changelog: vec!["v1.2.0".into()],
                orphan_sections: vec!["1.1.0".into()],
            }),
        );

        assert!(!report.ok);
        assert_eq!(report.checks[0].status, "ok");
        assert_eq!(report.checks[1].status, "drift");

        let json = render_report_json(&report).expect("the report serializes");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("the report is valid JSON");
        assert_eq!(parsed["changelog_drift"]["missing_in_changelog"][0], "v1.2.0", "{json}");
        assert_eq!(parsed["changelog_drift"]["orphan_sections"][0], "1.1.0", "{json}");
    }
}
