//! Typed client for the Cutver plugin registry index.
//!
//! Domain logic — entry parsing, release selection, digest-shape validation —
//! is pure and lives here (Pillar I.1). The only I/O is the thin HTTP boundary
//! in [`fetch`], which adds a timeout and a response size cap.
//!
//! The whole module ships behind the `plugins` feature (decision A/B): the
//! management surface and its HTTP client are not part of the lean default build.

mod digest;
mod entry;
mod fetch;
mod release;
mod tier;
mod url;

pub use digest::{SHA256_HEX_LEN, Sha256Digest, Sha256DigestError};
pub use entry::{RegistryEntry, RegistryRelease, parse_entry};
pub use fetch::{ENTRY_TIMEOUT, INDEX_BASE, MAX_ENTRY_BYTES, entry_url, fetch_entry};
pub use release::select_release;
pub use tier::{Tier, TierParseError};
pub use url::{HttpsUrl, UrlError, WasmAssetUrl};

use crate::plugin::error::PluginError;
use crate::plugin::types::PluginName;

/// Fetches and parses the registry entry for `name` in one boundary call.
///
/// The requested name is derived from the fetched file's stem (`<name>.toml`)
/// so a mismatch against the declared `name` fails closed in [`parse_entry`].
pub fn resolve_entry(name: &PluginName) -> Result<RegistryEntry, PluginError> {
    let body = fetch_entry(name)?;
    parse_entry(&format!("{name}.toml"), &body)
}
