#![cfg(feature = "plugins")]
//! Hermetic integration tests for the typed registry client.
//!
//! No test in this file touches the network: the HTTP boundary is covered by the
//! pure parse/select tests and the index-base constant, while entry fixtures live
//! under `tests/fixtures/registry/`.

use cutver::plugin::PluginError;
use cutver::plugin::registry::{
    INDEX_BASE, RegistryEntry, SHA256_HEX_LEN, Sha256Digest, Tier, entry_url, parse_entry, select_release,
};
use cutver::plugin::types::{Capability, PluginName};

/// Reads a registry fixture from the checked-in fixture directory.
fn fixture(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/registry")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
}

/// Parses a fixture as `<stem>.toml`.
fn parse_fixture(stem: &str) -> Result<RegistryEntry, PluginError> {
    parse_entry(&format!("{stem}.toml"), &fixture(&format!("{stem}.toml")))
}

#[test]
fn index_base_is_the_canonical_registry_url() {
    assert_eq!(
        INDEX_BASE,
        "https://raw.githubusercontent.com/cutver/registry/main/plugins/"
    );

    let name = PluginName::new("github-releases").unwrap();
    assert_eq!(
        entry_url(&name),
        "https://raw.githubusercontent.com/cutver/registry/main/plugins/github-releases.toml"
    );
}

#[test]
fn parses_the_real_github_releases_entry() {
    let entry = parse_fixture("github-releases").unwrap();

    assert_eq!(entry.name.as_str(), "github-releases");
    assert_eq!(entry.author, "cutver");
    assert_eq!(entry.tier, Tier::Official);
    assert_eq!(entry.repository.as_str(), "https://github.com/cutver/plugins");
    assert_eq!(entry.capabilities, vec![Capability::ChangelogV1]);
    assert_eq!(entry.releases.len(), 1);
    assert_eq!(entry.releases[0].version.to_string(), "0.1.0");
    assert_eq!(
        entry.releases[0].sha256.as_str(),
        "a828cbabd247afdaf78fec358ae5a27271097c7c2ef6fc9ec0fe04af22edf841"
    );
}

#[test]
fn parses_a_fixture_staged_in_an_isolated_tempdir() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("github-releases.toml");
    std::fs::write(&path, fixture("github-releases.toml")).unwrap();

    let body = std::fs::read_to_string(&path).unwrap();
    let entry = parse_entry("github-releases.toml", &body).unwrap();

    assert_eq!(entry.name.as_str(), "github-releases");
}

#[test]
fn unknown_capability_fails_closed_naming_the_string() {
    let err = parse_fixture("unknown-capability").unwrap_err();

    match err {
        PluginError::RegistryUnknownCapability { name, capability } => {
            assert_eq!(name, "unknown-capability");
            assert_eq!(capability, "time-travel.v9");
        }
        other => panic!("expected RegistryUnknownCapability, got {other:?}"),
    }
}

#[test]
fn malformed_digest_fails_closed_naming_the_value() {
    let err = parse_fixture("bad-digest").unwrap_err();

    match err {
        PluginError::RegistryDigestShape { name, value } => {
            assert_eq!(name, "bad-digest");
            assert_eq!(value.len(), SHA256_HEX_LEN - 1);
        }
        other => panic!("expected RegistryDigestShape, got {other:?}"),
    }
}

#[test]
fn insecure_release_url_fails_closed() {
    let err = parse_fixture("insecure-url").unwrap_err();

    match err {
        PluginError::RegistryReleaseUrl { name, url, .. } => {
            assert_eq!(name, "insecure-url");
            assert!(url.starts_with("http://"), "unexpected url: {url}");
        }
        other => panic!("expected RegistryReleaseUrl, got {other:?}"),
    }
}

#[test]
fn non_wasm_release_url_fails_closed() {
    let err = parse_fixture("non-wasm-url").unwrap_err();

    match err {
        PluginError::RegistryReleaseUrl {
            name,
            version,
            url,
            reason,
        } => {
            assert_eq!(name, "non-wasm-url");
            assert_eq!(version, "0.1.0");
            assert!(url.ends_with(".tar.gz"));
            assert!(reason.contains(".wasm"), "unexpected reason: {reason}");
        }
        other => panic!("expected RegistryReleaseUrl, got {other:?}"),
    }
}

#[test]
fn name_mismatch_fails_closed() {
    let err = parse_fixture("name-mismatch").unwrap_err();

    match err {
        PluginError::RegistryNameMismatch { requested, declared } => {
            assert_eq!(requested, "name-mismatch");
            assert_eq!(declared, "declared-name");
        }
        other => panic!("expected RegistryNameMismatch, got {other:?}"),
    }
}

#[test]
fn duplicate_version_fails_closed() {
    let err = parse_fixture("duplicate-version").unwrap_err();
    let message = err.to_string();

    assert!(matches!(err, PluginError::RegistryMalformedEntry { .. }));
    assert!(message.contains("duplicate"), "unexpected message: {message}");
}

#[test]
fn empty_releases_fails_closed() {
    let err = parse_fixture("empty-releases").unwrap_err();

    assert!(matches!(err, PluginError::RegistryMalformedEntry { .. }));
}

