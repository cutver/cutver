use std::io::ErrorKind;
use std::process::{Command, Stdio};

use crate::cli::style::Theme;

/// Spawns an external plugin subcommand formatted as `cutver-<subcommand>` from `$PATH`.
pub fn run_external(args: &[String]) -> i32 {
    let Some(subcmd) = args.first() else {
        return 1;
    };

    let binary = format!("cutver-{subcmd}");
    let trailing_args = &args[1..];

    match Command::new(&binary)
        .args(trailing_args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
    {
        Ok(status) => status.code().unwrap_or(1),
        Err(err) if err.kind() == ErrorKind::NotFound => {
            let theme = Theme::stderr();
            eprintln!(
                "{}\n  {}\n  {}",
                theme.error(format!("error: unknown subcommand '{subcmd}'.")),
                theme.muted(format!("Where: searching for '{binary}' in system PATH")),
                theme.muted(format!(
                    "Fix: ensure '{binary}' is installed and present in your system PATH, or run 'cutver --help'."
                ))
            );
            1
        }
        Err(err) => {
            let theme = Theme::stderr();
            eprintln!(
                "{}\n  {}\n  {}",
                theme.error(format!(
                    "error: failed to execute external subcommand '{subcmd}': {err}"
                )),
                theme.muted(format!("Where: executing '{binary}'")),
                theme.muted("Fix: verify binary permissions and system environment.")
            );
            1
        }
    }
}
