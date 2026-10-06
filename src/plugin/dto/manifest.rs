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
