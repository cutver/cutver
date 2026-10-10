//! Validated SHA-256 digest newtype.

use std::fmt;

/// Number of hexadecimal characters in a SHA-256 digest.
pub const SHA256_HEX_LEN: usize = 64;

/// Error returned when a value is not exactly 64 lowercase hexadecimal characters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sha256DigestError {
    value: String,
}

impl Sha256DigestError {
    /// The offending value, for callers that need to embed it in a domain error.
    pub fn value(&self) -> &str {
        &self.value
    }
}

impl fmt::Display for Sha256DigestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "'{}' is not 64 lowercase hexadecimal characters", self.value)
    }
}

impl std::error::Error for Sha256DigestError {}

/// A SHA-256 digest validated at construction (Pillar III.1).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Sha256Digest(String);

impl Sha256Digest {
    /// Parses `value`, failing closed unless it is exactly 64 lowercase hex chars.
    pub fn parse(value: &str) -> Result<Self, Sha256DigestError> {
        let shaped = value.len() == SHA256_HEX_LEN
            && value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));

        if shaped {
            return Ok(Self(value.to_string()));
        }

        Err(Sha256DigestError {
            value: value.to_string(),
        })
    }

    /// Returns the validated digest as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Sha256Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
