//! The next version a release cuts, the tag it follows, and the commits the changelog renders from.
//!
//! One authority for both readers: `cutver bump` and `cutver plan` cannot disagree about the
//! release they describe because there is only one computation.

use std::sync::Arc;

use semver::Version;

use crate::bump::Error;
use crate::bump::types::BumpLevel;
use crate::config::Config;
use crate::conventional;
use crate::git;
use crate::plugin::PluginManager;
use crate::semver_bump::{self, Bump, Channel};

pub(super) type NextVersionResolution = (
    Version,
    Option<String>,
    Option<Vec<conventional::ConventionalCommit>>,
    Option<conventional::BumpRationale>,
);

/// The version with the channel the caller asked for, if they asked for one (#326 S1). A caller that
/// named no channel keeps the version the level and the manifests already decided.
fn on_channel(version: Version, channel: Option<Channel>) -> Version {
    match channel {
        Some(channel) => semver_bump::apply_channel(&version, channel),
        None => version,
    }
}

/// The next version, the tag it follows, and the commits the changelog renders from.
///
/// Extracted from the pipeline unchanged, so `cutver plan` and `cutver bump` cannot disagree about
/// which version the workspace is about to cut.
pub(super) fn resolve_next_version(
    config: &Config,
    repo: &std::path::Path,
    current: &Version,
    bump_level: BumpLevel,
    first_release: bool,
    channel: Option<Channel>,
    plugin_manager: Option<&Arc<PluginManager>>,
) -> Result<NextVersionResolution, Error> {
    if first_release {
        return Ok((on_channel(current.clone(), channel), None, None, None));
    }
    let previous_tag = git::latest_tag(repo, Some(&config.git.tag_prefix))?;

    if config.version.strategy == "plugin" {
        // A plugin-free caller reaches this only through a configuration `reject_plugin_dependencies`
        // already refused; keeping the branch explicit means no `None` is ever silently ignored.
        let Some(plugin_manager) = plugin_manager else {
            return Err(Error::PlanNeedsPlugin {
                what: "the next version comes from a plugin (`[version] strategy = \"plugin\"`)".to_string(),
                fix: "run `cutver bump --dry-run` to rehearse a plugin-driven release.".to_string(),
            });
        };
        let plugin_name = match &config.version.plugin {
            Some(name) => {
                let p_name = crate::plugin::types::PluginName::new(name)
                    .map_err(|e| Error::InvalidPluginName(name.clone(), e))?;
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

        let req = crate::plugin::dto::VersioningComputeRequest {
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

        return Ok((on_channel(next, channel), previous_tag, None, rationale));
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
    let next = on_channel(semver_bump::bump(current, bump_semver), channel);
    Ok((next, previous_tag, auto_commits, rationale))
}
