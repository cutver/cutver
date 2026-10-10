//! Pure registry entry parsing: TOML text in, validated types out (Pillar I.1).

use std::path::Path;
use std::str::FromStr;

use semver::Version;
use serde::Deserialize;

use super::digest::Sha256Digest;
use super::tier::Tier;
use super::url::{HttpsUrl, WasmAssetUrl};
use crate::plugin::error::PluginError;
use crate::plugin::types::{Capability, PluginName};

/// A validated registry index entry for one plugin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryEntry {
    /// Plugin identifier; equals the requested entry file stem.
    pub name: PluginName,
    /// Human-readable description.
    pub description: String,
    /// Entry author or maintainer.
    pub author: String,
    /// Trust tier.
    pub tier: Tier,
    /// Canonical project repository (absolute https URL).
    pub repository: HttpsUrl,
    /// Declared capabilities, at least one.
    pub capabilities: Vec<Capability>,
    /// Published releases, at least one.
    pub releases: Vec<RegistryRelease>,
}

/// A single published release of a registry entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryRelease {
    /// Release version.
    pub version: Version,
    /// Secure artifact URL ending in `.wasm`.
    pub url: WasmAssetUrl,
    /// Validated artifact digest.
    pub sha256: Sha256Digest,
}

/// Raw, unvalidated TOML shape. Validation happens field by field below.
#[derive(Debug, Deserialize)]
struct RawEntry {
    name: String,
    description: String,
    author: String,
    tier: String,
    repository: String,
    capabilities: Vec<String>,
    #[serde(default)]
    release: Vec<RawRelease>,
}

/// Raw, unvalidated release shape.
#[derive(Debug, Deserialize)]
struct RawRelease {
    version: String,
    url: String,
    sha256: String,
}

/// Parses a registry entry from TOML, validating every field (Pillar III.1).
///
/// `file_name` supplies the requested plugin name: it must equal the declared
/// `name`, else the parse fails closed. No I/O or network is performed.
pub fn parse_entry(file_name: &str, toml: &str) -> Result<RegistryEntry, PluginError> {
    let requested = requested_name(file_name)?;
    let raw: RawEntry = toml::from_str(toml).map_err(|err| malformed(&requested, err.to_string()))?;
    validate(requested, raw)
}

/// Derives the requested plugin name from the entry file stem (`<name>.toml`).
fn requested_name(file_name: &str) -> Result<PluginName, PluginError> {
    let stem = Path::new(file_name)
        .file_stem()
        .and_then(|part| part.to_str())
        .unwrap_or(file_name);

    PluginName::new(stem).map_err(|err| PluginError::RegistryMalformedEntry {
        name: file_name.to_string(),
        reason: format!("entry file name '{file_name}' does not yield a valid plugin name: {err}"),
    })
}

/// Validates all raw fields and assembles the typed entry.
fn validate(requested: PluginName, raw: RawEntry) -> Result<RegistryEntry, PluginError> {
    let declared = declared_name(&requested, &raw.name)?;
    if declared != requested {
        return Err(PluginError::RegistryNameMismatch {
            requested: requested.to_string(),
            declared: raw.name,
        });
    }

    require_text(&requested, "description", &raw.description)?;
    require_text(&requested, "author", &raw.author)?;
    let repository = parse_repository(&requested, &raw.repository)?;
    let tier = Tier::from_str(&raw.tier).map_err(|_| PluginError::RegistryUnsupportedTier {
        name: requested.to_string(),
        tier: raw.tier.clone(),
    })?;
    let capabilities = parse_capabilities(&requested, &raw.capabilities)?;
    let releases = parse_releases(&requested, raw.release)?;

    Ok(RegistryEntry {
        name: declared,
        description: raw.description,
        author: raw.author,
        tier,
        repository,
        capabilities,
        releases,
    })
}

/// Parses the declared name, which must itself be a valid plugin identifier.
fn declared_name(requested: &PluginName, raw: &str) -> Result<PluginName, PluginError> {
    PluginName::new(raw).map_err(|err| malformed(requested, format!("declared name '{raw}' is invalid: {err}")))
}

/// Rejects blank description/author values.
fn require_text(name: &PluginName, field: &str, value: &str) -> Result<(), PluginError> {
    if value.trim().is_empty() {
        return Err(malformed(name, format!("'{field}' must not be empty")));
    }
    Ok(())
}

/// Validates the repository as an absolute https URL.
fn parse_repository(name: &PluginName, value: &str) -> Result<HttpsUrl, PluginError> {
    HttpsUrl::parse(value).map_err(|err| malformed(name, format!("repository '{value}' is invalid: {err}")))
}

/// Maps every capability string through the canonical `Capability` enum.
fn parse_capabilities(name: &PluginName, values: &[String]) -> Result<Vec<Capability>, PluginError> {
    if values.is_empty() {
        return Err(malformed(name, "'capabilities' must list at least one capability"));
    }

    values
        .iter()
        .map(|value| {
            Capability::from_str(value).map_err(|_| PluginError::RegistryUnknownCapability {
                name: name.to_string(),
                capability: value.clone(),
            })
        })
        .collect()
}

/// Validates the release list: non-empty, unique semver versions.
fn parse_releases(name: &PluginName, raws: Vec<RawRelease>) -> Result<Vec<RegistryRelease>, PluginError> {
    if raws.is_empty() {
        return Err(malformed(name, "at least one [[release]] is required"));
    }

    let mut seen: Vec<Version> = Vec::with_capacity(raws.len());
    let mut releases = Vec::with_capacity(raws.len());
    for raw in raws {
        let release = parse_release(name, raw)?;
        if seen.contains(&release.version) {
            return Err(malformed(
                name,
                format!("duplicate release version '{}'", release.version),
            ));
        }
        seen.push(release.version.clone());
        releases.push(release);
    }

    Ok(releases)
}

/// Validates one release: semver, secure `.wasm` URL, digest shape.
fn parse_release(name: &PluginName, raw: RawRelease) -> Result<RegistryRelease, PluginError> {
    let version = Version::parse(&raw.version).map_err(|err| {
        malformed(
            name,
            format!("release version '{}' is not valid semver: {err}", raw.version),
        )
    })?;
    let url = WasmAssetUrl::parse(&raw.url).map_err(|err| PluginError::RegistryReleaseUrl {
        name: name.to_string(),
        version: version.to_string(),
        url: raw.url.clone(),
        reason: err.to_string(),
    })?;
    let sha256 = Sha256Digest::parse(&raw.sha256).map_err(|_| PluginError::RegistryDigestShape {
        name: name.to_string(),
        value: raw.sha256.clone(),
    })?;

    Ok(RegistryRelease { version, url, sha256 })
}

/// Builds a malformed-entry error naming the plugin and the reason.
fn malformed(name: &PluginName, reason: impl Into<String>) -> PluginError {
    PluginError::RegistryMalformedEntry {
        name: name.to_string(),
        reason: reason.into(),
    }
}
