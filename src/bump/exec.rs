mod changelog;
mod command;
mod doctor;
mod guard;
mod hooks;
mod manifest;
mod pipeline;
mod publish;
mod transaction;

#[cfg(test)]
mod tests;

// `crate::bump::plan` is declared in `src/bump.rs`, a sibling of this module, so the submodules
// above are private to it and cannot be named from there. These re-exports widen the visibility of
// exactly what the extracted computation reuses — the pipeline and the plan reader call the same
// functions, which is what keeps one authority instead of two descriptions of the release.
pub(crate) use changelog::{ChangelogPlan, ChangelogPlanParams, assemble_scope, prepare_changelog, preview_notes};
pub(crate) use command::format_command;
pub(crate) use guard::{TagCollision, build_context, bump_level_name, find_tag_collision, validate_tag_name};
pub(crate) use manifest::{compute, current_source};

#[allow(unused_imports)]
pub(crate) use command::{is_known_lockfile, run_publish_command};
pub use doctor::{doctor, doctor_changelog};
pub use pipeline::{run, run_deferred_publish, run_with_first_release};
pub use publish::{PublishRequest, PublishSummary, run_publish};
#[allow(unused_imports)]
pub use transaction::MutationTransaction;
