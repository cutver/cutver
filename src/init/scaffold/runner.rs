//! Main initialization execution workflow and terminal reporting for `cutver init`.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;

use super::config_gen::{generate_fresh_config, update_existing_config};
use super::templates::{DEFAULT_RELEASE_TEMPLATE, DEFAULT_TEMPLATE_PATH, STARTER_CHANGELOG};
use crate::init::discovery::{DiscoveredKind, DiscoveryResult, discover_project};

/// Errors that can occur during the initialization workflow.
#[derive(Debug, Error)]
pub enum InitError {
    #[error(
        "cutver.toml already exists at '{path}'.\n  To discover and add new manifests without overwriting existing settings:\n    cutver init --update\n  To overwrite the existing configuration:\n    cutver init --force"
    )]
    AlreadyExists { path: String },

    #[error("cannot update: '{path}' does not exist.\n  Run 'cutver init' to generate a new configuration.")]
    NotFoundForUpdate { path: String },

    #[error("failed to read existing '{path}': {source}")]
    ReadConfig { path: String, source: io::Error },

    #[error("failed to parse existing '{path}': {source}")]
    ParseConfig { path: String, source: toml_edit::TomlError },

    #[error("failed to discover manifests in '{path}': {source}")]
    Discovery { path: String, source: io::Error },

    #[error("failed to write '{path}': {source}")]
    WriteFile { path: String, source: crate::atomic::Error },

    #[error("io error: {0}")]
    Io(#[from] io::Error),
}

/// Returns a human-readable description of a discovered manifest kind.
pub fn describe_kind(kind: &DiscoveredKind) -> &'static str {
    match kind {
        DiscoveredKind::CargoPackage => "cargo-package",
        DiscoveredKind::Json { .. } => "json: version",
        DiscoveredKind::Gradle { .. } => "gradle",
        DiscoveredKind::Pyproject => "pyproject",
    }
}

/// Prints summary of initialized configuration and created files.
pub fn print_fresh_summary(target_dir: &Path, discovery: &DiscoveryResult, created_files: &[&str]) {
    if discovery.manifests.is_empty() {
        println!("⚠ Initialized cutver in {}\n", target_dir.display());
        println!("No package manifests were automatically discovered.");
        println!("A starter configuration has been created. Declare your manifests under [[manifest]].\n");
    } else {
        println!("✔ Initialized cutver configuration in {}\n", target_dir.display());
        println!("Discovered manifests:");
        for (i, m) in discovery.manifests.iter().enumerate() {
            let primary_badge = if i == 0 { " [primary source of truth]" } else { "" };
            println!("  ✔ {} ({}){}", m.path, describe_kind(&m.kind), primary_badge);
        }
        println!();
    }

    println!("Created files:");
    for f in created_files {
        println!("  ✔ {f}");
    }
    println!();

    print_next_steps();
}

/// Prints actionable next steps for the user after initialization.
pub fn print_next_steps() {
    println!("Next steps:");
    println!("  1. Validate your configuration:");
    println!("     cutver doctor");
    println!("  2. Simulate your first release:");
    println!("     cutver bump auto --dry-run");
}

/// Executes the initialization workflow: discovers workspace manifests and writes `cutver.toml`.
pub fn run_init(path: Option<PathBuf>, update: bool, force: bool, no_template: bool) -> Result<(), InitError> {
    let target_dir = match path {
        Some(p) => {
            if p.is_relative() {
                std::env::current_dir()?.join(p)
            } else {
                p
            }
        }
        None => std::env::current_dir()?,
    };

    let config_path = target_dir.join("cutver.toml");
    let changelog_path = target_dir.join("CHANGELOG.md");
    let template_path = target_dir.join(DEFAULT_TEMPLATE_PATH);

    let discovery = discover_project(&target_dir).map_err(|e| InitError::Discovery {
        path: target_dir.display().to_string(),
        source: e,
    })?;

    if update {
        if !config_path.is_file() {
            return Err(InitError::NotFoundForUpdate {
                path: config_path.display().to_string(),
            });
        }
        let added = update_existing_config(&config_path, &discovery.manifests)?;
        if added.is_empty() {
            println!("ℹ cutver.toml is already up to date (no new manifests discovered).");
        } else {
            println!("✔ Updated cutver.toml with {} new manifest(s):\n", added.len());
            for m in &added {
                println!("  ✔ {} ({})", m.path, describe_kind(&m.kind));
            }
            println!();
            print_next_steps();
        }
        return Ok(());
    }

    if config_path.exists() && !force {
        return Err(InitError::AlreadyExists {
            path: config_path.display().to_string(),
        });
    }

    let config_content = generate_fresh_config(&discovery, no_template);
    crate::atomic::write_atomic(&config_path, config_content.as_bytes()).map_err(|e| InitError::WriteFile {
        path: config_path.display().to_string(),
        source: e,
    })?;

    let mut created_files = vec!["cutver.toml"];
    if !changelog_path.exists() {
        crate::atomic::write_atomic(&changelog_path, STARTER_CHANGELOG.as_bytes()).map_err(|e| {
            InitError::WriteFile {
                path: changelog_path.display().to_string(),
                source: e,
            }
        })?;
        created_files.push("CHANGELOG.md");
    }

    if !no_template && (!template_path.exists() || force) {
        if let Some(parent) = template_path.parent() {
            fs::create_dir_all(parent)?;
        }
        crate::atomic::write_atomic(&template_path, DEFAULT_RELEASE_TEMPLATE.as_bytes()).map_err(|e| {
            InitError::WriteFile {
                path: template_path.display().to_string(),
                source: e,
            }
        })?;
        created_files.push(DEFAULT_TEMPLATE_PATH);
    }

    print_fresh_summary(&target_dir, &discovery, &created_files);
    Ok(())
}
