//! Facade over the wire DTOs owned by `cutver-pdk`.
//!
//! Every capability request/response DTO and the invocation envelope are defined
//! exactly once, in the PDK. Re-exporting them here keeps every existing
//! `crate::plugin::dto::*` caller resolving against the same path.
pub use cutver_pdk::{
    ChangelogRenderRequest, ChangelogRenderResponse, ManifestReadRequest, ManifestReadResponse, ManifestWriteRequest,
    ManifestWriteResponse, PluginCommitEntry, PluginContributor, PluginInvocation, PostBumpPayload, PostReleasePayload,
    PreBumpPayload, PreBumpResponse, VersioningComputeRequest, VersioningComputeResponse,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::types::{Capability, PluginCall, PluginOperation};

    #[test]
    fn test_plugin_invocation_new_builds_expected_envelope() {
        let payload = serde_json::json!({
            "path": "Chart.yaml",
            "content": "version: 1.0.0\n",
        });

        let invocation = PluginInvocation::new(PluginCall::ManifestRead, &payload).unwrap();

        assert_eq!(invocation.capability, Capability::ManifestV1);
        assert_eq!(invocation.operation, PluginOperation::Read);
        assert_eq!(invocation.payload, payload);
    }

    #[test]
    fn test_envelope_serializes_capability_before_operation() {
        let invocation = PluginInvocation::new(PluginCall::ChangelogRender, &serde_json::json!({})).unwrap();
        let serialized = serde_json::to_string(&invocation).unwrap();

        let capability_at = serialized.find("\"capability\":\"changelog.v1\"").unwrap();
        let operation_at = serialized.find("\"operation\":\"render\"").unwrap();
        assert!(capability_at < operation_at, "unexpected envelope order: {serialized}");
    }
}
