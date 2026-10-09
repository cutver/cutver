use serde::{Deserialize, Serialize};

/// Request payload sent to a plugin for reading a manifest version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestReadRequest {
    /// Workspace-relative path to the manifest file (strictly normalized forward slashes).
    pub path: String,
    /// Raw byte content of the manifest file.
    pub content: String,
}

/// Response payload returned by a plugin after reading a manifest version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestReadResponse {
    /// Extracted SemVer string (e.g. "1.2.3").
    pub version: String,
}

/// Request payload sent to a plugin for writing an updated version into a manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestWriteRequest {
    /// Workspace-relative path to the manifest file (strictly normalized forward slashes).
    pub path: String,
    /// Raw byte content of the original manifest file.
    pub content: String,
    /// Previous version string before the bump.
    pub current_version: String,
    /// Next target version string to write.
    pub next_version: String,
}

/// Response payload returned by a plugin after mutating a manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestWriteResponse {
    /// Updated manifest content with surgical edits preserved.
    pub content: String,
}

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
}
