use super::io::{read_latest, read_version};
use super::parser::{
    extract_heading_version, extract_latest, extract_version, is_release_heading, list_versions, normalize_version,
};
use crate::changelog::Error;
use std::fs;

static TMP_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn tmp_file(name: &str) -> std::path::PathBuf {
    let n = TMP_SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    std::env::temp_dir().join(format!("{}-{}-{}.md", name, std::process::id(), n))
}

#[test]
fn test_extract_latest_body_only() {
    let changelog = "\
# Changelog

All notable changes to this project will be documented in this file.

## [1.2.0] - 2026-03-01

### Features
- Add changelog latest command

### Bug Fixes
- Fix parsing edge cases
";
    let latest = extract_latest(changelog, false);
    let expected = "\
### Features
- Add changelog latest command

### Bug Fixes
- Fix parsing edge cases";
    assert_eq!(latest.as_deref(), Some(expected));
}

#[test]
fn test_extract_latest_with_header() {
    let changelog = "\
# Changelog

## [1.2.0] - 2026-03-01

### Features
- Add changelog latest command
";
    let latest = extract_latest(changelog, true);
    let expected = "\
## [1.2.0] - 2026-03-01

### Features
- Add changelog latest command";
    assert_eq!(latest.as_deref(), Some(expected));
}

#[test]
fn test_extract_latest_skips_unreleased() {
    let changelog_bracketed = "\
# Changelog

## [Unreleased]
- Ongoing work

## [1.2.0] - 2026-03-01
- Released feature
";
    assert_eq!(
        extract_latest(changelog_bracketed, false).as_deref(),
        Some("- Released feature")
    );

    let changelog_unbracketed = "\
# Changelog

## Unreleased
- Ongoing work

## 1.2.0
- Released feature
";
    assert_eq!(
        extract_latest(changelog_unbracketed, true).as_deref(),
        Some("## 1.2.0\n- Released feature")
    );

    let changelog_case = "\
# Changelog

## [unreleased]
- Ongoing work

## [v1.0.0] - 2026-01-01
- First release
";
    assert_eq!(
        extract_latest(changelog_case, false).as_deref(),
        Some("- First release")
    );
}

#[test]
fn test_extract_latest_stops_at_next_release() {
    let changelog = "\
# Changelog

## [1.2.0] - 2026-03-01
- Latest feature

## [1.1.0] - 2026-02-01
- Older feature

## [1.0.0] - 2026-01-01
- Initial release
";
    assert_eq!(extract_latest(changelog, false).as_deref(), Some("- Latest feature"));
    assert_eq!(
        extract_latest(changelog, true).as_deref(),
        Some("## [1.2.0] - 2026-03-01\n- Latest feature")
    );
}

#[test]
fn test_extract_latest_returns_none_when_no_releases() {
    // Empty content
    assert_eq!(extract_latest("", false), None);
    assert_eq!(extract_latest("", true), None);

    // Only title and preamble
    let only_title = "# Changelog\n\nAll notable changes will be documented here.\n";
    assert_eq!(extract_latest(only_title, false), None);
    assert_eq!(extract_latest(only_title, true), None);

    // Only unreleased
    let only_unreleased = "# Changelog\n\n## [Unreleased]\n- Some unreleased work\n";
    assert_eq!(extract_latest(only_unreleased, false), None);
    assert_eq!(extract_latest(only_unreleased, true), None);

    // Case-insensitive unreleased without brackets
    let unreleased_bare = "## UNRELEASED\n- Pending\n";
    assert_eq!(extract_latest(unreleased_bare, false), None);
}

