//! Git boundary operations and domain models.
//!
//! Decomposed into cohesive submodules behind this Facade adhering to `CONTRACT.md`
//! Pillar V.4 (Module Budget $\le 300-400$ lines) and Pillar V.5 (Domain Newtypes).

pub mod command;
pub mod log;
pub mod ops;
pub mod remote;
pub mod status;
pub mod tags;
pub mod types;

#[cfg(test)]
mod tests;

#[cfg(test)]
pub use tests::init_test_repo;

// Re-export types
pub use types::{CommitSha, Error, RawCommit, TagError, TagName, TagPrefix};

// Re-export command execution / helpers
pub use command::{run_git, stdout_text};

// Re-export status and branch operations
pub use status::{current_branch, is_clean, parse_branch, require_branch, require_clean_tree, rev_parse, status_files};

// Re-export mutation operations
pub use ops::{
    commit, commit_ext, commit_message, is_floating_major_tag, push, push_tag_force, stage, tag, update_floating_tag,
};

// Re-export tag querying and resolution
pub use tags::{has_remote, latest_tag, list_tags, remote_tag_exists, tag_exists, tag_name};

// Re-export log traversal and author resolution
pub use log::{
    commits_between, commits_since, first_time_contributors, list_authors_between, list_authors_since,
    raw_commits_between, raw_commits_since, resolve_author,
};

// Re-export remote repository extraction
pub use remote::{normalize_repo_url, remote_url};
