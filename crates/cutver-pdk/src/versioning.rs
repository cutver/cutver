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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_versioning_dto_roundtrip() {
        let req = VersioningComputeRequest {
            root_dir: "/workspace".to_string(),
            current_version: "1.0.0".to_string(),
            bump_level: "auto".to_string(),
            tag_prefix: "v".to_string(),
            previous_tag: Some("v1.0.0".to_string()),
            commits: vec!["feat: new feature".to_string(), "fix: bugfix".to_string()],
        };
        let json = serde_json::to_string(&req).unwrap();
        let deserialized: VersioningComputeRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req, deserialized);

        let req_none = VersioningComputeRequest {
            root_dir: "/workspace".to_string(),
            current_version: "0.1.0".to_string(),
            bump_level: "minor".to_string(),
            tag_prefix: "v".to_string(),
            previous_tag: None,
            commits: vec![],
        };
        let json_none = serde_json::to_string(&req_none).unwrap();
        assert!(!json_none.contains("previous_tag"));
        let deserialized_none: VersioningComputeRequest = serde_json::from_str(&json_none).unwrap();
        assert_eq!(req_none, deserialized_none);

        let res = VersioningComputeResponse {
            next_version: "1.1.0".to_string(),
            rationale: Some("found 1 feature commit".to_string()),
        };
        let res_json = serde_json::to_string(&res).unwrap();
        let deserialized_res: VersioningComputeResponse = serde_json::from_str(&res_json).unwrap();
        assert_eq!(res, deserialized_res);

        let res_no_rat = VersioningComputeResponse {
            next_version: "1.0.1".to_string(),
            rationale: None,
        };
        let res_no_rat_json = serde_json::to_string(&res_no_rat).unwrap();
        assert!(!res_no_rat_json.contains("rationale"));
        let deserialized_no_rat: VersioningComputeResponse = serde_json::from_str(&res_no_rat_json).unwrap();
        assert_eq!(res_no_rat, deserialized_no_rat);
    }
}
