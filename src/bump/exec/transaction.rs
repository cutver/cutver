use std::io;
use std::path::Path;

use crate::atomic;
use crate::bump::Error;

/// One rollback step that did not succeed.
///
/// Rollback runs from `Drop`, which cannot return an error, so every failure is collected here
/// instead of being discarded (Pillar III.4, *Zero Swallowed Errors*). [`MutationTransaction::abort`]
/// folds them into the error that triggered the rollback, so the operator learns the tree may no
/// longer match its committed content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RollbackFailure {
    /// The path that could not be restored, or the repository for a failed unstage.
    pub target: String,
    /// `restore` or `unstage`.
    pub action: &'static str,
    /// What the failing step reported.
    pub detail: String,
}

impl RollbackFailure {
    fn describe(&self) -> String {
        format!("{} of '{}' failed: {}", self.action, self.target, self.detail)
    }
}

pub struct MutationTransaction<'a> {
    repo: &'a Path,
    /// The bytes to restore per path. `None` means the file did not exist when the run started, so
    /// rollback deletes it: writing an empty file back is data loss dressed as a restoration.
    backups: Vec<(String, Option<String>)>,
    needs_unstage: bool,
    active: bool,
}

impl<'a> MutationTransaction<'a> {
    pub fn new(repo: &'a Path) -> Self {
        Self {
            repo,
            backups: Vec::new(),
            needs_unstage: false,
            active: true,
        }
    }

    /// Records what rollback must restore. `None` means the target did not exist yet.
    pub fn record_backup(&mut self, path: String, original: Option<String>) {
        self.backups.push((path, original));
    }

    pub fn mark_staged(&mut self) {
        self.needs_unstage = true;
    }

    pub fn commit(mut self) {
        self.active = false;
    }

    /// Rolls back now and folds every rollback failure into `error` (#261).
    ///
    /// The pipeline routes each fallible step through this instead of relying on `Drop`: a drop
    /// guard cannot return an error, and a silently failed restore is worse than the original
    /// failure, because the tree stays corrupted while the operator reads only the first error.
    pub fn abort(&mut self, error: Error) -> Error {
        let failures = self.rollback();
        if failures.is_empty() {
            return error;
        }
        Error::RollbackFailed {
            error: Box::new(error),
            failures: join_failures(&failures),
        }
    }

    /// Restores every recorded backup in reverse order and unstages, collecting failures.
    fn rollback(&mut self) -> Vec<RollbackFailure> {
        if !self.active {
            return Vec::new();
        }
        self.active = false;
        let mut failures = Vec::new();
        for (path, original) in self.backups.iter().rev() {
            if let Err(detail) = restore(path, original.as_deref()) {
                failures.push(RollbackFailure {
                    target: path.clone(),
                    action: "restore",
                    detail,
                });
            }
        }
        if self.needs_unstage
            && let Err(detail) = unstage(self.repo)
        {
            failures.push(RollbackFailure {
                target: self.repo.display().to_string(),
                action: "unstage",
                detail,
            });
        }
        failures
    }
}

impl Drop for MutationTransaction<'_> {
    fn drop(&mut self) {
        let failures = self.rollback();
        // Last-resort channel: every pipeline step routes its failure through `abort`, so reaching
        // this point means a failure escaped that path. Printing beats leaving a corrupted tree
        // unreported.
        for failure in &failures {
            eprintln!(
                "warning: rollback could not complete — {} (the working tree may no longer match its committed content)",
                failure.describe()
            );
        }
    }
}

fn join_failures(failures: &[RollbackFailure]) -> String {
    failures
        .iter()
        .map(RollbackFailure::describe)
        .collect::<Vec<_>>()
        .join("; ")
}

/// Restores one target to its recorded state: its bytes, or nothing when it never existed.
fn restore(path: &str, original: Option<&str>) -> Result<(), String> {
    match original {
        Some(bytes) => atomic::write_atomic(path, bytes).map_err(|e| e.to_string()),
        None => match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            // Already gone is exactly the state rollback wanted.
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        },
    }
}

/// Unstages whatever this run staged, reporting a failure instead of discarding it.
pub fn unstage(repo: &Path) -> Result<(), String> {
    let output = std::process::Command::new("git")
        .current_dir(repo)
        .args(["reset", "HEAD", "--quiet"])
        .output()
        .map_err(|e| format!("could not run `git reset HEAD --quiet`: {e}"))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stderr = stderr.trim();
    if stderr.is_empty() {
        return Err(format!("`git reset HEAD --quiet` exited with {}", output.status));
    }
    Err(format!(
        "`git reset HEAD --quiet` exited with {}: {stderr}",
        output.status
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("cutver-tx-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// #261 AC2: a restore that fails must reach the operator instead of being discarded by the drop
    /// guard, which cannot return an error.
    #[test]
    fn a_failed_restore_is_reported_instead_of_swallowed() {
        let dir = scratch("restore-fail");
        let target = dir.join("manifest.json");
        // The path became a directory after the backup was taken, so writing its bytes back fails.
        std::fs::create_dir_all(&target).unwrap();
        let mut transaction = MutationTransaction::new(&dir);
        transaction.record_backup(
            target.to_string_lossy().into_owned(),
            Some("{\"version\":\"1.0.0\"}".to_string()),
        );

        let err = transaction.abort(Error::Read {
            path: "trigger.json".to_string(),
            source: std::io::Error::other("boom"),
        });
        let message = err.to_string();

        assert!(message.contains("Rollback failed"), "{message}");
        assert!(message.contains("manifest.json"), "{message}");
    }

    /// #261: a successful rollback leaves the triggering error untouched, so callers that match on
    /// the variant keep working.
    #[test]
    fn a_successful_rollback_returns_the_original_error_unchanged() {
        let dir = scratch("restore-ok");
        let target = dir.join("manifest.json");
        std::fs::write(&target, "{\"version\":\"9.9.9\"}").unwrap();
        let mut transaction = MutationTransaction::new(&dir);
        transaction.record_backup(target.to_string_lossy().into_owned(), Some("original".to_string()));

        let err = transaction.abort(Error::Read {
            path: "trigger.json".to_string(),
            source: std::io::Error::other("boom"),
        });

        assert!(
            matches!(err, Error::Read { .. }),
            "expected the original error, got {err:?}"
        );
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "original");
    }

    /// #261: an absent target is deleted on rollback, never recreated as an empty file.
    #[test]
    fn an_absent_target_is_removed_by_rollback() {
        let dir = scratch("absent");
        let target = dir.join("generated.ts");
        std::fs::write(&target, "generated").unwrap();
        let mut transaction = MutationTransaction::new(&dir);
        transaction.record_backup(target.to_string_lossy().into_owned(), None);

        let err = transaction.abort(Error::Read {
            path: "trigger.json".to_string(),
            source: std::io::Error::other("boom"),
        });

        assert!(
            matches!(err, Error::Read { .. }),
            "a clean rollback reports nothing extra: {err:?}"
        );
        assert!(!target.exists(), "an absent target must be removed, not left empty");
    }
}
