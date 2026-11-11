//! Clap argument shapes for `cutver plugin`.
//!
//! This module is deliberately dependency-free so it compiles in the lean
//! (no-`plugins`) build too: the subcommand is declared in every build and the
//! runner fails closed when the feature is absent (decision D12).

use clap::{Args, Subcommand};

#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum PluginCommands {
    /// List configured plugins and report whether each artifact is installed and digest-matched
    List {
        /// Print the plugin report as a machine-readable JSON object
        #[arg(long)]
        json: bool,
    },
    /// Install a plugin from the registry into the cache and cutver.toml
    Add(PluginAddArgs),
    /// Restore declared plugins by fetching missing artifacts into the local cache
    Restore(PluginRestoreArgs),
    /// Update declared plugins to the latest release from the registry
    Update(PluginUpdateArgs),
    /// Show detailed metadata and releases for a plugin from the registry
    Info(PluginInfoArgs),
    /// Search the registry for available plugins
    Search(PluginSearchArgs),
    /// Remove a `[plugins.<name>]` declaration and its unreferenced cached artifact
    Remove {
        /// Name of the plugin to remove (as declared under `[plugins.<name>]`)
        #[arg(value_name = "NAME")]
        name: String,
        /// Keep the cached artifact even when no other plugin references it
        #[arg(long)]
        keep_artifact: bool,
    },
}

/// Arguments for `cutver plugin add`.
#[derive(Args, Debug, Clone, PartialEq, Eq)]
pub struct PluginAddArgs {
    /// Name of the plugin to add
    #[arg(value_name = "NAME")]
    pub name: String,
    /// Version requirement to select a release (e.g. "0.1.0" or "^0.1")
    #[arg(long, value_name = "VERSION")]
    pub version: Option<String>,
    /// Registry index base URL (https-only, defaults to the Cutver registry)
    #[arg(long, value_name = "BASE")]
    pub registry: Option<String>,
    /// Event the plugin handles; required by lifecycle.v1 (repeatable)
    #[arg(long = "event", value_name = "EVENT")]
    pub events: Vec<String>,
    /// Manifest glob the plugin matches; required by manifest.v1 (repeatable)
    #[arg(long = "manifest-match", value_name = "GLOB")]
    pub manifest_match: Vec<String>,
    /// Resolve and print the plan without downloading or writing anything
    #[arg(long)]
    pub dry_run: bool,
    /// Replace an existing `[plugins.<name>]` declaration
    #[arg(long)]
    pub force: bool,
}

/// Arguments for `cutver plugin restore`.
#[derive(Args, Debug, Clone, PartialEq, Eq, Default)]
pub struct PluginRestoreArgs {
    /// Registry index base URL (https-only, defaults to the Cutver registry)
    #[arg(long, value_name = "BASE")]
    pub registry: Option<String>,
    /// Print what would be downloaded without writing to the cache
    #[arg(long)]
    pub dry_run: bool,
}

/// Arguments for `cutver plugin update`.
#[derive(Args, Debug, Clone, PartialEq, Eq, Default)]
pub struct PluginUpdateArgs {
    /// Name of the specific plugin to update (omitted updates all declared plugins)
    #[arg(value_name = "NAME")]
    pub name: Option<String>,
    /// Version requirement to select a release (e.g. "0.2.0" or "^0.2")
    #[arg(long, value_name = "VERSION")]
    pub version: Option<String>,
    /// Registry index base URL (https-only, defaults to the Cutver registry)
    #[arg(long, value_name = "BASE")]
    pub registry: Option<String>,
    /// Print what would be updated without writing anything to disk
    #[arg(long)]
    pub dry_run: bool,
}

/// Arguments for `cutver plugin info`.
#[derive(Args, Debug, Clone, PartialEq, Eq)]
pub struct PluginInfoArgs {
    /// Name of the plugin to inspect
    #[arg(value_name = "NAME")]
    pub name: String,
    /// Registry index base URL (https-only, defaults to the Cutver registry)
    #[arg(long, value_name = "BASE")]
    pub registry: Option<String>,
    /// Print metadata as a machine-readable JSON object
    #[arg(long)]
    pub json: bool,
}

/// Arguments for `cutver plugin search`.
#[derive(Args, Debug, Clone, PartialEq, Eq, Default)]
pub struct PluginSearchArgs {
    /// Query string to filter plugins by name, description, author, or capability
    #[arg(value_name = "QUERY")]
    pub query: Option<String>,
    /// Registry index base URL (https-only, defaults to the Cutver registry)
    #[arg(long, value_name = "BASE")]
    pub registry: Option<String>,
    /// Print search results as a machine-readable JSON array
    #[arg(long)]
    pub json: bool,
}
