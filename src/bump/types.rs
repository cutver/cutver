use crate::preflight;
use semver::Version;
use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("git guard failed: {0}")]
    Git(#[from] crate::git::Error),
    #[error("failed to read '{path}': {source}")]
    Read { path: String, source: io::Error },
    #[error("current source '{path}': {source}")]
    CurrentSource {
        path: String,
        source: crate::manifest::Error,
    },
    #[error("manifest '{path}': {source}")]
    Manifest {
        path: String,
        source: crate::manifest::Error,
    },
    #[error("preflight failed: {0}")]
    Preflight(#[from] preflight::Error),
    #[error("changelog failed: {0}")]
    Changelog(#[from] crate::changelog::Error),
    #[error("stage failed: {0}")]
    Stage(#[source] crate::git::Error),
    #[error("commit failed: {0}")]
    Commit(#[source] crate::git::Error),
    #[error("tag '{tag}' failed: {source}")]
    Tag {
        tag: String,
        #[source]
        source: crate::git::Error,
    },
    #[error("release tag {tag} already exists (points at {commit}) — resolve it before re-running")]
    TagExists { tag: String, commit: String },
    #[error(
        "release tag '{tag}' already exists remotely on '{remote}' (points at {commit}) — delete or resolve the remote tag before re-running"
    )]
    RemoteTagExists {
        tag: String,
        remote: String,
        commit: String,
    },
    #[error("write failed for '{path}'; rollback attempted. {rollback}")]
    WriteRollback {
        path: String,
        #[source]
        source: crate::atomic::Error,
        rollback: String,
    },
    #[error("post_bump hook '{command}' failed with status {status}")]
    PostBumpHookFailed {
        command: String,
        status: std::process::ExitStatus,
    },
    #[error("failed to run post_bump hook '{command}': {source}")]
    PostBumpHookSpawn {
        command: String,
        #[source]
        source: io::Error,
    },
    #[error("publish push failed: {0}")]
    Push(#[source] crate::git::Error),
    #[error("publish command '{command}' failed with status {status}")]
    PublishCommandFailed {
        command: String,
        status: std::process::ExitStatus,
    },
    #[error("failed to run publish command '{command}': {source}")]
    PublishCommandSpawn {
        command: String,
        #[source]
        source: io::Error,
    },
    #[error("publish command '{command}' timed out after {elapsed_ms}ms (limit {timeout}s)")]
    PublishCommandTimeout {
        command: String,
        timeout: u64,
        elapsed_ms: u128,
    },
}

#[derive(Debug)]
pub struct Touched {
    pub path: String,
    pub old: String,
    pub new: String,
}

#[derive(Debug)]
pub struct Summary {
    pub source: String,
    pub current: Version,
    pub next: Version,
    pub dry_run: bool,
    pub preflight: Vec<preflight::Step>,
    pub touched: Vec<Touched>,
    pub changelog: Option<String>,
    pub commit_message: String,
    pub tag: String,
    pub tag_skipped: bool,
    pub floating_tag: Option<String>,
    pub post_bump: Option<String>,
    pub publish_push: bool,
    pub publish_push_command: Option<String>,
    pub publish_commands: Vec<String>,
    pub rationale: Option<crate::conventional::BumpRationale>,
    pub root_dir: Option<std::path::PathBuf>,
}

#[derive(Debug)]
pub struct Drift {
    pub path: String,
    pub expected: String,
    pub actual: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChangelogDrift {
    pub missing_in_changelog: Vec<String>,
    pub orphan_sections: Vec<String>,
}

impl ChangelogDrift {
    pub fn is_empty(&self) -> bool {
        self.missing_in_changelog.is_empty() && self.orphan_sections.is_empty()
    }
}

#[derive(Debug)]
pub(crate) struct Change {
    pub path: String,
    pub old: Version,
    pub original: String,
    pub new: String,
    pub changed: bool,
}
