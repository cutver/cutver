//! The irreversible half of a release, in its own module so it can run after verification (#259).
//!
//! `run_with_first_release` does everything that must precede the release commit and then delegates
//! here: the annotated tag, its push, the floating major tag, `[publish].commands` and the
//! `on_post_release` plugins. `cutver publish` runs this same code from a separate process, after
//! the built artifacts were smoked, which is why the sequence lives here instead of inline in the
//! pipeline: one authority, two entry points.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use semver::Version;

use crate::bump::Error;
use crate::bump::exec::command::{format_command, read, run_publish_command};
use crate::bump::exec::guard;
use crate::bump::exec::manifest::current_source;
use crate::bump::types::BumpLevel;
use crate::changelog;
use crate::config::Config;
use crate::git;
use crate::git::ops::{tag_force_push_args, tag_push_args};
use crate::plugin::PluginManager;
use crate::plugin::dto::PostReleasePayload;

/// Everything the publication half reads. One struct instead of ten positional arguments.
pub(crate) struct PublicationScope<'a> {
    pub config: &'a Config,
    pub plugin_manager: &'a PluginManager,
    pub repo: &'a Path,
    pub interp_ctx: &'a changelog::InterpolationContext,
    pub version: &'a Version,
    pub tag: &'a str,
    pub commit: &'a str,
    pub notes: Option<&'a str>,
    pub dry_run: bool,
    /// Stop after the branch push and leave the rest to `cutver publish` (#259).
    pub defer_publish: bool,
    /// Which entry point is publishing, so a failure describes the state that phase really leaves.
    pub phase: Phase,
}

/// The two entry points. `Inline` is `cutver bump`, which owns the branch push and creates the tag
/// before it pushes anything; `Closing` is `cutver publish`, which starts after an earlier run
/// already pushed the branch, so it pushes no branch and cannot claim nothing was pushed.
#[derive(Clone, Copy)]
pub(crate) enum Phase {
    Inline,
    Closing,
}

/// What the publication half did, for the run summary.
pub(crate) struct Published {
    pub tag_skipped: bool,
    pub floating_tag: Option<String>,
    pub publish_push_command: Option<String>,
    pub publish_commands: Vec<String>,
    /// True when this run stopped before publishing, so the summary can name the command that
    /// finishes it instead of claiming nothing was configured.
    pub deferred: bool,
}

/// The pushes a publication run made, kept per caller: `planned` is what a dry run reports (the
/// pipeline's summary names a command only when it did not run it), `performed` is what was really
/// sent, which is what `cutver publish` reports after it closed a release.
#[derive(Default)]
pub(crate) struct PushLog {
    pub planned: Vec<String>,
    pub performed: Vec<String>,
}

impl Published {
    fn deferred(planned: &[String]) -> Self {
        Self {
            tag_skipped: false,
            floating_tag: None,
            publish_push_command: joined(planned),
            publish_commands: Vec::new(),
            deferred: true,
        }
    }
}

/// The closing phase of a release `bump --defer-publish` prepared (#259).
pub struct PublishRequest {
    pub tag: String,
    /// The release commit the caller verified; when given it must be HEAD.
    pub commit: Option<String>,
    /// Release notes for the `on_post_release` payload.
    pub notes_file: Option<PathBuf>,
    pub dry_run: bool,
}

/// What the closing phase did, for `cutver publish` to print.
#[derive(Debug)]
pub struct PublishSummary {
    pub tag: String,
    pub commit: String,
    pub floating_tag: Option<String>,
    pub publish_push_command: Option<String>,
    pub publish_commands: Vec<String>,
    pub dry_run: bool,
}

/// Publishes a prepared release: tag, tag push, floating major tag, `[publish].commands` and the
/// post-release plugins — after validating every input the caller supplied.
///
/// The validation runs before anything is written or pushed, so a tag that does not match the
/// manifests at HEAD, a commit that is not HEAD, or an existing tag fails closed with the workspace
/// untouched.
pub fn run_publish(config: &Config, request: &PublishRequest) -> Result<PublishSummary, Error> {
    // Runs before any other validation: the declared version is read from the working tree below, so
    // the tree must be the one the tag names — an uncommitted manifest would validate one version
    // and publish another (#259 findings R1-002 and R3-001).
    git::require_clean_tree(&config.root_dir, config.git.require_clean_tree)?;
    let plugin_manager = Arc::new(PluginManager::from_config(&config.plugins)?);
    let (version, head) = validate(config, request, &plugin_manager)?;
    publish_prepared(config, request, &version, &head, &plugin_manager)
}

