use std::path::Path;

use semver::Version;

use crate::bump::exec::changelog::apply_changelog;
use crate::bump::exec::guard::bump_level_name;
use crate::bump::exec::hooks::{run_post_bump_hook, run_pre_bump_hook};
use crate::bump::exec::manifest::apply;
use crate::bump::exec::publish::{self, Phase, PublicationScope};
use crate::bump::exec::transaction::MutationTransaction;
use crate::bump::plan::{self, ComputeRequest, PlanMode};
use crate::bump::types::BumpLevel;
use crate::bump::{Error, Summary};
use crate::changelog;
use crate::config::Config;
use crate::git;
use crate::plugin::PluginManager;
use crate::plugin::dto::{PostBumpPayload, PreBumpPayload};
use crate::preflight;

pub fn run(
    config: &Config,
    bump_kind: impl Into<BumpLevel>,
    dry_run: bool,
    skip_preflight: &[String],
) -> Result<Summary, Error> {
    run_with_first_release(config, bump_kind, dry_run, skip_preflight, false)
}

pub fn run_with_first_release(
    config: &Config,
    bump_kind: impl Into<BumpLevel>,
    dry_run: bool,
    skip_preflight: &[String],
    first_release: bool,
) -> Result<Summary, Error> {
    run_staged(config, bump_kind, dry_run, skip_preflight, first_release, false)
}

/// Same as [`run_with_first_release`], but stops after the bump commit and the branch push: the tag,
/// its pushes, `[publish] commands` and the post-release plugins are left to `cutver publish`, so a
/// CI job can smoke the artifacts built from that commit first (#259).
pub fn run_deferred_publish(
    config: &Config,
    bump_kind: impl Into<BumpLevel>,
    dry_run: bool,
    skip_preflight: &[String],
    first_release: bool,
) -> Result<Summary, Error> {
    run_staged(config, bump_kind, dry_run, skip_preflight, first_release, true)
}

