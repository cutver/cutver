//! What `cutver publish --resume` accepts as evidence, and what it refuses (#326 S4, D2, D4).
//!
//! Nothing persists a run record, so the plan a resume re-reads is the repository itself: the release
//! tag here and on the remote, the floating major tag, and one `publish-<i>` attestation per
//! `[publish] commands` entry. Within that evidence a resume repeats only the steps that are harmless
//! to repeat; everything this module refuses would repeat a step whose evidence names another release,
//! which is the failure D2 exists to prevent — a retry that re-runs `cargo publish` because the tool
//! could not tell.
//!
//! `[publish] commands` is the one step that is not repeatable, because it is an arbitrary command. A
//! gate name becomes an attestation *file name* (`src/config/gates/store.rs`), so a command line
//! cannot be one: the identity is the entry's 1-based position in declaration order — `publish-1`,
//! `publish-2` (D4). The index is stable because the list lives in `cutver.toml`, and a reordering is
//! a config change a reviewer sees.

use crate::bump::Error;
use crate::bump::exec::guard;
use crate::config::Config;
use crate::config::gates::{self, GateName};
use crate::git;

use super::PublishRequest;

/// The remote every push and every remote tag question names (`git::ops` sends `origin`).
const REMOTE: &str = "origin";

/// The gate name that attests `[publish] commands` entry `index`, which is 1-based (D4).
const COMMAND_GATE_PREFIX: &str = "publish-";

/// Everything a resume needs the repository to agree with before it repeats a single step.
pub(super) fn require_evidence(config: &Config, request: &PublishRequest, commit: &str) -> Result<(), Error> {
    require_release_tag(config, request, commit)?;
    require_publish_commands(config, commit)
}

/// The release tag a resume continues, wherever it already exists: absent is a step to repeat, a tag
/// at `commit` is a step already done, and a tag anywhere else is another release's.
fn require_release_tag(config: &Config, request: &PublishRequest, commit: &str) -> Result<(), Error> {
    let repo = &config.root_dir;
    if git::tag_exists(repo, &request.tag)? {
        let actual = git::rev_parse(repo, &format!("{}^{{commit}}", request.tag))?;
        same_commit(&request.tag, "in this repository", commit, actual)?;
    }
    match git::remote_tag_exists(repo, REMOTE, &request.tag) {
        Ok(Some(actual)) => same_commit(&request.tag, "on remote 'origin'", commit, actual),
        Ok(None) => Ok(()),
        // A remote that cannot answer is not "no remote tag" (#262), and a resume must not treat an
        // unknown as evidence of anything.
        Err(unreachable) => Err(guard::TagCollision::Unreachable {
            remote: unreachable.remote,
            detail: unreachable.detail,
        }
        .into_error(&request.tag)),
    }
}

/// Accepts the tag only when it names the release commit this resume continues.
fn same_commit(tag: &str, found: &str, expected: &str, actual: String) -> Result<(), Error> {
    if actual.eq_ignore_ascii_case(expected) {
        return Ok(());
    }
    Err(Error::ResumeTagElsewhere {
        tag: tag.to_string(),
        found: found.to_string(),
        expected: expected.to_string(),
        actual,
    })
}

/// One `publish-<i>` attestation per `[publish] commands` entry, in declaration order (D4).
///
/// A recorded `failure` answers no better than an absent file: the point of the rule is that the gate
/// channel already holds the step as done for this commit, and a `failure` says it is not.
fn require_publish_commands(config: &Config, commit: &str) -> Result<(), Error> {
    for index in 1..=config.publish.commands.len() {
        let gate = command_gate(index);
        let attested = gates::read(config, &gate).filter(|attested| attested.covers(commit));
        if attested.is_some_and(|attested| attested.conclusion.passed()) {
            continue;
        }
        return Err(Error::ResumePublishCommandUnattested {
            index,
            command: config.publish.commands[index - 1].clone(),
            gate: gate.to_string(),
            commit: commit.to_string(),
            path: gates::relative(config, &gates::path_of(config, &gate)),
        });
    }
    Ok(())
}

/// The gate name of `[publish] commands` entry `index`, which is 1-based (D4).
fn command_gate(index: usize) -> GateName {
    // `<prefix><n>` is letters, a dash and digits: the parse cannot fail on input, so a failure here
    // would be a bug in this file rather than something a caller can act on.
    GateName::parse(format!("{COMMAND_GATE_PREFIX}{index}")).expect("`publish-<n>` is a valid gate name")
}
