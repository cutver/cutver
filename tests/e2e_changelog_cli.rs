#![cfg(unix)]

mod common;

use common::*;
use std::process::Command;

#[test]
fn changelog_latest_cli_extracts_body_from_discovered_config() {
    let guard = FixtureGuard::new("changelog-latest-discovered");
    let fixture = guard.fixture();
    let cutver_toml = r#"[version]
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[changelog]
path = "CHANGELOG.md"
"#;
    fixture.write("cutver.toml", cutver_toml);
    fixture.write("package.json", r#"{"version": "1.2.0"}"#);
    let changelog_content = r#"# Changelog

## [Unreleased]
- work in progress

## [1.2.0] - 2026-03-01

### Features
- new thing
"#;
    fixture.write("CHANGELOG.md", changelog_content);

    let output = Command::new(env!("CARGO_BIN_EXE_cutver"))
        .current_dir(&fixture.dir)
        .args(["changelog", "latest"])
        .output()
        .expect("failed to execute cutver binary");

    assert!(
        output.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();
    assert!(
        trimmed.contains("### Features\n- new thing"),
        "stdout does not contain features: {stdout}"
    );
    assert!(
        !stdout.contains("## [1.2.0]"),
        "stdout should not contain release header: {stdout}"
    );
    assert!(
        !stdout.contains("[Unreleased]"),
        "stdout should not contain [Unreleased]: {stdout}"
    );
}

#[test]
fn changelog_latest_cli_with_include_header() {
    let guard = FixtureGuard::new("changelog-latest-header");
    let fixture = guard.fixture();
    let cutver_toml = r#"[version]
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[changelog]
path = "CHANGELOG.md"
"#;
    fixture.write("cutver.toml", cutver_toml);
    fixture.write("package.json", r#"{"version": "1.2.0"}"#);
    let changelog_content = r#"# Changelog

## [Unreleased]
- work in progress

## [1.2.0] - 2026-03-01

### Features
- new thing
"#;
    fixture.write("CHANGELOG.md", changelog_content);

    let output = Command::new(env!("CARGO_BIN_EXE_cutver"))
        .current_dir(&fixture.dir)
        .args(["changelog", "latest", "-H"])
        .output()
        .expect("failed to execute cutver binary");

    assert!(
        output.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.trim().starts_with("## [1.2.0] - 2026-03-01"),
        "stdout does not start with header: {stdout}"
    );
    assert!(
        stdout.contains("### Features\n- new thing"),
        "stdout does not contain features: {stdout}"
    );
}

#[test]
fn changelog_latest_cli_with_explicit_path() {
    let guard = FixtureGuard::new("changelog-latest-explicit-path");
    let fixture = guard.fixture();
    let releases_content = r#"# Releases

## [Unreleased]
- upcoming changes

## [2.0.0] - 2026-03-01

- custom release notes
"#;
    fixture.write("RELEASES.md", releases_content);

    let output = Command::new(env!("CARGO_BIN_EXE_cutver"))
        .current_dir(&fixture.dir)
        .args(["changelog", "latest", "-p", "RELEASES.md"])
        .output()
        .expect("failed to execute cutver binary");

    assert!(
        output.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "- custom release notes");
}

#[test]
fn changelog_latest_cli_fails_cleanly_when_no_releases() {
    let guard = FixtureGuard::new("changelog-latest-no-releases");
    let fixture = guard.fixture();
    let changelog_content = r#"# Changelog

## [Unreleased]
- work in progress
"#;
    fixture.write("CHANGELOG.md", changelog_content);

    let output = Command::new(env!("CARGO_BIN_EXE_cutver"))
        .current_dir(&fixture.dir)
        .args(["changelog", "latest"])
        .output()
        .expect("failed to execute cutver binary");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("no release section found in changelog"),
        "stderr does not contain expected error: {stderr}"
    );
}

#[test]
fn changelog_show_cli_extracts_historical_version() {
    let guard = FixtureGuard::new("changelog-show-historical");
    let fixture = guard.fixture();
    let cutver_toml = r#"[version]
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[changelog]
path = "CHANGELOG.md"
"#;
    fixture.write("cutver.toml", cutver_toml);
    fixture.write("package.json", r#"{"version": "1.2.0"}"#);
    let changelog_content = r#"# Changelog

## [1.2.0] - 2026-03-01

- Release 1.2.0 notes

## [1.1.0] - 2026-02-01

- Release 1.1.0 notes

## [1.0.0] - 2026-01-01

- Release 1.0.0 notes
"#;
    fixture.write("CHANGELOG.md", changelog_content);

    let output = Command::new(env!("CARGO_BIN_EXE_cutver"))
        .current_dir(&fixture.dir)
        .args(["changelog", "show", "1.1.0"])
        .output()
        .expect("failed to execute cutver binary");

    assert!(
        output.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("- Release 1.1.0 notes"),
        "stdout does not contain 1.1.0 notes: {stdout}"
    );
    assert!(
        !stdout.contains("- Release 1.2.0 notes"),
        "stdout should not contain 1.2.0 notes: {stdout}"
    );
    assert!(
        !stdout.contains("- Release 1.0.0 notes"),
        "stdout should not contain 1.0.0 notes: {stdout}"
    );
    assert!(
        !stdout.contains("## [1.1.0]"),
        "stdout should not contain release header: {stdout}"
    );
}

#[test]
fn changelog_show_cli_with_include_header() {
    let guard = FixtureGuard::new("changelog-show-header");
    let fixture = guard.fixture();
    let cutver_toml = r#"[version]
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[changelog]
path = "CHANGELOG.md"
"#;
    fixture.write("cutver.toml", cutver_toml);
    fixture.write("package.json", r#"{"version": "1.2.0"}"#);
    let changelog_content = r#"# Changelog

## [1.2.0] - 2026-03-01

- Release 1.2.0 notes

## [1.1.0] - 2026-02-01

- Release 1.1.0 notes

## [1.0.0] - 2026-01-01

- Release 1.0.0 notes
"#;
    fixture.write("CHANGELOG.md", changelog_content);

    let output = Command::new(env!("CARGO_BIN_EXE_cutver"))
        .current_dir(&fixture.dir)
        .args(["changelog", "show", "1.1.0", "-H"])
        .output()
        .expect("failed to execute cutver binary");

    assert!(
        output.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.trim().starts_with("## [1.1.0]"),
        "stdout does not start with header ## [1.1.0]: {stdout}"
    );
    assert!(
        stdout.contains("- Release 1.1.0 notes"),
        "stdout does not contain 1.1.0 notes: {stdout}"
    );
}

#[test]
fn changelog_show_cli_with_version_prefix_v() {
    let guard = FixtureGuard::new("changelog-show-prefix-v");
    let fixture = guard.fixture();
    let cutver_toml = r#"[version]
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[changelog]
path = "CHANGELOG.md"
"#;
    fixture.write("cutver.toml", cutver_toml);
    fixture.write("package.json", r#"{"version": "1.2.0"}"#);
    let changelog_content = r#"# Changelog

## [1.2.0] - 2026-03-01

- Release 1.2.0 notes

## [1.1.0] - 2026-02-01

- Release 1.1.0 notes

## [1.0.0] - 2026-01-01

- Release 1.0.0 notes
"#;
    fixture.write("CHANGELOG.md", changelog_content);

    let output = Command::new(env!("CARGO_BIN_EXE_cutver"))
        .current_dir(&fixture.dir)
        .args(["changelog", "show", "v1.1.0"])
        .output()
        .expect("failed to execute cutver binary");

    assert!(
        output.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "- Release 1.1.0 notes");

    let output_header = Command::new(env!("CARGO_BIN_EXE_cutver"))
        .current_dir(&fixture.dir)
        .args(["changelog", "show", "v1.1.0", "-H"])
        .output()
        .expect("failed to execute cutver binary");

    assert!(
        output_header.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&output_header.stderr)
    );
    assert_eq!(output_header.status.code(), Some(0));
    let stdout_header = String::from_utf8_lossy(&output_header.stdout);
    assert!(
        stdout_header.trim().starts_with("## [1.1.0]"),
        "stdout with -H does not match ## [1.1.0]: {stdout_header}"
    );
}

#[test]
fn changelog_show_cli_fails_cleanly_when_version_missing() {
    let guard = FixtureGuard::new("changelog-show-missing");
    let fixture = guard.fixture();
    let cutver_toml = r#"[version]
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[changelog]
path = "CHANGELOG.md"
"#;
    fixture.write("cutver.toml", cutver_toml);
    fixture.write("package.json", r#"{"version": "1.2.0"}"#);
    let changelog_content = r#"# Changelog

## [1.2.0] - 2026-03-01

- Release 1.2.0 notes
"#;
    fixture.write("CHANGELOG.md", changelog_content);

    let output = Command::new(env!("CARGO_BIN_EXE_cutver"))
        .current_dir(&fixture.dir)
        .args(["changelog", "show", "9.9.9"])
        .output()
        .expect("failed to execute cutver binary");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("version '9.9.9' not found in changelog"),
        "stderr does not contain expected error: {stderr}"
    );
}

#[test]
fn changelog_latest_cli_terminal_styling_with_clicolor_force() {
    let guard = FixtureGuard::new("changelog-latest-style");
    let fixture = guard.fixture();
    let cutver_toml = r#"[version]
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[changelog]
path = "CHANGELOG.md"
"#;
    fixture.write("cutver.toml", cutver_toml);
    fixture.write("package.json", r#"{"version": "1.2.0"}"#);
    let changelog_content = r#"# Changelog

## [1.2.0] - 2026-03-01

### 🚀 Features
- **parser**: improve speed (#42)
"#;
    fixture.write("CHANGELOG.md", changelog_content);

    // Styled run
    let output = Command::new(env!("CARGO_BIN_EXE_cutver"))
        .current_dir(&fixture.dir)
        .env("CLICOLOR_FORCE", "1")
        .args(["changelog", "latest", "-H"])
        .output()
        .expect("failed to execute cutver binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\x1b[36m\x1b[1m[1.2.0] - 2026-03-01\x1b[0m\x1b[0m"));
    assert!(stdout.contains("\x1b[1m🚀 Features\x1b[0m"));
    assert!(stdout.contains("\x1b[36m\x1b[1mparser\x1b[0m\x1b[0m"));
    assert!(stdout.contains("\x1b[90m(#42)\x1b[0m"));

    // Raw run with NO_COLOR
    let output_no_color = Command::new(env!("CARGO_BIN_EXE_cutver"))
        .current_dir(&fixture.dir)
        .env("NO_COLOR", "1")
        .args(["changelog", "latest", "-H"])
        .output()
        .expect("failed to execute cutver binary");

    assert!(output_no_color.status.success());
    let stdout_no_color = String::from_utf8_lossy(&output_no_color.stdout);
    assert!(!stdout_no_color.contains("\x1b["));
    assert!(stdout_no_color.contains("## [1.2.0] - 2026-03-01"));
}

#[test]
fn changelog_latest_cli_json_export() {
    let guard = FixtureGuard::new("changelog-latest-json");
    let fixture = guard.fixture();
    let cutver_toml = r#"[version]
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[changelog]
path = "CHANGELOG.md"
"#;
    fixture.write("cutver.toml", cutver_toml);
    fixture.write("package.json", r#"{"version": "1.2.0"}"#);
    let changelog_content = r#"# Changelog

## [1.2.0] - 2026-03-01

### Features
- **cli**: add json support (#66)
"#;
    fixture.write("CHANGELOG.md", changelog_content);

    let output = Command::new(env!("CARGO_BIN_EXE_cutver"))
        .current_dir(&fixture.dir)
        .env("CLICOLOR_FORCE", "1")
        .args(["changelog", "latest", "--json"])
        .output()
        .expect("failed to execute cutver binary");

    assert!(
        output.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.status.code(), Some(0));

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("\x1b["), "stdout contains ANSI escapes: {stdout}");

    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("stdout should be parseable as valid JSON");
    assert_eq!(parsed["version"], "1.2.0");
    assert_eq!(parsed["major"], 1);
    assert_eq!(parsed["minor"], 2);
    assert_eq!(parsed["patch"], 0);
    assert_eq!(parsed["date"], "2026-03-01");
}

#[test]
fn changelog_show_cli_json_export() {
    let guard = FixtureGuard::new("changelog-show-json");
    let fixture = guard.fixture();
    let cutver_toml = r#"[version]
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[changelog]
path = "CHANGELOG.md"
"#;
    fixture.write("cutver.toml", cutver_toml);
    fixture.write("package.json", r#"{"version": "2.0.0"}"#);
    let changelog_content = r#"# Changelog

## [2.0.0] - 2026-04-01

### Features
- major overhaul

## [1.2.0] - 2026-03-01

### Features
- **cli**: add json support (#66)

## [1.1.0] - 2026-02-01

### Fixes
- bug fix
"#;
    fixture.write("CHANGELOG.md", changelog_content);

    let output = Command::new(env!("CARGO_BIN_EXE_cutver"))
        .current_dir(&fixture.dir)
        .env("CLICOLOR_FORCE", "1")
        .args(["changelog", "show", "v1.2.0", "--json"])
        .output()
        .expect("failed to execute cutver binary");

    assert!(
        output.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.status.code(), Some(0));

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("\x1b["), "stdout contains ANSI escapes: {stdout}");

    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("stdout should be parseable as valid JSON");
    assert_eq!(parsed["version"], "1.2.0");
    assert_eq!(parsed["previous_version"], "1.1.0");
    assert_eq!(parsed["major"], 1);
    assert_eq!(parsed["minor"], 2);
    assert_eq!(parsed["patch"], 0);
    assert_eq!(parsed["date"], "2026-03-01");
}

#[test]
fn changelog_latest_and_show_cli_with_template_enriches_commits_from_git() {
    assert!(
        git_available(),
        "git CLI is required for e2e tests but was not found in PATH"
    );
    let guard = FixtureGuard::new("changelog-cli-raw-commits");
    let fixture = guard.fixture();

    let cutver_toml = r#"[version]
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[changelog]
path = "CHANGELOG.md"

[git]
tag_prefix = "v"
"#;
    fixture.write("cutver.toml", cutver_toml);
    fixture.write("package.json", r#"{"version": "1.2.0"}"#);

    init_git_repo(fixture);
    initial_commit(fixture);
    run_git_ok(&fixture.dir, &["tag", "v1.1.0"]);

    run_git_ok(
        &fixture.dir,
        &["commit", "--allow-empty", "-m", "feat(cli): add raw commits (#42)"],
    );
    run_git_ok(&fixture.dir, &["tag", "v1.2.0"]);

    let changelog_content = r#"# Changelog

## [1.2.0] - 2026-03-01

### Features
- **cli**: add raw commits (#42)

## [1.1.0] - 2026-02-01

- initial release
"#;
    fixture.write("CHANGELOG.md", changelog_content);

    let template = "Release {{ version }}: {% for c in commits %}[{{ c.short_hash }}] {{ c.description }} by {{ c.author }}{% endfor %}";
    fixture.write("custom.j2", template);

    // Test changelog latest --template custom.j2
    let output_latest = Command::new(env!("CARGO_BIN_EXE_cutver"))
        .current_dir(&fixture.dir)
        .args(["changelog", "latest", "--template", "custom.j2"])
        .output()
        .expect("failed to execute cutver binary");

    assert!(
        output_latest.status.success(),
        "latest command failed: {}",
        String::from_utf8_lossy(&output_latest.stderr)
    );
    let stdout_latest = String::from_utf8_lossy(&output_latest.stdout);
    assert!(stdout_latest.contains("Release 1.2.0:"));
    assert!(stdout_latest.contains("add raw commits (#42) by Test User"));
    assert!(
        !stdout_latest.contains("[]"),
        "short_hash should be populated in latest output: {stdout_latest}"
    );

    // Test changelog show v1.2.0 --template custom.j2
    let output_show = Command::new(env!("CARGO_BIN_EXE_cutver"))
        .current_dir(&fixture.dir)
        .args(["changelog", "show", "v1.2.0", "--template", "custom.j2"])
        .output()
        .expect("failed to execute cutver binary");

    assert!(
        output_show.status.success(),
        "show command failed: {}",
        String::from_utf8_lossy(&output_show.stderr)
    );
    let stdout_show = String::from_utf8_lossy(&output_show.stdout);
    assert!(stdout_show.contains("Release 1.2.0:"));
    assert!(stdout_show.contains("add raw commits (#42) by Test User"));
    assert!(
        !stdout_show.contains("[]"),
        "short_hash should be populated in show output: {stdout_show}"
    );
}
