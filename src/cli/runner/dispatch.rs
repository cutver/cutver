use std::path::{Path, PathBuf};

use crate::cli::args::{Cli, Commands};
use crate::cli::ci;
use crate::cli::machine::{self, FailureCode};
use crate::cli::style::Theme;
use crate::config;
use crate::config::gates;

use super::bump::{BumpOutput, run_bump};
use super::changelog::run_changelog;
use super::doctor::run_doctor;
use super::external::run_external;
use super::open::run_open;
use super::plan::run_plan;
use super::plugin::run_plugin;
use super::publish::run_publish;

/// Reports a failure.
///
/// When the invocation asked for machine output, the failure is one JSON envelope on stderr; when
/// it did not, the rendered line is byte-for-byte the one this command always printed. On a detected
/// CI either of them is preceded by one `::error::` annotation on the same stream (S4).
pub fn print_error(code: FailureCode, msg: impl std::fmt::Display) {
    let text = msg.to_string();
    ci::annotate_error(&text);
    if machine::try_emit_failure(code, &text) {
        return;
    }
    let theme = Theme::stderr();
    eprintln!("{} {}", theme.error("Error:"), text);
}

/// Reports a failure whose human line carries its own marker instead of `Error:`.
///
/// The machine path folds the marker into `error.message`, so the diagnosis reads the same in both
/// channels; the human path keeps the exact line it printed before.
pub fn print_error_prefixed(code: FailureCode, prefix: &str, msg: impl std::fmt::Display) {
    let text = msg.to_string();
    let message = format!("{prefix} {text}");
    ci::annotate_error(&message);
    if machine::try_emit_failure(code, &message) {
        return;
    }
    let theme = Theme::stderr();
    eprintln!("{} {}", theme.error(prefix), text);
}

pub fn run(args: Cli) -> i32 {
    machine::select(&args.command);
    ci::select(ci::CiMode::from_flags(args.ci, args.no_ci));
    // The same seam `--ci` uses (S1): one global flag resolved before dispatch picks a command, so
    // no command can be the reason a plugin's diagnostics are unreachable. It is written to the
    // plugin layer's own state rather than threaded through the manager or the driver constructor,
    // which are public and wide-fanout, and which must not learn about the command line.
    crate::plugin::diag::select(args.verbose);
    match args.command {
        Commands::Init {
            update,
            force,
            path,
            no_template,
        } => match crate::init::run_init(path, update, force, no_template) {
            Ok(()) => 0,
            Err(e) => {
                print_error(FailureCode::Init, e);
                1
            }
        },
        Commands::Changelog { command } => run_changelog(args.config.as_deref(), command),
        Commands::Doctor { check_changelog, json } => {
            let config = match load_config(args.config) {
                Ok(c) => c,
                Err(code) => return code,
            };
            run_doctor(&config, check_changelog, json)
        }
        Commands::Bump {
            level,
            dry_run,
            skip_preflight,
            first_release,
            defer_publish,
            json,
            report,
            ..
        } => {
            let config = match load_config(args.config) {
                Ok(c) => c,
                Err(code) => return code,
            };
            run_bump(
                &config,
                level,
                dry_run,
                &skip_preflight,
                first_release,
                defer_publish,
                BumpOutput { json, report },
            )
        }
        Commands::Plan {
            level,
            json,
            skip_preflight,
            first_release,
            ..
        } => {
            let config = match load_config(args.config) {
                Ok(c) => c,
                Err(code) => return code,
            };
            run_plan(&config, level, &skip_preflight, first_release, json)
        }
        Commands::Publish {
            tag,
            commit,
            notes_file,
            require_gate,
            dry_run,
        } => {
            let config = match load_config(args.config) {
                Ok(c) => c,
                Err(code) => return code,
            };
            let required = match parse_gates(&require_gate) {
                Ok(required) => required,
                Err(e) => {
                    print_error(FailureCode::Publish, e);
                    return 1;
                }
            };
            run_publish(
                &config,
                crate::bump::PublishRequest {
                    tag,
                    commit,
                    notes_file,
                    require_gate: required,
                    dry_run,
                },
            )
        }
        Commands::Attest {
            gate,
            commit,
            conclusion,
            run,
        } => {
            let config = match load_config(args.config) {
                Ok(c) => c,
                Err(code) => return code,
            };
            run_attest(&config, &gate, &commit, conclusion.into(), run.as_deref())
        }
        Commands::Open {
            target,
            print_url,
            browser,
        } => run_open(args.config.as_deref(), target.as_deref(), print_url, browser.as_deref()),
        Commands::Plugin { command } => run_plugin(args.config, command),
        Commands::External(ref ext_args) => run_external(ext_args),
    }
}

