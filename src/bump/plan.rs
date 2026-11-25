//! The release a workspace is about to cut, computed once and reported without writing (#326).
//!
//! `cutver bump` and `cutver plan` read the same struct: the pipeline takes the versions, the tag, the
//! commit message, the preflight steps, the manifest changes and the changelog entry it is about to
//! write from a [`ReleasePlan`], and `cutver plan` takes the document. One computation, two readers —
//! a plan cannot drift from the release it describes because there is nothing else to drift from.
//!
//! The computation runs in two stages, and the pipeline keeps its own steps between them:
//! [`ReleasePlan::prepare`] decides everything a run must settle before it touches the tree, then the
//! pipeline runs `[preflight]`, dispatches `on_pre_bump` and runs the `pre_bump` hook exactly where it
//! always did, and [`ReleasePlan::finish`] assembles the release scope, computes the manifest changes
//! and plans the changelog entry. That split is what preserves `bump`'s ordering: the changelog is
//! still read after those steps, never before them.
//!
//! `cutver plan` is the read-only caller ([`read_only`]): it dispatches no plugin at all, executes no
//! `[hooks]`, no `[preflight]` and no `[publish] commands`, and opens no transaction, so the guards a
//! real run aborts on become `blockers` instead (#326 S1, S4, S5).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use semver::Version;
use serde::ser::SerializeSeq;
use serde::{Serialize, Serializer};

use crate::bump::Error;
use crate::bump::exec::{
    ChangelogPlan, build_context, bump_level_name, current_source, find_tag_collision, validate_tag_name,
};
use crate::bump::types::{BumpLevel, Change};
use crate::changelog::{self, InterpolationContext};
use crate::config::Config;
use crate::conventional;
use crate::git;
use crate::plugin::PluginManager;
use crate::preflight;
use crate::semver_bump::Channel;

mod finish;
mod guards;
mod request;
mod resolve;

#[cfg(test)]
mod tests;

use self::finish::{PlanArtifact, PlanGate};
use self::guards::{branch_blocker, collision_blocker, dirty_tree_blocker, one_line, undeclared_compatibility};
pub(crate) use self::request::{ComputeRequest, PlanMode};
use self::resolve::resolve_next_version;
/// Everything one release decides, plus the document `cutver plan` prints.
///
/// The keys and their order are the contract of `cutver plan --json` (#326 S2); the fields below the
/// document marker are the run's own state and are never serialized.
#[derive(Serialize)]
pub(crate) struct ReleasePlan {
    /// `true` for every plan: computing one dispatches nothing and writes nothing.
    pub(crate) read_only: bool,
    /// The level the release cuts, resolved: `auto` reports the level the commits deduced, and a
    /// `--first-release` plan reports `initial`.
    pub(crate) level: String,
    #[serde(serialize_with = "serialize_version")]
    pub(crate) current_version: Version,
    #[serde(serialize_with = "serialize_version")]
    pub(crate) next_version: Version,
    pub(crate) tag: String,
    pub(crate) floating_tag: Option<String>,
    /// The commit the release would be cut from, read from `HEAD`.
    pub(crate) commit: String,
    pub(crate) branch: Option<String>,
    pub(crate) commit_message: String,
    /// The rendered entry the release would prepend. Never `null`: a plan that cannot render it
    /// fails closed instead (#326 S3).
    pub(crate) notes: String,
    pub(crate) changelog: Option<PlanChangelog>,
    pub(crate) manifests: Vec<PlanManifest>,
    #[serde(serialize_with = "serialize_preflight")]
    pub(crate) preflight: Vec<preflight::Step>,
    pub(crate) phases: Vec<PlanPhase>,
    /// One entry per declared `[[artifact]]`: the declaration plus the count it observes now (#326
    /// S4). Never a blocker — at plan time the assets are not built yet (L7).
    pub(crate) artifacts: Vec<PlanArtifact>,
    /// One entry per gate the release requires, with the conclusion an attestation covers for the
    /// commit this plan reports (#326 S11).
    pub(crate) gates: Vec<PlanGate>,
    /// One self-contained reason per guard a real run would abort on. A blocked plan still exits `0`
    /// (#326 S4, L6).
    pub(crate) blockers: Vec<String>,
    // ---- below here: the run's own state, never part of the document ----
    #[serde(skip)]
    pub(crate) source: String,
    #[serde(skip)]
    pub(crate) root_dir: PathBuf,
    #[serde(skip)]
    pub(crate) rationale: Option<conventional::BumpRationale>,
    #[serde(skip)]
    pub(crate) interp_ctx: InterpolationContext,
    #[serde(skip)]
    auto_commits: Option<Vec<conventional::ConventionalCommit>>,
    #[serde(skip)]
    changes: Vec<Change>,
    #[serde(skip)]
    changelog_plan: Option<ChangelogPlan>,
    #[serde(skip)]
    pub(crate) publish_push: bool,
    #[serde(skip)]
    pub(crate) publish_commands: Vec<String>,
    #[serde(skip)]
    first_release: bool,
    #[serde(skip)]
    pub(crate) warnings: Vec<String>,
    /// The manager a real run dispatches through, loaded by [`ReleasePlan::prepare`] *after* the
    /// guards succeed, so a broken plugin configuration still reports the guard that failed first
    /// exactly like it always did. `None` is `cutver plan`: no plugin is loaded and none is
    /// dispatched (#326 L5).
    #[serde(skip)]
    plugin_manager: Option<Arc<PluginManager>>,
}

