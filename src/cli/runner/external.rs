use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::cli::machine::{self, FailureCode};
use crate::cli::style::Theme;

/// Spawns an external plugin subcommand formatted as `cutver-<subcommand>` from `$PATH`.
pub fn run_external(args: &[String]) -> i32 {
    run_external_with_path(args, std::env::var_os("PATH"))
}

/// Dispatches an external subcommand searching `path_var` instead of the process `PATH`.
///
/// The process environment is read at exactly one place, [`run_external`]; everything below the
/// boundary takes the search path as an argument, so a test can point the search at a scratch
/// directory without writing to a variable every other thread in the test binary can read (#352).
pub fn run_external_with_path(args: &[String], path_var: Option<OsString>) -> i32 {
    let Some(subcmd) = args.first() else {
        return 1;
    };

    let Some(binary_path) = resolve_binary(subcmd, path_var.as_deref()) else {
        print_not_found_error(subcmd);
        return 1;
    };

    execute_binary(&binary_path, &args[1..])
}

fn resolve_binary(subcmd: &str, path_var: Option<&OsStr>) -> Option<PathBuf> {
    let base = format!("cutver-{subcmd}");
    let path_var = path_var?;
    let extensions: &[&str] = if cfg!(windows) {
        &[".exe", ".cmd", ".bat", ""]
    } else {
        &[""]
    };

    for dir in std::env::split_paths(path_var) {
        for ext in extensions {
            let candidate = dir.join(format!("{base}{ext}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn execute_binary(path: &Path, trailing_args: &[String]) -> i32 {
    let mut cmd = build_command(path);
    cmd.args(trailing_args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    match cmd.status() {
        Ok(status) => status.code().unwrap_or(1),
        Err(err) => {
            let text = format!("error: failed to execute external subcommand: {err}");
            let where_ = format!("Where: executing '{path:?}'");
            let fix = "Fix: verify binary permissions and system environment.";
            if !machine::try_emit_failure(FailureCode::External, &format!("{text}\n  {where_}\n  {fix}")) {
                let theme = Theme::stderr();
                eprintln!(
                    "{}\n  {}\n  {}",
                    theme.error(text),
                    theme.muted(where_),
                    theme.muted(fix)
                );
            }
            1
        }
    }
}

fn build_command(path: &Path) -> Command {
    let is_batch = cfg!(windows)
        && path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("bat") || ext.eq_ignore_ascii_case("cmd"))
            .unwrap_or(false);

    if is_batch {
        let mut cmd = Command::new("cmd");
        cmd.arg("/C").arg(path);
        cmd
    } else {
        Command::new(path)
    }
}

fn print_not_found_error(subcmd: &str) {
    let binary = format!("cutver-{subcmd}");
    let text = format!("error: unknown subcommand '{subcmd}'.");
    let where_ = format!("Where: searching for '{binary}' in system PATH");
    let fix = format!("Fix: ensure '{binary}' is installed and present in your system PATH, or run 'cutver --help'.");
    if machine::try_emit_failure(FailureCode::External, &format!("{text}\n  {where_}\n  {fix}")) {
        return;
    }
    let theme = Theme::stderr();
    eprintln!(
        "{}\n  {}\n  {}",
        theme.error(text),
        theme.muted(where_),
        theme.muted(fix)
    );
}
