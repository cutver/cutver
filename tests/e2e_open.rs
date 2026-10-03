use common::*;
use std::process::Command;

mod common;

#[test]
fn test_open_print_url_repo_default() {
    let guard = FixtureGuard::new("open-print-url-repo");
    let fixture = guard.fixture();
    init_git_repo(fixture);
    fixture.write("README.md", "# Test");
    initial_commit(fixture);

    // Set origin remote
    let res = Command::new("git")
        .current_dir(&fixture.dir)
        .args(["remote", "add", "origin", "https://github.com/Row0902/cutver.git"])
        .status()
        .expect("failed to add git remote");
    assert!(res.success());

    let output = Command::new(env!("CARGO_BIN_EXE_cutver"))
        .current_dir(&fixture.dir)
        .args(["open", "--print-url"])
        .output()
        .expect("failed to execute cutver open");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "https://github.com/Row0902/cutver");
}

#[test]
fn test_open_print_url_version_release() {
    let guard = FixtureGuard::new("open-print-url-version");
    let fixture = guard.fixture();
    init_git_repo(fixture);
    fixture.write("README.md", "# Test");
    initial_commit(fixture);

    let res = Command::new("git")
        .current_dir(&fixture.dir)
        .args(["remote", "add", "origin", "https://github.com/Row0902/cutver.git"])
        .status()
        .expect("failed to add git remote");
    assert!(res.success());

    // Test with bare version "0.9.1"
    let output = Command::new(env!("CARGO_BIN_EXE_cutver"))
        .current_dir(&fixture.dir)
        .args(["open", "0.9.1", "--print-url"])
        .output()
        .expect("failed to execute cutver open");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "https://github.com/Row0902/cutver/releases/tag/v0.9.1");

    // Test with prefixed version "v0.9.1"
    let output2 = Command::new(env!("CARGO_BIN_EXE_cutver"))
        .current_dir(&fixture.dir)
        .args(["open", "v0.9.1", "--print-url"])
        .output()
        .expect("failed to execute cutver open");

    assert!(output2.status.success());
    let stdout2 = String::from_utf8_lossy(&output2.stdout);
    assert_eq!(stdout2.trim(), "https://github.com/Row0902/cutver/releases/tag/v0.9.1");
}

#[test]
fn test_open_print_url_compare_range() {
    let guard = FixtureGuard::new("open-print-url-compare");
    let fixture = guard.fixture();
    init_git_repo(fixture);
    fixture.write("README.md", "# Test");
    initial_commit(fixture);

    let res = Command::new("git")
        .current_dir(&fixture.dir)
        .args(["remote", "add", "origin", "https://github.com/Row0902/cutver.git"])
        .status()
        .expect("failed to add git remote");
    assert!(res.success());

    // Test "compare" keyword
    let output = Command::new(env!("CARGO_BIN_EXE_cutver"))
        .current_dir(&fixture.dir)
        .args(["open", "compare", "--print-url"])
        .output()
        .expect("failed to execute cutver open compare");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "https://github.com/Row0902/cutver/compare/HEAD");

    // Test range "v1.0.0...v1.1.0"
    let output_range = Command::new(env!("CARGO_BIN_EXE_cutver"))
        .current_dir(&fixture.dir)
        .args(["open", "v1.0.0...v1.1.0", "--print-url"])
        .output()
        .expect("failed to execute cutver open range");

    assert!(output_range.status.success());
    let stdout_range = String::from_utf8_lossy(&output_range.stdout);
    assert_eq!(
        stdout_range.trim(),
        "https://github.com/Row0902/cutver/compare/v1.0.0...v1.1.0"
    );
}

#[test]
fn test_changelog_open_print_url() {
    let guard = FixtureGuard::new("changelog-open-print-url");
    let fixture = guard.fixture();
    init_git_repo(fixture);
    fixture.write("README.md", "# Test");
    initial_commit(fixture);

    let res = Command::new("git")
        .current_dir(&fixture.dir)
        .args(["remote", "add", "origin", "https://gitlab.com/group/cutver.git"])
        .status()
        .expect("failed to add git remote");
    assert!(res.success());

    let output = Command::new(env!("CARGO_BIN_EXE_cutver"))
        .current_dir(&fixture.dir)
        .args(["changelog", "open", "1.2.0", "--print-url"])
        .output()
        .expect("failed to execute cutver changelog open");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "https://gitlab.com/group/cutver/-/releases/v1.2.0");
}

#[test]
fn test_open_missing_remote_fails_cleanly() {
    let guard = FixtureGuard::new("open-missing-remote");
    let fixture = guard.fixture();
    init_git_repo(fixture);
    fixture.write("README.md", "# Test");
    initial_commit(fixture);

    let output = Command::new(env!("CARGO_BIN_EXE_cutver"))
        .current_dir(&fixture.dir)
        .args(["open", "--print-url"])
        .output()
        .expect("failed to execute cutver open");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("failed to determine repository remote URL"));
}
