use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseCapabilityError(pub String);

impl fmt::Display for ParseCapabilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unsupported capability '{}': expected one of manifest.v1, lifecycle.v1, changelog.v1, versioning.v1",
            self.0
        )
    }
}

impl std::error::Error for ParseCapabilityError {}

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsePluginOperationError(pub String);

impl fmt::Display for ParsePluginOperationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unsupported operation '{}': expected one of read, write, on_pre_bump, on_post_bump, on_post_release, render, compute",
            self.0
        )
    }
}

impl std::error::Error for ParsePluginOperationError {}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_parse_error_messages_match_wire_domains() {
        let capability = "unknown.v1".parse::<Capability>().unwrap_err();
        assert_eq!(
            capability.to_string(),
            "unsupported capability 'unknown.v1': expected one of manifest.v1, lifecycle.v1, changelog.v1, versioning.v1"
        );

        let operation = "bogus".parse::<PluginOperation>().unwrap_err();
        assert_eq!(
            operation.to_string(),
            "unsupported operation 'bogus': expected one of read, write, on_pre_bump, on_post_bump, on_post_release, render, compute"
        );
    }
}