pub fn load_config(config_path: Option<PathBuf>) -> Result<config::Config, i32> {
    load_config_with_path(config_path).map(|(config, _)| config)
}

/// `cutver attest`: records one gate's verdict for one commit (#326 S8).
///
/// The command records what somebody else verified; it verifies nothing itself, which is why the
/// docs say an attestation is only as trustworthy as the machine that wrote it (S12).
fn run_attest(
    config: &config::Config,
    gate: &str,
    commit: &str,
    conclusion: gates::Conclusion,
    run: Option<&str>,
) -> i32 {
    let gate = match gates::GateName::parse(gate) {
        Ok(gate) => gate,
        Err(e) => {
            print_error(FailureCode::Attest, e);
            return 1;
        }
    };
    match gates::record(config, &gate, commit, conclusion, run) {
        Ok(recorded) => {
            println!(
                "attested '{}' as {} for commit {} ({})",
                recorded.attestation.gate, recorded.attestation.conclusion, recorded.attestation.commit, recorded.path
            );
            0
        }
        Err(e) => {
            print_error(FailureCode::Attest, e);
            1
        }
    }
}

/// Parses every `--require-gate` value into a gate name before anything is read or tagged.
fn parse_gates(declared: &[String]) -> Result<Vec<gates::GateName>, gates::GateNameError> {
    declared.iter().map(gates::GateName::parse).collect()
}

/// Loads the config and returns it alongside the resolved config file path.
///
/// The path is what `plugin remove` edits with `toml_edit`: an explicit
/// `--config` is used verbatim (resolved against the current directory), and a
/// discovered config is always the `cutver.toml` inside the resolved root.
/// Discovery itself stays in [`config::discover`] (Pillar V.6).
pub fn load_config_with_path(config_path: Option<PathBuf>) -> Result<(config::Config, PathBuf), i32> {
    match config_path {
        Some(path) => {
            let resolved = absolute_config_path(path);
            config::load(&resolved)
                .map(|config| (config, resolved))
                .map_err(report_config_error)
        }
        None => {
            let start_dir = match std::env::current_dir() {
                Ok(dir) => dir,
                Err(e) => {
                    print_error(
                        FailureCode::Config,
                        format!("unable to determine current directory: {e}"),
                    );
                    return Err(1);
                }
            };
            config::discover(&start_dir)
                .map(|config| {
                    let path = config.root_dir.join("cutver.toml");
                    (config, path)
                })
                .map_err(report_config_error)
        }
    }
}

fn absolute_config_path(path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        return path;
    }

    std::env::current_dir().map(|dir| dir.join(&path)).unwrap_or(path)
}

fn report_config_error(error: config::ConfigError) -> i32 {
    print_error_prefixed(FailureCode::Config, "Error loading config:", error);
    1
}

pub fn resolve_changelog_path(config_override: Option<&Path>, path: Option<PathBuf>) -> Result<PathBuf, i32> {
    if let Some(p) = path {
        return Ok(p);
    }

    let config_result = match config_override {
        Some(p) => config::load(p),
        None => match std::env::current_dir() {
            Ok(dir) => config::discover(dir),
            Err(e) => {
                print_error(
                    FailureCode::Changelog,
                    format!("unable to determine current directory: {e}"),
                );
                return Err(1);
            }
        },
    };

    match config_result {
        Ok(config) => {
            if let Some(ref cl_path) = config.changelog.path {
                let cl_pb = Path::new(cl_path);
                if cl_pb.is_absolute() {
                    Ok(cl_pb.to_path_buf())
                } else {
                    Ok(config.root_dir.join(cl_pb))
                }
            } else {
                let candidate = config.root_dir.join("CHANGELOG.md");
                if candidate.exists() {
                    Ok(candidate)
                } else {
                    let fallback = PathBuf::from("CHANGELOG.md");
                    if fallback.exists() {
                        Ok(fallback)
                    } else {
                        print_error(
                            FailureCode::Changelog,
                            "no changelog path configured and CHANGELOG.md not found",
                        );
                        Err(1)
                    }
                }
            }
        }
        Err(e) => {
            let fallback = PathBuf::from("CHANGELOG.md");
            if fallback.exists() {
                Ok(fallback)
            } else {
                print_error(FailureCode::Changelog, e);
                Err(1)
            }
        }
    }
}
