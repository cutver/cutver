//! Injectable HTTP transport for registry entries and artifact downloads.
//!
//! The production [`UreqTransport`] is the only `ureq` caller (decision D17):
//! it applies a global timeout, a response size cap and non-2xx fail-closed
//! semantics. Tests inject their own [`Transport`], so no test touches the
//! network (decisions D9/D10).

use std::time::Duration;

use ureq::Agent;

use crate::plugin::error::TransportError;

/// Default end-to-end timeout for an artifact download.
pub const ARTIFACT_TIMEOUT: Duration = Duration::from_secs(120);

/// Maximum accepted artifact response body (32 MiB).
pub const MAX_ARTIFACT_BYTES: u64 = 32 * 1024 * 1024;

/// A blocking HTTP fetch boundary over `url`, bounded by `cap_bytes`.
///
/// Callers pass the cap so the registry-entry and artifact paths can use
/// different limits against the same transport.
pub trait Transport {
    /// Fetches `url`, failing closed on transport errors or an oversized body.
    fn fetch(&self, url: &str, cap_bytes: u64) -> Result<Vec<u8>, TransportError>;
}

/// Production [`Transport`] backed by a shared `ureq` agent.
pub struct UreqTransport {
    agent: Agent,
}

impl UreqTransport {
    /// Builds a transport whose requests share one global timeout.
    pub fn new(timeout: Duration) -> Self {
        let agent: Agent = Agent::config_builder().timeout_global(Some(timeout)).build().into();
        Self { agent }
    }
}

impl Transport for UreqTransport {
    fn fetch(&self, url: &str, cap_bytes: u64) -> Result<Vec<u8>, TransportError> {
        let response = self.agent.get(url).call().map_err(|err| map_ureq(url, err))?;

        let status = response.status();
        if !status.is_success() {
            return Err(TransportError::TransportHttpStatus {
                url: url.to_string(),
                status: status.as_u16(),
            });
        }

        response
            .into_body()
            .into_with_config()
            .limit(cap_bytes)
            .read_to_vec()
            .map_err(|err| map_ureq(url, err))
    }
}

/// Maps a `ureq` transport error onto a typed [`TransportError`].
fn map_ureq(url: &str, err: ureq::Error) -> TransportError {
    match err {
        ureq::Error::StatusCode(status) => TransportError::TransportHttpStatus {
            url: url.to_string(),
            status,
        },
        ureq::Error::BodyExceedsLimit(cap_bytes) => TransportError::TransportTooLarge {
            url: url.to_string(),
            cap_bytes,
        },
        ureq::Error::Timeout(_) => TransportError::TransportUnreachable {
            url: url.to_string(),
            reason: "request timed out".to_string(),
        },
        other => TransportError::TransportUnreachable {
            url: url.to_string(),
            reason: other.to_string(),
        },
    }
}
