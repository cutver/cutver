use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::cli::style::Theme;

/// Spawns an external plugin subcommand formatted as `cutver-<subcommand>` from `$PATH`.
pub fn run_external(args: &[String]) -> i32 {
    let Some(subcmd) = args.first() else {
        return 1;
    };

    let Some(binary_path) = resolve_binary(subcmd) else {
        print_not_found_error(subcmd);
        return 1;
    };

    execute_binary(&binary_path, &args[1..])
}

fn resolve_binary(subcmd: &str) -> Option<PathBuf> {
    let base = format!("cutver-{subcmd}");
    let path_var = std::env::var_os("PATH")?;
    let extensions: &[&str] = if cfg!(windows) {
        &[".exe", ".cmd", ".bat", ""]
    } else {
        &[""]
    };

    for dir in std::env::split_paths(&path_var) {
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
            let theme = Theme::stderr();
            eprintln!(
                "{}\n  {}\n  {}",
                theme.error(format!("error: failed to execute external subcommand: {err}")),
                theme.muted(format!("Where: executing '{path:?}'")),
                theme.muted("Fix: verify binary permissions and system environment.")
            );
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
    let theme = Theme::stderr();
    let binary = format!("cutver-{subcmd}");
    eprintln!(
        "{}\n  {}\n  {}",
        theme.error(format!("error: unknown subcommand '{subcmd}'.")),
        theme.muted(format!("Where: searching for '{binary}' in system PATH")),
        theme.muted(format!(
            "Fix: ensure '{binary}' is installed and present in your system PATH, or run 'cutver --help'."
        ))
    );
}
