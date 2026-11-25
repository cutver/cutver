//! The irreversible half of a release, in its own module so it can run after verification (#259).
//!
//! `run_with_first_release` does everything that must precede the release commit and then delegates
//! here: the annotated tag, its push, the floating major tag, `[publish].commands` and the
//! `on_post_release` plugins. `cutver publish` runs this same code from a separate process, after
//! the built artifacts were smoked, which is why the sequence lives here instead of inline in the
//! pipeline: one authority, two entry points.
//!
//! A facade, per Contract Pillar V.4: the checks that run before the first write live in
//! [`validate`], and the evidence `cutver publish --resume` accepts lives in [`resume`] (#326 S4).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use semver::Version;

use crate::bump::Error;
use crate::bump::exec::command::{format_command, run_publish_command};
use crate::bump::exec::guard;
use crate::changelog;
use crate::config::Config;
use crate::config::gates::{self, GateName};
use crate::git;
use crate::git::ops::{tag_force_push_args, tag_push_args};
use crate::plugin::PluginManager;

mod context;
mod post_release;
mod resume;
mod validate;

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

/// The entry points. `Inline` is `cutver bump`, which owns the branch push and creates the tag
/// before it pushes anything; `Closing` is `cutver publish`, which starts after an earlier run
/// already pushed the branch, so it pushes no branch and cannot claim nothing was pushed.
/// `Resuming` is the closing phase entered through `--resume`: it repeats the closing steps but
/// dispatches no `on_post_release` plugin, because a plugin is arbitrary code that may already have
/// run in the attempt being continued, which is what a resume refuses to do on a guess (#326 D2).
#[derive(Clone, Copy)]
pub(crate) enum Phase {
    Inline,
    Closing,
    Resuming,
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
    /// The `on_post_release` plugins a resume left alone, named where the phase decision was made.
    pub post_release_left_alone: Vec<String>,
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
            post_release_left_alone: Vec::new(),
        }
    }
}

/// The closing phase of a release `bump --defer-publish` prepared (#259).
pub struct PublishRequest {
    pub tag: String,
    /// The release commit the caller verified; when given it must be HEAD.
    pub commit: Option<String>,
    /// Release notes for the `on_post_release` payload. The value `-` means standard input.
    pub notes_file: Option<PathBuf>,
    /// Gates this release must have an attestation for, in the order the caller asked (#326 S9).
    pub require_gate: Vec<GateName>,
    pub dry_run: bool,
    /// Continue a release an earlier run left unfinished, driven by repository evidence (#326 S4).
    pub resume: bool,
}

/// `--notes-file -` names standard input, not a file called `-`: the notes never have to exist in the
/// worktree the clean-tree guard is checking, which is what makes a transient render impossible to
/// mistake for a change (#306).
pub const NOTES_STDIN: &str = "-";

/// Whether this invocation carries its notes over a stream instead of naming a file.
fn notes_are_streamed(request: &PublishRequest) -> bool {
    request.notes_file.as_deref() == Some(Path::new(NOTES_STDIN))
}

/// What the closing phase did, for `cutver publish` to print.
#[derive(Debug)]
pub struct PublishSummary {
    pub tag: String,
    pub commit: String,
    pub floating_tag: Option<String>,
    pub publish_push_command: Option<String>,
    pub publish_commands: Vec<String>,
    /// The `on_post_release` plugins this run left alone, so the runner prints what happened instead
    /// of rebuilding the manager to name them (#326 D2).
    pub post_release_left_alone: Vec<String>,
    pub dry_run: bool,
}

/// Publishes a prepared release: tag, tag push, floating major tag, `[publish].commands` and the
/// post-release plugins — after validating every input the caller supplied.
///
/// The validation runs before anything is written or pushed, so a tag that does not match the
/// manifests at HEAD, a commit that is not HEAD, or an existing tag fails closed with the workspace
/// untouched.
pub fn run_publish(config: &Config, request: &PublishRequest) -> Result<PublishSummary, Error> {
    // D5, and before anything is read: a resume continues a named release, and `--commit` is what
    // names it. Nothing persists a run record, so without the commit the invocation could only be a
    // plain publish wearing a flag that promises the opposite.
    if request.resume && request.commit.is_none() {
        return Err(Error::ResumeNeedsCommit);
    }
    // Runs before any other validation: the declared version is read from the working tree below, so
    // the tree must be the one the tag names — an uncommitted manifest would validate one version
    // and publish another (#259 findings R1-002 and R3-001).
    // The notes file is the only input this invocation declares, and the release workflow renders it
    // inside the checkout, so the guard is told about it instead of refusing the command's own
    // argument (#302). A streamed invocation declares nothing: it reads stdin, so there is no path to
    // excuse and every untracked entry still aborts (#306). Nothing else is excused: another
    // untracked file, or any tracked change, still aborts.
    let declared_inputs: Vec<PathBuf> = if notes_are_streamed(request) {
        Vec::new()
    } else {
        request.notes_file.iter().cloned().collect()
    };
    // The attestation directory is the second input this invocation declares: an attestation is
    // evidence the command reads, not a change in the tree the tag would carry (#326 S10).
    let mut declared_inputs = declared_inputs;
    declared_inputs.extend(gates::declared_input(config));
    git::status::require_clean_tree_except(&config.root_dir, config.git.require_clean_tree, &declared_inputs)?;
    let plugin_manager = Arc::new(PluginManager::from_config(&config.plugins, &config.root_dir)?);
    let (version, head) = validate::validate(config, request, &plugin_manager)?;
    validate::verify_artifacts(config)?;
    guard::verify_gates(config, &request.require_gate, &head)?;
    publish_prepared(config, request, &version, &head, &plugin_manager)
}

/// Phase 2: build the release context the publish commands interpolate against, then publish.
fn publish_prepared(
    config: &Config,
    request: &PublishRequest,
    version: &Version,
    head: &str,
    plugin_manager: &PluginManager,
) -> Result<PublishSummary, Error> {
    let (interp_ctx, notes) = context::publish_context(config, request, version)?;
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
        phase: if request.resume {
            Phase::Resuming
        } else {
            Phase::Closing
        },
    };
    let published = finalize(&scope, &mut pushes)?;
    Ok(PublishSummary {
        tag: request.tag.clone(),
        commit: head.to_string(),
        floating_tag: published.floating_tag,
        publish_push_command: joined(&pushes.performed),
        publish_commands: published.publish_commands,
        post_release_left_alone: published.post_release_left_alone,
        dry_run: request.dry_run,
    })
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
    let post_release_left_alone = post_release::dispatch(scope)?;
    Ok(Published {
        tag_skipped,
        floating_tag,
        publish_push_command: joined(&pushes.planned),
        publish_commands,
        deferred: false,
        post_release_left_alone,
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

/// The tag failure each phase can report honestly: the inline phase is still before every push, and
/// both closing phases are not.
fn tag_error(scope: &PublicationScope<'_>, source: crate::git::Error) -> Error {
    let (tag, commit, source) = (scope.tag.to_string(), scope.commit.to_string(), Box::new(source));
    match scope.phase {
        Phase::Inline => Error::Tag { tag, commit, source },
        Phase::Closing | Phase::Resuming => Error::TagInClosingPhase { tag, commit, source },
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
