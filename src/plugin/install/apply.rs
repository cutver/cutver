//! Phase 2 of `cutver plugin add`: atomic I/O with an RAII rollback guard (D19).
//!
//! Order: create the cache root, write the bytes to a temp file inside the
//! target directory, verify SHA-256 through the single canonical verifier,
//! rename into place, then write the config. If the config write fails the
//! artifact guard restores (or removes) the artifact, so the config can never
//! point at a file the apply did not leave in place.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use semver::Version;

use super::plan::InstallPlan;
use crate::atomic;
use crate::plugin::driver::wasm::verify_sha256;
use crate::plugin::error::{InstallError, PluginError};
use crate::plugin::registry::{MAX_ARTIFACT_BYTES, Transport};
use crate::plugin::types::PluginName;

static TEMP_SEQ: AtomicU64 = AtomicU64::new(0);

/// Result of applying an [`InstallPlan`].
#[derive(Debug, Clone)]
pub struct InstallOutcome {
    pub name: PluginName,
    pub config_path: PathBuf,
    pub artifact_path: PathBuf,
    pub artifact_url: String,
    pub version: Version,
    pub digest: String,
    pub index: String,
}

/// Phase 2: download, verify, install the artifact and write the config.
pub fn apply_add(plan: &InstallPlan, transport: &dyn Transport) -> Result<InstallOutcome, PluginError> {
    let bytes = transport.fetch(&plan.artifact_url, MAX_ARTIFACT_BYTES)?;
    let mut guard = install_artifact(plan, &bytes)?;
    write_config(plan)?;
    guard.disarm();

    Ok(outcome(plan))
}

/// Creates the cache root, verifies and renames the artifact, returning an armed guard.
fn install_artifact(plan: &InstallPlan, bytes: &[u8]) -> Result<ArtifactGuard, PluginError> {
    let Some(dir) = plan.artifact_path.parent() else {
        return Err(write_error(&plan.artifact_path, "artifact path has no parent directory").into());
    };
    fs::create_dir_all(dir).map_err(|err| write_error(dir, &err.to_string()))?;

    let temp = temp_path(&plan.artifact_path, dir)?;
    fs::write(&temp, bytes).map_err(|err| write_error(&temp, &err.to_string()))?;
    let mut temp_guard = TempGuard::new(&temp);

    verify_sha256(&plan.name, bytes, Some(&plan.digest))?;

    let original = read_original(&plan.artifact_path)?;
    fs::rename(&temp, &plan.artifact_path).map_err(|err| write_error(&plan.artifact_path, &err.to_string()))?;
    temp_guard.disarm();

    Ok(ArtifactGuard {
        path: plan.artifact_path.clone(),
        original,
        active: true,
    })
}

/// Writes the plan's rendered document atomically.
fn write_config(plan: &InstallPlan) -> Result<(), PluginError> {
    atomic::write_atomic(&plan.config_path, plan.new_text.as_bytes()).map_err(|err| PluginError::PluginConfigWrite {
        path: plan.config_path.display().to_string(),
        reason: err.to_string(),
    })
}

/// Produces the outcome echoed to the user after a successful apply.
fn outcome(plan: &InstallPlan) -> InstallOutcome {
    InstallOutcome {
        name: plan.name.clone(),
        config_path: plan.config_path.clone(),
        artifact_path: plan.artifact_path.clone(),
        artifact_url: plan.artifact_url.clone(),
        version: plan.version.clone(),
        digest: plan.digest.clone(),
        index: plan.index.clone(),
    }
}

/// Builds a unique same-directory temp path so the final rename is atomic.
fn temp_path(target: &Path, dir: &Path) -> Result<PathBuf, InstallError> {
    let Some(name) = target.file_name().and_then(|part| part.to_str()) else {
        return Err(write_error(target, "artifact path has no file name"));
    };
    let seq = TEMP_SEQ.fetch_add(1, Ordering::Relaxed);

    Ok(dir.join(format!(".{name}.cutver-tmp-{}-{seq}", std::process::id())))
}

/// Reads the pre-existing artifact bytes, failing closed when a present file cannot be read.
///
/// A read failure must not be degraded to `None`: the rollback guard treats
/// `None` as "no original" and deletes the path, which would destroy a
/// pre-existing artifact that should have been restored.
fn read_original(path: &Path) -> Result<Option<Vec<u8>>, InstallError> {
    if !path.is_file() {
        return Ok(None);
    }

    let bytes = fs::read(path).map_err(|err| InstallError::ArtifactRead {
        path: path.display().to_string(),
        reason: err.to_string(),
    })?;

    Ok(Some(bytes))
}

/// Builds an artifact-write error naming the path.
fn write_error(path: &Path, reason: &str) -> InstallError {
    InstallError::ArtifactWrite {
        path: path.display().to_string(),
        reason: reason.to_string(),
    }
}

/// RAII guard restoring the artifact if the config write fails.
struct ArtifactGuard {
    path: PathBuf,
    original: Option<Vec<u8>>,
    active: bool,
}

impl ArtifactGuard {
    /// Leaves the installed artifact in place.
    fn disarm(&mut self) {
        self.active = false;
    }
}

impl Drop for ArtifactGuard {
    /// Best-effort rollback: `Drop` cannot propagate an I/O failure, so the
    /// restore result is deliberately discarded. The dangerous read failure
    /// is instead handled before the rename in [`read_original`].
    fn drop(&mut self) {
        if !self.active {
            return;
        }

        let _ = restore(&self.path, self.original.as_deref());
    }
}

/// Best-effort rollback: restore the original bytes, or remove a fresh artifact.
fn restore(path: &Path, original: Option<&[u8]>) -> std::io::Result<()> {
    match original {
        Some(bytes) => fs::write(path, bytes),
        None => fs::remove_file(path),
    }
}

/// RAII guard removing the temp file unless the rename succeeds.
struct TempGuard<'a> {
    path: &'a Path,
    active: bool,
}

impl<'a> TempGuard<'a> {
    fn new(path: &'a Path) -> Self {
        Self { path, active: true }
    }

    fn disarm(&mut self) {
        self.active = false;
    }
}

impl Drop for TempGuard<'_> {
    fn drop(&mut self) {
        if !self.active {
            return;
        }

        let _ = fs::remove_file(self.path);
    }
}