/// The changelog a release updates, as the document reports it.
#[derive(Debug, Serialize)]
pub(crate) struct PlanChangelog {
    pub(crate) path: String,
    /// The version the entry the run prepends carries.
    pub(crate) prepends: String,
}

/// One manifest the release rewrites.
#[derive(Debug, Serialize)]
pub(crate) struct PlanManifest {
    pub(crate) path: String,
    pub(crate) from: String,
    pub(crate) to: String,
}

/// One step of the release, in the order the run performs it.
#[derive(Debug, Serialize)]
pub(crate) struct PlanPhase {
    pub(crate) name: String,
    /// Whether reaching this phase writes to the workspace, the repository or a remote.
    pub(crate) mutates: bool,
    pub(crate) steps: Vec<String>,
}

/// The plan `cutver plan` reports: computed read-only, plugin-free, with the guards as data.
pub(crate) fn read_only(
    config: &Config,
    level: BumpLevel,
    skip_preflight: &[String],
    first_release: bool,
    channel: Option<Channel>,
) -> Result<ReleasePlan, Error> {
    reject_plugin_dependencies(config)?;
    let request = ComputeRequest {
        level,
        skip_preflight,
        first_release,
        channel,
        mode: PlanMode::Preview,
    };
    let plan = ReleasePlan::prepare(config, request)?;
    // A no-op settles before stage 2: it has no release scope, no manifest change and no entry, and
    // computing them would render a changelog entry the run never writes (#326 D1, D1a).
    if plan.no_op() {
        return Ok(plan);
    }
    plan.finish(config, request)
}

/// Refuses the configurations a plugin-free plan cannot compute (#326 S1).
///
/// The alternative is dispatching third-party code inside the one command that promises to change
/// nothing, so the plan fails closed and names the surfaces it cannot read instead.
fn reject_plugin_dependencies(config: &Config) -> Result<(), Error> {
    let manifest_plugin = config
        .manifest
        .iter()
        .any(|manifest| matches!(manifest.kind, crate::config::ManifestKind::Plugin { .. }));
    let what = if config.version.strategy == "plugin" {
        "the next version comes from a plugin (`[version] strategy = \"plugin\"`)"
    } else if config.changelog.format == "plugin" {
        "the changelog entry comes from a plugin (`[changelog] format = \"plugin\"`)"
    } else if manifest_plugin {
        "a manifest is read and written by a plugin (`kind = \"plugin\"`)"
    } else {
        return Ok(());
    };
    Err(Error::PlanNeedsPlugin {
        what: what.to_string(),
        fix: "run `cutver bump --dry-run` to rehearse a plugin-driven release, or move the surface \
              this configuration names onto a built-in kind so `cutver plan` can compute it without \
              dispatching a plugin."
            .to_string(),
    })
}

