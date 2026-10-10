//! Minimal https URL validation using standard-library string parsing.
//!
//! No URL crate is introduced (Pillar IV.3): the registry contract only needs an
//! absolute `https://` check and, for releases, a `.wasm` path suffix.

use std::fmt;

/// Error returned when a URL is not a valid, secure Cutver registry URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UrlError {
    /// The value does not start with `https://`.
    NotHttps { value: String },
    /// The authority/host component is empty.
    MissingHost { value: String },
    /// The authority/host contains characters that cannot be a host.
    InvalidHost { value: String },
    /// The URL path does not end in `.wasm`.
    NotWasmExtension { value: String },
}

impl fmt::Display for UrlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotHttps { value } => {
                write!(f, "'{value}' is not an absolute https:// URL")
            }
            Self::MissingHost { value } => write!(f, "'{value}' has no host"),
            Self::InvalidHost { value } => write!(f, "'{value}' has an invalid host"),
            Self::NotWasmExtension { value } => {
                write!(f, "'{value}' does not end in '.wasm'")
            }
        }
    }
}

impl std::error::Error for UrlError {}

/// An absolute `https://` URL validated at construction.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HttpsUrl(String);

impl HttpsUrl {
    /// Parses `value`, failing closed unless it is an absolute https URL with a sane host.
    pub fn parse(value: &str) -> Result<Self, UrlError> {
        let Some(rest) = value.strip_prefix("https://") else {
            return Err(UrlError::NotHttps {
                value: value.to_string(),
            });
        };

        let host = rest.split('/').next().unwrap_or("");
        if host.is_empty() {
            return Err(UrlError::MissingHost {
                value: value.to_string(),
            });
        }

        let invalid = host
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || c == '@' || c == '\\');
        if invalid {
            return Err(UrlError::InvalidHost {
                value: value.to_string(),
            });
        }

        Ok(Self(value.to_string()))
    }

    /// Returns the validated URL as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the path component without the leading slash (empty when absent).
    pub fn path(&self) -> &str {
        let rest = self.0.strip_prefix("https://").unwrap_or(&self.0);
        rest.split_once('/').map(|(_, path)| path).unwrap_or("")
    }
}

impl fmt::Display for HttpsUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A release artifact URL: an [`HttpsUrl`] whose path ends in `.wasm`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WasmAssetUrl(HttpsUrl);

impl WasmAssetUrl {
    /// Parses `value`, requiring a secure URL with a `.wasm` path suffix.
    pub fn parse(value: &str) -> Result<Self, UrlError> {
        let url = HttpsUrl::parse(value)?;
        if url.path().ends_with(".wasm") {
            return Ok(Self(url));
        }

        Err(UrlError::NotWasmExtension {
            value: value.to_string(),
        })
    }

    /// Returns the validated URL as a string slice.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for WasmAssetUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