fn run_staged(
    config: &Config,
    bump_kind: impl Into<BumpLevel>,
    dry_run: bool,
    skip_preflight: &[String],
    first_release: bool,
    defer_publish: bool,
) -> Result<Summary, Error> {
    let repo = &config.root_dir;
    let request = ComputeRequest {
        level: bump_kind.into(),
        skip_preflight,
        first_release,
        mode: if dry_run { PlanMode::DryRun } else { PlanMode::Write },
    };
    // Stage 1 of the one computation (`src/bump/plan.rs`): the guards, the versions, the tag, the
    // commit message, the tag collision and the preflight plan — the same values `cutver plan`
    // reports, so a plan and a bump cannot disagree about the release.
    let prepared = plan::ReleasePlan::prepare(config, request)?;
    let plugin_manager = prepared
        .plugin_manager()
        .expect("a release run loads its plugin manager before the guards run");
    let preflight_plan = prepared.preflight().to_vec();
    if !dry_run {
        preflight::run(repo, &preflight_plan)?;
    }

    run_plugin_pre_bump(
        plugin_manager.as_ref(),
        repo,
        &prepared.current_version,
        &prepared.next_version,
        request.level,
        &prepared.tag,
        dry_run,
    )?;
    let summary_pre_bump = run_pre_bump_hook(config, &prepared.interp_ctx, dry_run)?;

    // Stage 2: the release scope, the manifest changes and the changelog entry. It stays here so the
    // changelog is still read after the steps above, which is what the pre-write order always was.
    let mut plan = prepared.finish(config, request)?;
    let changelog_plan = plan.take_changelog_plan();
    let plugin_manager = plan
        .plugin_manager()
        .expect("a release run loads its plugin manager before the guards run");
    // The entry the host just rendered, not a second derivation: a plugin publishes exactly what
    // the changelog says, heading included. `None` when the project declares no changelog.
    let release_notes = changelog_plan
        .update
        .as_ref()
        .and_then(|update| update.updated.as_deref())
        .and_then(changelog::release_notes);
    let mut transaction = MutationTransaction::new(repo);
    // Every fallible step from here to `transaction.commit()` routes its error through
    // `transaction.abort`, which rolls back now and folds any rollback failure into the returned
    // error. Relying on the drop guard alone would discard those failures, because `Drop` cannot
    // report anything (#261).
    let (touched, mut paths_to_stage) =
        apply(plan.changes(), &plan.next_version, dry_run, &mut transaction).map_err(|e| transaction.abort(e))?;
    apply_changelog(changelog_plan.update, dry_run, &mut transaction, &mut paths_to_stage)
        .map_err(|e| transaction.abort(e))?;

    run_plugin_post_bump(
        plugin_manager.as_ref(),
        repo,
        &plan.current_version,
        &plan.next_version,
        &plan.tag,
        &paths_to_stage,
        dry_run,
    )
    .map_err(|e| transaction.abort(e))?;
    let summary_post_bump = run_post_bump_hook(config, repo, &plan.interp_ctx, dry_run, &mut paths_to_stage)
        .map_err(|e| transaction.abort(e))?;

    git::stage(repo, &paths_to_stage, dry_run).map_err(|e| transaction.abort(Error::Stage(e)))?;
    if !dry_run {
        transaction.mark_staged();
    }
    // The point of no return. Once this line succeeds a commit exists, so the files can no longer be
    // rolled back: `transaction.commit()` must stay immediately below, never later. Everything that
    // can be validated without a commit is already done above — the tag name (Phase 1), the tag
    // collisions, the preflight, the changelog render and every hook that mutates files. Everything
    // that follows genuinely needs the commit, so it cannot be moved earlier:
    //
    //   * the annotated tag names the release commit, so it cannot be created before it exists;
    //   * both pushes send that commit and that tag, so they cannot precede either of them;
    //   * the floating major tag is moved onto the release commit, and only after the release push
    //     succeeded (#263);
    //   * the publish commands publish what the pushes sent.
    //
    // Deliberately not reordered: `transaction.commit()` stays here. Moving it after the tag or the
    // pushes would leave the drop guard armed while a commit exists, and a later failure would then
    // rewrite files underneath an existing commit — worse than a clean failure. Every failure below
    // therefore names the commit that already landed and how to unwind it.
    git::commit_ext(repo, &plan.commit_message, dry_run, paths_to_stage.is_empty())
        .map_err(|e| transaction.abort(Error::Commit(e)))?;
    transaction.commit();
    // The sha every post-commit failure names. Empty on a dry run, where nothing is written and the
    // only post-commit step that can still fail is a plugin dispatch.
    let release_commit = if dry_run {
        String::new()
    } else {
        git::rev_parse(repo, "HEAD")?
    };
    // Publication is ordered by dependency: the branch, then the release tag, then the floating
    // major tag. Each push names exactly one ref, so a rejected ref can never roll back a ref that
    // already landed, and the floating tag moves only after the release push succeeded (#263).
    //
    // Publishing lives in `publish::finalize` so `cutver publish` can run the very same code from a
    // separate process once the built artifacts pass their smoke (#259). The branch push is part of
    // it: the tag must exist before the branch reaches the remote (finding R4-001).
    let mut push_commands = publish::PushLog::default();
    let published = publish::finalize(
        &PublicationScope {
            config,
            plugin_manager: plugin_manager.as_ref(),
            repo,
            interp_ctx: &plan.interp_ctx,
            version: &plan.next_version,
            tag: &plan.tag,
            commit: &release_commit,
            notes: release_notes.as_deref(),
            dry_run,
            defer_publish,
            phase: Phase::Inline,
        },
        &mut push_commands,
    )?;

    Ok(Summary {
        source: plan.source,
        current: plan.current_version,
        next: plan.next_version,
        dry_run,
        preflight: preflight_plan,
        touched,
        changelog: config.changelog.path.clone(),
        commit_message: plan.commit_message,
        commit: release_commit.clone(),
        tag: plan.tag,
        tag_skipped: published.tag_skipped,
        floating_tag: published.floating_tag,
        pre_bump: summary_pre_bump,
        post_bump: summary_post_bump,
        warnings: plan.warnings,
        publish_push: config.publish.push,
        publish_push_command: published.publish_push_command,
        publish_commands: published.publish_commands,
        deferred: published.deferred,
        rationale: plan.rationale,
        root_dir: Some(config.root_dir.clone()),
    })
}

fn run_plugin_pre_bump(
    manager: &PluginManager,
    repo: &Path,
    current: &Version,
    next: &Version,
    bump_level: BumpLevel,
    tag: &str,
    dry_run: bool,
) -> Result<(), Error> {
    let payload = PreBumpPayload {
        root_dir: repo.to_string_lossy().replace('\\', "/"),
        current_version: current.to_string(),
        next_version: next.to_string(),
        bump_level: bump_level_name(bump_level).to_string(),
        tag_name: tag.to_string(),
        dry_run,
    };
    for plugin_name in manager.plugins_for_event("on_pre_bump") {
        let res = manager.dispatch_pre_bump(plugin_name, &payload)?;
        if !res.allow {
            return Err(Error::PreBumpRejected {
                plugin: plugin_name.clone(),
                reason: res.reason.unwrap_or_else(|| "pre-bump checks failed".to_string()),
            });
        }
    }
    Ok(())
}

fn run_plugin_post_bump(
    manager: &PluginManager,
    repo: &Path,
    current: &Version,
    next: &Version,
    tag: &str,
    modified_files: &[String],
    dry_run: bool,
) -> Result<(), Error> {
    let payload = PostBumpPayload {
        root_dir: repo.to_string_lossy().replace('\\', "/"),
        current_version: current.to_string(),
        next_version: next.to_string(),
        tag_name: tag.to_string(),
        modified_files: modified_files.to_vec(),
        dry_run,
    };
    for plugin_name in manager.plugins_for_event("on_post_bump") {
        manager.dispatch_post_bump(plugin_name, &payload)?;
    }
    Ok(())
}