impl ReleasePlan {
    /// Stage 1: everything a run must settle before it touches the tree, or anything else.
    ///
    /// The guards (clean tree, required branch, tag-name validity, existing release tag) abort a real
    /// run exactly as they always did; in [`PlanMode::Preview`] they are collected into `blockers`
    /// instead, because a dirty tree on the wrong branch is the state a CI job plans in (#326 S4, S5).
    pub(crate) fn prepare(config: &Config, request: ComputeRequest<'_>) -> Result<Self, Error> {
        let repo = &config.root_dir;
        let read_only = request.mode.read_only();
        let mut blockers = Vec::new();

        match git::require_clean_tree(repo, config.git.require_clean_tree) {
            Ok(()) => {}
            Err(err) if read_only => blockers.push(dirty_tree_blocker(&err)),
            Err(err) => return Err(err.into()),
        }
        match git::require_branch(repo, config.git.require_branch.as_deref()) {
            Ok(()) => {}
            Err(err) if read_only => blockers.push(branch_blocker(&err, config.git.require_branch.as_deref())),
            Err(err) => return Err(err.into()),
        }

        // A real run loads its plugins only here — after the guards, and never for a plan, whose only
        // promise is that nothing changes.
        let plugin_manager = if read_only {
            None
        } else {
            Some(Arc::new(PluginManager::from_config(&config.plugins, &config.root_dir)?))
        };
        let (source_entry, _editor, current) = current_source(config, plugin_manager.as_ref())?;
        let (next, prev_tag, auto_commits, rationale) = resolve_next_version(
            config,
            repo,
            &current,
            request.level,
            request.first_release,
            request.channel,
            plugin_manager.as_ref(),
        )?;
        // #326 D1a: the request can settle on the version the workspace already carries. Stage 2
        // computes a release scope, manifest changes and a changelog entry, and a no-op has none of
        // them — so it settles here, with every one of those empty, instead of being computed and
        // discarded. This is also the only way the plan reports `notes` without a real release.
        let no_op = next == current && !request.first_release;
        // The level the release cuts, resolved before the guard that reads it. `initial` is reported
        // for `auto` on a first release and for nothing else: an explicitly named level is reported
        // as named, and the guard exempts a first release on its missing previous tag instead.
        let level = match (&rationale, request.level) {
            (_, BumpLevel::Auto) if request.first_release => "initial".to_string(),
            (Some(deduced), BumpLevel::Auto) => bump_level_name(deduced.level.into()).to_string(),
            (_, level) => bump_level_name(level).to_string(),
        };
        let tag = git::tag_name(&config.git.tag_prefix, &next.to_string());
        // #307: a non-patch release declares what it does about compatibility before anything is
        // written — a breaking change, or the reviewed-none footer.
        match undeclared_compatibility(repo, &level, prev_tag.as_deref(), auto_commits.as_deref()) {
            Ok(()) => {}
            // A no-op releases nothing, so there is no compatibility decision it could owe.
            Err(_) if no_op => {}
            Err(err) if read_only => blockers.push(one_line(&err.to_string())),
            Err(err) => return Err(err),
        }
        // Phase 1: the tag name is validated before anything is written. `git::tag_name` only
        // concatenates, so an invalid `tag_prefix` used to surface when `git tag` ran — after the
        // release commit existed (#265, finding F-09). The check reuses the `TagName` invariants rather
        // than a second rule (finding F-15: `TagPrefix::format_tag` still returns an unvalidated name),
        // and it runs on a dry run too, so a rehearsal cannot approve a prefix a real run would refuse.
        match validate_tag_name(&config.git.tag_prefix, &tag) {
            Ok(()) => {}
            Err(err) if read_only => blockers.push(one_line(&err.to_string())),
            Err(err) => return Err(err),
        }
        let interp_ctx = build_context(
            repo,
            &current,
            &next,
            &tag,
            prev_tag.as_deref(),
            request.level,
            request.first_release,
        );
        let commit_message =
            changelog::interpolate_string(&config.git.commit_message, &interp_ctx).map_err(Error::Changelog)?;
        // A no-op creates no tag, so there is nothing to collide with and no remote to ask: the
        // release tag of the version the workspace is on is exactly the state it reports.
        let collision = if no_op {
            None
        } else {
            find_tag_collision(repo, &tag, config.publish.push)?
        };
        match collision {
            None => {}
            Some(collision) if read_only => blockers.push(collision_blocker(&collision)),
            Some(collision) => return Err(collision.into_error(&tag)),
        }

        let preflight = preflight::plan(&config.preflight, request.skip_preflight);
        // Read only when the document will report them: a `bump` never prints either, and a failing
        // `git rev-parse HEAD` must not start a release run that used to succeed.
        let (commit, branch) = if read_only {
            (git::rev_parse(repo, "HEAD")?, interp_ctx.branch.clone())
        } else {
            (String::new(), None)
        };
        let floating_tag = (config.git.floating_major_tag && !no_op)
            .then(|| git::tag_name(&config.git.tag_prefix, &next.major.to_string()));

        Ok(Self {
            read_only: true,
            level,
            current_version: current,
            next_version: next,
            tag,
            floating_tag,
            commit,
            branch,
            commit_message,
            notes: String::new(),
            changelog: None,
            manifests: Vec::new(),
            preflight,
            phases: Vec::new(),
            artifacts: Vec::new(),
            gates: Vec::new(),
            blockers,
            source: source_entry.path.clone(),
            root_dir: repo.clone(),
            rationale,
            interp_ctx,
            auto_commits,
            changes: Vec::new(),
            changelog_plan: None,
            publish_push: config.publish.push,
            publish_commands: Vec::new(),
            first_release: request.first_release,
            warnings: Vec::new(),
            plugin_manager,
        })
    }

