//! Changelog reading, extraction, rendering and writing.
//!
//! A pure facade, per Contract Pillar IV.4. Every other file under `changelog/` is a focused
//! submodule: [`error`] owns the failure vocabulary, [`context`] assembles the release context,
//! [`extract`] reads entries back out, [`render`] produces the body, [`sections`] groups it per
//! package, and [`update`] writes it. This file declares and re-exports; it carries no
//! implementation.

mod context;
mod error;
mod extract;
mod render;
mod sections;
mod update;

pub use context::{
    AssembleContextParams, CommitContext, ForgeMetadata, InterpolationContext, ReleaseContext,
    assemble_release_context, build_context, build_context_auto, build_context_with_filter, build_context_with_raw,
    build_context_with_raw_and_filter, enrich_commit_context, filter_commits, parse_repo_forge,
};
pub use error::{ChangelogError, Error};
pub use extract::{extract_latest, extract_version, list_versions, read_latest, read_version};
pub use render::{
    create_environment, interpolate_string, render_body, render_body_with_context, render_body_with_plugin,
    render_source, render_template,
};
pub use sections::{PackageSection, render_body_with_packages, supports_package_sections};
pub use update::{compute_update, format_date, update, update_with_options};
