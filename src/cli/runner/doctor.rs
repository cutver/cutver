use crate::bump;
use crate::cli::style::Theme;
use crate::config;

use super::dispatch::print_error;

pub fn run_doctor(config: &config::Config, check_changelog: bool) -> i32 {
    let mut has_drift = false;
    let err_theme = Theme::stderr();
    let out_theme = Theme::stdout();

    match bump::doctor(config) {
        Ok(drifts) => {
            if !drifts.is_empty() {
                eprint!("{}", crate::cli::doctor::render_version_drift(&drifts, &err_theme));
                has_drift = true;
            }
        }
        Err(e) => {
            print_error(e);
            return 1;
        }
    }

    if check_changelog {
        match bump::doctor_changelog(config) {
            Ok(cl_drift) => {
                if !cl_drift.is_empty() {
                    eprint!("{}", crate::cli::doctor::render_changelog_drift(&cl_drift, &err_theme));
                    has_drift = true;
                }
            }
            Err(e) => {
                let theme = Theme::stderr();
                eprintln!("{} {e}", theme.error("Error checking changelog:"));
                return 1;
            }
        }
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
