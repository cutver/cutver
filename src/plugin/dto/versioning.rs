use serde::{Deserialize, Serialize};

/// Input payload sent to a versioning plugin during version computation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersioningComputeRequest {
    /// Workspace root directory path.
    pub root_dir: String,
    /// Current workspace SemVer version string.
    pub current_version: String,
    /// Bump level requested ("major", "minor", "patch", "auto").
    pub bump_level: String,
    /// Git tag prefix configured in cutver.toml (e.g. "v").
    pub tag_prefix: String,
    /// Previous release Git tag if one exists in the repository.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_tag: Option<String>,
    /// List of commit messages since the previous release tag.
    #[serde(default)]
    pub commits: Vec<String>,
}

/// Output payload returned by a versioning plugin containing the computed version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersioningComputeResponse {
    /// Next SemVer version string computed by the plugin.
    pub next_version: String,
    /// Optional human-readable rationale explaining the version computation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
}