#[test]
fn unsupported_tier_fails_closed() {
    let err = parse_fixture("bad-tier").unwrap_err();

    match err {
        PluginError::RegistryUnsupportedTier { name, tier } => {
            assert_eq!(name, "bad-tier");
            assert_eq!(tier, "experimental");
        }
        other => panic!("expected RegistryUnsupportedTier, got {other:?}"),
    }
}

#[test]
fn every_malformed_fixture_reports_what_where_and_fix() {
    let stems = [
        "unknown-capability",
        "bad-digest",
        "insecure-url",
        "non-wasm-url",
        "name-mismatch",
        "duplicate-version",
        "empty-releases",
        "bad-tier",
    ];

    for stem in stems {
        let message = parse_fixture(stem).unwrap_err().to_string();
        assert!(message.contains("Where:"), "{stem} lacks Where: {message}");
        assert!(message.contains("Fix:"), "{stem} lacks Fix: {message}");
    }
}

#[test]
fn digest_shape_accepts_and_rejects() {
    let valid = "0".repeat(SHA256_HEX_LEN);
    let too_short = "0".repeat(SHA256_HEX_LEN - 1);
    let uppercase = "A".repeat(SHA256_HEX_LEN);
    let mut non_hex = valid.clone();
    non_hex.pop();
    non_hex.push('g');
    let empty = String::new();

    let cases = [
        (valid.as_str(), true),
        (too_short.as_str(), false),
        (uppercase.as_str(), false),
        (non_hex.as_str(), false),
        (empty.as_str(), false),
    ];

    for (value, expected_ok) in cases {
        let result = Sha256Digest::parse(value);
        assert_eq!(
            result.is_ok(),
            expected_ok,
            "value '{value}' should be {}",
            if expected_ok { "accepted" } else { "rejected" }
        );
    }
}

#[test]
fn capability_strings_map_to_the_existing_enum() {
    let cases = [
        ("manifest.v1", Capability::ManifestV1),
        ("lifecycle.v1", Capability::LifecycleV1),
        ("changelog.v1", Capability::ChangelogV1),
        ("versioning.v1", Capability::VersioningV1),
    ];

    for (text, expected) in cases {
        let entry = parse_entry("capability-map.toml", &entry_with_capability(text)).unwrap();
        assert_eq!(entry.capabilities, vec![expected]);
    }
}

#[test]
fn selects_the_highest_stable_release() {
    let entry = parse_fixture("multi-release").unwrap();
    let release = select_release(&entry, None).unwrap();

    assert_eq!(release.version.to_string(), "0.2.0");
}

#[test]
fn prerelease_is_not_selected_by_default() {
    let entry = parse_fixture("multi-release").unwrap();
    let release = select_release(&entry, None).unwrap();

    assert!(release.version.pre.is_empty());
    assert_ne!(release.version.to_string(), "1.0.0-rc.1");
}

#[test]
fn prerelease_is_selected_when_the_requirement_names_one() {
    let entry = parse_fixture("multi-release").unwrap();
    let release = select_release(&entry, Some("1.0.0-rc.1")).unwrap();

    assert_eq!(release.version.to_string(), "1.0.0-rc.1");
}

#[test]
fn requirement_without_a_prerelease_does_not_match_one() {
    let entry = parse_fixture("multi-release").unwrap();
    let err = select_release(&entry, Some("^1.0")).unwrap_err();

    assert!(matches!(err, PluginError::RegistryNoMatchingRelease { .. }));
}

#[test]
fn fails_closed_when_nothing_satisfies_and_lists_versions() {
    let entry = parse_fixture("multi-release").unwrap();
    let err = select_release(&entry, Some("9.9.9")).unwrap_err();
    let message = err.to_string();

    assert!(matches!(err, PluginError::RegistryNoMatchingRelease { .. }));
    assert!(message.contains("0.1.0"), "missing 0.1.0: {message}");
    assert!(message.contains("0.2.0"), "missing 0.2.0: {message}");
    assert!(message.contains("1.0.0-rc.1"), "missing rc: {message}");
}

#[test]
fn fails_closed_when_only_prereleases_exist() {
    let entry = parse_fixture("prerelease-only").unwrap();
    let err = select_release(&entry, None).unwrap_err();

    assert!(matches!(err, PluginError::RegistryNoStableRelease { .. }));
}

#[test]
fn invalid_version_requirement_fails_closed() {
    let entry = parse_fixture("multi-release").unwrap();
    let err = select_release(&entry, Some("not-a-requirement")).unwrap_err();

    assert!(matches!(err, PluginError::RegistryInvalidVersionRequirement { .. }));
}

/// Builds a minimal valid entry declaring exactly one capability.
fn entry_with_capability(capability: &str) -> String {
    format!(
        r#"
name = "capability-map"
description = "Entry used to map capability strings."
author = "cutver"
tier = "community"
repository = "https://github.com/cutver/plugins"
capabilities = ["{capability}"]

[[release]]
version = "0.1.0"
url = "https://github.com/cutver/plugins/releases/download/capability-map-v0.1.0/capability-map.wasm"
sha256 = "a828cbabd247afdaf78fec358ae5a27271097c7c2ef6fc9ec0fe04af22edf841"
"#
    )
}