/// Phase 1 of the closing phase: the same three checks a full run performs, plus the tag/commit
/// match that only a two-phase release needs. Returns the declared version and HEAD.
fn validate(
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
    guard::check_tag_collisions(repo, &request.tag, true)?;
    Ok((version, head))
}

/// Phase 2: build the release context the publish commands interpolate against, then publish.
fn publish_prepared(
    config: &Config,
    request: &PublishRequest,
    version: &Version,
    head: &str,
    plugin_manager: &PluginManager,
) -> Result<PublishSummary, Error> {
    let (interp_ctx, notes) = publish_context(config, request, version)?;
    let mut pushes = PushLog::default();
    let scope = PublicationScope {
        config,
        plugin_manager,
        repo: &config.root_dir,
        interp_ctx: &interp_ctx,
        version,
        tag: &request.tag,
        commit: head,
        notes: notes.as_deref(),
        dry_run: request.dry_run,
        defer_publish: false,
        phase: Phase::Closing,
    };
    let published = finalize(&scope, &mut pushes)?;
    Ok(PublishSummary {
        tag: request.tag.clone(),
        commit: head.to_string(),
        floating_tag: published.floating_tag,
        publish_push_command: joined(&pushes.performed),
        publish_commands: published.publish_commands,
        dry_run: request.dry_run,
    })
}

/// The interpolation context `[publish].commands` render against, plus the notes file when given.
fn publish_context(
    config: &Config,
    request: &PublishRequest,
    version: &Version,
) -> Result<(changelog::InterpolationContext, Option<String>), Error> {
    let repo = &config.root_dir;
    let prev_tag = git::latest_tag(repo, Some(&config.git.tag_prefix))?;
    // The closing phase runs after the bump, so the previous version is only readable from the tag.
    let previous = previous_version(prev_tag.as_deref(), &config.git.tag_prefix, version);
    let context = guard::build_context(
        repo,
        &previous,
        version,
        &request.tag,
        prev_tag.as_deref(),
        BumpLevel::Auto,
        false,
    );
    let notes = request.notes_file.as_ref().map(read).transpose()?;
    Ok((context, notes))
}

/// The version the previous tag names, so the closing phase renders `previous_version` the way the
/// inline phase does. A tag that does not carry `tag_prefix` or a semver after it names no version.
fn previous_version(prev_tag: Option<&str>, tag_prefix: &str, version: &Version) -> Version {
    prev_tag
        .and_then(|tag| tag.strip_prefix(tag_prefix))
        .and_then(|rest| Version::parse(rest).ok())
        .unwrap_or_else(|| version.clone())
}

/// Publishes the release: tag, branch push, tag push, floating major tag, `[publish].commands`,
/// post-release plugins — in that order, recording every push it made in `pushes`.
pub(crate) fn finalize(scope: &PublicationScope<'_>, pushes: &mut PushLog) -> Result<Published, Error> {
    // A deferred run stops after the branch push: the tag, its pushes, the publish commands and
    // the post-release plugins belong to `cutver publish`, which runs once the built artifacts
    // pass their smoke (#259). The commit must be reachable by the jobs that build and smoke it.
    if scope.defer_publish {
        push_branch(scope, pushes)?;
        return Ok(Published::deferred(&pushes.planned));
    }
    // The baseline order: the tag exists locally before the branch reaches the remote, so a failed
    // tag never leaves a release commit published without one (#259 finding R4-001). Only the
    // inline phase pushes the branch; the closing phase starts after an earlier run pushed it.
    let tag_skipped = create_tag(scope)?;
    if let Phase::Inline = scope.phase {
        push_branch(scope, pushes)?;
    }
    push_release_tag(scope, pushes)?;
    let floating_tag = handle_floating_tag(scope, pushes)?;
    let publish_commands = run_publish_commands(scope)?;
    dispatch_post_release(scope)?;
    Ok(Published {
        tag_skipped,
        floating_tag,
        publish_push_command: joined(&pushes.planned),
        publish_commands,
        deferred: false,
    })
}

fn joined(push_commands: &[String]) -> Option<String> {
    if push_commands.is_empty() {
        None
    } else {
        Some(push_commands.join(" && "))
    }
}

/// The command line one push argv names, in the same display form `git::ops::run_push` uses: the
/// summary must name exactly what `git` received, so the argv comes from the same builders.
fn push_command(argv: &[String]) -> String {
    format!("git {}", argv.join(" "))
}

