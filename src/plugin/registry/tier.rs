//! Registry trust tier — a closed set.

use std::fmt;
use std::str::FromStr;

/// Error returned when a tier string is not part of the closed set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TierParseError(pub String);

impl fmt::Display for TierParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unsupported tier '{}': expected one of official, verified, community",
            self.0
        )
    }
}

impl std::error::Error for TierParseError {}

/// Trust tier of a registry entry (Pillar V.2: exhaustive, no wildcard dispatch).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tier {
    /// Curated and maintained by the Cutver maintainers.
    Official,
    /// Third-party, reviewed against the registry policy.
    Verified,
    /// Community-published.
    Community,
}

impl Tier {
    /// Returns the canonical registry wire identifier.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Official => "official",
            Self::Verified => "verified",
            Self::Community => "community",
        }
    }
}

impl fmt::Display for Tier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Tier {
    type Err = TierParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        // A named fallback is required to parse arbitrary wire strings; every
        // match over the `Tier` enum itself stays exhaustive.
        match value {
            "official" => Ok(Self::Official),
            "verified" => Ok(Self::Verified),
            "community" => Ok(Self::Community),
            other => Err(TierParseError(other.to_string())),
        }
    }
}
