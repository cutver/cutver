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
    /// The value contains a backslash, which must never reach a URL (Pillar II.3).
    Backslash { value: String },
    /// The authority/host component is empty.
    MissingHost { value: String },
    /// The authority/host contains characters that cannot be a host.
    InvalidHost { value: String },
    /// The authority declares a port that is not a number in `1..=65535`.
    InvalidPort { value: String },
    /// The URL path does not end in `.wasm`.
    NotWasmExtension { value: String },
}

impl fmt::Display for UrlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotHttps { value } => {
                write!(f, "'{value}' is not an absolute https:// URL")
            }
            Self::Backslash { value } => {
                write!(f, "'{value}' contains a backslash; URLs must use forward slashes")
            }
            Self::MissingHost { value } => write!(f, "'{value}' has no host"),
            Self::InvalidHost { value } => write!(f, "'{value}' has an invalid host"),
            Self::InvalidPort { value } => write!(f, "'{value}' has an invalid port (expected 1-65535)"),
            Self::NotWasmExtension { value } => {
                write!(f, "'{value}' does not end in '.wasm'")
            }
        }
    }
}

impl std::error::Error for UrlError {}

/// Splits `host[:port]`, returning the host and the optional port text.
fn split_port(authority: &str) -> (&str, Option<&str>) {
    match authority.rsplit_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (authority, None),
    }
}

/// Which authority component failed validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AuthorityError {
    Host,
    Port,
}

/// Validates `host[:port]` or a bracketed `[IPv6][:port]` authority.
fn parse_authority(authority: &str) -> Result<(), AuthorityError> {
    if let Some(bracketed) = authority.strip_prefix('[') {
        let Some((literal, rest)) = bracketed.split_once(']') else {
            return Err(AuthorityError::Host);
        };
        if !is_valid_ipv6_literal(literal) {
            return Err(AuthorityError::Host);
        }
        return validate_port_suffix(rest);
    }

    if authority.contains('[') || authority.contains(']') {
        return Err(AuthorityError::Host);
    }

    let (host, port) = split_port(authority);
    if !is_valid_host(host) {
        return Err(AuthorityError::Host);
    }
    if !is_valid_port(port) {
        return Err(AuthorityError::Port);
    }

    Ok(())
}

/// Validates the text after a bracketed IPv6 literal: empty, or `:port`.
fn validate_port_suffix(rest: &str) -> Result<(), AuthorityError> {
    if rest.is_empty() {
        return Ok(());
    }

    let Some(port) = rest.strip_prefix(':') else {
        return Err(AuthorityError::Host);
    };

    if is_valid_port(Some(port)) {
        return Ok(());
    }

    Err(AuthorityError::Port)
}

/// A plausible bracketed IPv6 literal: non-empty, colon-bearing, hex/./: only.
fn is_valid_ipv6_literal(literal: &str) -> bool {
    !literal.is_empty()
        && literal.contains(':')
        && literal.chars().all(|c| c.is_ascii_hexdigit() || c == ':' || c == '.')
}

/// A port is valid when absent, or numeric and within `1..=65535`.
fn is_valid_port(port: Option<&str>) -> bool {
    match port {
        None => true,
        Some(text) => text.parse::<u32>().is_ok_and(|value| (1..=65535).contains(&value)),
    }
}

/// A host is valid when non-empty and every dot-separated label is non-empty
/// and free of leading or trailing hyphens.
fn is_valid_host(host: &str) -> bool {
    !host.is_empty()
        && host
            .split('.')
            .all(|label| !label.is_empty() && !label.starts_with('-') && !label.ends_with('-'))
}

/// An absolute `https://` URL validated at construction.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HttpsUrl(String);

impl HttpsUrl {
    /// Parses `value`, failing closed unless it is an absolute https URL with a sane host.
    pub fn parse(value: &str) -> Result<Self, UrlError> {
        if value.contains('\\') {
            return Err(UrlError::Backslash {
                value: value.to_string(),
            });
        }

        let Some(rest) = value.strip_prefix("https://") else {
            return Err(UrlError::NotHttps {
                value: value.to_string(),
            });
        };

        let authority = rest.split('/').next().unwrap_or("");
        if authority.is_empty() {
            return Err(UrlError::MissingHost {
                value: value.to_string(),
            });
        }

        let unsupported = authority
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || c == '@');
        if unsupported {
            return Err(UrlError::InvalidHost {
                value: value.to_string(),
            });
        }

        match parse_authority(authority) {
            Ok(()) => {}
            Err(AuthorityError::Host) => {
                return Err(UrlError::InvalidHost {
                    value: value.to_string(),
                });
            }
            Err(AuthorityError::Port) => {
                return Err(UrlError::InvalidPort {
                    value: value.to_string(),
                });
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_well_formed_https_asset_url() {
        assert!(WasmAssetUrl::parse("https://github.com/cutver/p/releases/x.wasm").is_ok());
        assert!(HttpsUrl::parse("https://github.com:8443/a/b.wasm").is_ok());
        assert!(HttpsUrl::parse("https://localhost/x.wasm").is_ok());
    }

    #[test]
    fn rejects_a_backslash_anywhere_in_the_url() {
        let err = HttpsUrl::parse("https://github.com/a\\b.wasm").unwrap_err();
        assert!(matches!(err, UrlError::Backslash { .. }));
    }

    #[test]
    fn rejects_a_malformed_or_out_of_range_port() {
        for value in [
            "https://github.com:notaport/x.wasm",
            "https://github.com:99999/x.wasm",
            "https://github.com:0/x.wasm",
            "https://github.com:/x.wasm",
        ] {
            let err = HttpsUrl::parse(value).unwrap_err();
            assert!(matches!(err, UrlError::InvalidPort { .. }), "accepted {value}");
        }
    }

    #[test]
    fn rejects_malformed_host_labels() {
        for value in ["https://.com/x.wasm", "https://-bad-.com/x.wasm"] {
            let err = HttpsUrl::parse(value).unwrap_err();
            assert!(matches!(err, UrlError::InvalidHost { .. }), "accepted {value}");
        }
    }

    #[test]
    fn accepts_bracketed_ipv6_literals() {
        for value in [
            "https://[::1]/x.wasm",
            "https://[2001:db8::1]/x.wasm",
            "https://[::1]:8443/x.wasm",
            "https://[2001:db8::1]:65535/x.wasm",
        ] {
            assert!(HttpsUrl::parse(value).is_ok(), "rejected {value}");
        }
    }

    #[test]
    fn rejects_an_unterminated_or_unbalanced_bracket() {
        for value in ["https://[::1", "https://[::1]]/x.wasm"] {
            let err = HttpsUrl::parse(value).unwrap_err();
            assert!(matches!(err, UrlError::InvalidHost { .. }), "accepted {value}");
        }
    }

    #[test]
    fn preserves_ordinary_authority_and_path_shapes() {
        for value in [
            "https://github.com/cutver/p/releases/x.wasm",
            "https://GITHUB.COM/Cutver/P/x.wasm",
            "https://github.com:65535/x.wasm",
            "https://github.com/x.wasm?query=1",
            "https://github.com/x.wasm#fragment",
            "https://github.com/a%20b/x.wasm",
        ] {
            assert!(HttpsUrl::parse(value).is_ok(), "rejected {value}");
        }
    }
}
