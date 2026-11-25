use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

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

/// `ETXTBSY`, "Text file busy": the kernel refuses to `exec` a file that still has a descriptor
/// open for writing. Linux 26, and the same number on the other unices this program targets.
#[cfg(unix)]
const ETXTBSY: i32 = 26;

/// Spawn attempts allowed when the kernel refused because the target was still open for writing.
const BUSY_ATTEMPTS: u32 = 5;

/// Delay between those attempts.
///
/// #375: `cargo test` runs a suite in threads of one process, so any thread's `fork` can duplicate
/// an open write descriptor into an unrelated child, where it outlives the write that created it.
/// That window is milliseconds and does not belong to the file's writer, which is why the same job
/// passed on rerun with nothing changed. A file genuinely held for writing still fails, with the
/// same message, once the attempts run out — on a kernel that enforces the refusal. Linux does.
/// macOS, measured on this change's own CI, executes the file instead, so the retry is inert there
/// and costs nothing.
const BUSY_DELAY: Duration = Duration::from_millis(10);

fn execute_binary(path: &Path, trailing_args: &[String]) -> i32 {
    let mut cmd = build_command(path);
    cmd.args(trailing_args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    match wait_for_spawn(&mut cmd) {
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

/// Waits for the child, retrying only the one error that is a race rather than a verdict.
fn wait_for_spawn(cmd: &mut Command) -> std::io::Result<std::process::ExitStatus> {
    let mut attempt = 1;
    loop {
        match cmd.status() {
            Err(err) if attempt < BUSY_ATTEMPTS && is_executable_busy(&err) => {
                std::thread::sleep(BUSY_DELAY);
                attempt += 1;
            }
            outcome => return outcome,
        }
    }
}

#[cfg(unix)]
fn is_executable_busy(err: &std::io::Error) -> bool {
    err.raw_os_error() == Some(ETXTBSY)
}

/// No descriptor-sharing between processes exists in the same shape on Windows, and the batch path
/// goes through `cmd`, so there is nothing to retry.
#[cfg(not(unix))]
fn is_executable_busy(_: &std::io::Error) -> bool {
    false
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
