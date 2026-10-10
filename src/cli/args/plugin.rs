//! Clap argument shapes for `cutver plugin`.
//!
//! This module is deliberately dependency-free so it compiles in the lean
//! (no-`plugins`) build too: the subcommand is declared in every build and the
//! runner fails closed when the feature is absent (decision D12).

use clap::Subcommand;

#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum PluginCommands {
    /// List configured plugins and report whether each artifact is installed and digest-matched
    List {
        /// Print the plugin report as a machine-readable JSON object
        #[arg(long)]
        json: bool,
    },
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
