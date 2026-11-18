use crate::bump;
use crate::cli::doctor::{ChangelogDriftJson, DoctorReport, VersionDriftJson, render_report_json};
use crate::cli::style::Theme;
use crate::config;

use super::dispatch::print_error;

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
            print_error(e);
            return 1;
        }
    };

    let changelog_drift = if check_changelog {
        match bump::doctor_changelog(config) {
            Ok(drift) => Some(drift),
            Err(e) => {
                let theme = Theme::stderr();
                eprintln!("{} {e}", theme.error("Error checking changelog:"));
                return 1;
            }
        }
    } else {
        None
    };

    let has_drift = !version_drift.is_empty() || changelog_drift.as_ref().is_some_and(|drift| !drift.is_empty());

    if machine {
        let report = DoctorReport::new(
            version_drift
                .iter()
                .map(|drift| VersionDriftJson::from_drift(drift, Some(&config.root_dir)))
                .collect(),
            changelog_drift.as_ref().map(ChangelogDriftJson::from_drift),
        );
        match render_report_json(&report) {
            Ok(text) => println!("{text}"),
            Err(e) => {
                print_error(format!("failed to serialize the diagnosis to JSON: {e}"));
                return 1;
            }
        }
        return if has_drift { 2 } else { 0 };
    }

    if !version_drift.is_empty() {
        eprint!(
            "{}",
            crate::cli::doctor::render_version_drift(&version_drift, Some(&config.root_dir), &err_theme)
        );
    }

    if let Some(drift) = &changelog_drift
        && !drift.is_empty()
    {
        eprint!("{}", crate::cli::doctor::render_changelog_drift(drift, &err_theme));
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
