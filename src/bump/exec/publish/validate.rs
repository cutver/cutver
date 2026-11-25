//! The checks that run before the closing phase writes, tags or pushes anything.
//!
//! `validate` derives the tag from the version the manifests declare at HEAD instead of trusting the
//! command line, refuses a commit that is not HEAD, and settles the one question a resume answers
//! differently: a tag that already exists. `verify_artifacts` refuses a declared `[[artifact]]` glob
//! that matches fewer files than it requires (#326 S2). Both run before `publish_prepared`, whose
//! first write is the tag, and both run on `--dry-run` too: a rehearsal promises the verdict the real
//! run would report.

use std::sync::Arc;

use semver::Version;

use crate::bump::Error;
use crate::bump::exec::guard;
use crate::bump::exec::manifest::current_source;
use crate::config::Config;
use crate::config::artifact::matched_counts;
use crate::git;
use crate::plugin::PluginManager;

use super::PublishRequest;
use super::resume;

/// Verifies that every declared `[[artifact]]` exists before the first irreversible step (#326 S2).
///
/// Runs after [`validate`] and before `publish_prepared`, whose first write is the tag. A dry run
/// verifies too: it promises the verdict the real run would report.
pub(super) fn verify_artifacts(config: &Config) -> Result<(), Error> {
    for (artifact, found) in config.artifact.iter().zip(matched_counts(config)?) {
        if found < artifact.at_least {
            return Err(Error::ArtifactMissing {
                label: artifact.label().to_string(),
                glob: artifact.glob.clone(),
                found,
                required: artifact.at_least,
            });
        }
    }
    Ok(())
}

/// Phase 1 of the closing phase: the same three checks a full run performs, plus the tag/commit
/// match that only a two-phase release needs. Returns the declared version and HEAD.
///
/// The release tag is the one question the two entry points answer differently. A first
/// `cutver publish` fails closed on a tag that already exists, locally or on the remote
/// ([`guard::check_tag_collisions`]); `--resume` reads the same tag as evidence of the release it
/// continues and asks [`resume`] what else the repository has to say about it (#326 D2).
pub(super) fn validate(
    config: &Config,
    request: &PublishRequest,
    plugin_manager: &Arc<PluginManager>,
) -> Result<(Version, String), Error> {
    let repo = &config.root_dir;
    guard::validate_tag_name(&config.git.tag_prefix, &request.tag)?;
    // The manifests checked out at HEAD are the ones the release commit bumped: the tag is derived
    // from what they declare instead of trusted from the command line.
    let (_source, _editor, version) = current_source(config, Some(plugin_manager))?;
    let declared = git::tag_name(&config.git.tag_prefix, &version.to_string());
    if declared != request.tag {
        return Err(Error::PublishTagMismatch {
            tag: request.tag.clone(),
            declared,
        });
    }
    let head = git::rev_parse(repo, "HEAD")?;
    if let Some(expected) = &request.commit
        && !expected.eq_ignore_ascii_case(&head)
    {
        return Err(Error::PublishCommitMismatch {
            expected: expected.clone(),
            actual: head,
        });
    }
    if request.resume {
        resume::require_evidence(config, request, &head)?;
    } else {
        guard::check_tag_collisions(repo, &request.tag, true)?;
    }
    Ok((version, head))
}
