use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use thiserror::Error;

static ATOMIC_SEQ: AtomicU64 = AtomicU64::new(0);

/// Total attempts for the final replace-rename, the first one included.
const RENAME_ATTEMPTS: u32 = 3;
/// Pause between replace-rename attempts: long enough to outlast a momentary handle
/// collision, short enough not to hide a genuine permission error.
const RENAME_RETRY_DELAY: Duration = Duration::from_millis(20);

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid target path '{path}'")]
    InvalidPath { path: String },
    #[error("failed to write temp file '{path}': {source}")]
    TempWrite { path: String, source: io::Error },
    #[error("failed to rename '{src}' to '{dst}': {source}")]
    Rename {
        src: String,
        dst: String,
        source: io::Error,
    },
}

/// RAII guard that removes the temporary file at `path` on drop unless explicitly disarmed.
///
/// Cleanup invariants:
/// - Initialized in the armed state (`active: true`) as soon as the temp file path is prepared.
/// - If `File::create`, writing, syncing, or `fs::rename` fails, the guard drops while still
///   armed and cleans up the temporary file from disk (best-effort).
/// - If `File::create` failed before creating the file, `remove_file` in drop safely ignores NotFound.
/// - Only when `fs::rename` succeeds is `disarm()` called, ensuring the file remains in place.
struct TempFileGuard<'a> {
    path: &'a Path,
    active: bool,
}

impl<'a> TempFileGuard<'a> {
    fn new(path: &'a Path) -> Self {
        Self { path, active: true }
    }

    fn disarm(&mut self) {
        self.active = false;
    }
}

impl Drop for TempFileGuard<'_> {
    fn drop(&mut self) {
        if self.active {
            let _ = fs::remove_file(self.path);
        }
    }
}

/// Whether `err` is a collision the platform reports for a replace-rename that a bounded
/// retry can outlast, as opposed to a genuine, permanent failure.
///
/// Windows `MoveFileEx` with replace-existing answers `ERROR_ACCESS_DENIED` or
/// `ERROR_SHARING_VIOLATION` while another handle holds the destination (an editor, an
/// indexer, an antivirus, or a concurrent writer). POSIX `rename` has no such failure
/// mode, so nothing is retried there and its behaviour is unchanged.
#[cfg(windows)]
fn is_transient_rename_error(err: &io::Error) -> bool {
    const ERROR_ACCESS_DENIED: i32 = 5;
    const ERROR_SHARING_VIOLATION: i32 = 32;
    matches!(
        err.raw_os_error(),
        Some(ERROR_ACCESS_DENIED) | Some(ERROR_SHARING_VIOLATION)
    )
}

#[cfg(not(windows))]
fn is_transient_rename_error(_err: &io::Error) -> bool {
    false
}

/// Bounded retry policy for the final replace-rename.
struct RenamePlan {
    attempts: u32,
    delay: Duration,
    is_transient: fn(&io::Error) -> bool,
}

impl RenamePlan {
    /// What this process promises: `RENAME_ATTEMPTS` attempts, the platform's transient
    /// errors only, `RENAME_RETRY_DELAY` between attempts.
    fn platform() -> Self {
        Self {
            attempts: RENAME_ATTEMPTS,
            delay: RENAME_RETRY_DELAY,
            is_transient: is_transient_rename_error,
        }
    }

    /// Runs `rename`, retrying only the errors `is_transient` accepts.
    ///
    /// A permanent error fails on the first attempt, exhausted retries return the last
    /// error unchanged, and no error produces more than `attempts` calls.
    fn apply<F>(&self, tmp: &Path, dst: &Path, mut rename: F) -> io::Result<()>
    where
        F: FnMut(&Path, &Path) -> io::Result<()>,
    {
        let mut attempt = 1;
        loop {
            let err = match rename(tmp, dst) {
                Ok(()) => return Ok(()),
                Err(err) => err,
            };
            if attempt >= self.attempts || !(self.is_transient)(&err) {
                return Err(err);
            }
            if !self.delay.is_zero() {
                std::thread::sleep(self.delay);
            }
            attempt += 1;
        }
    }
}

fn temp_file_path(path: &Path) -> Result<PathBuf, Error> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let name = path
        .file_name()
        .ok_or_else(|| Error::InvalidPath {
            path: path.display().to_string(),
        })?
        .to_string_lossy();
    let pid = std::process::id();
    let seq = ATOMIC_SEQ.fetch_add(1, Ordering::Relaxed);
    Ok(dir.join(format!(".{name}.cutver-tmp-{pid}-{seq}")))
}

/// Write `contents` to `path` atomically using a unique same-directory temp file,
/// `sync_all`, and a final `rename`. Cleans up the temporary file on failure.
///
/// The replace is retried a bounded number of times for the transient collisions Windows
/// reports while another handle holds the destination; any other error, and exhausted
/// retries, are surfaced as-is with the destination left as it was.
pub fn write_atomic(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> Result<(), Error> {
    write_atomic_impl(path.as_ref(), contents.as_ref(), |file, data| {
        file.write_all(data)?;
        file.sync_all()
    })
}

fn write_atomic_impl<F>(path: &Path, contents: &[u8], write_and_sync: F) -> Result<(), Error>
where
    F: FnOnce(&mut fs::File, &[u8]) -> io::Result<()>,
{
    let tmp = temp_file_path(path)?;
    let mut guard = TempFileGuard::new(&tmp);

    {
        let mut file = fs::File::create(&tmp).map_err(|e| Error::TempWrite {
            path: tmp.display().to_string(),
            source: e,
        })?;

        write_and_sync(&mut file, contents).map_err(|e| Error::TempWrite {
            path: tmp.display().to_string(),
            source: e,
        })?;
    }

    RenamePlan::platform()
        .apply(&tmp, path, |from, to| fs::rename(from, to))
        .map_err(|e| Error::Rename {
            src: tmp.display().to_string(),
            dst: path.display().to_string(),
            source: e,
        })?;

    guard.disarm();
    Ok(())
}

#[cfg(test)]
mod tests;
