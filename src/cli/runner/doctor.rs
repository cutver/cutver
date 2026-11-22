//! The `doctor` run: one read of the workspace, two renderers.
//!
//! The drift blocks live here rather than in `src/cli/doctor.rs` because that file is under the
//! 400-line source ceiling of `CONTRACT.md:174` as a ratchet entry — it may shrink but not grow — and
//! this module is the command that decides when the blocks are printed. `src/cli/doctor.rs` keeps the
//! report types and the dashboard grid.

use std::fmt::Write as _;
use std::path::Path;

use crate::bump::{self, ChangelogDrift, Drift};
use crate::cli::doctor::{ChangelogDriftJson, DoctorReport, VersionDriftJson, plugin_section, render_report_json};
use crate::cli::path::relativize_path;
use crate::cli::style::Theme;
use crate::config;

use super::dispatch::{print_error, print_error_prefixed};
use crate::cli::machine::FailureCode;

/// Validates the workspace and reports drift.
///
/// Two readers, one run. The human path renders the dashboard, the drift blocks and the suggested
/// fixes on the same channels it always used. The machine path (`--json`) writes one object to
/// stdout and renders nothing, so a CI job reads drift as data instead of scraping prose. The exit
/// code is the outcome either way: `0` clean, `2` drift, `1` a check could not run.
pub fn run_doctor(config: &config::Config, check_changelog: bool, machine: bool) -> i32 {
    let err_theme = Theme::stderr();
    let out_theme = Theme::stdout();

    let version_drift = match bump::doctor(config) {
        Ok(drifts) => drifts,
        Err(e) => {
            print_error(FailureCode::Doctor, e);
            return 1;
        }
    };

    let changelog_drift = if check_changelog {
        match bump::doctor_changelog(config) {
            Ok(drift) => Some(drift),
            Err(e) => {
                print_error_prefixed(FailureCode::ChangelogCheck, "Error checking changelog:", e);
                return 1;
            }
        }
    } else {
        None
    };

    let has_drift = !version_drift.is_empty() || changelog_drift.as_ref().is_some_and(|drift| !drift.is_empty());

    if machine {
        let mut report = DoctorReport::new(
            version_drift
                .iter()
                .map(|drift| VersionDriftJson::from_drift(drift, Some(&config.root_dir)))
                .collect(),
            changelog_drift.as_ref().map(ChangelogDriftJson::from_drift),
        );
        report.warnings = config
            .deprecations
            .iter()
            .map(|deprecation| deprecation.message())
            .collect();
        // #268 S5, D1: what resolution recorded for every plugin, and why the manager could not be
        // built when that is what happened. Both are data here, on the same run as the drift.
        (report.plugins, report.plugin_load_error) = plugin_section(config);
        match render_report_json(&report) {
            Ok(text) => println!("{text}"),
            Err(e) => {
                print_error(
                    FailureCode::Doctor,
                    format!("failed to serialize the diagnosis to JSON: {e}"),
                );
                return 1;
            }
        }
        return if has_drift { 2 } else { 0 };
    }

    if !version_drift.is_empty() {
        eprint!(
            "{}",
            render_version_drift(&version_drift, Some(&config.root_dir), &err_theme)
        );
    }

    if let Some(drift) = &changelog_drift
        && !drift.is_empty()
    {
        eprint!("{}", render_changelog_drift(drift, &err_theme));
    }

    if has_drift {
        return 2;
    }

    print!(
        "{}",
        crate::cli::doctor::render_dashboard(config, check_changelog, &out_theme)
    );
    0
}

/// Renders version drift diagnostic block answering: what, where, and how to fix.
pub fn render_version_drift(drifts: &[Drift], root_dir: Option<&Path>, theme: &Theme) -> String {
    let mut out = String::new();
    let count = drifts.len();
    let count_styled = theme.accent(count.to_string());
    let _ = writeln!(
        out,
        "{} Version drift detected ({count_styled} manifest(s) out of sync):",
        theme.error_icon()
    );

    for Drift { path, expected, actual } in drifts {
        let norm_path = relativize_path(path, root_dir);
        let exp = theme.accent(expected);
        let act = theme.error(actual);
        let _ = writeln!(out, "  {} {norm_path}: expected {exp}, found {act}", theme.bullet());
    }

    let _ = writeln!(out, "\nSuggested fix:");
    let _ = writeln!(
        out,
        "  {} Align version numbers manually across all files.",
        theme.bullet()
    );
    let bump_cmd = theme.accent("cutver bump patch");
    let _ = writeln!(
        out,
        "  {} Or run '{bump_cmd}' to synchronize all manifests in one step.",
        theme.bullet()
    );
    out
}

/// Renders changelog drift diagnostic block.
pub fn render_changelog_drift(cl_drift: &ChangelogDrift, theme: &Theme) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "{} Changelog drift detected:", theme.error_icon());

    append_missing_changelog(&mut out, &cl_drift.missing_in_changelog, theme);
    append_orphan_sections(&mut out, &cl_drift.orphan_sections, theme);

    let _ = writeln!(out, "\nSuggested next steps:");
    let cmd = theme.accent("cutver changelog show <version>");
    let _ = writeln!(
        out,
        "  {} Add missing release sections to CHANGELOG.md or query with '{cmd}'.",
        theme.bullet()
    );
    let _ = writeln!(
        out,
        "  {} Check if orphan versions were tagged with a different prefix or not yet pushed.",
        theme.bullet()
    );
    out
}

fn append_missing_changelog(out: &mut String, tags: &[String], theme: &Theme) {
    if tags.is_empty() {
        return;
    }
    let _ = writeln!(out, "\n  Missing in changelog (Git tag exists):");
    for tag in tags {
        let _ = writeln!(out, "    {} {}", theme.bullet(), theme.accent(tag));
    }
}

fn append_orphan_sections(out: &mut String, sections: &[String], theme: &Theme) {
    if sections.is_empty() {
        return;
    }
    let _ = writeln!(out, "\n  Orphan changelog sections (no Git tag exists):");
    for sec in sections {
        let _ = writeln!(out, "    {} {}", theme.bullet(), theme.error(sec));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_version_drift() {
        let theme = Theme::new(false);
        let drifts = vec![Drift {
            path: "/path/to/repo/Cargo.toml".into(),
            expected: "1.2.0".into(),
            actual: "1.1.0".into(),
        }];
        let root = Path::new("/path/to/repo");
        let rendered = render_version_drift(&drifts, Some(root), &theme);
        assert!(rendered.contains("Version drift detected (1 manifest(s) out of sync):"));
        assert!(rendered.contains("Cargo.toml: expected 1.2.0, found 1.1.0"));
        assert!(rendered.contains("Suggested fix:"));
        assert!(rendered.contains("cutver bump patch"));
    }

    #[test]
    fn test_render_changelog_drift() {
        let theme = Theme::new(false);
        let cl_drift = ChangelogDrift {
            missing_in_changelog: vec!["v1.1.0".into()],
            orphan_sections: vec!["1.2.0".into()],
        };
        let rendered = render_changelog_drift(&cl_drift, &theme);
        assert!(rendered.contains("Changelog drift detected:"));
        assert!(rendered.contains("Missing in changelog (Git tag exists):"));
        assert!(rendered.contains("v1.1.0"));
        assert!(rendered.contains("Orphan changelog sections (no Git tag exists):"));
        assert!(rendered.contains("1.2.0"));
        assert!(rendered.contains("Suggested next steps:"));
    }
}
