//! The `[gates]` table: where attestations live, and which gates a release requires.

use std::path::Path;

use serde::Deserialize;

use super::name::GateName;
use crate::config::types::ConfigError;

/// Where attestations live when `[gates] dir` is not declared.
pub const DEFAULT_DIR: &str = ".cutver/attestations";

/// The `[gates]` table, exactly as `cutver.toml` spells it.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Gates {
    /// Workspace-relative directory holding one JSON object per gate.
    #[serde(default = "default_dir")]
    pub dir: String,
    /// Gates a release requires an attestation for, in the order `cutver plan` reports them.
    #[serde(default)]
    pub require: Vec<GateName>,
}

fn default_dir() -> String {
    DEFAULT_DIR.to_string()
}

impl Default for Gates {
    fn default() -> Self {
        Self {
            dir: default_dir(),
            require: Vec::new(),
        }
    }
}

impl Gates {
    /// Accepts only a workspace-relative directory that stays inside the workspace.
    ///
    /// `cutver publish` exempts that directory from the clean-tree guard, because an attestation is
    /// an input to the command rather than a change in the tree (#326 S10). A directory spelled `.`,
    /// or one that walks upwards, would exempt the whole working tree — including the very changes
    /// the guard exists to see — so it is refused while the configuration loads, never narrowed
    /// silently afterwards.
    pub(crate) fn validate(&self) -> Result<(), ConfigError> {
        let dir = self.dir.as_str();
        let invalid = |reason: &str| {
            Err(ConfigError::GatesDirInvalid {
                dir: dir.to_string(),
                reason: reason.to_string(),
            })
        };
        if dir.trim().is_empty() {
            return invalid("it is empty");
        }
        if !is_exemptible(Path::new(dir)) {
            return invalid("it must start with a directory name: no root, no drive, no `.`, no `..`");
        }
        Ok(())
    }
}

/// Whether a declared `[gates] dir` may be exempted as a subtree: `.` and anything that walks
/// upwards would whitelist work the clean-tree guard exists to see.
pub(crate) fn is_exemptible(declared: &Path) -> bool {
    let mut components = declared.components();
    let Some(first) = components.next() else {
        return false;
    };
    if !matches!(first, std::path::Component::Normal(_)) {
        return false;
    }
    !declared
        .components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
}
