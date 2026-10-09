use serde::{Deserialize, Serialize};

use crate::{Capability, PluginOperation};

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
    ///
    /// The call is any value convertible into a `(Capability, PluginOperation)`
    /// pair. The host's `PluginCall` dispatch enum provides that conversion, so
    /// the coupling lives on the host side and this crate stays a pure contract.
    pub fn new<Req: Serialize>(
        call: impl Into<(Capability, PluginOperation)>,
        payload: &Req,
    ) -> Result<Self, serde_json::Error> {
        let (capability, operation) = call.into();
        Ok(Self {
            capability,
            operation,
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

        let invocation = PluginInvocation::new((Capability::ManifestV1, PluginOperation::Read), &payload).unwrap();

        assert_eq!(invocation.capability, Capability::ManifestV1);
        assert_eq!(invocation.operation, PluginOperation::Read);
        assert_eq!(invocation.payload, payload);
    }
}
