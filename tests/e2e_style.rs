#![cfg(unix)]

mod common;

use common::*;
use std::process::Command;

#[test]
fn test_style_no_color_suppresses_ansi_sequences() {
    assert!(
        git_available(),
        "git CLI is required for e2e tests but was not found in PATH"
    );
    let guard = FixtureGuard::new("style-no-color");
    let fixture = guard.fixture();
    let cutver_toml = r#"[version]
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"
"#;
    fixture.write("cutver.toml", cutver_toml);
    fixture.write("package.json", r#"{"version": "1.0.0"}"#);
    init_git_repo(fixture);
    initial_commit(fixture);

    // Run bump dry-run with NO_COLOR=1
    let output = Command::new(env!("CARGO_BIN_EXE_cutver"))
        .current_dir(&fixture.dir)
        .env("NO_COLOR", "1")
        .args(["bump", "patch", "--dry-run"])
        .output()
        .expect("failed to execute cutver binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("\x1b["),
        "stdout contains ANSI escape sequences under NO_COLOR=1: {stdout}"
    );
    assert!(stdout.contains("SIMULATION MODE"));
    assert!(stdout.contains("No files, commits, or git tags will be modified."));
    assert!(stdout.contains("Release Plan:"));

    // Run doctor with NO_COLOR=1
    let output = Command::new(env!("CARGO_BIN_EXE_cutver"))
        .current_dir(&fixture.dir)
        .env("NO_COLOR", "1")
        .args(["doctor"])
        .output()
        .expect("failed to execute cutver binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("\x1b["),
        "stdout contains ANSI escape sequences under NO_COLOR=1: {stdout}"
    );
    assert!(stdout.contains("✔ cutver.toml is valid"));
}

#[test]
fn test_style_clicolor_force_emits_ansi_sequences() {
    assert!(
        git_available(),
        "git CLI is required for e2e tests but was not found in PATH"
    );
    let guard = FixtureGuard::new("style-clicolor-force");
    let fixture = guard.fixture();
    let cutver_toml = r#"[version]
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"
"#;
    fixture.write("cutver.toml", cutver_toml);
    fixture.write("package.json", r#"{"version": "1.0.0"}"#);
    init_git_repo(fixture);
    initial_commit(fixture);

    // Run bump dry-run with CLICOLOR_FORCE=1 (piped execution normally has no TTY)
    let output = Command::new(env!("CARGO_BIN_EXE_cutver"))
        .current_dir(&fixture.dir)
        .env("CLICOLOR_FORCE", "1")
        .args(["bump", "patch", "--dry-run"])
        .output()
        .expect("failed to execute cutver binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\x1b["),
        "stdout expected ANSI escape sequences under CLICOLOR_FORCE=1: {stdout}"
    );
    // Verify yellow info icon "\x1b[33mℹ\x1b[0m" and bold simulation mode
    assert!(stdout.contains("\x1b[33mℹ\x1b[0m"));
    assert!(stdout.contains("\x1b[1mSIMULATION MODE\x1b[0m"));

    // Run doctor with CLICOLOR_FORCE=1
    let output = Command::new(env!("CARGO_BIN_EXE_cutver"))
        .current_dir(&fixture.dir)
        .env("CLICOLOR_FORCE", "1")
        .args(["doctor"])
        .output()
        .expect("failed to execute cutver binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\x1b["),
        "stdout expected ANSI escape sequences under CLICOLOR_FORCE=1: {stdout}"
    );
    // Verify green checkmark "\x1b[32m✔\x1b[0m"
    assert!(stdout.contains("\x1b[32m✔\x1b[0m"));
}

#[test]
fn test_style_error_output_with_clicolor_force_and_no_color() {
    let guard = FixtureGuard::new("style-error");
    let fixture = guard.fixture();

    // 1. Error with CLICOLOR_FORCE=1
    let output = Command::new(env!("CARGO_BIN_EXE_cutver"))
        .current_dir(&fixture.dir)
        .env("CLICOLOR_FORCE", "1")
        .args(["doctor"])
        .output()
        .expect("failed to execute cutver binary");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("\x1b[31mError:\x1b[0m") || stderr.contains("\x1b[31mError loading config:\x1b[0m"));

    // 2. Error with NO_COLOR=1
    let output = Command::new(env!("CARGO_BIN_EXE_cutver"))
        .current_dir(&fixture.dir)
        .env("NO_COLOR", "1")
        .args(["doctor"])
        .output()
        .expect("failed to execute cutver binary");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("\x1b["),
        "stderr contains ANSI escape sequences under NO_COLOR=1: {stderr}"
    );
    assert!(stderr.contains("Error: ") || stderr.contains("Error loading config: "));
}
