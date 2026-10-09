use semver::Version;
use std::io;
use std::path::Path;

use crate::atomic;
use crate::bump::Error;
use crate::bump::exec::command::read;
use crate::bump::exec::transaction::MutationTransaction;
use crate::changelog;
use crate::config::Config;
use crate::conventional;
use crate::git;

pub(crate) struct ChangelogPlanParams<'a> {
    pub(crate) config: &'a Config,
    pub(crate) repo: &'a Path,
    pub(crate) current: &'a Version,
    pub(crate) next: &'a Version,
    pub(crate) tag: &'a str,
    pub(crate) auto_commits: Option<Vec<conventional::ConventionalCommit>>,
    pub(crate) dry_run: bool,
    pub(crate) first_release: bool,
    pub(crate) plugin_manager: Option<&'a crate::plugin::PluginManager>,
}

pub(crate) struct ChangelogUpdate {
    pub(crate) path: String,
    pub(crate) original: Option<String>,
    pub(crate) updated: Option<String>,
}

/// Result of planning a changelog update: the optional update plus every non-fatal degradation
/// recorded as data. Terminal output belongs to the boundary adapters (Pillar I.1).
pub(crate) struct ChangelogPlan {
    pub(crate) update: Option<ChangelogUpdate>,
    pub(crate) warnings: Vec<String>,
}

/// Records an unreadable contributor list as data instead of silently swallowing it (Pillar III.4).
fn contributor_list_or_warn(result: io::Result<Vec<String>>, warnings: &mut Vec<String>) -> Vec<String> {
    match result {
        Ok(authors) => authors,
        Err(err) => {
            warnings.push(format!(
                "contributor list omitted, could not read commit authors: {err}"
            ));
            Vec::new()
        }
    }
}

/// Records unreadable prior history as data instead of silently swallowing it (Pillar III.4).
fn first_time_contributors_or_warn(result: io::Result<Vec<String>>, warnings: &mut Vec<String>) -> Vec<String> {
    match result {
        Ok(names) => names,
        Err(err) => {
            warnings.push(format!(
                "\"New Contributors\" section omitted, prior history is unreadable: {err}"
            ));
            Vec::new()
        }
    }
}

pub(crate) fn prepare_changelog(params: ChangelogPlanParams<'_>) -> Result<ChangelogPlan, Error> {
    let Some(cl_path) = &params.config.changelog.path else {
        return Ok(ChangelogPlan {
            update: None,
            warnings: Vec::new(),
        });
    };
    let mut warnings = Vec::new();
    let original = if !params.dry_run { Some(read(cl_path)?) } else { None };
    let updated = if !params.dry_run {
        let latest_tag = if params.first_release {
            None
        } else {
            git::latest_tag(params.repo, Some(&params.config.git.tag_prefix))?
        };
        let raw_commits = git::raw_commits_since(params.repo, latest_tag.as_deref()).ok();
        let commits = match (params.first_release, params.auto_commits) {
            (true, _) => {
                let msgs = git::commits_since(params.repo, None)?;
                conventional::parse_and_deduce_bump(&msgs).1
            }
            (false, Some(parsed)) => parsed,
            (false, None) => {
                let msgs = git::commits_since(params.repo, latest_tag.as_deref())?;
                conventional::parse_and_deduce_bump(&msgs).1
            }
        };
        let contributors = contributor_list_or_warn(
            git::list_authors_since(params.repo, latest_tag.as_deref()),
            &mut warnings,
        );
        // Fail-safe policy lives in `git::first_time_contributors`: a first release marks
        // everyone new, but unreadable prior history announces nobody and is reported here.
        let first_time_contributors = first_time_contributors_or_warn(
            git::first_time_contributors(params.repo, latest_tag.as_deref(), &contributors),
            &mut warnings,
        );
        let repository = git::remote_url(params.repo);
        let today = changelog::format_date(std::time::SystemTime::now());
        let (next_ver, curr_ver) = (params.next.to_string(), params.current.to_string());
        let context = changelog::assemble_release_context(changelog::AssembleContextParams {
            version: &next_ver,
            prev_version: Some(&curr_ver),
            tag: params.tag,
            prev_tag: latest_tag.as_deref(),
            date: &today,
            repository,
            parsed_commits: &commits,
            raw_commits: raw_commits.as_deref(),
            contributors,
            first_time_contributors,
            changelog_config: &params.config.changelog,
        });
        let body = changelog::render_body_with_plugin(
            &params.config.changelog,
            &commits,
            Some(&context),
            params.plugin_manager,
        )?;
        let orig_content = original.as_deref().unwrap_or_default();
        changelog::compute_update(
            orig_content,
            params.tag,
            &body,
            params.config.changelog.full_template,
            params.config.changelog.header_template.as_deref(),
            Some(&context),
        )?
    } else {
        None
    };
    Ok(ChangelogPlan {
        update: Some(ChangelogUpdate {
            path: cl_path.clone(),
            original,
            updated,
        }),
        warnings,
    })
}

pub(crate) fn apply_changelog(
    changelog_update: Option<ChangelogUpdate>,
    dry_run: bool,
    transaction: &mut MutationTransaction<'_>,
    paths_to_stage: &mut Vec<String>,
) -> Result<(), Error> {
    let Some(update) = changelog_update else {
        return Ok(());
    };
    if !dry_run && let Some(new_content) = update.updated {
        if let Some(orig) = update.original {
            transaction.record_backup(update.path.clone(), orig);
        }
        if let Err(e) = atomic::write_atomic(&update.path, &new_content) {
            return Err(Error::Changelog(changelog::Error::Write {
                path: update.path,
                source: e,
            }));
        }
    }
    paths_to_stage.push(update.path);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unreadable_prior_history_is_captured_as_warning_data() {
        let mut warnings = Vec::new();
        let cause = io::Error::other("git log failed");

        let names = first_time_contributors_or_warn(Err(cause), &mut warnings);

        assert!(names.is_empty());
        assert_eq!(
            warnings,
            vec!["\"New Contributors\" section omitted, prior history is unreadable: git log failed".to_string()]
        );
    }
}
