//! Transport-layer errors for the plugin HTTP boundary.
//!
//! The blocking [`Transport`][crate::plugin::registry::Transport] trait returns
//! these, so the transport adapter never constructs the wider
//! [`PluginError`][super::PluginError]; callers re-label them with registry or
//! install context.

use thiserror::Error;

/// A failure raised by the blocking HTTP transport boundary.
#[derive(Debug, Error)]
pub enum TransportError {
    #[error(
        "network request to '{url}' failed: {reason}.\n  \
        Where: fetching over HTTPS during plugin installation\n  \
        Fix: check your network connection or proxy, then retry."
    )]
    TransportUnreachable { url: String, reason: String },

    #[error(
        "remote server returned HTTP {status} for '{url}'.\n  \
        Where: fetching over HTTPS during plugin installation\n  \
        Fix: confirm the URL is correct and reachable; an HTTP 404 means the resource does not exist."
    )]
    TransportHttpStatus { url: String, status: u16 },

    #[error(
        "response from '{url}' exceeds the {cap_bytes}-byte size cap.\n  \
        Where: reading a remote response body during plugin installation\n  \
        Fix: the artifact or index entry is oversized; report it to the publisher."
    )]
    TransportTooLarge { url: String, cap_bytes: u64 },
}
