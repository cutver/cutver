//! Facade over the wire DTOs owned by `cutver-pdk`.
//!
//! Every capability request/response DTO and the invocation envelope are defined
//! exactly once, in the PDK. Re-exporting them here keeps every existing
//! `crate::plugin::dto::*` caller resolving against the same path.
pub use cutver_pdk::PROTOCOL_VERSION;
pub use cutver_pdk::{
    ChangelogRenderRequest, ChangelogRenderResponse, ManifestReadRequest, ManifestReadResponse, ManifestWriteRequest,
    ManifestWriteResponse, PluginCommitEntry, PluginContributor, PluginInvocation, PostBumpPayload, PostReleasePayload,
    PreBumpPayload, PreBumpResponse, VersioningComputeRequest, VersioningComputeResponse,
};

use serde::de::DeserializeOwned;

use crate::plugin::error::PluginError;
use crate::plugin::types::PluginName;

/// Decodes one plugin answer, refusing one that declares a protocol the host does not speak.
///
/// The answer is the capability's response DTO, and a plugin **may** put a top-level `protocol`
/// beside those fields (#369 S3). Three cases, and the middle one is the whole point of the field:
///
/// - **absent** — the plugin answers with the DTO alone, which is every plugin that exists today;
///   that is protocol [`PROTOCOL_VERSION`] by definition, so it is read as such.
/// - **present and equal** — the same thing, said out loud.
/// - **present and different** — [`PluginError::ProtocolMismatch`], naming both versions. Without
///   this check the extra key is silently ignored and a plugin built against a future shape fails
///   later with a decode error, or worse, decodes into a shape it did not mean.
///
/// The version is checked here rather than in a driver because a driver only serializes the request
/// and never sees the shape of the answer.
pub fn decode_answer<Res: DeserializeOwned>(
    name: &PluginName,
    capability: &str,
    answer: &[u8],
) -> Result<Res, PluginError> {
    let decoded: serde_json::Value = serde_json::from_slice(answer).map_err(|source| PluginError::InvalidPayload {
        name: name.clone(),
        capability: capability.to_string(),
        source,
    })?;

    if let Some(declared) = decoded.get("protocol") {
        // A declaration this host cannot read is still a declaration. Reading a non-string as
        // "absent" would bless `"protocol": 2` — the numeric style this very program uses for
        // `schemaVersion` — as protocol 1 and tell the author nothing, which is the failure mode
        // this field exists to end.
        let declared = declared.as_str().map_or_else(|| declared.to_string(), str::to_owned);
        if declared != PROTOCOL_VERSION {
            return Err(PluginError::ProtocolMismatch {
                name: name.clone(),
                host: PROTOCOL_VERSION,
                plugin: declared,
            });
        }
    }

    serde_json::from_value(decoded).map_err(|source| PluginError::InvalidPayload {
        name: name.clone(),
        capability: capability.to_string(),
        source,
    })
}

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
