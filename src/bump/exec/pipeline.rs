use std::path::Path;
use std::sync::Arc;

use semver::Version;

use crate::bump::exec::changelog::{ChangelogPlanParams, apply_changelog, assemble_scope, prepare_changelog};
use crate::bump::exec::command::{is_known_lockfile, run_publish_command};
use crate::bump::exec::manifest::{apply, compute, current_source};
use crate::bump::exec::transaction::MutationTransaction;
use crate::bump::types::BumpLevel;
use crate::bump::{Error, Summary};
use crate::changelog;
use crate::config::Config;
use crate::conventional;
use crate::git;
use crate::plugin::PluginManager;
use crate::plugin::dto::{PostBumpPayload, PostReleasePayload, PreBumpPayload, VersioningComputeRequest};
use crate::plugin::types::PluginName;
use crate::preflight;
use crate::semver_bump::{self, Bump};

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
    let repo = &config.root_dir;
    git::require_clean_tree(repo, config.git.require_clean_tree)?;
    git::require_branch(repo, config.git.require_branch.as_deref())?;

    let plugin_manager = Arc::new(PluginManager::from_config(&config.plugins)?);
    let (source_entry, _editor, current) = current_source(config, Some(&plugin_manager))?;
    let bump_level = bump_kind.into();
    let (next, prev_tag, auto_commits, rationale) =
        resolve_next_version(config, repo, &current, bump_level, first_release, &plugin_manager)?;
    let tag = git::tag_name(&config.git.tag_prefix, &next.to_string());
    // Phase 1: the tag name is validated before anything is written. `git::tag_name` only
    // concatenates, so an invalid `tag_prefix` used to surface when `git tag` ran — after the
    // release commit existed (#265, finding F-09). The check reuses the `TagName` invariants rather
    // than a second rule (finding F-15: `TagPrefix::format_tag` still returns an unvalidated name),
    // and it runs on a dry run too, so a rehearsal cannot approve a prefix a real run would refuse.
    validate_tag_name(&config.git.tag_prefix, &tag)?;
    let interp_ctx = build_context(
        repo,
        &current,
        &next,
        &tag,
        prev_tag.as_deref(),
        bump_level,
        first_release,
    );
    let commit_message =
        changelog::interpolate_string(&config.git.commit_message, &interp_ctx).map_err(Error::Changelog)?;

    check_tag_collisions(repo, &tag, config.publish.push)?;
    let preflight_plan = preflight::plan(&config.preflight, skip_preflight);
    if !dry_run {
        preflight::run(&preflight_plan)?;
    }

    run_plugin_pre_bump(&plugin_manager, repo, &current, &next, bump_level, &tag, dry_run)?;
    let summary_pre_bump = run_pre_bump_hook(config, &interp_ctx, dry_run)?;

    let cl_params = ChangelogPlanParams {
        config,
        repo,
        current: &current,
        next: &next,
        tag: &tag,
        auto_commits,
        dry_run,
        first_release,
        plugin_manager: Some(&plugin_manager),
        scope: None,
    };
    // The release scope is assembled before manifests are computed, so a template sink renders
    // with the same context the changelog will use. A dry run renders nothing.
    // Assembled on a dry run too: a template sink renders its preview from the same context, and
    // skipping assembly made `bump --dry-run` fail for any configuration that declares a sink.
    let scope = Some(assemble_scope(&cl_params)?);
    let computed = compute(config, &next, scope.as_ref().map(|s| &s.context), Some(&plugin_manager))?;
    let cl_params = ChangelogPlanParams { scope, ..cl_params };
    let changelog_plan = prepare_changelog(cl_params)?;
    // The entry the host just rendered, not a second derivation: a plugin publishes exactly what
    // the changelog says, heading included. `None` when the project declares no changelog.
    let release_notes = changelog_plan
        .update
        .as_ref()
        .and_then(|update| update.updated.as_deref())
        .and_then(changelog::release_notes);
    let mut transaction = MutationTransaction::new(repo);
    let (touched, mut paths_to_stage) = apply(&computed, &next, dry_run, &mut transaction)?;
    apply_changelog(changelog_plan.update, dry_run, &mut transaction, &mut paths_to_stage)?;

    run_plugin_post_bump(&plugin_manager, repo, &current, &next, &tag, &paths_to_stage, dry_run)?;
    let summary_post_bump = run_post_bump_hook(config, repo, &interp_ctx, dry_run, &mut paths_to_stage)?;

    git::stage(repo, &paths_to_stage, dry_run).map_err(Error::Stage)?;
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
    git::commit_ext(repo, &commit_message, dry_run, paths_to_stage.is_empty()).map_err(Error::Commit)?;
    transaction.commit();
    // The sha every post-commit failure names. Empty on a dry run, where nothing is written and the
    // only post-commit step that can still fail is a plugin dispatch.
    let release_commit = if dry_run {
        String::new()
    } else {
        git::rev_parse(repo, "HEAD")?
    };
    let tag_report = git::tag(repo, &tag, &next.to_string(), dry_run).map_err(|source| Error::Tag {
        tag: tag.clone(),
        commit: release_commit.clone(),
        source: Box::new(source),
    })?;
    let tag_skipped = tag_report.is_some() && !dry_run;

    // Publication is ordered by dependency: the branch, then the release tag, then the floating
    // major tag. Each push names exactly one ref, so a rejected ref can never roll back a ref that
    // already landed, and the floating tag moves only after the release push succeeded (#263).
    let mut push_commands = Vec::new();
    if config.publish.push {
        if let Some(cmd) =
            git::push(repo, config.git.require_branch.as_deref(), dry_run).map_err(|source| Error::Push {
                commit: release_commit.clone(),
                tag: tag.clone(),
                source: Box::new(source),
            })?
        {
            push_commands.push(cmd);
        }
        if let Some(cmd) = git::push_tag(repo, &tag, dry_run).map_err(|source| Error::Push {
            commit: release_commit.clone(),
            tag: tag.clone(),
            source: Box::new(source),
        })? {
            push_commands.push(cmd);
        }
    }
    let floating_tag = handle_floating_tag(config, repo, &next, &release_commit, dry_run, &mut push_commands)?;
    let publish_push_command = if push_commands.is_empty() {
        None
    } else {
        Some(push_commands.join(" && "))
    };
    let publish_commands = run_publish_commands(config, &interp_ctx, dry_run, &release_commit, &tag)?;

    run_plugin_post_release(
        &plugin_manager,
        repo,
        &next,
        &tag,
        &release_commit,
        dry_run,
        release_notes.as_deref(),
    )?;

    Ok(Summary {
        source: source_entry.path.clone(),
        current,
        next,
        dry_run,
        preflight: preflight_plan,
        touched,
        changelog: config.changelog.path.clone(),
        commit_message,
        tag,
        tag_skipped,
        floating_tag,
        pre_bump: summary_pre_bump,
        post_bump: summary_post_bump,
        warnings: changelog_plan.warnings,
        publish_push: config.publish.push,
        publish_push_command,
        publish_commands,
        rationale,
        root_dir: Some(config.root_dir.clone()),
    })
}

