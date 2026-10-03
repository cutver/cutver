//! Scaffolding and template generation module for `cutver init`.

pub mod config_gen;
pub mod runner;
pub mod templates;

pub use config_gen::{format_manifest_entry, generate_fresh_config, update_existing_config};
pub use runner::{InitError, describe_kind, print_fresh_summary, print_next_steps, run_init};
pub use templates::{DEFAULT_RELEASE_TEMPLATE, DEFAULT_TEMPLATE_PATH, STARTER_CHANGELOG};

#[cfg(test)]
mod tests;
