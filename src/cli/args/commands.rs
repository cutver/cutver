use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

use super::plugin::PluginCommands;

/// Cut a release. Bump SemVer. Every project, every language.
#[derive(Parser, Debug)]
#[command(name = "cutver", version, about)]
pub struct Cli {
    /// Path to configuration file (defaults to discovering cutver.toml walking up from current directory)
    #[arg(short, long, global = true, value_name = "PATH")]
    pub config: Option<PathBuf>,
    /// Turn the CI output channel on: write this run's outputs to `$GITHUB_OUTPUT` and its summary to `$GITHUB_STEP_SUMMARY`, and annotate a failure on stderr even when neither file is named and `GITHUB_ACTIONS` is unset. The CI output channel never writes stdout, so `--json` is unaffected
    #[arg(long, global = true)]
    pub ci: bool,
    /// Turn the CI output channel off: no outputs, no step summary and no failure annotation, even on a detected CI
    #[arg(long = "no-ci", global = true, conflicts_with = "ci")]
    pub no_ci: bool,
    /// Forward a `runtime = "process"` plugin child's stderr to cutver's own stderr, unchanged, on a call that exits 0. Without it a successful call's stderr is drained and dropped; a failed call's stderr is its error message either way. A WASM plugin has no child and no stderr, so the flag changes nothing for it
    #[arg(long, global = true)]
    pub verbose: bool,
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
        /// Publish the version the level cuts on this channel: `alpha`, `beta` and `rc` append the channel and start its counter (`1.3.0` becomes `1.3.0-alpha.1`), and `stable` is the release itself. The bare word "channel" always means this ladder, never the CI output channel or the gate channel
        #[arg(long, value_enum, value_name = "CHANNEL")]
        channel: Option<Channel>,
        /// Simulate the release pipeline without modifying files or creating Git commits/tags
        #[arg(long)]
        dry_run: bool,
        /// Skip named preflight verification steps (repeatable)
        #[arg(long, value_name = "STEP")]
        skip_preflight: Vec<String>,
        /// Initial release without incrementing version strings in manifests (Gradle versionCode still increments)
        #[arg(long = "first-release", visible_alias = "fr")]
        first_release: bool,
        /// Create the release commit and push the branch, then stop: the tag, its pushes, `[publish] commands` and the post-release plugins wait for `cutver publish`
        #[arg(long)]
        defer_publish: bool,
        /// Print the release result as a single JSON object on stdout instead of the plan tree. The object is the last line cutver writes to stdout; `[preflight]` steps and git subprocesses may print above it — use `--report` when a machine must read the object alone
        #[arg(long)]
        json: bool,
        /// Write the release result to this file as one JSON line, atomically, and nothing else. Applies to real and `--dry-run` runs alike, and is independent of `--json`
        #[arg(long, value_name = "PATH")]
        report: Option<PathBuf>,
    },
    /// Compute the release the workspace is about to cut and print it without changing anything
    Plan {
        /// SemVer level to increment
        #[arg(value_enum, default_value = "auto")]
        level: BumpLevel,
        /// Report the version the run would cut on this channel: `alpha`, `beta` and `rc` append the channel and start its counter (`1.3.0` becomes `1.3.0-alpha.1`), and `stable` is the release itself. A plan computes it and writes nothing, exactly like a plan on the ladder without the flag
        #[arg(long, value_enum, value_name = "CHANNEL")]
        channel: Option<Channel>,
        /// Print the plan as one JSON object on stdout instead of the plan tree
        #[arg(long)]
        json: bool,
        /// Skip named preflight verification steps (repeatable). A skipped step is reported as skipped and never run
        #[arg(long, value_name = "STEP")]
        skip_preflight: Vec<String>,
        /// Plan the initial release, which tags the version the manifests already declare
        #[arg(long = "first-release", visible_alias = "fr")]
        first_release: bool,
    },
    /// Publish a release `bump --defer-publish` prepared: create the tag, push it, move the floating major tag, and run `[publish] commands`
    Publish {
        /// Release tag to publish; it must match the version the manifests declare at HEAD
        #[arg(long, value_name = "TAG")]
        tag: String,
        /// Release commit the caller verified; it must be HEAD
        #[arg(long, value_name = "SHA")]
        commit: Option<String>,
        /// Release notes for the `on_post_release` plugins. The clean-tree check exempts this
        /// file: it is an input to the command, not a change in the tree. Pass `-` to read the
        /// notes from standard input, which declares no input at all
        #[arg(long, value_name = "PATH")]
        notes_file: Option<PathBuf>,
        /// Refuse to publish unless this gate has an attestation for the commit being published
        /// (repeatable). The attestation directory is exempt from the clean-tree check, exactly
        /// like `--notes-file`
        #[arg(long, value_name = "NAME")]
        require_gate: Vec<String>,
        /// Validate and print the plan without creating a tag, pushing anything or running a command
        #[arg(long)]
        dry_run: bool,
        /// Continue a release an earlier `cutver publish` left unfinished, driven by repository evidence instead of a run record: steps that are harmless to repeat are repeated, and anything the evidence contradicts is refused. Requires `--commit`, and a `[publish] commands` entry runs only when `publish-<i>` (its 1-based position) is attested with conclusion `success` for that commit
        #[arg(long)]
        resume: bool,
    },
    /// Record a gate's verdict for a commit, so `publish --require-gate` can read it
    Attest {
        /// Gate this attestation covers; it becomes the file name in `[gates] dir`
        #[arg(value_name = "GATE")]
        gate: String,
        /// The commit the gate verified, as the full sha `git rev-parse HEAD` prints
        #[arg(long, value_name = "SHA")]
        commit: String,
        /// The verdict the gate reached
        #[arg(long, value_enum)]
        conclusion: Conclusion,
        /// The run that produced the verdict, recorded with it and reported by `cutver plan`
        #[arg(long, value_name = "URL")]
        run: Option<String>,
    },
    /// Validate configuration and report version drift across declared manifests
    Doctor {
        /// Also validate that CHANGELOG.md is consistent with Git release tags
        #[arg(long)]
        check_changelog: bool,
        /// Print the diagnosis as one JSON object on stdout instead of the dashboard, so a machine reads drift, and the checks that were skipped, as data. The exit code still reports the outcome: 0 clean, 2 drift, 1 a check could not run
        #[arg(long)]
        json: bool,
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
    /// Manage installed plugins
    #[command(hide = !cfg!(feature = "plugins"))]
    Plugin {
        #[command(subcommand)]
        command: PluginCommands,
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
pub enum Conclusion {
    /// The gate passed for that commit
    Success,
    /// The gate ran and failed for that commit
    Failure,
}

impl From<Conclusion> for crate::config::gates::Conclusion {
    fn from(conclusion: Conclusion) -> Self {
        match conclusion {
            Conclusion::Success => Self::Success,
            Conclusion::Failure => Self::Failure,
        }
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Channel {
    /// Publish on the alpha channel: `1.3.0` becomes `1.3.0-alpha.1`
    Alpha,
    /// Publish on the beta channel: `1.3.0` becomes `1.3.0-beta.1`
    Beta,
    /// Publish on the rc channel: `1.3.0` becomes `1.3.0-rc.1`
    Rc,
    /// Publish the release itself, with no prerelease segment
    Stable,
}

impl From<Channel> for crate::semver_bump::Channel {
    fn from(channel: Channel) -> Self {
        match channel {
            Channel::Alpha => Self::Alpha,
            Channel::Beta => Self::Beta,
            Channel::Rc => Self::Rc,
            Channel::Stable => Self::Stable,
        }
    }
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

impl From<BumpLevel> for crate::bump::BumpLevel {
    fn from(level: BumpLevel) -> Self {
        match level {
            BumpLevel::Patch => crate::bump::BumpLevel::Patch,
            BumpLevel::Minor => crate::bump::BumpLevel::Minor,
            BumpLevel::Major => crate::bump::BumpLevel::Major,
            BumpLevel::Auto => crate::bump::BumpLevel::Auto,
        }
    }
}

impl From<crate::bump::BumpLevel> for BumpLevel {
    fn from(level: crate::bump::BumpLevel) -> Self {
        match level {
            crate::bump::BumpLevel::Patch => BumpLevel::Patch,
            crate::bump::BumpLevel::Minor => BumpLevel::Minor,
            crate::bump::BumpLevel::Major => BumpLevel::Major,
            crate::bump::BumpLevel::Auto => BumpLevel::Auto,
        }
    }
}

/// S1's parse half (#377). The flag's behaviour is `tests/plugin_process_stderr_verbose.rs`; what a
/// parser test can state is that it is one global switch — accepted on either side of the subcommand
/// name, so a command never decides whether the diagnostics are reachable — that defaults to off and
/// has no short form to collide with one.
///
/// It lives here rather than in `args/tests.rs`, which sits at its recorded line cap (S7).
#[cfg(test)]
mod verbose_flag {
    use super::Cli;
    use clap::Parser;

    /// The subcommands the flag is exercised on, chosen across families: a release command, a
    /// configuration check, a plugin command and a command whose required arguments make `--verbose`
    /// compete with a parser that has already rejected the invocation.
    fn accepted() -> Vec<Vec<&'static str>> {
        vec![
            vec!["cutver", "doctor", "--verbose"],
            vec!["cutver", "--verbose", "plan"],
            vec!["cutver", "bump", "patch", "--verbose"],
            vec!["cutver", "changelog", "latest", "--verbose"],
            vec!["cutver", "plugin", "list", "--verbose"],
            vec![
                "cutver",
                "attest",
                "gate",
                "--commit",
                "abc",
                "--conclusion",
                "success",
                "--verbose",
            ],
        ]
    }

    #[test]
    fn the_flag_is_global_and_defaults_to_off() {
        for args in accepted() {
            let cli =
                Cli::try_parse_from(args.clone()).unwrap_or_else(|err| panic!("{args:?} must accept --verbose: {err}"));
            assert!(cli.verbose, "{args:?} must set the one switch");
        }

        assert!(!Cli::try_parse_from(["cutver", "doctor"]).unwrap().verbose);
        assert!(!Cli::try_parse_from(["cutver", "doctor", "--no-ci"]).unwrap().verbose);
    }

    #[test]
    fn the_flag_has_no_short_form() {
        assert!(
            Cli::try_parse_from(["cutver", "doctor", "-v"]).is_err(),
            "S1 gives the flag no short form, so `-v` is not a second spelling of it"
        );
        assert!(Cli::try_parse_from(["cutver", "doctor", "--verbose"]).is_ok());
    }
}

/// ADR-0005: `--channel` names the prerelease ladder, so the bare word "channel" belongs to it and
/// this surface keeps its qualifier wherever a user reads it. The test is why the qualifier cannot
/// erode silently in a later edit.
#[cfg(test)]
mod ci_output_channel_wording {
    use super::Cli;
    use clap::CommandFactory;

    #[test]
    fn the_ci_flags_name_an_output_channel() {
        let command = Cli::command();

        for id in ["ci", "no_ci"] {
            let arg = command
                .get_arguments()
                .find(|arg| arg.get_id().as_str() == id)
                .unwrap_or_else(|| panic!("the `{id}` flag exists"));
            let help = arg.get_help().map(ToString::to_string).unwrap_or_default();

            assert!(
                help.contains("CI output channel"),
                "`--{}` must name the CI output channel, its help reads: {help}",
                id.replace('_', "-")
            );
        }
    }
}
