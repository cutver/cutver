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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lifecycle_dto_roundtrip() {
        let pre_req = PreBumpPayload {
            root_dir: "/workspace".to_string(),
            current_version: "1.0.0".to_string(),
            next_version: "1.1.0".to_string(),
            bump_level: "minor".to_string(),
            tag_name: "v1.1.0".to_string(),
            dry_run: false,
        };
        let pre_json = serde_json::to_string(&pre_req).unwrap();
        let deserialized_pre: PreBumpPayload = serde_json::from_str(&pre_json).unwrap();
        assert_eq!(pre_req, deserialized_pre);

        let pre_res = PreBumpResponse {
            allow: true,
            reason: None,
        };
        let res_json = serde_json::to_string(&pre_res).unwrap();
        assert!(!res_json.contains("reason"));
        let deserialized_res: PreBumpResponse = serde_json::from_str(&res_json).unwrap();
        assert_eq!(pre_res, deserialized_res);

        let post_bump = PostBumpPayload {
            root_dir: "/workspace".to_string(),
            current_version: "1.0.0".to_string(),
            next_version: "1.1.0".to_string(),
            tag_name: "v1.1.0".to_string(),
            modified_files: vec!["Cargo.toml".to_string()],
            dry_run: false,
        };
        let post_json = serde_json::to_string(&post_bump).unwrap();
        let deserialized_post: PostBumpPayload = serde_json::from_str(&post_json).unwrap();
        assert_eq!(post_bump, deserialized_post);

        let post_rel = PostReleasePayload {
            root_dir: "/workspace".to_string(),
            version: "1.1.0".to_string(),
            tag_name: "v1.1.0".to_string(),
            commit_sha: "abcdef123456".to_string(),
            dry_run: false,
        };
        let post_rel_json = serde_json::to_string(&post_rel).unwrap();
        let deserialized_rel: PostReleasePayload = serde_json::from_str(&post_rel_json).unwrap();
        assert_eq!(post_rel, deserialized_rel);
    }
}
