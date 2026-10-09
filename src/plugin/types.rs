use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use thiserror::Error;

/// Error returned when validating a `PluginName`.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PluginNameError {
    #[error("plugin name cannot be empty")]
    Empty,
    #[error("invalid plugin name '{name}': only ASCII alphanumeric characters, hyphens, and underscores are allowed")]
    InvalidCharacters { name: String },
}

/// Validated plugin name newtype ensuring ASCII alphanumeric, hyphens, and underscores, non-empty.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct PluginName(String);

impl PluginName {
    /// Creates a validated `PluginName`.
    pub fn new(name: impl Into<String>) -> Result<Self, PluginNameError> {
        let s = name.into();
        Self::validate(&s)?;
        Ok(Self(s))
    }

    /// Validates the plugin name format.
    fn validate(s: &str) -> Result<(), PluginNameError> {
        if s.is_empty() {
            return Err(PluginNameError::Empty);
        }
        let is_valid = s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
        if !is_valid {
            return Err(PluginNameError::InvalidCharacters { name: s.to_string() });
        }
        Ok(())
    }

    /// Returns a string slice representing the plugin name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for PluginName {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for PluginName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for PluginName {
    type Err = PluginNameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl TryFrom<String> for PluginName {
    type Error = PluginNameError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<PluginName> for String {
    fn from(name: PluginName) -> Self {
        name.0
    }
}

/// Canonical capabilities supported by the Cutver Microkernel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Capability {
    #[serde(rename = "manifest.v1")]
    ManifestV1,
    #[serde(rename = "lifecycle.v1")]
    LifecycleV1,
    #[serde(rename = "changelog.v1")]
    ChangelogV1,
    #[serde(rename = "versioning.v1")]
    VersioningV1,
}

impl Capability {
    /// Returns the canonical capability wire identifier.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::ManifestV1 => "manifest.v1",
            Self::LifecycleV1 => "lifecycle.v1",
            Self::ChangelogV1 => "changelog.v1",
            Self::VersioningV1 => "versioning.v1",
        }
    }
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Capability {
    type Err = ParseCapabilityError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "manifest.v1" => Ok(Self::ManifestV1),
            "lifecycle.v1" => Ok(Self::LifecycleV1),
            "changelog.v1" => Ok(Self::ChangelogV1),
            "versioning.v1" => Ok(Self::VersioningV1),
            _ => Err(ParseCapabilityError(s.to_string())),
        }
    }
}

/// Error returned when parsing an unrecognized capability string.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("unsupported capability '{0}': expected one of manifest.v1, lifecycle.v1, changelog.v1, versioning.v1")]
pub struct ParseCapabilityError(pub String);

/// Canonical operation within a capability. Wire form is snake_case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginOperation {
    Read,
    Write,
    OnPreBump,
    OnPostBump,
    OnPostRelease,
    Render,
    Compute,
}

impl PluginOperation {
    /// Returns the canonical operation wire identifier.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::OnPreBump => "on_pre_bump",
            Self::OnPostBump => "on_post_bump",
            Self::OnPostRelease => "on_post_release",
            Self::Render => "render",
            Self::Compute => "compute",
        }
    }
}

impl fmt::Display for PluginOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for PluginOperation {
    type Err = ParsePluginOperationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "read" => Ok(Self::Read),
            "write" => Ok(Self::Write),
            "on_pre_bump" => Ok(Self::OnPreBump),
            "on_post_bump" => Ok(Self::OnPostBump),
            "on_post_release" => Ok(Self::OnPostRelease),
            "render" => Ok(Self::Render),
            "compute" => Ok(Self::Compute),
            _ => Err(ParsePluginOperationError(s.to_string())),
        }
    }
}

/// Error returned when parsing an unrecognized operation string.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error(
    "unsupported operation '{0}': expected one of read, write, on_pre_bump, on_post_bump, on_post_release, render, compute"
)]
pub struct ParsePluginOperationError(pub String);

/// A dispatchable plugin call. Capability and operation are coupled by construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PluginCall {
    ManifestRead,
    ManifestWrite,
    PreBump,
    PostBump,
    PostRelease,
    ChangelogRender,
    VersioningCompute,
}

impl PluginCall {
    /// Returns the capability this call targets.
    pub const fn capability(self) -> Capability {
        match self {
            Self::ManifestRead | Self::ManifestWrite => Capability::ManifestV1,
            Self::PreBump | Self::PostBump | Self::PostRelease => Capability::LifecycleV1,
            Self::ChangelogRender => Capability::ChangelogV1,
            Self::VersioningCompute => Capability::VersioningV1,
        }
    }

    /// Returns the operation this call performs.
    pub const fn operation(self) -> PluginOperation {
        match self {
            Self::ManifestRead => PluginOperation::Read,
            Self::ManifestWrite => PluginOperation::Write,
            Self::PreBump => PluginOperation::OnPreBump,
            Self::PostBump => PluginOperation::OnPostBump,
            Self::PostRelease => PluginOperation::OnPostRelease,
            Self::ChangelogRender => PluginOperation::Render,
            Self::VersioningCompute => PluginOperation::Compute,
        }
    }
}

