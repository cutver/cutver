//! The two names a gate attestation carries: the gate's own name, and the verdict's name.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

/// The longest gate name accepted, so that `[gates] dir` plus the file name stays a legal file name.
const MAX_GATE_NAME: usize = 100;

/// Why a string cannot name a gate.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("invalid gate name '{name}': {reason}")]
pub struct GateNameError {
    pub name: String,
    pub reason: String,
}

/// One gate's name: a label in messages, and the stem of the file its attestation lives in.
///
/// Parsed at every boundary — `cutver attest <GATE>`, `publish --require-gate <NAME>` and
/// `[gates] require` — so a name that could leave the attestation directory is unrepresentable.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GateName(String);

impl GateName {
    pub fn parse(text: impl AsRef<str>) -> Result<Self, GateNameError> {
        let text = text.as_ref();
        let invalid = |reason: String| GateNameError {
            name: text.to_string(),
            reason,
        };
        if text.is_empty() {
            return Err(invalid("it is empty".to_string()));
        }
        if text.len() > MAX_GATE_NAME {
            return Err(invalid(format!("it is longer than {MAX_GATE_NAME} characters")));
        }
        if text.starts_with('.') {
            return Err(invalid(
                "it starts with '.', which would hide the file it names".to_string(),
            ));
        }
        if let Some(offender) = text
            .chars()
            .find(|c| !(c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')))
        {
            return Err(invalid(format!(
                "'{offender}' is not a letter, a digit, '-' or '_' ('.' is allowed inside the name)"
            )));
        }
        Ok(Self(text.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The file this gate's attestation lives in, inside a gates directory.
    pub fn file_name(&self) -> String {
        format!("{}.json", self.0)
    }
}

impl fmt::Display for GateName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Serialize for GateName {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

/// A gate name read from a document is parsed, not validated later: `[gates] require` and every
/// attestation file go through the same rule as the command line.
impl<'de> Deserialize<'de> for GateName {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::parse(&text).map_err(serde::de::Error::custom)
    }
}

/// The verdict a gate reached. `failure` is recorded, not discarded: `publish` refuses on it in the
/// same voice it refuses a missing attestation, and `plan` reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Conclusion {
    Success,
    Failure,
}

impl Conclusion {
    pub fn as_str(self) -> &'static str {
        match self {
            Conclusion::Success => "success",
            Conclusion::Failure => "failure",
        }
    }

    /// Whether this verdict lets a release that requires the gate publish.
    pub fn passed(self) -> bool {
        matches!(self, Conclusion::Success)
    }
}

impl fmt::Display for Conclusion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
