//! Two-phase installation of a plugin from the registry (decisions D14-D19).
//!
//! `plan_add` computes the resolved release, the exact `cutver.toml` edit and
//! the artifact location in memory and mutates nothing. `apply_add` downloads,
//! verifies SHA-256 before the final cache name, moves the artifact into place
//! and only then writes the configuration, with an RAII guard that restores the
//! artifact if the config write fails (Pillar I.2).

mod apply;
mod plan;

pub use apply::{InstallOutcome, apply_add};
pub use plan::{AddOptions, InstallPlan, plan_add};
