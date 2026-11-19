//! The recorded documents and the store they live in: one JSON object per gate.
//!
//! Reads are total on purpose (see [`read`]): the channel is data, so a file that does not parse is
//! "no attestation", never a crash in the middle of a release.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::name::{Conclusion, GateName};
use super::table::is_exemptible;
use crate::atomic::write_atomic;
use crate::config::types::Config;
use crate::git::CommitSha;

/// One recorded verdict, exactly as the file spells it. Key order is the file's contract: the
/// document is written from this struct, never from a map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attestation {
    /// What was verified.
    pub gate: GateName,
    /// For which commit, as the 40 (or 64) hex characters git prints.
    pub commit: String,
    /// With which conclusion.
    pub conclusion: Conclusion,
    /// And when: the UTC day the verdict was recorded, from the clock the changelog heading uses.
    pub verified_at: String,
    /// The run that produced the verdict, when the gate names one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run: Option<String>,
}

impl Attestation {
    /// Whether this attestation answers for `commit`. Commit ids are hexadecimal, so the comparison
    /// ignores case instead of trusting the spelling a caller passed.
    pub fn covers(&self, commit: &str) -> bool {
        self.commit.eq_ignore_ascii_case(commit)
    }
}

/// A recorded verdict, plus where it landed.
#[derive(Debug)]
pub struct Recorded {
    pub attestation: Attestation,
    /// The file, spelled relative to the workspace root with forward slashes.
    pub path: String,
}

/// Why `cutver attest` could not record a verdict.
#[derive(Debug, Error)]
pub enum AttestError {
    #[error(
        "commit '{commit}' cannot be attested: {reason}\n  Where: `cutver attest`, before anything is written\n  Fix: pass the full commit the gate verified — the sha `git rev-parse HEAD` prints, not a tag, a branch or a short form — so the attestation can be matched against the release commit."
    )]
    InvalidCommit { commit: String, reason: String },
    #[error(
        "failed to write the attestation for gate '{gate}' to '{path}': {source}\n  Where: `cutver attest`, while recording the verdict\n  Fix: make '{dir}' writable and re-run. The write is atomic, so no partial attestation is left behind."
    )]
    Write {
        gate: String,
        path: String,
        dir: String,
        #[source]
        source: io::Error,
    },
}

/// The directory this workspace's attestations live in, resolved against the workspace root.
pub fn dir(config: &Config) -> PathBuf {
    let declared = Path::new(&config.gates.dir);
    if declared.is_absolute() {
        declared.to_path_buf()
    } else {
        config.root_dir.join(declared)
    }
}

/// The file one gate's attestation lives in.
pub fn path_of(config: &Config, gate: &GateName) -> PathBuf {
    dir(config).join(gate.file_name())
}

/// A path inside the workspace, as messages spell it: relative to the root, with forward slashes.
pub fn relative(config: &Config, path: &Path) -> String {
    let shown = path.strip_prefix(&config.root_dir).unwrap_or(path);
    shown.to_string_lossy().replace('\\', "/")
}

/// Records one verdict, creating the attestation directory when it does not exist yet.
///
/// The commit is parsed here, so a value that could never match a release commit is refused before
/// anything is written.
pub fn record(
    config: &Config,
    gate: &GateName,
    commit: &str,
    conclusion: Conclusion,
    run: Option<&str>,
) -> Result<Recorded, AttestError> {
    let commit = CommitSha::parse(commit).map_err(|error| AttestError::InvalidCommit {
        commit: commit.to_string(),
        reason: match error {
            crate::git::Error::InvalidSha { reason, .. } => reason,
            other => other.to_string(),
        },
    })?;
    let attestation = Attestation {
        gate: gate.clone(),
        commit: commit.as_str().to_string(),
        conclusion,
        verified_at: crate::changelog::format_date(std::time::SystemTime::now()),
        run: run.map(str::to_string),
    };
    let path = path_of(config, gate);
    write(config, gate, &path, &attestation)?;
    Ok(Recorded {
        attestation,
        path: relative(config, &path),
    })
}

/// Writes one document into the gate channel, one file per gate, atomically.
fn write(config: &Config, gate: &GateName, path: &Path, attestation: &Attestation) -> Result<(), AttestError> {
    let directory = dir(config);
    let failure = |source: io::Error| AttestError::Write {
        gate: gate.to_string(),
        path: relative(config, path),
        dir: relative(config, &directory),
        source,
    };
    fs::create_dir_all(&directory).map_err(failure)?;
    let mut bytes = serde_json::to_vec(attestation).map_err(|error| failure(io::Error::other(error.to_string())))?;
    bytes.push(b'\n');
    write_atomic(path, &bytes).map_err(|source| failure(io::Error::other(source.to_string())))
}

/// The attestation recorded for `gate`, when this workspace carries one that parses and names the
/// same gate. Anything else — a missing file, a truncated document, a file that names another gate —
/// is "no attestation", never an error: a forged or damaged file must not crash a release, and it
/// cannot vouch for anything either.
pub fn read(config: &Config, gate: &GateName) -> Option<Attestation> {
    let text = fs::read_to_string(path_of(config, gate)).ok()?;
    let attestation: Attestation = serde_json::from_str(&text).ok()?;
    (attestation.gate == *gate).then_some(attestation)
}

/// The directory `cutver publish` declares to the clean-tree guard (#326 S10).
///
/// An attestation is an input to the closing phase, not a change in the tree, so the directory it
/// lives in is exempt exactly like `--notes-file` is. [`is_exemptible`] refuses a declaration that
/// would cover the whole workspace, and the guard itself refuses a directory outside the root.
pub fn declared_input(config: &Config) -> Option<PathBuf> {
    if !is_exemptible(Path::new(&config.gates.dir)) {
        return None;
    }
    // Belt and braces: the configuration already rejects a `dir` that walks upwards, and the guard
    // already refuses a declared directory that is not strictly inside the workspace.
    let path = dir(config);
    path.starts_with(&config.root_dir).then_some(path)
}
