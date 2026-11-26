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

impl super::ReleasePlan {
    /// Nothing to release: the request settled on the version the workspace already carries (#326
    /// D1, D1a). Only a `stable` channel with no level named on a stable version settles there, and
    /// a versioning plugin that answers with the current version is reported the same way. A first
    /// release is never a no-op: the version it names has no tag yet, which is the whole release.
    pub(crate) fn no_op(&self) -> bool {
        !self.first_release && self.next_version == self.current_version
    }
}

/// What a release does about the floating major tag: the tag it moves, or the line saying why it
/// deliberately moves none (#326 T6).
///
/// One value instead of two calls, so the report cannot disagree with the decision it explains. A
/// deliberate skip is reported, never silently dropped: the tree prints a `Stable-only` section the way
/// it prints a `No-op` one, because a line that simply disappears makes a reader infer the reason.
pub(super) struct FloatingTag {
    pub(super) tag: Option<String>,
    /// `Some` only for the skip a reader has to be told about: a prerelease target on a configuration
    /// that asked for the tag. The two reasons a release named no floating tag before T6 — a no-op and
    /// `floating_major_tag = false` — still report nothing, exactly as they did.
    pub(super) skipped: Option<String>,
}

/// The floating major tag this release moves, or the line that says why it moves none.
///
/// **Stable-only, and it is one rule with two decision points.** ADR-0005's decision states it whole: a
/// prerelease publishes exactly its own version, and the floating major tag, container `latest`, the
/// Homebrew tap and the Scoop bucket are stable-only. `uses: cutver/setup@v0` defaults to
/// `version: latest`, so a `v0` that follows a prerelease hands an alpha to every consumer who asked
/// for the default — the one hazard the rule exists for. This function is the plan's half of that
/// decision and is deliberately a function rather than an inline expression, because
/// `src/bump/exec/publish.rs` decides the same thing again for `cutver publish`, a separate process
/// that never reads a plan, and the two must answer from the same rule: the version being released.
///
/// A no-op moves nothing, which is why the caller passes it in: a plan that releases nothing reports no
/// tag either (#326 D1a).
pub(super) fn floating_tag(config: &Config, next: &Version, no_op: bool) -> FloatingTag {
    let named = git::tag_name(&config.git.tag_prefix, &next.major.to_string());
    if no_op || !config.git.floating_major_tag {
        return FloatingTag {
            tag: None,
            skipped: None,
        };
    }
    if semver_bump::channel_of(next) == Channel::Stable.as_str() {
        return FloatingTag {
            tag: Some(named),
            skipped: None,
        };
    }
    FloatingTag {
        tag: None,
        skipped: Some(format!(
            "the floating major tag '{named}' is not moved: {next} is a prerelease, and that tag is stable-only (ADR-0005)"
        )),
    }
}

/// The version the run cuts: the level (or the plugin) decides the base, and a channel the caller
/// named decides what the workspace is on top of it (#326 S1, D1). A caller that named no channel
/// keeps the version the level and the manifests already decided.
fn on_channel(current: &Version, level_version: Version, channel: Option<Channel>) -> Version {
    match channel {
        Some(channel) => semver_bump::next_on_channel(current, &level_version, channel),
        None => level_version,
    }
}

/// The next version, the tag it follows, and the commits the changelog renders from.
///
/// Extracted from the pipeline unchanged, so `cutver plan` and `cutver bump` cannot disagree about
/// which version the workspace is about to cut.
///
/// **What a promotion does to the release, stated before it is written (#326 T3).** Promotion is a
/// channel applied to a version that is already a prerelease, and what it produces is an ordinary
/// release, not a special case of one:
///
/// * the tag is the release tag with no suffix — `1.2.0-rc.1 --channel stable` creates `v1.2.0`;
/// * the floating major tag moves onto it exactly like any other stable release moves it, because
///   that is what this is: the move depends on the release, not on the channel that carried it here;
/// * the changelog entry is the release's own version with no suffix. The configured heading renders
///   the release *tag*, so the default `tag_prefix = "v"` makes it `## [v<X.Y.Z>]`
///   (`src/changelog/update.rs:97`, fed from `params.tag` in `src/bump/exec/changelog.rs:303`), and
///   never the suffix the workspace promoted from. The prerelease entries stay as their own history:
///   the run prepends one entry and rewrites none, and the already-carried check matches the exact
///   `[<tag>]` (`src/changelog/update.rs:51`), which `## [v1.2.0-rc.1]` does not — so the promotion
///   always gets its own entry instead of being suppressed by the candidate it replaces.
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
        // A first release tags the version the manifests already declare, so a channel is applied to
        // that version and nothing is promoted: promotion is about a prerelease that already exists
        // (#326 D1), and this is the one run whose version the operator wrote by hand.
        let next = match channel {
            Some(channel) => semver_bump::apply_channel(current, channel),
            None => current.clone(),
        };
        return Ok((next, None, None, None));
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

        return Ok((on_channel(current, next, channel), previous_tag, None, rationale));
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
    // #326 D1a: naming `stable` is never by itself a no-op — a named level still cuts the version it
    // names — but with *no* level named (`auto`, the CLI default) `stable` on a version that is
    // already a release asks for the release the workspace already is. Returning it unchanged is what
    // the caller reports as a no-op, instead of cutting a patch nobody asked for.
    let no_level_named = matches!(bump_level, BumpLevel::Auto);
    if channel == Some(Channel::Stable) && no_level_named && current.pre.is_empty() {
        return Ok((current.clone(), previous_tag, auto_commits, rationale));
    }
    let next = on_channel(current, semver_bump::bump(current, bump_semver), channel);
    Ok((next, previous_tag, auto_commits, rationale))
}
