pub mod io;
pub mod parser;
#[cfg(test)]
mod tests;

use crate::changelog::Error;
use crate::git::TagPrefix;
use std::fs;
use std::path::Path;

pub use io::{read_latest, read_version};
pub use parser::{
    extract_latest, extract_latest_with_prefix, extract_version, extract_version_with_prefix, list_versions,
    list_versions_with_prefix,
};

/// Read a changelog file and extract its latest release notes under `prefix`.
///
/// The prefix-aware form of [`read_latest`]: `[git] tag_prefix = "release-"` makes
/// `## release-1.2.0` the newest release heading.
/// Returns `Ok(String)` with the release notes or `Err(Error::NoReleaseSection)` if no release
/// section is present.
pub fn read_latest_with_prefix(
    path: impl AsRef<Path>,
    include_header: bool,
    prefix: &TagPrefix,
) -> Result<String, Error> {
    let path = path.as_ref();
    let path_str = path.display().to_string();
    let content = read_changelog(path)?;

    extract_latest_with_prefix(&content, include_header, prefix).ok_or(Error::NoReleaseSection { path: path_str })
}

/// Read a changelog file and extract release notes for a specific `target_version` under `prefix`.
///
/// The prefix-aware form of [`read_version`]: the configured `prefix` decides what a heading
/// carries, and a `v`/`V` heading marker is still accepted.
/// Returns `Ok(String)` with the release notes or `Err(Error::VersionNotFound)` if the version is
/// not present.
pub fn read_version_with_prefix(
    path: impl AsRef<Path>,
    target_version: &str,
    include_header: bool,
    prefix: &TagPrefix,
) -> Result<String, Error> {
    let path = path.as_ref();
    let path_str = path.display().to_string();
    let content = read_changelog(path)?;

    extract_version_with_prefix(&content, target_version, include_header, prefix).ok_or_else(|| {
        Error::VersionNotFound {
            version: target_version.to_string(),
            path: path_str,
        }
    })
}

fn read_changelog(path: &Path) -> Result<String, Error> {
    fs::read_to_string(path).map_err(|e| Error::Read {
        path: path.display().to_string(),
        source: e,
    })
}

/// The release entry a published body should carry: the newest section of an already-rendered
/// changelog, heading included.
///
/// The host renders the changelog through the project's template first, so this is the same text
/// the release entry holds — one source, no second rendering. A plugin receives it instead of
/// re-deriving notes from the file, which is what keeps a published release and its changelog
/// byte-identical.
pub fn release_notes(rendered_changelog: &str) -> Option<String> {
    extract_latest(rendered_changelog, true)
}
