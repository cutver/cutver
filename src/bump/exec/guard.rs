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

/// Fails closed on an existing local or remote release tag (Pillar II.2).
pub(crate) fn check_tag_collisions(repo: &Path, tag: &str, publish_push: bool) -> Result<(), Error> {
    if git::tag_exists(repo, tag)? {
        let at = git::rev_parse(repo, &format!("{tag}^{{commit}}"))?;
        return Err(Error::TagExists {
            tag: tag.to_string(),
            commit: at,
        });
    }
    if (git::has_remote(repo, "origin") || publish_push)
        && let Some(commit) = git::remote_tag_exists(repo, "origin", tag)?
    {
        return Err(Error::RemoteTagExists {
            tag: tag.to_string(),
            remote: "origin".to_string(),
            commit,
        });
    }
    Ok(())
}
