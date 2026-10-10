//! Pure release selection over a validated registry entry (Pillar I.1).

use semver::{Version, VersionReq};

use super::entry::{RegistryEntry, RegistryRelease};
use crate::plugin::error::PluginError;

/// Selects a release for a validated entry.
///
/// With no `requirement`, the highest **stable** release wins and an entry that
/// only carries pre-releases fails closed. With a `requirement`, it is parsed as
/// a [`VersionReq`] and the highest satisfying release wins; pre-releases are
/// eligible only when the requirement itself names one (semver's own rule).
pub fn select_release<'a>(
    entry: &'a RegistryEntry,
    requirement: Option<&str>,
) -> Result<&'a RegistryRelease, PluginError> {
    let Some(text) = requirement else {
        return select_latest_stable(entry);
    };

    let req = VersionReq::parse(text).map_err(|err| PluginError::RegistryInvalidVersionRequirement {
        name: entry.name.to_string(),
        requirement: text.to_string(),
        reason: err.to_string(),
    })?;

    select_matching(entry, &req, text)
}

/// Highest stable release, or a fail-closed error listing the versions found.
fn select_latest_stable(entry: &RegistryEntry) -> Result<&RegistryRelease, PluginError> {
    let latest = entry
        .releases
        .iter()
        .filter(|release| release.version.pre.is_empty())
        .max_by(|a, b| a.version.cmp(&b.version));

    let Some(latest) = latest else {
        return Err(PluginError::RegistryNoStableRelease {
            name: entry.name.to_string(),
            available: available_versions(entry),
        });
    };

    Ok(latest)
}

/// Highest release satisfying `req`, or a fail-closed error listing the versions found.
fn select_matching<'a>(
    entry: &'a RegistryEntry,
    req: &VersionReq,
    text: &str,
) -> Result<&'a RegistryRelease, PluginError> {
    let best = entry
        .releases
        .iter()
        .filter(|release| req.matches(&release.version))
        .max_by(|a, b| a.version.cmp(&b.version));

    let Some(best) = best else {
        return Err(PluginError::RegistryNoMatchingRelease {
            name: entry.name.to_string(),
            requirement: text.to_string(),
            available: available_versions(entry),
        });
    };

    Ok(best)
}

/// Comma-separated available versions in ascending semver order.
fn available_versions(entry: &RegistryEntry) -> String {
    let mut versions: Vec<&Version> = entry.releases.iter().map(|release| &release.version).collect();
    versions.sort();
    versions
        .iter()
        .map(|version| version.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}
