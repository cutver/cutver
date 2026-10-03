use std::path::Path;
use std::process::{Command, Output};

use super::types::Error;

pub fn run_git(repo: impl AsRef<Path>, args: &[&str]) -> Result<Output, Error> {
    Command::new("git")
        .current_dir(repo)
        .args(args)
        .output()
        .map_err(|e| Error::Command {
            command: args.join(" "),
            source: e,
        })
}

pub fn stdout_text(output: Output, command: &str) -> Result<String, Error> {
    if !output.status.success() {
        return Err(Error::Status {
            command: command.into(),
            status: output.status,
        });
    }
    String::from_utf8(output.stdout).map_err(|e| Error::Output {
        source: std::io::Error::new(std::io::ErrorKind::InvalidData, e),
    })
}
