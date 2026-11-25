//! The `on_post_release` half of the closing phase, out of the facade to stay under the source
//! ceiling. A pure move: the order, the payload and the refusals are unchanged.
//!
//! A resume repeats only what is harmless to repeat, and an `on_post_release` plugin is arbitrary
//! code that may already have run against this release (#326 D2). A rehearsal skips it too, so a
//! `--dry-run` reports the verdict the real run would.

use crate::bump::Error;
use crate::git;
use crate::plugin::PluginOperation;
use crate::plugin::dto::PostReleasePayload;

use super::{Phase, PublicationScope};

pub(super) fn dispatch(scope: &PublicationScope<'_>) -> Result<(), Error> {
    if let Phase::Resuming = scope.phase {
        return Ok(());
    }
    let commit_sha = if scope.dry_run {
        "0000000000000000000000000000000000000000".to_string()
    } else {
        git::rev_parse(scope.repo, "HEAD")?
    };
    let payload = PostReleasePayload {
        root_dir: scope.repo.to_string_lossy().replace('\\', "/"),
        version: scope.version.to_string(),
        tag_name: scope.tag.to_string(),
        commit_sha,
        dry_run: scope.dry_run,
        notes: scope.notes.map(str::to_string),
    };
    for plugin_name in scope
        .plugin_manager
        .plugins_for_event(PluginOperation::OnPostRelease.as_str())
    {
        scope
            .plugin_manager
            .dispatch_post_release(plugin_name, &payload)
            .map_err(|source| {
                if scope.dry_run {
                    // Nothing landed, so the failure must not claim a commit exists.
                    Error::Plugin(source)
                } else {
                    Error::PostReleasePlugin {
                        plugin: plugin_name.to_string(),
                        tag: scope.tag.to_string(),
                        commit: scope.commit.to_string(),
                        source: Box::new(source),
                    }
                }
            })?;
    }
    Ok(())
}
