//! Phase 1 guards and the release interpolation context.
//!
//! Extracted from the pipeline so `cutver publish` runs the very same checks instead of
//! re-implementing them (Pillar V.6): a tag name that `git tag` would reject, a release tag that
//! already exists locally or remotely, and the context `[publish].commands` interpolate against.

use std::path::Path;

use semver::Version;

use crate::bump::Error;
use crate::bump::types::BumpLevel;
use crate::changelog;
use crate::config::Config;
use crate::config::gates::{self, Attestation, GateName};
use crate::git;

pub(crate) fn build_context(
    repo: &Path,
    current: &Version,
    next: &Version,
    tag: &str,
    prev_tag: Option<&str>,
    bump_level: BumpLevel,
    first_release: bool,
) -> changelog::InterpolationContext {
    let current_branch = git::current_branch(repo).ok();
    let repo_url = git::remote_url(repo);
    let today = changelog::format_date(std::time::SystemTime::now());
    let bump_str = if first_release {
        "initial"
    } else {
        bump_level_name(bump_level)
    };
    let prev_ver = if first_release { None } else { Some(current.to_string()) };
    changelog::InterpolationContext::build(
        &next.to_string(),
        prev_ver.as_deref(),
        tag,
        prev_tag,
        bump_str,
        &today,
        current_branch.as_deref(),
        repo_url.as_deref(),
    )
}

pub(crate) fn bump_level_name(bump_level: BumpLevel) -> &'static str {
    match bump_level {
        BumpLevel::Patch => "patch",
        BumpLevel::Minor => "minor",
        BumpLevel::Major => "major",
        BumpLevel::Auto => "auto",
    }
}

/// Refuses a tag name `git tag` would reject, before the workspace is written.
///
/// The name is derived from user configuration, so it is validated with the `TagName` invariants —
/// the same rule `git tag` relies on — instead of being handed to `git tag` after the commit.
pub(crate) fn validate_tag_name(prefix: &str, tag: &str) -> Result<(), Error> {
    git::TagName::parse(tag)
        .map(|_| ())
        .map_err(|source| Error::InvalidTagName {
            tag: tag.to_string(),
            prefix: prefix.to_string(),
            source,
        })
}

/// Verifies that every gate this release requires attests the commit being published (#326 S9).
///
/// Runs beside the artifact verification, after `validate` and before `publish_prepared`, whose
/// first write is the tag: a required gate that is missing, or whose recorded conclusion is `failure`,
/// fails closed before anything irreversible. A dry run verifies too, so it reports the verdict the
/// real run would.
pub(crate) fn verify_gates(config: &Config, requested: &[GateName], commit: &str) -> Result<(), Error> {
    for gate in required_gates(config, requested) {
        let recorded = gates::read(config, gate);
        let Some(attested) = recorded.as_ref().filter(|attested| attested.covers(commit)) else {
            return Err(Error::GateMissing {
                gate: gate.to_string(),
                commit: commit.to_string(),
                dir: gates::relative(config, &gates::dir(config)),
                note: standing_note(config, gate, recorded.as_ref()),
            });
        };
        if !attested.conclusion.passed() {
            return Err(Error::GateFailed {
                gate: gate.to_string(),
                commit: commit.to_string(),
                recorded: recorded_line(config, gate, attested),
            });
        }
    }
    Ok(())
}

/// The gates this release requires: the standing `[gates] require` declaration in union with this
/// invocation's `--require-gate`, in that order and without repetition.
///
/// The declaration is enforced, not reported: a key named `require` that only `plan` read would be a
/// decorative option, and a CI job that declares its gates once in `cutver.toml` would still have to
/// repeat every one of them on the command line.
fn required_gates<'a>(config: &'a Config, requested: &'a [GateName]) -> Vec<&'a GateName> {
    let mut required: Vec<&GateName> = Vec::new();
    for gate in config.gates.require.iter().chain(requested) {
        if !required.contains(&gate) {
            required.push(gate);
        }
    }
    required
}

/// The line a refusal adds about what the gate channel does hold, so a stale or unreadable file is
/// never mistaken for an absent one.
fn standing_note(config: &Config, gate: &GateName, recorded: Option<&Attestation>) -> String {
    match recorded {
        None => format!(
            "Note: '{}' does not exist or does not read as an attestation for this gate.",
            gates::relative(config, &gates::path_of(config, gate))
        ),
        Some(attested) => format!(
            "Note: the attestation on file covers commit {}, not the commit being published.",
            attested.commit
        ),
    }
}

/// How a recorded failure describes itself: when it was recorded, where, and which run produced it.
fn recorded_line(config: &Config, gate: &GateName, attested: &Attestation) -> String {
    let run = attested.run.as_deref().unwrap_or("no run recorded");
    format!(
        "recorded {} in '{}', {run}",
        attested.verified_at,
        gates::relative(config, &gates::path_of(config, gate))
    )
}

/// A release tag that already exists, or a remote that did not answer (#262).
///
/// One query, two interpretations: a real `bump` aborts on it ([`check_tag_collisions`]), and a
/// `cutver plan` reports it as a blocker (#326 S4). Reporting instead of raising is what lets both
/// paths ask the same question without duplicating the query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TagCollision {
    /// The tag exists in this repository.
    Local { commit: String },
    /// The tag exists on the remote.
    Remote { remote: String, commit: String },
    /// The remote was asked and could not answer, so a tag collision is unknown, not absent.
    Unreachable { remote: String, detail: String },
}

impl TagCollision {
    /// The failure a real `bump` raises for this collision.
    pub(crate) fn into_error(self, tag: &str) -> Error {
        match self {
            TagCollision::Local { commit } => Error::TagExists {
                tag: tag.to_string(),
                commit,
            },
            TagCollision::Remote { remote, commit } => Error::RemoteTagExists {
                tag: tag.to_string(),
                remote,
                commit,
            },
            TagCollision::Unreachable { remote, detail } => Error::RemoteTagCheck {
                tag: tag.to_string(),
                remote,
                detail,
            },
        }
    }
}

/// The release tag that already exists locally or on the remote, if any.
///
/// Read-only: it issues exactly the queries [`check_tag_collisions`] aborts on, so a plan and a bump
/// can never disagree about whether the tag is free.
pub(crate) fn find_tag_collision(repo: &Path, tag: &str, publish_push: bool) -> Result<Option<TagCollision>, Error> {
    if git::tag_exists(repo, tag)? {
        let commit = git::rev_parse(repo, &format!("{tag}^{{commit}}"))?;
        return Ok(Some(TagCollision::Local { commit }));
    }
    if git::has_remote(repo, "origin") || publish_push {
        match git::remote_tag_exists(repo, "origin", tag) {
            Ok(Some(commit)) => {
                return Ok(Some(TagCollision::Remote {
                    remote: "origin".to_string(),
                    commit,
                }));
            }
            Ok(None) => {}
            Err(e) => {
                return Ok(Some(TagCollision::Unreachable {
                    remote: e.remote,
                    detail: e.detail,
                }));
            }
        }
    }
    Ok(None)
}

/// Fails closed on an existing local or remote release tag (Pillar II.2), and fails closed too when
/// a configured remote cannot be reached: an unanswered remote check is not "no remote tag" (#262).
pub(crate) fn check_tag_collisions(repo: &Path, tag: &str, publish_push: bool) -> Result<(), Error> {
    match find_tag_collision(repo, tag, publish_push)? {
        None => Ok(()),
        Some(collision) => Err(collision.into_error(tag)),
    }
}