#[test]
fn test_read_latest_file() {
    let path = tmp_file("cutver-cl-read-latest");
    let content = "# Changelog\n\n## [1.0.0] - 2026-01-01\n\n- First release notes\n";
    fs::write(&path, content).unwrap();

    let res = read_latest(&path, false).unwrap();
    assert_eq!(res, "- First release notes");

    let res_with_header = read_latest(&path, true).unwrap();
    assert_eq!(res_with_header, "## [1.0.0] - 2026-01-01\n\n- First release notes");

    // File without releases returns Error::NoReleaseSection
    let empty_path = tmp_file("cutver-cl-read-empty");
    fs::write(&empty_path, "# Changelog\n\n## [Unreleased]\n- WIP\n").unwrap();
    let err = read_latest(&empty_path, false).unwrap_err();
    match err {
        Error::NoReleaseSection { path: p } => {
            assert_eq!(p, empty_path.display().to_string());
        }
        other => panic!("expected NoReleaseSection error, got: {other:?}"),
    }

    // Non-existent file returns Error::Read
    let missing_path = tmp_file("cutver-cl-missing");
    let err = read_latest(&missing_path, false).unwrap_err();
    match err {
        Error::Read { path: p, .. } => {
            assert_eq!(p, missing_path.display().to_string());
        }
        other => panic!("expected Read error, got: {other:?}"),
    }
}

#[test]
fn test_extract_version_middle_release() {
    let changelog = "\
# Changelog

All notable changes will be documented in this file.

## [1.2.0] - 2026-03-01
- Latest feature

## [1.1.0] - 2026-02-01
### Features
- Middle feature 1
- Middle feature 2

### Bug Fixes
- Middle bug fix

## [1.0.0] - 2026-01-01
- Initial release
";
    let res = extract_version(changelog, "1.1.0", false);
    let expected = "\
### Features
- Middle feature 1
- Middle feature 2

### Bug Fixes
- Middle bug fix";
    assert_eq!(res.as_deref(), Some(expected));

    let first = extract_version(changelog, "1.2.0", false);
    assert_eq!(first.as_deref(), Some("- Latest feature"));

    let last = extract_version(changelog, "1.0.0", false);
    assert_eq!(last.as_deref(), Some("- Initial release"));
}

#[test]
fn test_extract_version_with_header() {
    let changelog = "\
# Changelog

## [1.1.0] - 2026-02-01
### Features
- Feature A

## [1.0.0] - 2026-01-01
- Initial
";
    let res = extract_version(changelog, "1.1.0", true);
    let expected = "\
## [1.1.0] - 2026-02-01
### Features
- Feature A";
    assert_eq!(res.as_deref(), Some(expected));
}

#[test]
fn test_extract_version_prefix_flexibility() {
    let changelog = "\
# Changelog

## [v0.2.0] - 2026-09-19
- Note for 0.2.0

## [0.1.0] - 2026-08-10
- Note for 0.1.0

## 0.0.9 - 2026-07-01
- Note for 0.0.9
";
    // 0.2.0 matches [v0.2.0]
    assert_eq!(
        extract_version(changelog, "0.2.0", false).as_deref(),
        Some("- Note for 0.2.0")
    );
    // v0.2.0 matches [v0.2.0]
    assert_eq!(
        extract_version(changelog, "v0.2.0", false).as_deref(),
        Some("- Note for 0.2.0")
    );
    // V0.2.0 matches [v0.2.0]
    assert_eq!(
        extract_version(changelog, "V0.2.0", false).as_deref(),
        Some("- Note for 0.2.0")
    );

    // v0.1.0 matches [0.1.0]
    assert_eq!(
        extract_version(changelog, "v0.1.0", false).as_deref(),
        Some("- Note for 0.1.0")
    );
    // 0.1.0 matches [0.1.0]
    assert_eq!(
        extract_version(changelog, "0.1.0", false).as_deref(),
        Some("- Note for 0.1.0")
    );

    // Heading without brackets: 0.0.9 matches 0.0.9 and v0.0.9
    assert_eq!(
        extract_version(changelog, "0.0.9", false).as_deref(),
        Some("- Note for 0.0.9")
    );
    assert_eq!(
        extract_version(changelog, "v0.0.9", false).as_deref(),
        Some("- Note for 0.0.9")
    );
}

