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

mod plugins;

pub use plugins::{EnvGrantJson, GrantJson, PluginJson, plugin_section};

use plugins::{build_manager, render_plugin_rows};

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
