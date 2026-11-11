//! Runner for `cutver plugin update`.

use std::path::PathBuf;
use std::time::Duration;

use crate::cli::args::PluginUpdateArgs;
use crate::cli::runner::dispatch::{load_config_with_path, print_error};
use crate::cli::style::Theme;
use crate::plugin::cache::PluginCacheRoot;
use crate::plugin::manage::{PluginUpdateOutcome, UpdateOptions, update_plugins};
use crate::plugin::registry::{RegistryIndexUrl, UreqTransport};
use crate::plugin::types::PluginName;

const UPDATE_TIMEOUT: Duration = Duration::from_secs(120);

/// Runs `cutver plugin update` in a plugin-enabled build.
pub fn run_update(config_override: Option<PathBuf>, args: &PluginUpdateArgs) -> i32 {
    let (config, config_path) = match load_config_with_path(config_override) {
        Ok(pair) => pair,
        Err(code) => return code,
    };

    if config.plugins.is_empty() {
        println!("No plugins declared in configuration.");
        return 0;
    }

    let target_name = match &args.name {
        Some(name_str) => match PluginName::new(name_str) {
            Ok(name) => {
                if !config.plugins.contains_key(&name) {
                    eprintln!("Error: plugin '{name}' is not declared in cutver.toml.");
                    return 1;
                }
                Some(name)
            }
            Err(err) => {
                print_error(err);
                return 1;
            }
        },
        None => None,
    };

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

    let transport = UreqTransport::new(UPDATE_TIMEOUT);
    let options = UpdateOptions {
        name: target_name,
        version: args.version.clone(),
        dry_run: args.dry_run,
    };

    let report = match update_plugins(&config_path, &options, &index, &cache, &transport) {
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

fn print_report(report: &crate::plugin::manage::UpdateReport, dry_run: bool, theme: &Theme) {
    let mode_str = if dry_run { " (dry-run)" } else { "" };
    let updated = report.updated_count();

    if updated > 0 {
        let label = if dry_run { "Would update" } else { "Updated" };
        println!(
            "{}{} {} plugin{}:",
            theme.success("✔ "),
            label,
            updated,
            if updated == 1 { "" } else { "s" }
        );
    } else if report.failed_count() == 0 {
        println!("{}All plugins are up to date{}", theme.success("✔ "), mode_str);
    }

    for outcome in &report.outcomes {
        match outcome {
            PluginUpdateOutcome::Updated {
                name,
                from_version,
                to_version,
                artifact,
            } => {
                println!(
                    "  {} {}: v{} -> v{} ({})",
                    theme.success("✔"),
                    name,
                    from_version,
                    to_version,
                    artifact
                );
            }
            PluginUpdateOutcome::UpToDate { name, version } => {
                println!("  {} {}: v{} (latest)", theme.muted("•"), name, version);
            }
            PluginUpdateOutcome::Failed { name, reason } => {
                println!("  {} {}: {}", theme.error("✖"), name, reason);
            }
            PluginUpdateOutcome::Skipped { .. } => {}
        }
    }
}
