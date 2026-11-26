//! The failure half of the machine contract.
//!
//! When an invocation asked for machine output on stdout (`--json`, or `--report` for `bump`), its
//! failure is machine output too: one JSON object on **stderr**, so a caller never has to parse
//! `Error: …` prose. The exit code is untouched by the format — a failure still exits `1`.
//!
//! When the invocation did not ask for machine output, nothing here runs: [`try_emit_failure`]
//! returns `false` and the caller prints the rendered human line it always printed (S6).
//!
//! The activation rule is per-command, not a new global flag (L6): the caller that asked for machine
//! output on stdout gets machine output on stderr, and no second flag has to be remembered in CI.

use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;

use super::args::{ChangelogCommands, Commands, PluginCommands};
use super::json::Versioned;

/// Whether this invocation asked for machine output. Written once by [`select`] at the top of the
/// run, read by every failure site; a single relaxed flag is enough because one process runs one
/// command, and the reader only needs to see the value that was stored before it ran.
static MACHINE: AtomicBool = AtomicBool::new(false);

/// Whether this invocation asked for machine output.
pub fn is_machine() -> bool {
    MACHINE.load(Ordering::Relaxed)
}

/// Whether `command` asks for machine output on stdout, which is what selects the machine failure
/// channel for it. `bump --report <PATH>` counts: it is the flag for a machine that must read the
/// object alone, so its failure has to be readable alone too.
pub fn wants_machine(command: &Commands) -> bool {
    match command {
        Commands::Bump { json, report, .. } => *json || report.is_some(),
        Commands::Plan { json, .. } => *json,
        Commands::Publish { json, .. } => *json,
        Commands::Doctor { json, .. } => *json,
        Commands::Changelog { command } => matches!(
            command,
            ChangelogCommands::Latest { json: true, .. } | ChangelogCommands::Show { json: true, .. }
        ),
        // `PluginCommands` compiles in the lean build too (decision D12), so this needs no feature
        // gate: without the feature the command fails closed and the failure is still envelope-shaped.
        Commands::Plugin { command } => match command {
            PluginCommands::List { json } => *json,
            PluginCommands::Info(args) => args.json,
            PluginCommands::Search(args) => args.json,
            PluginCommands::Add(_)
            | PluginCommands::Restore(_)
            | PluginCommands::Update(_)
            | PluginCommands::Remove { .. } => false,
        },
        Commands::Init { .. } | Commands::Open { .. } | Commands::External(_) => false,
        // `attest` prints one human line and has no `--json` surface of its own, so its failure
        // keeps the human path.
        Commands::Attest { .. } => false,
    }
}

/// Records whether this invocation asked for machine output.
pub fn select(command: &Commands) {
    MACHINE.store(wants_machine(command), Ordering::Relaxed);
}

/// The class a failure belongs to. `code` is a stable machine string, not a message: it names the
/// subsystem, so a consumer can branch without matching prose, and it grows additively.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureCode {
    Attest,
    Bump,
    Changelog,
    ChangelogCheck,
    Config,
    Doctor,
    External,
    Init,
    Open,
    Plan,
    Plugin,
    Publish,
    Report,
}

impl FailureCode {
    /// The stable, lowercase snake_case string the envelope carries in `error.code`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Attest => "attest",
            Self::Bump => "bump",
            Self::Changelog => "changelog",
            Self::ChangelogCheck => "changelog_check",
            Self::Config => "config",
            Self::Doctor => "doctor",
            Self::External => "external",
            Self::Init => "init",
            Self::Open => "open",
            Self::Plan => "plan",
            Self::Plugin => "plugin",
            Self::Publish => "publish",
            Self::Report => "report",
        }
    }

    /// The process exit code this failure produces, which the envelope repeats as `exitCode`.
    ///
    /// It is always `1`: `2` is drift, and drift is an outcome `doctor` reports as data rather than
    /// a failure, while a usage error never reaches this module because clap exits first.
    pub fn exit_code(self) -> u8 {
        1
    }
}

/// The envelope's `error` key. Field order is part of the contract, so this is a struct rather than
/// a map: `code`, then `exitCode`, then `message`.
#[derive(Serialize)]
struct FailureJson<'a> {
    error: FailureBody<'a>,
}

#[derive(Serialize)]
struct FailureBody<'a> {
    code: &'a str,
    #[serde(rename = "exitCode")]
    exit_code: u8,
    message: &'a str,
}

/// Renders the compact envelope:
/// `{"schemaVersion":1,"error":{"code":"…","exitCode":1,"message":"…"}}`.
///
/// `message` is the human text the caller would print, so the diagnosis does not change shape
/// between the two channels. It goes through [`Versioned`], which keeps `schemaVersion` first.
pub fn render_failure(code: FailureCode, message: &str) -> Result<String, serde_json::Error> {
    let payload = FailureJson {
        error: FailureBody {
            code: code.as_str(),
            exit_code: code.exit_code(),
            message,
        },
    };
    serde_json::to_string(&Versioned::new(&payload))
}

