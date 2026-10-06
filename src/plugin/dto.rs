pub mod changelog;
pub mod lifecycle;
pub mod manifest;

pub use changelog::{ChangelogRenderRequest, ChangelogRenderResponse, PluginCommitEntry};
pub use lifecycle::{PostBumpPayload, PostReleasePayload, PreBumpPayload, PreBumpResponse};
pub use manifest::{ManifestReadRequest, ManifestReadResponse, ManifestWriteRequest, ManifestWriteResponse};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_dto_roundtrip() {
        let req = ManifestReadRequest {
            path: "charts/app/Chart.yaml".to_string(),
            content: "version: 0.1.0\n".to_string(),
        };
        let serialized = serde_json::to_string(&req).unwrap();
        let deserialized: ManifestReadRequest = serde_json::from_str(&serialized).unwrap();
        assert_eq!(req, deserialized);

        let res = ManifestReadResponse {
            version: "0.1.0".to_string(),
        };
        let serialized_res = serde_json::to_string(&res).unwrap();
        let deserialized_res: ManifestReadResponse = serde_json::from_str(&serialized_res).unwrap();
        assert_eq!(res, deserialized_res);

        let write_req = ManifestWriteRequest {
            path: "charts/app/Chart.yaml".to_string(),
            content: "version: 0.1.0\n".to_string(),
            current_version: "0.1.0".to_string(),
            next_version: "0.2.0".to_string(),
        };
        let write_json = serde_json::to_string(&write_req).unwrap();
        let deserialized_write: ManifestWriteRequest = serde_json::from_str(&write_json).unwrap();
        assert_eq!(write_req, deserialized_write);

        let write_res = ManifestWriteResponse {
            content: "version: 0.2.0\n".to_string(),
        };
        let write_res_json = serde_json::to_string(&write_res).unwrap();
        let deserialized_write_res: ManifestWriteResponse = serde_json::from_str(&write_res_json).unwrap();
        assert_eq!(write_res, deserialized_write_res);
    }

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
        };
        let json = serde_json::to_string(&req).unwrap();
        let deserialized: ChangelogRenderRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req, deserialized);

        let res = ChangelogRenderResponse {
            body: "### Features\n- new plugin system".to_string(),
        };
        let res_json = serde_json::to_string(&res).unwrap();
        let deserialized_res: ChangelogRenderResponse = serde_json::from_str(&res_json).unwrap();
        assert_eq!(res, deserialized_res);
    }
}
