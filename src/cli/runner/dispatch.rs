use std::path::{Path, PathBuf};

use crate::cli::args::{Cli, Commands};
use crate::cli::style::Theme;
use crate::config;

use super::bump::run_bump;
use super::changelog::run_changelog;
use super::doctor::run_doctor;
use super::external::run_external;
use super::open::run_open;
use super::plugin::run_plugin;

pub fn print_error(msg: impl std::fmt::Display) {
    let theme = Theme::stderr();
    eprintln!("{} {}", theme.error("Error:"), msg);
}

pub fn run(args: Cli) -> i32 {
    match args.command {
        Commands::Init {
            update,
            force,
            path,
            no_template,
        } => match crate::init::run_init(path, update, force, no_template) {
            Ok(()) => 0,
            Err(e) => {
                print_error(e);
                1
            }
        },
        Commands::Changelog { command } => run_changelog(args.config.as_deref(), command),
        Commands::Doctor { check_changelog } => {
            let config = match load_config(args.config) {
                Ok(c) => c,
                Err(code) => return code,
            };
            run_doctor(&config, check_changelog)
        }
        Commands::Bump {
            level,
            dry_run,
            skip_preflight,
            first_release,
        } => {
            let config = match load_config(args.config) {
                Ok(c) => c,
                Err(code) => return code,
            };
            run_bump(&config, level, dry_run, &skip_preflight, first_release)
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
                    print_error(format!("unable to determine current directory: {e}"));
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
    let theme = Theme::stderr();
    eprintln!("{} {error}", theme.error("Error loading config:"));
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
                print_error(format!("unable to determine current directory: {e}"));
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
                        print_error("no changelog path configured and CHANGELOG.md not found");
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
                print_error(e);
                Err(1)
            }
        }
    }
}