/// Plugin runtime kind: WebAssembly or native system process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuntimeKind {
    Wasm,
    Process,
}

impl RuntimeKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Wasm => "wasm",
            Self::Process => "process",
        }
    }
}

impl fmt::Display for RuntimeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Sandboxed permissions configuration for WASM plugins.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionsConfig {
    #[serde(default)]
    pub network: Vec<String>,
    #[serde(default)]
    pub env: Vec<String>,
    #[serde(default)]
    pub filesystem: Vec<String>,
}

/// Plugin declaration parsed from `cutver.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginConfig {
    pub runtime: RuntimeKind,
    pub source: Option<String>,
    pub hash: Option<String>,
    pub command: Option<String>,
    #[serde(default)]
    pub capabilities: Vec<Capability>,
    #[serde(default)]
    pub events: Vec<String>,
    #[serde(default)]
    pub manifest_match: Vec<String>,
    #[serde(default)]
    pub permissions: PermissionsConfig,
    pub timeout_seconds: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_name_valid() {
        let name = PluginName::new("helm-adapter_v1").unwrap();
        assert_eq!(name.as_str(), "helm-adapter_v1");
        assert_eq!(name.to_string(), "helm-adapter_v1");
    }

    #[test]
    fn test_plugin_name_empty_fails() {
        let err = PluginName::new("").unwrap_err();
        assert_eq!(err, PluginNameError::Empty);
    }

    #[test]
    fn test_plugin_name_invalid_chars_fails() {
        let err = PluginName::new("helm/adapter").unwrap_err();
        assert_eq!(
            err,
            PluginNameError::InvalidCharacters {
                name: "helm/adapter".to_string()
            }
        );
    }

    #[test]
    fn test_capability_serialization_and_parsing() {
        let cap = Capability::ManifestV1;
        assert_eq!(cap.as_str(), "manifest.v1");
        assert_eq!(cap.to_string(), "manifest.v1");
        assert_eq!("lifecycle.v1".parse::<Capability>().unwrap(), Capability::LifecycleV1);
        assert!("unknown.v1".parse::<Capability>().is_err());

        let json = serde_json::to_string(&Capability::LifecycleV1).unwrap();
        assert_eq!(json, "\"lifecycle.v1\"");
        let deserialized: Capability = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, Capability::LifecycleV1);
    }

    #[test]
    fn test_operation_wire_form_roundtrip() {
        for (op, wire) in [
            (PluginOperation::Read, "read"),
            (PluginOperation::Write, "write"),
            (PluginOperation::OnPreBump, "on_pre_bump"),
            (PluginOperation::OnPostBump, "on_post_bump"),
            (PluginOperation::OnPostRelease, "on_post_release"),
            (PluginOperation::Render, "render"),
            (PluginOperation::Compute, "compute"),
        ] {
            assert_eq!(op.as_str(), wire);
            assert_eq!(wire.parse::<PluginOperation>().unwrap(), op);
            assert_eq!(serde_json::to_string(&op).unwrap(), format!("\"{wire}\""));
        }
        assert!("bogus".parse::<PluginOperation>().is_err());
    }

    #[test]
    fn test_plugin_call_couples_capability_and_operation() {
        assert_eq!(PluginCall::ManifestRead.capability(), Capability::ManifestV1);
        assert_eq!(PluginCall::ManifestRead.operation(), PluginOperation::Read);
        assert_eq!(PluginCall::PreBump.capability(), Capability::LifecycleV1);
        assert_eq!(PluginCall::PreBump.operation(), PluginOperation::OnPreBump);
        assert_eq!(PluginCall::ChangelogRender.capability(), Capability::ChangelogV1);
        assert_eq!(PluginCall::ChangelogRender.operation(), PluginOperation::Render);
        assert_eq!(PluginCall::VersioningCompute.capability(), Capability::VersioningV1);
        assert_eq!(PluginCall::VersioningCompute.operation(), PluginOperation::Compute);
    }

    #[test]
    fn test_runtime_kind_serde() {
        let wasm: RuntimeKind = serde_json::from_str("\"wasm\"").unwrap();
        assert_eq!(wasm, RuntimeKind::Wasm);
        let process: RuntimeKind = serde_json::from_str("\"process\"").unwrap();
        assert_eq!(process, RuntimeKind::Process);
    }

    #[test]
    fn test_plugin_config_toml_deserialize() {
        let toml_str = r#"
            runtime = "process"
            command = "./scripts/helm-version-adapter.sh"
            capabilities = ["manifest.v1"]
            manifest_match = ["Chart.yaml"]
            timeout_seconds = 10
        "#;
        let config: PluginConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.runtime, RuntimeKind::Process);
        assert_eq!(config.command.as_deref(), Some("./scripts/helm-version-adapter.sh"));
        assert_eq!(config.capabilities, vec![Capability::ManifestV1]);
        assert_eq!(config.manifest_match, vec!["Chart.yaml"]);
        assert_eq!(config.timeout_seconds, Some(10));
    }
}
