//! Runner for `cutver plugin restore`.

use std::path::PathBuf;
use std::time::Duration;

use crate::cli::args::PluginRestoreArgs;
use crate::cli::runner::dispatch::{load_config, print_error};
use crate::cli::style::Theme;
use crate::plugin::cache::PluginCacheRoot;
use crate::plugin::manage::{PluginRestoreOutcome, RestoreOptions, restore_plugins};
use crate::plugin::registry::{RegistryIndexUrl, UreqTransport};

const RESTORE_TIMEOUT: Duration = Duration::from_secs(120);

/// Runs `cutver plugin restore` in a plugin-enabled build.
pub fn run_restore(config_override: Option<PathBuf>, args: &PluginRestoreArgs) -> i32 {
    let config = match load_config(config_override) {
        Ok(config) => config,
        Err(code) => return code,
    };

    if config.plugins.is_empty() {
        println!("No plugins declared in configuration.");
        return 0;
    }

    let index = match args.registry.as_deref() {
        Some(base) => match RegistryIndexUrl::parse(base) {
            Ok(index) => index,
            Err(err) => {
                print_error(err);
                return 1;
            }
        },
        None => RegistryIndexUrl::default_index(),
    };

    let cache = match PluginCacheRoot::resolve() {
        Ok(cache) => cache,
        Err(err) => {
            print_error(err);
            return 1;
        }
    };

    let transport = UreqTransport::new(RESTORE_TIMEOUT);
    let options = RestoreOptions { dry_run: args.dry_run };

    let report = match restore_plugins(&config, &options, &index, &cache, &transport) {
        Ok(report) => report,
        Err(err) => {
            print_error(err);
            return 1;
        }
    };

    let theme = Theme::stdout();
    print_report(&report, args.dry_run, &theme);

    if report.failed_count() > 0 { 1 } else { 0 }
}

fn print_report(report: &crate::plugin::manage::RestoreReport, dry_run: bool, theme: &Theme) {
    let mode_str = if dry_run { " (dry-run)" } else { "" };
    let restored = report.restored_count();

    if restored > 0 {
        let label = if dry_run { "Would restore" } else { "Restored" };
        println!(
            "{}{} {} plugin artifact{}:",
            theme.success("✔ "),
            label,
            restored,
            if restored == 1 { "" } else { "s" }
        );
    } else if report.failed_count() == 0 {
        println!(
            "{}All declared plugins are already present in cache{}",
            theme.success("✔ "),
            mode_str
        );
    }

    for outcome in &report.outcomes {
        match outcome {
            PluginRestoreOutcome::Restored { name, artifact, .. } => {
                println!("  {} {}: {} (verified)", theme.success("✔"), name, artifact);
            }
            PluginRestoreOutcome::Planned {
                name, artifact, url, ..
            } => {
                println!("  {} {}: {} from {}", theme.accent("•"), name, artifact, url);
            }
            PluginRestoreOutcome::AlreadyPresent { name, source } => {
                println!("  {} {}: {} (cached)", theme.muted("•"), name, source);
            }
            PluginRestoreOutcome::Failed { name, reason } => {
                println!("  {} {}: {}", theme.error("✖"), name, reason);
            }
            PluginRestoreOutcome::Skipped { .. } => {}
        }
    }
}
