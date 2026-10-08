use serde::{Deserialize, Serialize};

/// Envelope wrapping every plugin invocation across all runtimes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginInvocation {
    /// Capability being invoked, e.g. "changelog.v1".
    pub capability: String,
    /// Operation within the capability, e.g. "render".
    pub operation: String,
    /// Capability-specific request DTO.
    pub payload: serde_json::Value,
}

impl PluginInvocation {
    /// Builds an invocation, serializing the capability request DTO into `payload`.
    pub fn new<Req: Serialize>(
        capability: impl Into<String>,
        operation: impl Into<String>,
        payload: &Req,
    ) -> Result<Self, serde_json::Error> {
        Ok(Self {
            capability: capability.into(),
            operation: operation.into(),
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
            capability: "changelog.v1".to_string(),
            operation: "render".to_string(),
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

        let invocation = PluginInvocation::new("manifest.v1", "read", &payload).unwrap();

        assert_eq!(invocation.capability, "manifest.v1");
        assert_eq!(invocation.operation, "read");
        assert_eq!(invocation.payload, payload);
    }
}
