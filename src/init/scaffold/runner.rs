//! Main initialization execution workflow and terminal reporting for `cutver init`.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;

use super::config_gen::{generate_fresh_config, update_existing_config};
use super::templates::{DEFAULT_RELEASE_TEMPLATE, DEFAULT_TEMPLATE_PATH, STARTER_CHANGELOG};
use crate::init::discovery::{DiscoveredKind, DiscoveredManifest, DiscoveryResult, discover_project};

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

    #[error("failed to render the JSON object: {source}")]
    Serialize {
        #[source]
        source: serde_json::Error,
    },

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
pub fn print_fresh_summary(target_dir: &Path, discovery: &DiscoveryResult, created_files: &[String]) {
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

/// Prints the summary of an `--update` that appended to an existing configuration.
pub fn print_update_summary(added: &[DiscoveredManifest]) {
    if added.is_empty() {
        println!("ℹ cutver.toml is already up to date (no new manifests discovered).");
        return;
    }
    println!("✔ Updated cutver.toml with {} new manifest(s):\n", added.len());
    for m in added {
        println!("  ✔ {} ({})", m.path, describe_kind(&m.kind));
    }
    println!();
    print_next_steps();
}

/// The next steps `cutver init` prints, kept as data instead of loose `println!` calls so a test can
/// hold them to the flow the documentation recommends: read the plan before anything mutates, then cut
/// the release in two phases so the artifacts can be verified in between. A recommendation nobody
/// checks is how the first-run page and the first-run command drift apart.
pub const NEXT_STEPS: [&str; 7] = [
    "Next steps:",
    "  1. Validate your configuration:",
    "     cutver doctor",
    "  2. Read the release before it happens:",
    "     cutver plan",
    "  3. Cut it in two phases, so the artifacts can be verified in between:",
    "     cutver bump --defer-publish",
];

/// Prints actionable next steps for the user after initialization.
pub fn print_next_steps() {
    for line in NEXT_STEPS {
        println!("{line}");
    }
}

/// Executes the initialization workflow: discovers workspace manifests and writes `cutver.toml`.
///
/// `json` selects the machine channel: one compact object on stdout instead of the summary, both
/// describing the same run (#404 S4). The facts the object carries are threaded out of the workflow
/// as [`InitOutcome`] rather than printed where they are computed, so the two channels cannot
/// disagree about what the run found or wrote.
pub fn run_init(
    path: Option<PathBuf>,
    update: bool,
    force: bool,
    no_template: bool,
    json: bool,
) -> Result<(), InitError> {
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

    let outcome = if update {
        if !config_path.is_file() {
            return Err(InitError::NotFoundForUpdate {
                path: config_path.display().to_string(),
            });
        }
        let (added, primary) = update_existing_config(&config_path, &discovery.manifests)?;
        // An update that appended nothing wrote nothing, so the object reports no file instead of
        // claiming a configuration it never touched.
        let written = if added.is_empty() {
            Vec::new()
        } else {
            vec!["cutver.toml".to_string()]
        };
        InitOutcome {
            discovery,
            primary,
            written,
            kind: InitKind::Updated { added },
        }
    } else {
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

        let mut written = vec!["cutver.toml".to_string()];
        if !changelog_path.exists() {
            crate::atomic::write_atomic(&changelog_path, STARTER_CHANGELOG.as_bytes()).map_err(|e| {
                InitError::WriteFile {
                    path: changelog_path.display().to_string(),
                    source: e,
                }
            })?;
            written.push("CHANGELOG.md".to_string());
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
            written.push(DEFAULT_TEMPLATE_PATH.to_string());
        }

        // The first manifest a fresh configuration declares is its source of truth, which is the one
        // its own comment and the summary's badge name.
        let primary = discovery.manifests.first().map(|m| m.path.clone());
        InitOutcome {
            discovery,
            primary,
            written,
            kind: InitKind::Fresh { target_dir },
        }
    };

    report(&outcome, json)
}

/// What one `cutver init` run did, so both channels describe the same run instead of each deriving
/// its own version of it (#404 S4).
///
/// `written` holds the files *this* run wrote, relative to the target directory with forward slashes.
/// An `--update` writes no scaffold, so its list carries nothing but the configuration it appended
/// to: the object never claims a file the run did not create.
struct InitOutcome {
    /// The manifests discovery found, in the order the configuration declares them.
    discovery: DiscoveryResult,
    /// The primary source of truth, spelled as the configuration spells it; `None` when nothing was
    /// detected.
    primary: Option<String>,
    /// The files this run wrote.
    written: Vec<String>,
    /// What separates a scaffold from an update, for the human summary.
    kind: InitKind,
}

/// The two shapes one `init` run takes.
enum InitKind {
    Fresh { target_dir: PathBuf },
    Updated { added: Vec<DiscoveredManifest> },
}

impl InitOutcome {
    /// The summary a human reads, byte for byte what `init` always printed.
    fn print_human(&self) {
        match &self.kind {
            InitKind::Fresh { target_dir } => print_fresh_summary(target_dir, &self.discovery, &self.written),
            InitKind::Updated { added } => print_update_summary(added),
        }
    }
}

/// The machine-readable result of `cutver init` (#404 S4).
///
/// Field order is part of the contract, so the shape is a struct rather than a map; `src/cli/json.rs`
/// owns the `schemaVersion` this object opens with. Both lists are ordered lists rather than maps:
/// the configuration declares the manifests in the order reported, and the first is the source of
/// truth a wrapper would otherwise have to guess.
#[derive(serde::Serialize)]
struct InitJson<'a> {
    manifests: Vec<ManifestJson<'a>>,
    primary: Option<&'a str>,
    files_written: &'a [String],
}

/// One detected manifest: where it is, whether it sits at the target directory's root, and the kind
/// the configuration spells.
#[derive(serde::Serialize)]
struct ManifestJson<'a> {
    path: &'a str,
    is_root: bool,
    kind: &'a str,
}

/// The kind a manifest is declared as in `cutver.toml`, which is the closed set a wrapper maps to the
/// file it has to edit. [`describe_kind`] is the human flavour of the same fact and is not this:
/// `json: version` names a field, not a kind.
fn config_kind(kind: &DiscoveredKind) -> &'static str {
    match kind {
        DiscoveredKind::CargoPackage => "cargo-package",
        DiscoveredKind::Json { .. } => "json",
        DiscoveredKind::Gradle { .. } => "gradle",
        DiscoveredKind::Pyproject => "pyproject",
    }
}

/// Renders the run as the compact object `--json` prints: one line, `schemaVersion` first.
fn render_init_json(outcome: &InitOutcome) -> Result<String, serde_json::Error> {
    let fields = InitJson {
        manifests: outcome
            .discovery
            .manifests
            .iter()
            .map(|manifest| ManifestJson {
                path: &manifest.path,
                is_root: manifest.is_root,
                kind: config_kind(&manifest.kind),
            })
            .collect(),
        primary: outcome.primary.as_deref(),
        files_written: &outcome.written,
    };
    serde_json::to_string(&crate::cli::json::Versioned::new(&fields))
}

/// Reports the run on the channel the invocation asked for: one JSON object on stdout, or the
/// summary a human reads. A render that fails is an [`InitError`], so it reaches the caller's failure
/// envelope instead of printing half a document.
fn report(outcome: &InitOutcome, json: bool) -> Result<(), InitError> {
    if !json {
        outcome.print_human();
        return Ok(());
    }
    let object = render_init_json(outcome).map_err(|source| InitError::Serialize { source })?;
    println!("{object}");
    Ok(())
}
