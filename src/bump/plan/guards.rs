//! The guards a real release aborts on, and the one-line wording a plan reports them in.
//!
//! The wording lives next to the guard it explains: a blocker states the condition and then what
//! resolves it (#326 S4).

use std::path::Path;

use crate::bump::Error;
use crate::bump::exec::TagCollision;
use crate::conventional::{self, ConventionalCommit};
use crate::git;

/// What a plan names when the project declares no changelog at all.
pub(super) const NAMED_NOTHING: &str = "(no `[changelog] path`)";

/// The footer that declares a compatible range: the other direction of `BREAKING CHANGE:` (#307 S2).
pub(super) const NO_BREAKING_CHANGE: &str = "NO-BREAKING-CHANGE";

/// A multi-line error reduced to the one line a blocker is (#326 S4).
pub(super) fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The detail a guard error carries, without its `git guard failed:` prefix: a blocker states the
/// condition, not the layer that observed it.
fn guard_detail(err: &git::Error) -> String {
    match err {
        git::Error::Guard { detail } => one_line(detail),
        other => one_line(&other.to_string()),
    }
}

/// The clean-tree guard as a blocker: what is in the way, then what resolves it.
pub(super) fn dirty_tree_blocker(err: &git::Error) -> String {
    format!(
        "{}; commit or stash them, or set `[git] require_clean_tree = false` to plan a tree that is not \
         committed yet",
        guard_detail(err)
    )
}

/// The required-branch guard as a blocker.
pub(super) fn branch_blocker(err: &git::Error, required: Option<&str>) -> String {
    match required {
        Some(branch) => format!(
            "{}; check out '{branch}', or set `[git] require_branch` to the branch CI plans on",
            guard_detail(err)
        ),
        None => guard_detail(err),
    }
}

/// An existing release tag as a blocker, naming where it exists and what to do about it.
pub(super) fn collision_blocker(collision: &TagCollision) -> String {
    match collision {
        TagCollision::Local { commit } => format!(
            "release tag already exists locally (points at {commit}); delete it, or plan the next \
             version"
        ),
        TagCollision::Remote { remote, commit } => format!(
            "release tag already exists on remote '{remote}' (points at {commit}); delete or resolve \
             the remote tag before tagging"
        ),
        TagCollision::Unreachable { remote, detail } => format!(
            "could not check the release tag on remote '{remote}' ({detail}); make the remote \
             reachable and re-run, because an unanswered remote check is not a free tag"
        ),
    }
}

/// The levels a declaration binds: every version move that is not a `patch` (#307 S1).
///
/// `initial` is a promotion — a version move with no commits — so it is not this check's object.
fn owes_a_declaration(level: &str) -> bool {
    matches!(level, "minor" | "major")
}

/// Whether the range declared what it does about compatibility (#307 S2).
///
/// Two footers answer, and only as footers: a breaking change in either spelling (or a `!` header,
/// which the parser already turns into `is_breaking`), or the reviewed-none declaration. Prose that
/// happens to contain the sentinel is body text, so it stays silence (#307 S6).
fn declares_compatibility(commits: &[ConventionalCommit]) -> bool {
    commits
        .iter()
        .any(|commit| commit.is_breaking || commit.footers.iter().any(|(token, _)| token == NO_BREAKING_CHANGE))
}

/// The guard a `minor` or `major` release passes only when its range declares compatibility (#307).
///
/// `level` is the level the release resolved to. `parsed` is what `auto` already parsed from the same
/// range; `--level` never parses commits on its own, so the guard reads `<previous_tag>..HEAD` itself
/// and asks the same question `auto` asks. Both readers of the plan call this one function: a real
/// bump aborts on it and `cutver plan` reports it, before anything is written.
///
/// Two absences are not silence, and neither grows a branch of its own:
///
/// - no previous tag: nothing has been shipped, so there is no prior contract to declare against —
///   the same reason #307 S1 puts a promotion outside this check;
/// - an empty range: a version move with no commits, which is that same shape.
pub(super) fn undeclared_compatibility(
    repo: &Path,
    level: &str,
    previous_tag: Option<&str>,
    parsed: Option<&[ConventionalCommit]>,
) -> Result<(), Error> {
    if !owes_a_declaration(level) {
        return Ok(());
    }
    let Some(previous_tag) = previous_tag else {
        return Ok(());
    };
    let read;
    let commits = match parsed {
        Some(commits) => commits,
        None => {
            let messages = git::commits_since(repo, Some(previous_tag))?;
            read = conventional::parse_and_deduce_with_rationale(&messages).1;
            &read
        }
    };
    if commits.is_empty() || declares_compatibility(commits) {
        return Ok(());
    }
    Err(Error::UndeclaredCompatibility {
        level: level.to_string(),
        range: format!("{previous_tag}..HEAD"),
    })
}