fn create_tag(scope: &PublicationScope<'_>) -> Result<bool, Error> {
    let report = git::tag(scope.repo, scope.tag, &scope.version.to_string(), scope.dry_run)
        .map_err(|source| tag_error(scope, source))?;
    Ok(report.is_some() && !scope.dry_run)
}

/// The tag failure each phase can report honestly: the inline phase is still before every push, the
/// closing phase is not.
fn tag_error(scope: &PublicationScope<'_>, source: crate::git::Error) -> Error {
    let (tag, commit, source) = (scope.tag.to_string(), scope.commit.to_string(), Box::new(source));
    match scope.phase {
        Phase::Inline => Error::Tag { tag, commit, source },
        Phase::Closing => Error::TagInClosingPhase { tag, commit, source },
    }
}

/// Pushes the branch carrying the release commit, one named ref, and only when `[publish] push`
/// asks for it. A failure names the release tag, as it always has: the branch is what makes that
/// tag reachable.
fn push_branch(scope: &PublicationScope<'_>, pushes: &mut PushLog) -> Result<(), Error> {
    if !scope.config.publish.push {
        return Ok(());
    }
    let pushed =
        git::push(scope.repo, scope.config.git.require_branch.as_deref(), scope.dry_run).map_err(|source| {
            Error::Push {
                commit: scope.commit.to_string(),
                tag: scope.tag.to_string(),
                source: Box::new(source),
            }
        })?;
    if let Some(cmd) = pushed {
        pushes.planned.push(cmd);
    }
    Ok(())
}

/// Pushes the release tag, one named ref, and only when `[publish] push` asks for it.
fn push_release_tag(scope: &PublicationScope<'_>, pushes: &mut PushLog) -> Result<(), Error> {
    if !scope.config.publish.push {
        return Ok(());
    }
    let pushed = git::push_tag(scope.repo, scope.tag, scope.dry_run).map_err(|source| Error::Push {
        commit: scope.commit.to_string(),
        tag: scope.tag.to_string(),
        source: Box::new(source),
    })?;
    if let Some(cmd) = pushed {
        pushes.planned.push(cmd);
    }
    pushes.performed.push(push_command(&tag_push_args(scope.tag)));
    Ok(())
}

/// Moves the floating major tag and force-pushes it, in that order, and only once the release push
/// has succeeded. Moving it before the push is what orphaned `v0` in issue #263. The force refspec
/// lives on this path alone (Pillar II.2).
fn handle_floating_tag(scope: &PublicationScope<'_>, pushes: &mut PushLog) -> Result<Option<String>, Error> {
    if !scope.config.git.floating_major_tag {
        return Ok(None);
    }
    let f_tag = git::tag_name(&scope.config.git.tag_prefix, &scope.version.major.to_string());
    git::update_floating_tag(scope.repo, &f_tag, "HEAD", scope.dry_run).map_err(|source| Error::FloatingTag {
        tag: f_tag.clone(),
        commit: scope.commit.to_string(),
        source: Box::new(source),
    })?;
    if !scope.config.publish.push {
        return Ok(Some(f_tag));
    }
    if let Some(cmd) = git::push_tag_force(scope.repo, &f_tag, scope.dry_run).map_err(|source| Error::Push {
        commit: scope.commit.to_string(),
        tag: f_tag.clone(),
        source: Box::new(source),
    })? {
        pushes.planned.push(cmd);
    }
    pushes.performed.push(push_command(&tag_force_push_args(&f_tag)));
    Ok(Some(f_tag))
}

fn run_publish_commands(scope: &PublicationScope<'_>) -> Result<Vec<String>, Error> {
    let mut publish_commands = Vec::new();
    for cmd in &scope.config.publish.commands {
        let formatted = format_command(cmd, scope.interp_ctx)?;
        if !scope.dry_run {
            // The last step of a run whose commit and tag already exist, so the failure carries
            // them: it must never look like a failure before anything landed.
            run_publish_command(&formatted, &scope.config.root_dir, scope.config.publish.default_timeout).map_err(
                |source| Error::PublishAfterCommit {
                    commit: scope.commit.to_string(),
                    tag: scope.tag.to_string(),
                    source: Box::new(source),
                },
            )?;
        }
        publish_commands.push(formatted);
    }
    Ok(publish_commands)
}

fn dispatch_post_release(scope: &PublicationScope<'_>) -> Result<(), Error> {
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
    for plugin_name in scope.plugin_manager.plugins_for_event("on_post_release") {
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
