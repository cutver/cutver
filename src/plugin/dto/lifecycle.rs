use serde::{Deserialize, Serialize};

/// Payload delivered to a plugin during the `on_pre_bump` lifecycle event.
/// Computed entirely in-memory during Phase 1 (preflight validation).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreBumpPayload {
    /// Workspace root directory path.
    pub root_dir: String,
    /// Current workspace version.
    pub current_version: String,
    /// Target version to be bumped to.
    pub next_version: String,
    /// Bump level ("major", "minor", "patch", etc.).
    pub bump_level: String,
    /// Target release tag name.
    pub tag_name: String,
    /// Whether this is a simulation run (`--dry-run`).
    pub dry_run: bool,
}

/// Structured response returned by a plugin from `on_pre_bump`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreBumpResponse {
    /// Whether the preflight check passed. If false, release aborts.
    pub allow: bool,
    /// Optional rejection reason or informational notes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Payload delivered to a plugin during `on_post_bump` after files are mutated in memory/staged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PostBumpPayload {
    /// Workspace root directory path.
    pub root_dir: String,
    /// Previous version before the bump.
    pub current_version: String,
    /// Next target version.
    pub next_version: String,
    /// Target release tag name.
    pub tag_name: String,
    /// List of paths modified during the bump.
    #[serde(default)]
    pub modified_files: Vec<String>,
    /// Whether this is a simulation run (`--dry-run`).
    pub dry_run: bool,
}

/// Payload delivered to a plugin during `on_post_release` after Git commit/tag/push succeeds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PostReleasePayload {
    /// Workspace root directory path.
    pub root_dir: String,
    /// Released version.
    pub version: String,
    /// Release tag name.
    pub tag_name: String,
    /// Commit SHA created for the release.
    pub commit_sha: String,
    /// Whether this is a simulation run (`--dry-run`).
    pub dry_run: bool,
}
