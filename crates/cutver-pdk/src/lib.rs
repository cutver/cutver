//! The Cutver plugin wire contract.
//!
//! This crate owns the types that cross the host/plugin boundary: the capability
//! and operation domains, the invocation envelope, and every capability request
//! and response DTO. Both the Cutver core and the plugin SDKs depend on this
//! crate, so the contract is defined exactly once and cannot drift between them.
//!
//! The only dependencies are `serde` and `serde_json`: every dependency added
//! here is compiled into every plugin binary, so the surface stays intentionally
//! thin and `wasm32-wasip1`-friendly.

mod changelog;
mod invocation;
mod lifecycle;
mod manifest;
mod types;
mod versioning;

pub use changelog::{ChangelogRenderRequest, ChangelogRenderResponse, PluginCommitEntry, PluginContributor};
pub use invocation::PluginInvocation;
pub use lifecycle::{PostBumpPayload, PostReleasePayload, PreBumpPayload, PreBumpResponse};
pub use manifest::{ManifestReadRequest, ManifestReadResponse, ManifestWriteRequest, ManifestWriteResponse};
pub use types::{Capability, ParseCapabilityError, ParsePluginOperationError, PluginOperation};
pub use versioning::{VersioningComputeRequest, VersioningComputeResponse};
