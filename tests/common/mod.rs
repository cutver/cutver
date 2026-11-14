#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A per-test working tree.
///
/// Pillar IV.1: each fixture owns a `tempfile::TempDir`, so two tests scheduled concurrently never
/// share a directory, cleanup is automatic, and nothing is named after the process id. `dir` stays a
/// plain `PathBuf` copy of the temp-dir path, because every helper takes absolute paths.
pub struct Fixture {
    pub dir: PathBuf,
    _tmp: tempfile::TempDir,
}

impl Fixture {
    pub fn new(name: &str) -> Self {
        let tmp = tempfile::Builder::new()
            .prefix(&format!("cutver-e2e-{name}-"))
            .tempdir()
            .expect("failed to create a fixture temp dir");
        let dir = tmp.path().to_path_buf();
        fs::create_dir_all(&dir).unwrap();
        Self { dir, _tmp: tmp }
    }

    pub fn write(&self, path: impl AsRef<Path>, content: &str) {
        let path = self.dir.join(path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    pub fn read(&self, path: impl AsRef<Path>) -> String {
        fs::read_to_string(self.dir.join(path)).unwrap()
    }
}

pub struct FixtureGuard {
    fixture: Fixture,
}

/// The test-facing handle for a fixture.
///
/// It no longer switches the process CWD: that switch was process-global, so a test that resolved a
/// relative path could observe another test's directory. Everything a test needs is reachable from
/// `fixture()` as an absolute path.
impl FixtureGuard {
    pub fn new(name: &str) -> Self {
        Self {
            fixture: Fixture::new(name),
        }
    }

    pub fn fixture(&self) -> &Fixture {
        &self.fixture
    }
}

pub fn git_available() -> bool {
    Command::new("git")
        .args(["--version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn run_git(repo: impl AsRef<Path>, args: &[&str]) -> Output {
    Command::new("git")
        .current_dir(repo)
        .args(args)
        .output()
        .expect("git command failed to run")
}

pub fn run_git_ok(repo: impl AsRef<Path>, args: &[&str]) {
    let out = run_git(repo, args);
    assert!(
        out.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&out.stderr)
    );
}

pub fn init_git_repo(fixture: &Fixture) {
    let dir = &fixture.dir;
    run_git_ok(dir, &["init"]);
    run_git_ok(dir, &["config", "user.email", "test@example.com"]);
    run_git_ok(dir, &["config", "user.name", "Test User"]);
    run_git_ok(dir, &["config", "commit.gpgsign", "false"]);
    run_git_ok(dir, &["config", "tag.gpgsign", "false"]);
}

pub fn initial_commit(fixture: &Fixture) {
    run_git_ok(&fixture.dir, &["add", "-A"]);
    run_git_ok(&fixture.dir, &["commit", "-m", "init", "-q"]);
}

pub fn commit_count(fixture: &Fixture) -> usize {
    let out = run_git(&fixture.dir, &["rev-list", "--count", "HEAD"]);
    String::from_utf8_lossy(&out.stdout).trim().parse().unwrap()
}

pub fn tag_exists(fixture: &Fixture, tag: &str) -> bool {
    let out = run_git(&fixture.dir, &["tag", "-l", tag]);
    !String::from_utf8_lossy(&out.stdout).trim().is_empty()
}

pub fn is_annotated_tag(fixture: &Fixture, tag: &str) -> bool {
    let out = run_git(
        &fixture.dir,
        &["for-each-ref", "--format=%(objecttype)", &format!("refs/tags/{tag}")],
    );
    String::from_utf8_lossy(&out.stdout).trim() == "tag"
}

pub fn head_commit_message(fixture: &Fixture) -> String {
    let out = run_git(&fixture.dir, &["log", "-1", "--pretty=%s"]);
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

pub fn head_commit_files(fixture: &Fixture) -> Vec<String> {
    let out = run_git(
        &fixture.dir,
        &["diff-tree", "--no-commit-id", "--name-only", "-r", "HEAD"],
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

pub const PACKAGE_JSON: &str = r#"{
  "name": "cutver-e2e",
  "version": "1.2.3"
}
"#;

pub const CARGO_TOML: &str = r#"[package]
name = "cutver-e2e"
version = "1.2.3"
edition = "2024"
"#;

pub const GRADLE_KTS: &str = r#"plugins {
    id("com.android.application")
}

android {
    defaultConfig {
        versionName "1.2.3"
        versionCode 42
        applicationId = "com.example.app"
    }
}
"#;

pub const CHANGELOG_MD: &str = r#"# Changelog

All notable changes to this project will be documented in this file.
"#;

pub fn base_cutver_toml(preflight: &str) -> String {
    format!(
        r#"[version]
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"

[[manifest]]
path = "android/build.gradle.kts"
kind = "gradle"
version_name_field = "versionName"
version_code_field = "versionCode"

{preflight}

[changelog]
path = "CHANGELOG.md"
entry_template = "Maintenance and updates."

[git]
tag_prefix = "v"
require_clean_tree = true
"#
    )
}

pub fn write_fixture(guard: &FixtureGuard, preflight: &str) {
    let fixture = guard.fixture();
    fixture.write("package.json", PACKAGE_JSON);
    fixture.write("Cargo.toml", CARGO_TOML);
    fixture.write("android/build.gradle.kts", GRADLE_KTS);
    fixture.write("CHANGELOG.md", CHANGELOG_MD);
    fixture.write("cutver.toml", &base_cutver_toml(preflight));
    init_git_repo(fixture);
    initial_commit(fixture);
}

/// Load the fixture's `cutver.toml` by absolute path.
///
/// The process CWD is never a fixture, so a relative path here would resolve against whatever
/// directory the test binary inherited.
pub fn load_config(fixture: &Fixture) -> cutver::config::Config {
    cutver::config::load(fixture.dir.join("cutver.toml"))
        .unwrap_or_else(|e| panic!("fixture cutver.toml did not load: {e}"))
}

pub fn assert_versions_at_123(fixture: &Fixture) {
    assert!(fixture.read("package.json").contains("\"version\": \"1.2.3\""));
    assert!(fixture.read("Cargo.toml").contains("version = \"1.2.3\""));
    let gradle = fixture.read("android/build.gradle.kts");
    assert!(gradle.contains("versionName \"1.2.3\"") && gradle.contains("versionCode 42"));
}

pub fn assert_bumped(fixture: &Fixture) {
    assert!(fixture.read("package.json").contains("\"version\": \"1.3.0\""));
    assert!(fixture.read("Cargo.toml").contains("version = \"1.3.0\""));
    let gradle = fixture.read("android/build.gradle.kts");
    assert!(gradle.contains("versionName \"1.3.0\"") && gradle.contains("versionCode 43"));
    let changelog = fixture.read("CHANGELOG.md");
    assert_release_heading(&changelog, "v1.3.0");
}

/// Assert the changelog carries a release heading for `version` carrying an ISO date.
///
/// The expected date is deliberately not computed from a second `SystemTime::now()` read: two reads
/// can straddle midnight and fail a render that was correct. The date value itself is pinned by
/// `tests/changelog_render_clock.rs`, which injects it into the render core.
pub fn assert_release_heading(changelog: &str, version: &str) {
    release_heading_date(changelog, version);
}

/// The date the render wrote into the release heading for `version`.
///
/// Read back from the rendered bytes instead of recomputing `now()`: a second clock read can
/// straddle midnight and disagree with the render that already happened.
pub fn release_heading_date<'a>(changelog: &'a str, version: &str) -> &'a str {
    let prefix = format!("## [{version}] - ");
    let line = changelog
        .lines()
        .find(|line| line.starts_with(&prefix))
        .unwrap_or_else(|| panic!("no `{prefix}<date>` heading in the rendered changelog:\n{changelog}"));
    let date = line[prefix.len()..].trim();
    assert!(
        is_iso_date(date),
        "release heading date `{date}` is not YYYY-MM-DD:\n{changelog}"
    );
    date
}

pub fn is_iso_date(date: &str) -> bool {
    let bytes = date.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit())
}
