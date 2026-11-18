//! Stage 2 of the release computation: the release scope, the manifest changes, the changelog entry
//! and the phase list `cutver plan` reports.
//!
//! A sibling file so every module under `src/bump/plan/` stays inside the module budget; the entry
//! point is still [`ReleasePlan::finish`], called by the pipeline and by `cutver plan` alike.

use crate::bump::Error;
use crate::bump::exec::{
    ChangelogPlanParams, assemble_scope, compute, format_command, prepare_changelog, preview_notes,
};
use crate::config::Config;

use super::guards::NAMED_NOTHING;
use super::{ComputeRequest, PlanChangelog, PlanManifest, PlanPhase, ReleasePlan, relative};

impl ReleasePlan {
    /// Stage 2: the release scope, the manifest changes and the changelog entry.
    ///
    /// Runs after the steps a real run performs between the two stages (`[preflight]`, `on_pre_bump`,
    /// `[hooks] pre_bump`), so the changelog is still read after them and never before.
    pub(crate) fn finish(mut self, config: &Config, request: ComputeRequest<'_>) -> Result<Self, Error> {
        let cl_params = ChangelogPlanParams {
            config,
            repo: &config.root_dir,
            current: &self.current_version,
            next: &self.next_version,
            tag: &self.tag,
            auto_commits: self.auto_commits.take(),
            dry_run: !request.mode.writes_changelog(),
            first_release: self.first_release,
            plugin_manager: self.plugin_manager.as_deref(),
            scope: None,
        };
        let scope = assemble_scope(&cl_params)?;
        // The release scope is assembled before manifests are computed, so a template sink renders
        // with the same context the changelog will use.
        self.changes = compute(
            config,
            &self.next_version,
            Some(&scope.context),
            self.plugin_manager.as_ref(),
        )?;
        self.manifests = self
            .changes
            .iter()
            .map(|change| PlanManifest {
                path: relative(&change.path, &config.root_dir),
                from: change.old.to_string(),
                to: self.next_version.to_string(),
            })
            .collect();
        let cl_params = ChangelogPlanParams {
            scope: Some(scope),
            ..cl_params
        };
        self.changelog = config.changelog.path.clone().map(|path| PlanChangelog {
            path: relative(&path, &config.root_dir),
            prepends: self.next_version.to_string(),
        });
        if request.mode.read_only() {
            let path = config
                .changelog
                .path
                .clone()
                .unwrap_or_else(|| NAMED_NOTHING.to_string());
            self.notes = preview_notes(cl_params)?.ok_or(Error::PlanNotes {
                path,
                tag: self.tag.clone(),
                reason: "the changelog renders no entry this release could prepend".to_string(),
            })?;
            self.publish_commands = config
                .publish
                .commands
                .iter()
                .map(|command| format_command(command, &self.interp_ctx))
                .collect::<Result<Vec<_>, _>>()?;
            self.phases = self.derive_phases(config)?;
        } else {
            let plan = prepare_changelog(cl_params)?;
            self.warnings = plan.warnings.clone();
            self.changelog_plan = Some(plan);
        }
        Ok(self)
    }

    /// The release in order, derived from what the plan already decided (#326 S6).
    ///
    /// Every step names data this computation holds — the preflight steps it planned, the manifests it
    /// computes, the tag this run would create — so the phase list cannot describe a pipeline that is
    /// not the one `bump` runs.
    pub(super) fn derive_phases(&self, config: &Config) -> Result<Vec<PlanPhase>, Error> {
        let mut validate = Vec::new();
        if config.git.require_clean_tree {
            validate.push("working tree is clean".to_string());
        }
        if let Some(branch) = &config.git.require_branch {
            validate.push(format!("on branch '{branch}'"));
        }
        validate.push(format!("tag '{}' is a valid git ref", self.tag));
        validate.push(format!("release tag '{}' is free locally and on 'origin'", self.tag));
        for step in &self.preflight {
            let skipped = if step.skipped { " (skipped)" } else { "" };
            validate.push(format!("preflight {}: {}{skipped}", step.name, step.command));
        }

        let mut mutate = Vec::new();
        if let Some(raw) = &config.hooks.pre_bump {
            let command = format_command(raw, &self.interp_ctx)?;
            mutate.push(format!("hook pre_bump: {command}"));
        }
        for manifest in &self.manifests {
            mutate.push(format!(
                "manifest {}: {} -> {}",
                manifest.path, manifest.from, manifest.to
            ));
        }
        if let Some(changelog) = &self.changelog {
            mutate.push(format!("changelog {}: prepend {}", changelog.path, changelog.prepends));
        }
        if let Some(raw) = &config.hooks.post_bump {
            let command = format_command(raw, &self.interp_ctx)?;
            mutate.push(format!("hook post_bump: {command}"));
        }
        mutate.push(format!("commit: {}", self.commit_message));

        let mut publish = vec![format!("tag {} (annotated)", self.tag)];
        if config.publish.push {
            if let Some(branch) = &self.branch {
                publish.push(format!("push branch '{branch}'"));
            }
            publish.push(format!("push tag '{}'", self.tag));
        }
        if let Some(floating) = &self.floating_tag {
            publish.push(format!("move floating major tag '{floating}'"));
            if config.publish.push {
                publish.push(format!("force-push tag '{floating}'"));
            }
        }
        publish.extend(
            self.publish_commands
                .iter()
                .map(|command| format!("publish: {command}")),
        );

        Ok(vec![
            PlanPhase {
                name: "validate".to_string(),
                mutates: false,
                steps: validate,
            },
            PlanPhase {
                name: "mutate".to_string(),
                mutates: true,
                steps: mutate,
            },
            PlanPhase {
                name: "publish".to_string(),
                mutates: true,
                steps: publish,
            },
        ])
    }

    /// The tree renderer's view of the document: the plan, in the shape the dry run already prints.
    pub(crate) fn summary(&self) -> crate::bump::Summary {
        crate::bump::Summary {
            source: self.source.clone(),
            current: self.current_version.clone(),
            next: self.next_version.clone(),
            dry_run: true,
            preflight: self.preflight.clone(),
            touched: self
                .manifests
                .iter()
                .map(|manifest| crate::bump::Touched {
                    path: manifest.path.clone(),
                    old: manifest.from.clone(),
                    new: manifest.to.clone(),
                })
                .collect(),
            changelog: self.changelog.as_ref().map(|changelog| changelog.path.clone()),
            commit_message: self.commit_message.clone(),
            commit: self.commit.clone(),
            tag: self.tag.clone(),
            tag_skipped: false,
            floating_tag: self.floating_tag.clone(),
            pre_bump: None,
            post_bump: None,
            warnings: self.warnings.clone(),
            publish_push: self.publish_push,
            publish_push_command: None,
            publish_commands: self.publish_commands.clone(),
            deferred: false,
            rationale: self.rationale.clone(),
            root_dir: Some(self.root_dir.clone()),
        }
    }
}
