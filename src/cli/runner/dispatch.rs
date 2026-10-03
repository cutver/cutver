use std::path::{Path, PathBuf};

use crate::cli::args::{Cli, Commands};
use crate::cli::style::Theme;
use crate::config;

use super::bump::run_bump;
use super::changelog::run_changelog;
use super::doctor::run_doctor;
use super::open::run_open;

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
    }
}

pub fn load_config(config_path: Option<PathBuf>) -> Result<config::Config, i32> {
    let config = match config_path {
        Some(path) => config::load(path),
        None => {
            let start_dir = match std::env::current_dir() {
                Ok(d) => d,
                Err(e) => {
                    print_error(format!("unable to determine current directory: {e}"));
                    return Err(1);
                }
            };
            config::discover(start_dir)
        }
    };
    config.map_err(|e| {
        let theme = Theme::stderr();
        eprintln!("{} {e}", theme.error("Error loading config:"));
        1
    })
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
