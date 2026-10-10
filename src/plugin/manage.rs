//! Read-only status reporting and surgical removal for installed plugins.
//!
//! `plugin list` never mutates, downloads or executes a plugin; it reports the
//! effective state of the declarations in the resolved `cutver.toml`.
//! `plugin remove` computes a full in-memory plan first (Pillar I.2) and applies
//! it with a surgical `toml_edit` edit afterwards — comments, key order and
//! unrelated tables survive byte-for-byte (Pillar II.1).
//!
//! Resolution and the SHA-256 comparison reuse the loader's semantics through the
//! canonical [`crate::plugin::cache::read_path`], so `list`, `remove` and the
//! loader can never disagree about where an artifact lives. The whole module
//! ships behind the `plugins` feature (decision A/B).

mod remove;
mod status;

pub use remove::{ArtifactDecision, ArtifactOutcome, RemoveOutcome, RemovePlan, execute_remove, plan_remove};
pub use status::{DigestState, PluginStatus, list_statuses};