type NextVersionResolution = (
    Version,
    Option<String>,
    Option<Vec<conventional::ConventionalCommit>>,
    Option<conventional::BumpRationale>,
);

fn resolve_next_version(
    config: &Config,
    repo: &Path,
    current: &Version,
    bump_level: BumpLevel,
    first_release: bool,
    plugin_manager: &PluginManager,
) -> Result<NextVersionResolution, Error> {
    if first_release {
        return Ok((current.clone(), None, None, None));
    }
    let previous_tag = git::latest_tag(repo, Some(&config.git.tag_prefix))?;

    if config.version.strategy == "plugin" {
        let plugin_name = match &config.version.plugin {
            Some(name) => {
                let p_name = PluginName::new(name).map_err(|e| Error::InvalidPluginName(name.clone(), e))?;
                if plugin_manager.config(&p_name).is_none() {
                    return Err(Error::Plugin(crate::plugin::PluginError::PluginNotFound {
                        name: p_name,
                    }));
                }
                p_name
            }
            None => plugin_manager.resolve_versioning_plugin()?.clone(),
        };

        let commits = git::commits_since(repo, previous_tag.as_deref())?;
        let bump_level_str = match bump_level {
            BumpLevel::Patch => "patch",
            BumpLevel::Minor => "minor",
            BumpLevel::Major => "major",
            BumpLevel::Auto => "auto",
        };

        let req = VersioningComputeRequest {
            root_dir: repo.to_string_lossy().into_owned(),
            current_version: current.to_string(),
            bump_level: bump_level_str.to_string(),
            tag_prefix: config.git.tag_prefix.clone(),
            previous_tag: previous_tag.clone(),
            commits,
        };

        let resp = plugin_manager.dispatch_versioning(&plugin_name, &req)?;
        let next = Version::parse(&resp.next_version).map_err(|e| Error::InvalidPluginVersion {
            plugin: plugin_name.to_string(),
            version: resp.next_version,
            reason: e.to_string(),
        })?;

        let rationale = resp.rationale.map(|r| conventional::BumpRationale {
            level: Bump::Patch,
            breaking_count: 0,
            feat_count: 0,
            fix_count: 0,
            other_count: 0,
            breaking_sample: Some(r),
        });

        return Ok((next, previous_tag, None, rationale));
    }

    let mut auto_commits = None;
    let mut rationale = None;
    let bump_semver = match bump_level {
        BumpLevel::Patch => Bump::Patch,
        BumpLevel::Minor => Bump::Minor,
        BumpLevel::Major => Bump::Major,
        BumpLevel::Auto => {
            let commits = git::commits_since(repo, previous_tag.as_deref())?;
            let (deduced_rationale, parsed) = conventional::parse_and_deduce_with_rationale(&commits);
            let deduced = deduced_rationale.level;
            rationale = Some(deduced_rationale);
            auto_commits = Some(parsed);
            deduced
        }
    };
    let next = semver_bump::bump(current, bump_semver);
    Ok((next, previous_tag, auto_commits, rationale))
}

