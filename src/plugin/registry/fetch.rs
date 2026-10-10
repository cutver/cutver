//! Thin HTTP boundary for the registry index (Pillar I.1).
//!
//! Fetches a single entry with an end-to-end timeout and a response body size
//! cap, failing closed on non-2xx status, timeout, and oversized bodies. Nothing
//! is written to disk here.

use std::time::Duration;

use ureq::Agent;

use crate::plugin::error::PluginError;
use crate::plugin::types::PluginName;

/// Base URL of the Cutver registry index.
pub const INDEX_BASE: &str = "https://raw.githubusercontent.com/cutver/registry/main/plugins/";

/// End-to-end timeout for a single entry fetch.
pub const ENTRY_TIMEOUT: Duration = Duration::from_secs(15);

/// Maximum accepted response body for one entry (1 MiB).
pub const MAX_ENTRY_BYTES: u64 = 1024 * 1024;

/// Builds the index URL for a plugin entry.
pub fn entry_url(name: &PluginName) -> String {
    format!("{INDEX_BASE}{name}.toml")
}

/// Fetches the raw registry entry text for `name`.
///
/// The returned text is unvalidated; callers must pass it through
/// [`parse_entry`][super::parse_entry] before use.
pub fn fetch_entry(name: &PluginName) -> Result<String, PluginError> {
    let url = entry_url(name);
    let agent: Agent = Agent::config_builder()
        .timeout_global(Some(ENTRY_TIMEOUT))
        .build()
        .into();

    let response = agent.get(&url).call().map_err(|err| map_transport(name, &url, err))?;

    let status = response.status();
    if !status.is_success() {
        return Err(PluginError::RegistryHttpStatus {
            name: name.to_string(),
            url,
            status: status.as_u16(),
        });
    }

    response
        .into_body()
        .into_with_config()
        .limit(MAX_ENTRY_BYTES)
        .read_to_string()
        .map_err(|err| map_transport(name, &url, err))
}

/// Maps a ureq transport error onto an actionable registry error.
fn map_transport(name: &PluginName, url: &str, err: ureq::Error) -> PluginError {
    match err {
        ureq::Error::StatusCode(status) => PluginError::RegistryHttpStatus {
            name: name.to_string(),
            url: url.to_string(),
            status,
        },
        ureq::Error::BodyExceedsLimit(cap_bytes) => PluginError::RegistryResponseTooLarge {
            name: name.to_string(),
            url: url.to_string(),
            cap_bytes,
        },
        ureq::Error::Timeout(_) => PluginError::RegistryUnreachable {
            name: name.to_string(),
            url: url.to_string(),
            reason: format!("request timed out after {}s", ENTRY_TIMEOUT.as_secs()),
        },
        other => PluginError::RegistryUnreachable {
            name: name.to_string(),
            url: url.to_string(),
            reason: other.to_string(),
        },
    }
}
