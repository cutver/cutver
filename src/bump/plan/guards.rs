//! The guards a real release aborts on, and the one-line wording a plan reports them in.
//!
//! The wording lives next to the guard it explains: a blocker states the condition and then what
//! resolves it (#326 S4).

use crate::bump::exec::TagCollision;
use crate::git;

/// What a plan names when the project declares no changelog at all.
pub(super) const NAMED_NOTHING: &str = "(no `[changelog] path`)";

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
