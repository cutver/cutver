//! Runner for `cutver plugin list` and `cutver plugin remove`.
//!
//! The subcommand is declared in every build; a build without the `plugins`
//! feature fails closed with an actionable message instead of silently falling
//! through to the `cutver-plugin` external subcommand (decision D12).

use std::path::PathBuf;

#[cfg(feature = "plugins")]
use std::path::Path;

use crate::cli::args::PluginCommands;

#[cfg(feature = "plugins")]
use crate::cli::style::Theme;
#[cfg(feature = "plugins")]
use crate::plugin::PluginCacheRoot;
#[cfg(feature = "plugins")]
use crate::plugin::manage::{self, ArtifactOutcome, DigestState, PluginStatus, RemoveOutcome};
#[cfg(feature = "plugins")]
use crate::plugin::types::PluginName;

#[cfg(feature = "plugins")]
use super::dispatch::{load_config, load_config_with_path, print_error};

/// Dispatches `cutver plugin` in a plugin-enabled build.
#[cfg(feature = "plugins")]
pub fn run_plugin(config_override: Option<PathBuf>, command: PluginCommands) -> i32 {
    match command {
        PluginCommands::List { json } => run_list(config_override, json),
        PluginCommands::Remove { name, keep_artifact } => run_remove(config_override, &name, keep_artifact),
    }
}

/// Fails closed in a lean build: the `plugin` surface is not compiled in.
#[cfg(not(feature = "plugins"))]
pub fn run_plugin(_config_override: Option<PathBuf>, command: PluginCommands) -> i32 {
    let operation = match command {
        PluginCommands::List { .. } => "list",
        PluginCommands::Remove { .. } => "remove",
    };

    super::dispatch::print_error(crate::plugin::PluginError::PluginsFeatureDisabled {
        operation: operation.to_string(),
    });
    1
}

#[cfg(feature = "plugins")]
fn run_list(config_override: Option<PathBuf>, json: bool) -> i32 {
    let config = match load_config(config_override) {
        Ok(config) => config,
        Err(code) => return code,
    };
    let cache = match PluginCacheRoot::resolve() {
        Ok(cache) => cache,
        Err(error) => {
            print_error(error);
            return 1;
        }
    };

    let statuses = manage::list_statuses(&config, &cache);
    if json {
        return print_json(&statuses);
    }

    print_human(&statuses);
    0
}

#[cfg(feature = "plugins")]
fn run_remove(config_override: Option<PathBuf>, name: &str, keep_artifact: bool) -> i32 {
    let name = match PluginName::new(name) {
        Ok(name) => name,
        Err(error) => {
            print_error(error);
            return 1;
        }
    };

    let (_, config_path) = match load_config_with_path(config_override) {
        Ok(pair) => pair,
        Err(code) => return code,
    };
    let cache = match PluginCacheRoot::resolve() {
        Ok(cache) => cache,
        Err(error) => {
            print_error(error);
            return 1;
        }
    };

    let plan = match manage::plan_remove(&config_path, &name, keep_artifact, &cache) {
        Ok(plan) => plan,
        Err(error) => {
            print_error(error);
            return 1;
        }
    };

    match manage::execute_remove(&plan) {
        Ok(outcome) => {
            print_remove_outcome(&outcome);
            0
        }
        Err(error) => {
            print_error(error);
            1
        }
    }
}

#[cfg(feature = "plugins")]
fn print_json(statuses: &[PluginStatus]) -> i32 {
    let payload = serde_json::json!({ "plugins": statuses });
    match serde_json::to_string_pretty(&payload) {
        Ok(text) => {
            println!("{text}");
            0
        }
        Err(error) => {
            print_error(format!("failed to serialize plugin list to JSON: {error}"));
            1
        }
    }
}

#[cfg(feature = "plugins")]
fn print_human(statuses: &[PluginStatus]) {
    let theme = Theme::stdout();
    if statuses.is_empty() {
        println!("{}", theme.muted("No plugins are configured."));
        return;
    }

    println!("{}", theme.bold(format!("Plugins ({})", statuses.len())));
    for status in statuses {
        print_status(&theme, status);
    }
}

#[cfg(feature = "plugins")]
fn print_status(theme: &Theme, status: &PluginStatus) {
    println!();
    println!(
        "  {} {}",
        theme.accent(&status.name),
        theme.muted(format!("[{}]", status.runtime))
    );
    println!("    source:        {}", optional_value(theme, &status.source));
    println!("    declared_hash: {}", optional_value(theme, &status.declared_hash));
    println!("    installed:     {}", if status.installed { "yes" } else { "no" });
    println!("    digest:        {}", digest_label(theme, status.digest));
}

#[cfg(feature = "plugins")]
fn optional_value(theme: &Theme, value: &Option<String>) -> String {
    match value {
        Some(text) => text.clone(),
        None => theme.muted("(none)"),
    }
}

#[cfg(feature = "plugins")]
fn digest_label(theme: &Theme, state: DigestState) -> String {
    match state {
        DigestState::Matched => theme.success("matched"),
        DigestState::Mismatched => theme.error("mismatched"),
        DigestState::Unknown => theme.muted("unknown"),
    }
}

#[cfg(feature = "plugins")]
fn print_remove_outcome(outcome: &RemoveOutcome) {
    let theme = Theme::stdout();
    println!(
        "{} Removed plugin {} from {}",
        theme.success_icon(),
        theme.accent(&outcome.name),
        outcome.config_path.display()
    );

    print_artifact_outcome(&theme, &outcome.artifact);
}

/// Renders what happened to the artifact, one line per outcome.
#[cfg(feature = "plugins")]
fn print_artifact_outcome(theme: &Theme, outcome: &ArtifactOutcome) {
    match outcome {
        ArtifactOutcome::Deleted(path) => {
            println!("  {} Deleted cached artifact {}", theme.success_icon(), path.display());
        }
        ArtifactOutcome::NotPresent(path) => {
            println!(
                "  {} Cached artifact {} was already absent",
                theme.info_icon(),
                path.display()
            );
        }
        ArtifactOutcome::KeptShared { path, shared_with } => {
            let names = shared_with
                .iter()
                .map(PluginName::as_str)
                .collect::<Vec<_>>()
                .join(", ");
            print_kept(theme, "cached artifact", path, &format!("still referenced by {names}"));
        }
        ArtifactOutcome::KeptRequested(path) => print_kept(theme, "cached artifact", path, "--keep-artifact"),
        ArtifactOutcome::KeptLocal(path) => {
            print_kept(theme, "local source file", path, "remove only deletes cached artifacts");
        }
        ArtifactOutcome::KeptOutsideRoot(path) => {
            print_kept(theme, "artifact", path, "outside the plugin cache root");
        }
        ArtifactOutcome::NotApplicable => {
            println!("  {} No cached artifact to remove", theme.info_icon());
        }
    }
}

/// Renders a kept artifact with its reason. The noun differs per outcome on
/// purpose: a local source file is not a cached artifact.
#[cfg(feature = "plugins")]
fn print_kept(theme: &Theme, noun: &str, path: &Path, reason: &str) {
    println!("  {} Kept {noun} {} ({reason})", theme.info_icon(), path.display());
}
