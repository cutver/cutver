use serde::{Deserialize, Serialize};

/// Context payload passed to a plugin for generating or transforming changelog notes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangelogRenderRequest {
    /// Workspace root directory.
    pub root_dir: String,
    /// Target release version string.
    pub version: String,
    /// Release tag name.
    pub tag_name: String,
    /// Previous release tag name if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_tag: Option<String>,
    /// Release date string (e.g. "2025-05-18").
    pub release_date: String,
    /// Raw unformatted commit entries.
    #[serde(default)]
    pub commits: Vec<PluginCommitEntry>,
}

/// A single commit summary entry passed to changelog plugins.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginCommitEntry {
    /// Git commit SHA hash.
    pub sha: String,
    /// Commit header line.
    pub message: String,
    /// Conventional commit type if detected (e.g. "feat", "fix").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    /// Conventional commit scope if detected (e.g. "core", "cli").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    /// Author name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author_name: Option<String>,
    /// Pull request reference if extracted (e.g. "#42").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pr_number: Option<String>,
    /// Whether the commit introduces a breaking change.
    #[serde(default)]
    pub is_breaking: bool,
}

/// Response returned by a changelog plugin containing rendered notes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangelogRenderResponse {
    /// Rendered Markdown content of the changelog section body.
    pub body: String,
}