    /// The available manifest changes, in declaration order.
    pub(crate) fn changes(&self) -> &[Change] {
        &self.changes
    }

    /// The planned changelog update, for the run that writes it.
    pub(crate) fn take_changelog_plan(&mut self) -> ChangelogPlan {
        self.changelog_plan.take().unwrap_or_else(ChangelogPlan::none)
    }

    /// The preflight steps this plan settled, in declaration order.
    pub(crate) fn preflight(&self) -> &[preflight::Step] {
        &self.preflight
    }

    /// The plugin manager a real run dispatches through. `None` is a plan, which dispatches none.
    pub(crate) fn plugin_manager(&self) -> Option<Arc<PluginManager>> {
        self.plugin_manager.clone()
    }
}

/// A path as the document names it: relative to the workspace root with forward slashes, because a
/// consumer runs inside the checkout and a CI path like `/home/runner/work/…` is not part of the plan.
/// A path that is already relative names the same file and is left alone.
pub(super) fn relative(path: &str, root: &Path) -> String {
    match Path::new(path).strip_prefix(root) {
        Ok(relative) => relative.to_string_lossy().replace('\\', "/"),
        Err(_) => path.to_string(),
    }
}

fn serialize_version<S: Serializer>(version: &Version, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.collect_str(version)
}

/// The document's `preflight` entries: the name and command a consumer schedules, and whether the
/// invocation skipped it. The timeout stays out — it is the host's own budget, not the plan's.
fn serialize_preflight<S: Serializer>(steps: &[preflight::Step], serializer: S) -> Result<S::Ok, S::Error> {
    #[derive(Serialize)]
    struct StepJson<'a> {
        name: &'a str,
        command: &'a str,
        skipped: bool,
    }
    let mut seq = serializer.serialize_seq(Some(steps.len()))?;
    for step in steps {
        seq.serialize_element(&StepJson {
            name: &step.name,
            command: &step.command,
            skipped: step.skipped,
        })?;
    }
    seq.end()
}
