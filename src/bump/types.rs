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
    #[error(
        "release tag name '{tag}' is not a valid git ref (built from `[git] tag_prefix = \"{prefix}\"`): {source}\n  Where: tag naming, before any file, commit or tag is written\n  Fix: change `[git] tag_prefix` in cutver.toml so that the prefix plus the version forms a valid git ref (git forbids whitespace and control characters, `~`, `^`, `:`, `?`, `*`, `[`, `\\`, `..`, `@{{`, a leading or trailing `/`, and a trailing `.` or `.lock`). Nothing was written, staged, committed or tagged."
    )]
    InvalidTagName {
        tag: String,
        prefix: String,
        #[source]
        source: crate::git::TagError,
    },
    #[error(
        "tag '{tag}' failed after commit {commit} was created: {source}\n  Where: tag creation, immediately after the release commit and before any push\n  Fix: the bump is already committed as {commit} and tag '{tag}' was not created; nothing was pushed. Once the cause is fixed, create the tag by hand (`git tag -a {tag} {commit}`) or unwind with `git reset --hard {commit}~1` before re-running."
    )]
    Tag {
        tag: String,
        commit: String,
        // Boxed to keep `bump::Error` inside the 128-byte `result_large_err` budget
        // (`plugin::error::tests::plugin_error_stays_within_the_bump_result_budget`).
        #[source]
        source: Box<crate::git::Error>,
    },
    #[error(
        "floating major tag '{tag}' failed after commit {commit} was created: {source}\n  Where: floating major tag update, after the release commit and tag and, with `[publish] push`, the pushes\n  Fix: the release commit {commit} already exists locally. Recreate the floating tag once the cause is fixed (`git tag -f -a {tag} {commit}`), and push it by hand (`git push origin +refs/tags/{tag}:refs/tags/{tag}`) if the release tag was already published."
    )]
    FloatingTag {
        tag: String,
        commit: String,
        #[source]
        source: Box<crate::git::Error>,
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
    #[error("pre_bump hook '{command}' failed with status {status}")]
    PreBumpHookFailed {
        command: String,
        status: std::process::ExitStatus,
    },
    #[error("failed to run pre_bump hook '{command}': {source}")]
    PreBumpHookSpawn {
        command: String,
        #[source]
        source: io::Error,
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
    #[error(
        "publish push failed: {source}\n  Where: publish push, after commit {commit} and tag '{tag}' were created locally; the push step that failed and the refs that already landed are named above\n  Fix: the release commit {commit} and tag '{tag}' already exist locally: push the remaining ref by hand once the cause is fixed, or unwind with `git reset --hard {commit}~1` and delete tag '{tag}'. Do not re-run `cutver bump` against the already-bumped files."
    )]
    Push {
        commit: String,
        tag: String,
        #[source]
        source: Box<crate::git::Error>,
    },
    #[error("publish command '{command}' failed with status {status}")]
    PublishCommandFailed {
        command: String,
        status: std::process::ExitStatus,
    },
    #[error(
        "a publish command failed after release {tag} at commit {commit}: {source}\n  Where: publish commands, the last step after the commit, the tag and (with `[publish] push`) the pushes\n  Fix: the release commit {commit} and tag '{tag}' already exist; fix the publish command and re-run that command alone. Do not re-run `cutver bump`, which would bump again; to unwind, delete tag '{tag}' and run `git reset --hard {commit}~1`."
    )]
    PublishAfterCommit {
        commit: String,
        tag: String,
        #[source]
        source: Box<Error>,
    },
    #[error(
        "post-release plugin '{plugin}' failed after release {tag} at commit {commit}: {source}\n  Where: lifecycle hook 'on_post_release', after the commit, the tag and the publish commands\n  Fix: the release commit {commit} and tag '{tag}' already exist; fix the plugin and dispatch `on_post_release` by hand, or unwind with `git reset --hard {commit}~1` and delete tag '{tag}'."
    )]
    PostReleasePlugin {
        plugin: String,
        tag: String,
        commit: String,
        #[source]
        source: Box<crate::plugin::PluginError>,
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
    #[error("plugin error: {0}")]
    Plugin(#[from] crate::plugin::PluginError),
    #[error("invalid plugin name '{0}': {1}")]
    InvalidPluginName(String, crate::plugin::types::PluginNameError),
    #[error(
        "plugin '{plugin}' returned invalid SemVer version '{version}': {reason}\n  Where: plugin versioning resolution\n  Fix: ensure the versioning plugin produces a valid SemVer 2.0 version string."
    )]
    InvalidPluginVersion {
        plugin: String,
        version: String,
        reason: String,
    },
    #[error(
        "plugin '{plugin}' rejected pre-bump checks: {reason}\n  Where: lifecycle hook 'on_pre_bump'\n  Fix: resolve the pre-bump condition reported above."
    )]
    PreBumpRejected {
        plugin: crate::plugin::PluginName,
        reason: String,
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
    pub pre_bump: Option<String>,
    pub post_bump: Option<String>,
    pub warnings: Vec<String>,
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

/// Level or strategy to increment the version in the release pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BumpLevel {
    Patch,
    Minor,
    Major,
    Auto,
}

impl From<crate::semver_bump::Bump> for BumpLevel {
    fn from(b: crate::semver_bump::Bump) -> Self {
        match b {
            crate::semver_bump::Bump::Patch => BumpLevel::Patch,
            crate::semver_bump::Bump::Minor => BumpLevel::Minor,
            crate::semver_bump::Bump::Major => BumpLevel::Major,
        }
    }
}