/// Prints the envelope on stderr when this invocation asked for machine output, and reports whether
/// it did.
///
/// A `false` return means "print the human line instead", which is also the answer when rendering
/// fails: a failure must never be swallowed, so a broken render degrades to the human path.
pub fn try_emit_failure(code: FailureCode, message: &str) -> bool {
    if !is_machine() {
        return false;
    }
    match render_failure(code, message) {
        Ok(text) => {
            eprintln!("{text}");
            true
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::args::{BumpLevel, PluginAddArgs, PluginInfoArgs, PluginSearchArgs};

    // The global is deliberately not flipped here: the test binary runs these tests in parallel in
    // one process, so `select`/`is_machine` are proven by `tests/machine_error_contract.rs`.

    fn bump(json: bool, report: Option<&str>) -> Commands {
        Commands::Bump {
            level: BumpLevel::Auto,
            dry_run: false,
            skip_preflight: Vec::new(),
            first_release: false,
            defer_publish: false,
            channel: None,
            json,
            report: report.map(std::path::PathBuf::from),
        }
    }

    fn plan(json: bool) -> Commands {
        Commands::Plan {
            level: BumpLevel::Auto,
            json,
            skip_preflight: Vec::new(),
            first_release: false,
            channel: None,
        }
    }

    fn plugin(command: PluginCommands) -> Commands {
        Commands::Plugin { command }
    }

    fn publish(json: bool) -> Commands {
        Commands::Publish {
            tag: "v1.3.0".into(),
            commit: None,
            notes_file: None,
            require_gate: Vec::new(),
            dry_run: false,
            resume: false,
            json,
        }
    }

    #[test]
    fn the_envelope_is_one_compact_object_with_the_version_first_and_a_pinned_key_order() {
        let json = render_failure(FailureCode::Config, "no cutver.toml found").expect("the envelope renders");

        assert_eq!(
            json,
            r#"{"schemaVersion":1,"error":{"code":"config","exitCode":1,"message":"no cutver.toml found"}}"#
        );
    }

    #[test]
    fn the_plan_surface_has_its_own_failure_class() {
        let json =
            render_failure(FailureCode::Plan, "could not render the release entry").expect("the envelope renders");

        assert_eq!(
            json,
            r#"{"schemaVersion":1,"error":{"code":"plan","exitCode":1,"message":"could not render the release entry"}}"#
        );
    }

    #[test]
    fn a_multiline_message_stays_inside_one_json_line() {
        let json =
            render_failure(FailureCode::Bump, "first line\n  Where: somewhere\n  Fix: do this").expect("it renders");

        assert_eq!(json.lines().count(), 1, "the envelope is one line: {json}");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("it is valid JSON");
        assert_eq!(
            parsed["error"]["message"],
            "first line\n  Where: somewhere\n  Fix: do this"
        );
    }

    #[test]
    fn every_failure_class_has_its_documented_string() {
        let table = [
            (FailureCode::Attest, "attest"),
            (FailureCode::Bump, "bump"),
            (FailureCode::Changelog, "changelog"),
            (FailureCode::ChangelogCheck, "changelog_check"),
            (FailureCode::Config, "config"),
            (FailureCode::Doctor, "doctor"),
            (FailureCode::External, "external"),
            (FailureCode::Init, "init"),
            (FailureCode::Open, "open"),
            (FailureCode::Plan, "plan"),
            (FailureCode::Plugin, "plugin"),
            (FailureCode::Publish, "publish"),
            (FailureCode::Report, "report"),
        ];

        for (code, expected) in table {
            assert_eq!(code.as_str(), expected);
        }
    }

    #[test]
    fn every_failure_class_exits_one_because_drift_is_an_outcome_and_usage_is_claps() {
        for code in [
            FailureCode::Attest,
            FailureCode::Bump,
            FailureCode::Changelog,
            FailureCode::ChangelogCheck,
            FailureCode::Config,
            FailureCode::Doctor,
            FailureCode::External,
            FailureCode::Init,
            FailureCode::Open,
            FailureCode::Plan,
            FailureCode::Plugin,
            FailureCode::Publish,
            FailureCode::Report,
        ] {
            assert_eq!(code.exit_code(), 1, "{} must exit 1", code.as_str());
        }
    }

    #[test]
    fn a_command_that_asks_for_machine_output_selects_the_envelope() {
        assert!(wants_machine(&bump(true, None)), "`bump --json`");
        assert!(wants_machine(&bump(false, Some("release.json"))), "`bump --report`");
        assert!(wants_machine(&plan(true)), "`plan --json`");
        assert!(wants_machine(&publish(true)), "`publish --json`");
        assert!(wants_machine(&Commands::Doctor {
            check_changelog: false,
            json: true
        }));
        assert!(wants_machine(&Commands::Changelog {
            command: ChangelogCommands::Latest {
                include_header: false,
                path: None,
                template: None,
                json: true,
            },
        }));
        assert!(wants_machine(&Commands::Changelog {
            command: ChangelogCommands::Show {
                version: "1.0.0".into(),
                include_header: false,
                path: None,
                template: None,
                json: true,
            },
        }));
        assert!(wants_machine(&plugin(PluginCommands::List { json: true })));
        assert!(wants_machine(&plugin(PluginCommands::Info(PluginInfoArgs {
            name: "github-publish".into(),
            registry: None,
            json: true,
        }))));
        assert!(wants_machine(&plugin(PluginCommands::Search(PluginSearchArgs {
            query: None,
            registry: None,
            json: true,
        }))));
    }

    #[test]
    fn a_command_that_asks_for_human_output_keeps_the_human_line() {
        assert!(!wants_machine(&bump(false, None)), "`bump`");
        assert!(!wants_machine(&plan(false)), "`plan`");
        assert!(!wants_machine(&publish(false)), "`publish`");
        assert!(!wants_machine(&Commands::Doctor {
            check_changelog: false,
            json: false
        }));
        assert!(!wants_machine(&Commands::Changelog {
            command: ChangelogCommands::Latest {
                include_header: false,
                path: None,
                template: None,
                json: false,
            },
        }));
        assert!(!wants_machine(&plugin(PluginCommands::List { json: false })));
        assert!(!wants_machine(&plugin(PluginCommands::Add(PluginAddArgs {
            name: "github-publish".into(),
            version: None,
            registry: None,
            events: Vec::new(),
            manifest_match: Vec::new(),
            dry_run: false,
            force: false,
        }))));
    }
}