fn build_context(
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
        match bump_level {
            BumpLevel::Patch => "patch",
            BumpLevel::Minor => "minor",
            BumpLevel::Major => "major",
            BumpLevel::Auto => "auto",
        }
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

/// Phase 1 guard: refuses a tag name `git tag` would reject, before the workspace is written.
///
/// The name is derived from user configuration, so it is validated with the `TagName` invariants —
/// the same rule `git tag` relies on — instead of being handed to `git tag` after the commit.
fn validate_tag_name(prefix: &str, tag: &str) -> Result<(), Error> {
    git::TagName::parse(tag)
        .map(|_| ())
        .map_err(|source| Error::InvalidTagName {
            tag: tag.to_string(),
            prefix: prefix.to_string(),
            source,
        })
}

fn check_tag_collisions(repo: &Path, tag: &str, publish_push: bool) -> Result<(), Error> {
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

fn run_pre_bump_hook(
    config: &Config,
    interp_ctx: &changelog::InterpolationContext,
    dry_run: bool,
) -> Result<Option<String>, Error> {
    let Some(raw_pre_bump) = &config.hooks.pre_bump else {
        return Ok(None);
    };
    let cmd = format_command(raw_pre_bump, interp_ctx)?;
    if dry_run {
        return Ok(Some(cmd));
    }
    let (shell, flag) = if cfg!(windows) { ("cmd", "/C") } else { ("sh", "-c") };
    let status = std::process::Command::new(shell)
        .arg(flag)
        .arg(&cmd)
        .current_dir(&config.root_dir)
        .status()
        .map_err(|e| Error::PreBumpHookSpawn {
            command: cmd.clone(),
            source: e,
        })?;
    if !status.success() {
        return Err(Error::PreBumpHookFailed { command: cmd, status });
    }
    Ok(Some(cmd))
}

fn run_post_bump_hook(
    config: &Config,
    repo: &Path,
    interp_ctx: &changelog::InterpolationContext,
    dry_run: bool,
    paths_to_stage: &mut Vec<String>,
) -> Result<Option<String>, Error> {
    let Some(raw_post_bump) = &config.hooks.post_bump else {
        return Ok(None);
    };
    let cmd = format_command(raw_post_bump, interp_ctx)?;
    if dry_run {
        return Ok(Some(cmd));
    }
    let (shell, flag) = if cfg!(windows) { ("cmd", "/C") } else { ("sh", "-c") };
    let status = std::process::Command::new(shell)
        .arg(flag)
        .arg(&cmd)
        .current_dir(&config.root_dir)
        .status()
        .map_err(|e| Error::PostBumpHookSpawn {
            command: cmd.clone(),
            source: e,
        })?;
    if !status.success() {
        return Err(Error::PostBumpHookFailed { command: cmd, status });
    }
    let modified = git::status_files(repo).map_err(Error::Git)?;
    for f in modified {
        let rel_f = Path::new(&f);
        if !is_known_lockfile(rel_f) {
            continue;
        }
        let abs_f = repo.join(rel_f);
        let already_staged = paths_to_stage.iter().any(|p| {
            let p_path = Path::new(p);
            p_path == rel_f || p_path == abs_f
        });
        if !already_staged {
            paths_to_stage.push(f);
        }
    }
    Ok(Some(cmd))
}

/// Moves the floating major tag and force-pushes it, in that order, and only once the release
/// push has succeeded. Moving it before the push is what orphaned `v0` in issue #263. The force
/// refspec lives on this path alone (Pillar II.2).
fn handle_floating_tag(
    config: &Config,
    repo: &Path,
    next: &Version,
    release_commit: &str,
    dry_run: bool,
    push_commands: &mut Vec<String>,
) -> Result<Option<String>, Error> {
    if !config.git.floating_major_tag {
        return Ok(None);
    }
    let f_tag = git::tag_name(&config.git.tag_prefix, &next.major.to_string());
    git::update_floating_tag(repo, &f_tag, "HEAD", dry_run).map_err(|source| Error::FloatingTag {
        tag: f_tag.clone(),
        commit: release_commit.to_string(),
        source: Box::new(source),
    })?;
    if config.publish.push
        && let Some(cmd) = git::push_tag_force(repo, &f_tag, dry_run).map_err(|source| Error::Push {
            commit: release_commit.to_string(),
            tag: f_tag.clone(),
            source: Box::new(source),
        })?
    {
        push_commands.push(cmd);
    }
    Ok(Some(f_tag))
}

fn run_publish_commands(
    config: &Config,
    interp_ctx: &changelog::InterpolationContext,
    dry_run: bool,
    release_commit: &str,
    tag: &str,
) -> Result<Vec<String>, Error> {
    let mut publish_commands = Vec::new();
    for cmd in &config.publish.commands {
        let formatted = format_command(cmd, interp_ctx)?;
        if !dry_run {
            // The last step of a run whose commit and tag already exist, so the failure carries
            // them: it must never look like a failure before anything landed.
            run_publish_command(&formatted, &config.root_dir, config.publish.default_timeout).map_err(|source| {
                Error::PublishAfterCommit {
                    commit: release_commit.to_string(),
                    tag: tag.to_string(),
                    source: Box::new(source),
                }
            })?;
        }
        publish_commands.push(formatted);
    }
    Ok(publish_commands)
}

pub(crate) fn format_command(template: &str, context: &changelog::InterpolationContext) -> Result<String, Error> {
    changelog::interpolate_string(template, context).map_err(Error::Changelog)
}

fn bump_level_name(bump_level: BumpLevel) -> &'static str {
    match bump_level {
        BumpLevel::Patch => "patch",
        BumpLevel::Minor => "minor",
        BumpLevel::Major => "major",
        BumpLevel::Auto => "auto",
    }
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

fn run_plugin_post_release(
    manager: &PluginManager,
    repo: &Path,
    next: &Version,
    tag: &str,
    release_commit: &str,
    dry_run: bool,
    notes: Option<&str>,
) -> Result<(), Error> {
    let commit_sha = if dry_run {
        "0000000000000000000000000000000000000000".to_string()
    } else {
        git::rev_parse(repo, "HEAD")?
    };
    let payload = PostReleasePayload {
        root_dir: repo.to_string_lossy().replace('\\', "/"),
        version: next.to_string(),
        tag_name: tag.to_string(),
        commit_sha,
        dry_run,
        notes: notes.map(str::to_string),
    };
    for plugin_name in manager.plugins_for_event("on_post_release") {
        manager.dispatch_post_release(plugin_name, &payload).map_err(|source| {
            if dry_run {
                // Nothing landed, so the failure must not claim a commit exists.
                Error::Plugin(source)
            } else {
                Error::PostReleasePlugin {
                    plugin: plugin_name.to_string(),
                    tag: tag.to_string(),
                    commit: release_commit.to_string(),
                    source: Box::new(source),
                }
            }
        })?;
    }
    Ok(())
}
