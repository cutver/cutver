use serde::{Deserialize, Serialize};

use crate::plugin::types::{Capability, PluginCall, PluginOperation};

/// Envelope wrapping every plugin invocation across all runtimes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginInvocation {
    /// Capability being invoked, e.g. "changelog.v1".
    pub capability: Capability,
    /// Operation within the capability, e.g. "render".
    pub operation: PluginOperation,
    /// Capability-specific request DTO.
    pub payload: serde_json::Value,
}

impl PluginInvocation {
    /// Builds an invocation from a coupled call and a serialized request DTO.
    pub fn new<Req: Serialize>(call: PluginCall, payload: &Req) -> Result<Self, serde_json::Error> {
        Ok(Self {
            capability: call.capability(),
            operation: call.operation(),
            payload: serde_json::to_value(payload)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_invocation_roundtrip() {
        let invocation = PluginInvocation {
            capability: Capability::ChangelogV1,
            operation: PluginOperation::Render,
            payload: serde_json::json!({
                "version": "1.0.0",
                "commits": [],
            }),
        };

        let serialized = serde_json::to_string(&invocation).unwrap();
        let deserialized: PluginInvocation = serde_json::from_str(&serialized).unwrap();

        assert_eq!(invocation, deserialized);
    }

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
