use super::parser::{extract_latest, extract_version};
use crate::changelog::Error;
use std::fs;
use std::path::Path;

/// Read a changelog file and extract its latest release notes.
///
/// Returns `Ok(String)` with the release notes or `Err(Error::NoReleaseSection)` if no release section is present.
pub fn read_latest(path: impl AsRef<Path>, include_header: bool) -> Result<String, Error> {
    let path = path.as_ref();
    let path_str = path.display().to_string();
    let content = fs::read_to_string(path).map_err(|e| Error::Read {
        path: path_str.clone(),
        source: e,
    })?;

    extract_latest(&content, include_header).ok_or(Error::NoReleaseSection { path: path_str })
}

/// Read a changelog file and extract release notes for a specific `target_version`.
///
/// Returns `Ok(String)` with the release notes or `Err(Error::VersionNotFound)` if the version is not present.
pub fn read_version(path: impl AsRef<Path>, target_version: &str, include_header: bool) -> Result<String, Error> {
    let path = path.as_ref();
    let path_str = path.display().to_string();
    let content = fs::read_to_string(path).map_err(|e| Error::Read {
        path: path_str.clone(),
        source: e,
    })?;

    extract_version(&content, target_version, include_header).ok_or_else(|| Error::VersionNotFound {
        version: target_version.to_string(),
        path: path_str,
    })
}
