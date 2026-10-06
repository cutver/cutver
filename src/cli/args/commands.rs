use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// Cut a release. Bump SemVer. Every project, every language.
#[derive(Parser, Debug)]
#[command(name = "cutver", version, about)]
pub struct Cli {
    /// Path to configuration file (defaults to discovering cutver.toml walking up from current directory)
    #[arg(short, long, global = true, value_name = "PATH")]
    pub config: Option<PathBuf>,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize a new cutver.toml configuration by discovering project manifests
    Init {
        /// Update an existing cutver.toml with newly discovered manifests without overwriting settings
        #[arg(short, long)]
        update: bool,
        /// Overwrite existing configuration file if present
        #[arg(short, long)]
        force: bool,
        /// Target directory to inspect and initialize (defaults to current directory)
        #[arg(short, long, value_name = "DIR")]
        path: Option<PathBuf>,
        /// Skip scaffolding the default MiniJinja release template
        #[arg(long, visible_alias = "nt")]
        no_template: bool,
    },
    /// Synchronize versions across manifests, run preflight checks, update changelog, and create a Git commit and tag
    Bump {
        /// SemVer level to increment
        #[arg(value_enum, default_value = "auto")]
        level: BumpLevel,
        /// Simulate the release pipeline without modifying files or creating Git commits/tags
        #[arg(long)]
        dry_run: bool,
        /// Skip named preflight verification steps (repeatable)
        #[arg(long, value_name = "STEP")]
        skip_preflight: Vec<String>,
        /// Initial release without incrementing the version in manifests
        #[arg(long = "first-release", visible_alias = "fr")]
        first_release: bool,
    },
    /// Validate configuration and report version drift across declared manifests
    Doctor {
        /// Also validate that CHANGELOG.md is consistent with Git release tags
        #[arg(long)]
        check_changelog: bool,
    },
    /// Query or extract entries from the changelog
    Changelog {
        #[command(subcommand)]
        command: ChangelogCommands,
    },
    /// Open repository releases, tags, or comparisons in the browser
    Open {
        /// Optional target release version, compare range (e.g. '0.9.1', 'v1.0.0...v1.1.0', or 'compare')
        #[arg(value_name = "TARGET")]
        target: Option<String>,
        /// Print the resolved URL to stdout instead of launching a browser
        #[arg(long, visible_alias = "dry-run")]
        print_url: bool,
        /// Explicit browser or command to launch
        #[arg(long, value_name = "BROWSER")]
        browser: Option<String>,
    },
    /// External plugin subcommand resolved as `cutver-<subcommand>` from $PATH
    #[command(external_subcommand)]
    External(Vec<String>),
}

#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum ChangelogCommands {
    /// Extract the latest release notes from the changelog
    Latest {
        /// Include the release heading (e.g. '## [0.3.1] - 2026-09-20')
        #[arg(short = 'H', long)]
        include_header: bool,
        /// Explicit path to changelog file (defaults to changelog configured in cutver.toml or CHANGELOG.md)
        #[arg(short, long, value_name = "PATH")]
        path: Option<PathBuf>,
        /// Optional path to an arbitrary MiniJinja template file to format the output
        #[arg(long, value_name = "PATH")]
        template: Option<PathBuf>,
        /// Output release context as structured JSON directly to stdout
        #[arg(long)]
        json: bool,
    },
    /// Extract release notes for a specific version from the changelog
    Show {
        /// The version to extract (e.g. '0.2.0' or 'v0.2.0')
        #[arg(value_name = "VERSION")]
        version: String,
        /// Include the release heading (e.g. '## [0.2.0] - 2026-09-19')
        #[arg(short = 'H', long)]
        include_header: bool,
        /// Explicit path to changelog file (defaults to changelog configured in cutver.toml or CHANGELOG.md)
        #[arg(short, long, value_name = "PATH")]
        path: Option<PathBuf>,
        /// Optional path to an arbitrary MiniJinja template file to format the output
        #[arg(long, value_name = "PATH")]
        template: Option<PathBuf>,
        /// Output release context as structured JSON directly to stdout
        #[arg(long)]
        json: bool,
    },
    /// Open the changelog, release, or comparison in the browser
    Open {
        /// Optional target release version, compare range (e.g. '0.9.1', 'v1.0.0...v1.1.0', or 'compare')
        #[arg(value_name = "TARGET")]
        target: Option<String>,
        /// Print the resolved URL to stdout instead of launching a browser
        #[arg(long, visible_alias = "dry-run")]
        print_url: bool,
        /// Explicit browser or command to launch
        #[arg(long, value_name = "BROWSER")]
        browser: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum BumpLevel {
    /// Increment patch version (e.g. 1.2.3 -> 1.2.4)
    Patch,
    /// Increment minor version (e.g. 1.2.3 -> 1.3.0)
    Minor,
    /// Increment major version (e.g. 1.2.3 -> 2.0.0)
    Major,
    /// Automatically deduce bump level from Conventional Commits since the latest tag
    Auto,
}

impl From<crate::semver_bump::Bump> for BumpLevel {
    fn from(b: crate::semver_bump::Bump) -> Self {
        match b {
            crate::semver_bump::Bump::Patch => BumpLevel::Patch,
            crate::semver_bump::Bump::Minor => BumpLevel::Minor,
            crate::semver_bump::Bump::Major => BumpLevel::Major,
        }
    }
}
