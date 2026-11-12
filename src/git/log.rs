//! Git log queries: raw commit ranges, per-package selection and author extraction.
//!
//! A facade over three focused submodules, per Contract Pillar IV.4. [`range`] holds the raw
//! runner and its range semantics, [`package`] the path-filtered selection, and [`authors`] the
//! author queries. This file declares and re-exports; it carries no implementation.

mod authors;
mod package;
mod range;

pub use authors::{
    first_time_contributors, list_authors_before, list_authors_between, list_authors_since, resolve_author,
};
pub use package::{
    AnchoredPathspec, PackageCommitSelection, PackageCommits, commits_by_package, commits_by_package_with_range,
};
pub use range::{commits_between, commits_since, raw_commits_between, raw_commits_since};

pub(crate) use range::{range_args, run_log_checked};
