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
    #[error(
        "could not read '{path}' to record the bytes rollback would restore: {source}\n  Where: Phase 1, taking the pre-write snapshot, before anything was written\n  Fix: make the file readable again (its permissions, its encoding or the I/O path), then re-run. An unreadable target cannot be backed up, so nothing was written, staged, committed or tagged."
    )]
    ReadUnreadable { path: String, source: io::Error },
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
        "tag '{tag}' failed after commit {commit} was created: {source}\n  Where: tag creation, immediately after the release commit and before any push\n  Fix: the bump is already committed as {commit} and tag '{tag}' was not created; nothing was pushed. Once the cause is fixed, create the tag by hand (`git tag -a {tag} {commit}`) or undo the commit with a plain `git reset {commit}~1`, which keeps uncommitted work; do not reach for `--hard` here, it would discard changes this run never touched."
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
        "tag '{tag}' failed in the closing phase of the release at commit {commit}: {source}\n  Where: tag creation in `cutver publish`, after `cutver bump --defer-publish` already pushed commit {commit} to the remote branch\n  Fix: tag '{tag}' was not created, so no tag push, no floating tag and no publish command ran. Unlike `cutver bump`, this phase starts after the branch push, so the release commit may already be on the remote without a tag: confirm with `git ls-remote origin` before removing anything. Once the cause is fixed, re-run `cutver publish --tag {tag}`, which creates only the missing tag and repeats the remaining steps; do not re-run `cutver bump`, which would bump again."
    )]
    TagInClosingPhase {
        tag: String,
        commit: String,
        // Boxed for the same 128-byte `bump::Error` budget as `Tag` above.
        #[source]
        source: Box<crate::git::Error>,
    },
    #[error(
        "floating major tag '{tag}' failed after commit {commit} was created: {source}\n  Where: floating major tag update, which runs whether or not `[publish] push` is enabled; only the push of that tag depends on it\n  Fix: the local floating tag was not updated. Whether the branch and the release tag reached the remote depends on `[publish] push`; inspect with `git ls-remote origin` before removing anything, because a published ref stays published until you delete it. Recreate the floating tag by hand once the cause is fixed (`git tag -f -a {tag} {commit}`)."
    )]
    FloatingTag {
        tag: String,
        commit: String,
        #[source]
        source: Box<crate::git::Error>,
    },
    #[error(
        "release tag '{tag}' does not match the version the manifests declare at HEAD ('{declared}')\n  Where: `cutver publish` validation, before the tag, the pushes and the publish commands\n  Fix: publish the tag this commit declares (`cutver publish --tag {declared}`), or check out the release commit you meant to publish. Nothing was written."
    )]
    PublishTagMismatch { tag: String, declared: String },
    #[error(
        "release commit '{expected}' is not HEAD ('{actual}')\n  Where: `cutver publish` validation, before the tag, the pushes and the publish commands\n  Fix: check out the commit you meant to publish (`git checkout {expected}`) and re-run, or drop `--commit` to publish the current HEAD. Nothing was written."
    )]
    PublishCommitMismatch { expected: String, actual: String },
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
    #[error(
        "{error}\n  Where: rollback after the failure above\n  Rollback failed: {failures}\n  Fix: the working tree may no longer match its committed content. Inspect it with `git status` and restore each path named above from version control (`git checkout -- <path>`) or from your own copy; re-run the bump only once the tree is clean."
    )]
    RollbackFailed { error: Box<Error>, failures: String },
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
        "publish push failed: {source}\n  Where: publish push, after commit {commit} and tag '{tag}' exist locally; the refspec that failed is named above and `git push` lists the refs that did land\n  Fix: the refs are pushed one at a time, in order (branch, release tag, floating tag), so nothing after the failed one ran. Inspect the actual remote state with `git ls-remote origin` before removing or reverting anything: a ref this run published stays published until you delete it, and a ref that existed before this run is not yours to delete. Do not re-run `cutver bump` against already-bumped files."
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
        "a publish command failed after release {tag} at commit {commit}: {source}\n  Where: publish commands, the last step, which run whether or not `[publish] push` is enabled\n  Fix: fix the command and re-run that command alone; do not re-run `cutver bump`, which would bump again. Whether the branch and the tag are on the remote depends on `[publish] push`, so inspect with `git ls-remote origin` before deleting anything: a ref this run published stays published until you remove it, and a pre-existing ref is not yours to delete."
    )]
    PublishAfterCommit {
        commit: String,
        tag: String,
        #[source]
        source: Box<Error>,
    },
    #[error(
        "post-release plugin '{plugin}' failed after release {tag} at commit {commit}: {source}\n  Where: lifecycle hook 'on_post_release', the last step, which runs whether or not `[publish] push` is enabled\n  Fix: whether the release is published depends on `[publish] push`; inspect with `git ls-remote origin` before removing anything, because a ref this run published stays published until you delete it. Fix the plugin and dispatch `on_post_release` by hand rather than re-running `cutver bump`."
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
    /// The release commit sha, empty on a dry run where no commit is created.
    pub commit: String,
    pub tag: String,
    pub tag_skipped: bool,
    pub floating_tag: Option<String>,
    pub pre_bump: Option<String>,
    pub post_bump: Option<String>,
    pub warnings: Vec<String>,
    pub publish_push: bool,
    pub publish_push_command: Option<String>,
    pub publish_commands: Vec<String>,
    /// True when the run stopped before publishing: the tag, its pushes, `[publish] commands` and
    /// the post-release plugins wait for `cutver publish` (#259).
    pub deferred: bool,
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
    /// What rollback restores. `None` means the target did not exist before the run (#261).
    pub original: Option<String>,
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
