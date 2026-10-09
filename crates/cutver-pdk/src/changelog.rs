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
    /// Repository URL of the project, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    /// Compare URL between the previous and current tags, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compare_url: Option<String>,
    /// Whether the release is a prerelease.
    #[serde(default)]
    pub is_prerelease: bool,
    /// Contributors to the release, with first-contribution provenance.
    #[serde(default)]
    pub contributors: Vec<PluginContributor>,
}

/// A contributor to the release, with first-contribution provenance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginContributor {
    /// Contributor display name or resolved handle.
    pub name: String,
    /// True when this is the contributor's first release in the repository.
    #[serde(default)]
    pub is_first_contribution: bool,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_changelog_dto_roundtrip() {
        let entry = PluginCommitEntry {
            sha: "12345678".to_string(),
            message: "feat: new plugin system".to_string(),
            r#type: Some("feat".to_string()),
            scope: Some("plugin".to_string()),
            author_name: Some("Row".to_string()),
            pr_number: Some("#42".to_string()),
            is_breaking: false,
        };
        let req = ChangelogRenderRequest {
            root_dir: "/workspace".to_string(),
            version: "1.1.0".to_string(),
            tag_name: "v1.1.0".to_string(),
            previous_tag: Some("v1.0.0".to_string()),
            release_date: "2025-05-18".to_string(),
            commits: vec![entry],
            repository: Some("https://github.com/Row0902/cutver".to_string()),
            compare_url: Some("https://github.com/Row0902/cutver/compare/v1.0.0...v1.1.0".to_string()),
            is_prerelease: false,
            contributors: vec![PluginContributor {
                name: "Row".to_string(),
                is_first_contribution: true,
            }],
        };
        let json = serde_json::to_string(&req).unwrap();
        let deserialized: ChangelogRenderRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req, deserialized);

        // Backwards compatibility: a payload without any of the v2 fields deserializes.
        let legacy = serde_json::json!({
            "root_dir": "/workspace",
            "version": "1.1.0",
            "tag_name": "v1.1.0",
            "release_date": "2025-05-18",
            "commits": []
        });
        let legacy_req: ChangelogRenderRequest = serde_json::from_value(legacy).unwrap();
        assert_eq!(legacy_req.previous_tag, None);
        assert_eq!(legacy_req.repository, None);
        assert_eq!(legacy_req.compare_url, None);
        assert!(!legacy_req.is_prerelease);
        assert!(legacy_req.contributors.is_empty());

        let res = ChangelogRenderResponse {
            body: "### Features\n- new plugin system".to_string(),
        };
        let res_json = serde_json::to_string(&res).unwrap();
        let deserialized_res: ChangelogRenderResponse = serde_json::from_str(&res_json).unwrap();
        assert_eq!(res, deserialized_res);
    }
}