#[test]
fn test_extract_version_not_found() {
    let changelog = "\
# Changelog

## [1.0.0] - 2026-01-01
- Initial release
";
    // Nonexistent versions
    assert_eq!(extract_version(changelog, "2.0.0", false), None);
    assert_eq!(extract_version(changelog, "v2.0.0", true), None);

    // Empty or whitespace target versions
    assert_eq!(extract_version(changelog, "", false), None);
    assert_eq!(extract_version(changelog, "   ", false), None);
    assert_eq!(extract_version(changelog, "v", false), None);

    // Empty changelog
    assert_eq!(extract_version("", "1.0.0", false), None);

    // Unreleased only
    let unreleased_only = "# Changelog\n\n## [Unreleased]\n- WIP\n";
    assert_eq!(extract_version(unreleased_only, "1.0.0", false), None);
}

#[test]
fn test_list_versions_skips_unreleased_and_preserves_order() {
    let changelog = "\
# Changelog

All notable changes will be documented in this file.

## [Unreleased]
- WIP feature

## [v1.3.0] - 2026-03-01
- Feature 3

## [1.2.0] - 2026-02-15
- Feature 2

## 1.1.0 - 2026-01-10
- Feature 1

## [V1.0.0]
- Initial release
";
    let versions = list_versions(changelog);
    assert_eq!(versions, vec!["1.3.0", "1.2.0", "1.1.0", "1.0.0"]);

    // Test with unbracketed Unreleased
    let unbracketed = "\
## Unreleased
- WIP

## 0.1.0
- Alpha
";
    assert_eq!(list_versions(unbracketed), vec!["0.1.0"]);

    // Test with only unreleased
    assert!(list_versions("## [Unreleased]\n- WIP\n").is_empty());
    assert!(list_versions("## UNRELEASED\n- WIP\n").is_empty());

    // Test with empty content
    assert!(list_versions("").is_empty());
}

#[test]
fn test_read_version_file() {
    let path = tmp_file("cutver-cl-read-version");
    let content = "\
# Changelog

## [1.1.0] - 2026-02-01
- Second release notes

## [1.0.0] - 2026-01-01
- First release notes
";
    fs::write(&path, content).unwrap();

    // Matching version without header
    let res = read_version(&path, "1.0.0", false).unwrap();
    assert_eq!(res, "- First release notes");

    // Matching version with header and 'v' prefix
    let res_v = read_version(&path, "v1.1.0", true).unwrap();
    assert_eq!(res_v, "## [1.1.0] - 2026-02-01\n- Second release notes");

    // Missing version returns Error::VersionNotFound
    let err = read_version(&path, "9.9.9", false).unwrap_err();
    match err {
        Error::VersionNotFound { version, path: p } => {
            assert_eq!(version, "9.9.9");
            assert_eq!(p, path.display().to_string());
        }
        other => panic!("expected VersionNotFound error, got: {other:?}"),
    }

    // Non-existent file returns Error::Read
    let missing_path = tmp_file("cutver-cl-missing-version");
    let err = read_version(&missing_path, "1.0.0", false).unwrap_err();
    match err {
        Error::Read { path: p, .. } => {
            assert_eq!(p, missing_path.display().to_string());
        }
        other => panic!("expected Read error, got: {other:?}"),
    }
}

#[test]
fn test_heading_helpers_internal() {
    assert!(is_release_heading(" [1.0.0] - 2026-01-01"));
    assert!(is_release_heading("v2.1.0"));
    assert!(!is_release_heading("[Unreleased]"));
    assert!(!is_release_heading("Unreleased"));
    assert!(!is_release_heading("unknown-section"));

    assert_eq!(normalize_version("v1.2.3"), "1.2.3");
    assert_eq!(normalize_version("V1.2.3"), "1.2.3");
    assert_eq!(normalize_version("1.2.3"), "1.2.3");

    assert_eq!(extract_heading_version("[1.0.0] - date"), Some("1.0.0"));
    assert_eq!(extract_heading_version("1.0.0: date"), Some("1.0.0"));
    assert_eq!(extract_heading_version(""), None);
}
