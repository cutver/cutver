use std::fs;
use std::io;
use std::path::Path;
use std::time::Duration;

use crate::bump::Error;
use crate::preflight::{Ended, run_supervised};

pub fn read(path: impl AsRef<Path>) -> Result<String, Error> {
    let path = path.as_ref();
    fs::read_to_string(path).map_err(|e| Error::Read {
        path: path.display().to_string(),
        source: e,
    })
}

/// Reads a target that may legitimately not exist yet, and fails closed on anything else (#261).
///
/// An absent file is a normal state for a sink the run is about to generate and for a changelog a
/// run is about to create: `None` says "there was nothing here", which is a state rollback can
/// restore by removing the file. Any other read failure is *not* normal, because the bytes returned
/// here are the only copy rollback will restore. Backing an unreadable file up as empty and writing
/// that back would turn a real file into a zero-byte one.
pub(crate) fn read_existing(path: &str) -> Result<Option<String>, Error> {
    match fs::read_to_string(path) {
        Ok(content) => Ok(Some(content)),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(Error::ReadUnreadable {
            path: path.to_string(),
            source,
        }),
    }
}

pub const KNOWN_LOCKFILES: &[&str] = &[
    "Cargo.lock",
    "package-lock.json",
    "pnpm-lock.yaml",
    "yarn.lock",
    "bun.lockb",
    "bun.lock",
    "gradle.lockfile",
    "poetry.lock",
    "Pipfile.lock",
    "composer.lock",
    "uv.lock",
    "pdm.lock",
];

pub fn is_known_lockfile(path: impl AsRef<Path>) -> bool {
    path.as_ref()
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|name| KNOWN_LOCKFILES.contains(&name))
}

/// Renders a configured shell command against the release interpolation context. The one authority
/// shared by the hooks and `[publish].commands` (Pillar V.6).
pub fn format_command(template: &str, context: &crate::changelog::InterpolationContext) -> Result<String, Error> {
    crate::changelog::interpolate_string(template, context).map_err(Error::Changelog)
}

/// Runs one `[publish]` command through the shared bounded spawn, so a command that outlives
/// `timeout_secs` is killed with its whole process tree instead of holding the release open.
pub fn run_publish_command(command: &str, current_dir: &Path, timeout_secs: Option<u64>) -> Result<(), Error> {
    let timeout = timeout_secs.map(Duration::from_secs);
    match run_supervised(command, current_dir, timeout) {
        Ok(Ended::Exited(status)) if status.success() => Ok(()),
        Ok(Ended::Exited(status)) => Err(Error::PublishCommandFailed {
            command: command.to_string(),
            status,
        }),
        Ok(Ended::TimedOut { timeout, elapsed }) => Err(Error::PublishCommandTimeout {
            command: command.to_string(),
            timeout: timeout.as_secs(),
            elapsed_ms: elapsed.as_millis(),
        }),
        Err(e) => Err(Error::PublishCommandSpawn {
            command: command.to_string(),
            source: e.into_io(),
        }),
    }
}
