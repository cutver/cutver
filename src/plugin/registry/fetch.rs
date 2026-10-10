//! Thin HTTP boundary for the registry index (Pillar I.1).
//!
//! Fetches a single entry with an end-to-end timeout and a response body size
//! cap, failing closed on non-2xx status, timeout, and oversized bodies. The
//! `ureq` call itself lives in [`crate::plugin::registry::transport`]; this
//! module only builds the entry URL and re-labels transport errors with the
//! registry-entry context. Nothing is written to disk here.

use std::fmt;
use std::time::Duration;

use super::transport::{Transport, UreqTransport};
use super::url::HttpsUrl;
use crate::plugin::error::{InstallError, PluginError, TransportError};
use crate::plugin::types::PluginName;

/// Base URL of the Cutver registry index.
pub const INDEX_BASE: &str = "https://raw.githubusercontent.com/cutver/registry/main/plugins/";

/// End-to-end timeout for a single entry fetch.
pub const ENTRY_TIMEOUT: Duration = Duration::from_secs(15);

/// Maximum accepted response body for one entry (1 MiB).
pub const MAX_ENTRY_BYTES: u64 = 1024 * 1024;

/// Validated, slash-normalized base URL of a registry index (decision D18).
///
/// Parsing goes through [`HttpsUrl`], so an `http://` base fails closed; the
/// stored value always ends in exactly one `/`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RegistryIndexUrl(String);

impl RegistryIndexUrl {
    /// Parses and normalizes `base`, failing closed unless it is an absolute https URL.
    pub fn parse(base: &str) -> Result<Self, PluginError> {
        HttpsUrl::parse(base).map_err(|err| InstallError::IndexUrl {
            base: base.to_string(),
            reason: err.to_string(),
        })?;

        Ok(Self(format!("{}/", base.trim_end_matches('/'))))
    }

    /// The canonical Cutver registry index base.
    pub fn default_index() -> Self {
        Self(INDEX_BASE.to_string())
    }

    /// Returns the normalized base URL as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RegistryIndexUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Builds the index URL for a plugin entry using the canonical base.
pub fn entry_url(name: &PluginName) -> String {
    entry_url_with(INDEX_BASE, name)
}

/// Builds the index URL for a plugin entry under an explicit base.
pub fn entry_url_with(base: &str, name: &PluginName) -> String {
    format!("{base}{name}.toml")
}

/// Fetches the raw registry entry text for `name` from the canonical base.
///
/// The returned text is unvalidated; callers must pass it through
/// [`parse_entry`][super::parse_entry] before use.
pub fn fetch_entry(name: &PluginName) -> Result<String, PluginError> {
    fetch_entry_with(INDEX_BASE, name, &UreqTransport::new(ENTRY_TIMEOUT))
}

/// Fetches the raw registry entry text for `name` through an injected transport.
pub fn fetch_entry_with(base: &str, name: &PluginName, transport: &dyn Transport) -> Result<String, PluginError> {
    let url = entry_url_with(base, name);
    let bytes = transport
        .fetch(&url, MAX_ENTRY_BYTES)
        .map_err(|err| entry_error(name, &url, err))?;

    String::from_utf8(bytes).map_err(|err| PluginError::RegistryMalformedEntry {
        name: name.to_string(),
        reason: format!("registry index entry is not valid UTF-8: {err}"),
    })
}

/// Re-labels a generic transport failure with the registry-entry context.
///
/// Every [`TransportError`] is rewritten into its registry-specific
/// [`PluginError`] counterpart so the message names the plugin entry.
fn entry_error(name: &PluginName, url: &str, error: TransportError) -> PluginError {
    match error {
        TransportError::TransportUnreachable { reason, .. } => PluginError::RegistryUnreachable {
            name: name.to_string(),
            url: url.to_string(),
            reason,
        },
        TransportError::TransportHttpStatus { status, .. } => PluginError::RegistryHttpStatus {
            name: name.to_string(),
            url: url.to_string(),
            status,
        },
        TransportError::TransportTooLarge { cap_bytes, .. } => PluginError::RegistryResponseTooLarge {
            name: name.to_string(),
            url: url.to_string(),
            cap_bytes,
        },
    }
}
