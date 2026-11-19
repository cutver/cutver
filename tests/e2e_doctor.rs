#![cfg(unix)]

mod common;

use common::*;

#[test]
fn doctor_with_check_changelog_cli_succeeds_when_consistent() {
    assert!(
        git_available(),
        "git CLI is required for e2e tests but was not found in PATH"
    );
    let guard = FixtureGuard::new("doctor-check-changelog-consistent");
    let fixture = guard.fixture();
    let cutver_toml = r#"[version]
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"
"#;
    fixture.write("cutver.toml", cutver_toml);
    fixture.write("package.json", r#"{"version": "1.1.0"}"#);
    init_git_repo(fixture);
    initial_commit(fixture);
    run_git_ok(&fixture.dir, &["tag", "v1.0.0"]);
    run_git_ok(&fixture.dir, &["tag", "v1.1.0"]);
    fixture.write(
        "CHANGELOG.md",
        "## [1.1.0] - 2026-02-01\n- v1.1.0\n\n## [1.0.0] - 2026-01-01\n- v1.0.0\n",
    );

    let output = cutver_command(&fixture.dir)
        .args(["doctor", "--check-changelog"])
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
        stdout.contains("cutver.toml is valid"),
        "stdout does not contain 'cutver.toml is valid': {stdout}"
    );
    assert!(
        stdout.contains("Configuration   valid (cutver.toml)"),
        "stdout does not contain 'Configuration   valid (cutver.toml)': {stdout}"
    );
    assert!(
        stdout.contains("Manifests       1 tracked (package.json @ 1.1.0)"),
        "stdout does not contain relative manifest path: {stdout}"
    );
    assert!(
        stdout.contains("Changelog       consistent with Git tags"),
        "stdout does not contain 'Changelog       consistent with Git tags': {stdout}"
    );
}

#[test]
fn doctor_with_check_changelog_cli_succeeds_with_floating_major_tags() {
    assert!(
        git_available(),
        "git CLI is required for e2e tests but was not found in PATH"
    );
    let guard = FixtureGuard::new("doctor-check-changelog-floating-tags");
    let fixture = guard.fixture();
    let cutver_toml = r#"[version]
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[git]
floating_major_tag = true
"#;
    fixture.write("cutver.toml", cutver_toml);
    fixture.write("package.json", r#"{"version": "1.1.0"}"#);
    init_git_repo(fixture);
    initial_commit(fixture);
    run_git_ok(&fixture.dir, &["tag", "v1.0.0"]);
    run_git_ok(&fixture.dir, &["tag", "v1.1.0"]);
    run_git_ok(&fixture.dir, &["tag", "v1"]);
    fixture.write(
        "CHANGELOG.md",
        "## [1.1.0] - 2026-02-01\n- v1.1.0\n\n## [1.0.0] - 2026-01-01\n- v1.0.0\n",
    );

    let output = cutver_command(&fixture.dir)
        .args(["doctor", "--check-changelog"])
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
        stdout.contains("cutver.toml is valid"),
        "stdout does not contain 'cutver.toml is valid': {stdout}"
    );
    assert!(
        stdout.contains("Configuration   valid (cutver.toml)"),
        "stdout does not contain 'Configuration   valid (cutver.toml)': {stdout}"
    );
    assert!(
        stdout.contains("Changelog       consistent with Git tags"),
        "stdout does not contain 'Changelog       consistent with Git tags': {stdout}"
    );
}

#[test]
fn doctor_with_check_changelog_cli_fails_with_drift() {
    assert!(
        git_available(),
        "git CLI is required for e2e tests but was not found in PATH"
    );
    let guard = FixtureGuard::new("doctor-check-changelog-drift");
    let fixture = guard.fixture();
    let cutver_toml = r#"[version]
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"
"#;
    fixture.write("cutver.toml", cutver_toml);
    fixture.write("package.json", r#"{"version": "1.1.0"}"#);
    init_git_repo(fixture);
    initial_commit(fixture);
    run_git_ok(&fixture.dir, &["tag", "v1.0.0"]);
    run_git_ok(&fixture.dir, &["tag", "v1.2.0"]);
    fixture.write(
        "CHANGELOG.md",
        "## [1.1.0] - 2026-02-01\n- v1.1.0\n\n## [1.0.0] - 2026-01-01\n- v1.0.0\n",
    );

    let output = cutver_command(&fixture.dir)
        .args(["doctor", "--check-changelog"])
        .output()
        .expect("failed to execute cutver binary");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Changelog drift detected:"),
        "stderr does not contain 'Changelog drift detected:': {stderr}"
    );
    assert!(
        stderr.contains("Missing in changelog"),
        "stderr does not contain 'Missing in changelog': {stderr}"
    );
    assert!(stderr.contains("v1.2.0"), "stderr does not contain 'v1.2.0': {stderr}");
    assert!(
        stderr.contains("Orphan changelog sections"),
        "stderr does not contain 'Orphan changelog sections': {stderr}"
    );
    assert!(stderr.contains("1.1.0"), "stderr does not contain '1.1.0': {stderr}");
}

/// Two manifests that disagree: `doctor --json` must hand the drift to the consumer as data and
/// still exit 2, without printing the rendered block the human path uses.
#[test]
fn doctor_json_reports_drift_as_data_and_keeps_the_exit_code() {
    let guard = FixtureGuard::new("doctor-json-drift");
    let fixture = guard.fixture();
    let cutver_toml = r#"[version]
current_source = "a.json"

[[manifest]]
path = "a.json"
kind = "json"
field = "version"

[[manifest]]
path = "b.json"
kind = "json"
field = "version"
"#;
    fixture.write("cutver.toml", cutver_toml);
    fixture.write("a.json", r#"{"version": "1.1.0"}"#);
    fixture.write("b.json", r#"{"version": "1.0.0"}"#);

    let output = cutver_command(&fixture.dir)
        .args(["doctor", "--json"])
        .output()
        .expect("failed to execute cutver binary");

    assert_eq!(
        output.status.code(),
        Some(2),
        "drift keeps exit code 2; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|err| {
        panic!("stdout must be one JSON object, got {stdout:?}: {err}");
    });
    assert_eq!(parsed["schemaVersion"], 1, "{stdout}");
    assert_eq!(parsed["ok"], false, "{stdout}");
    assert_eq!(parsed["checks"][0]["name"], "versions", "{stdout}");
    assert_eq!(parsed["checks"][0]["status"], "drift", "{stdout}");
    assert_eq!(parsed["checks"][1]["name"], "changelog", "{stdout}");
    assert_eq!(
        parsed["checks"][1]["status"], "skipped",
        "a check that did not run is not an empty result: {stdout}"
    );
    assert_eq!(parsed["version_drift"][0]["path"], "b.json", "{stdout}");
    assert_eq!(parsed["version_drift"][0]["expected"], "1.1.0", "{stdout}");
    assert_eq!(parsed["version_drift"][0]["actual"], "1.0.0", "{stdout}");
    assert!(parsed["changelog_drift"].is_null(), "{stdout}");

    let version_at = stdout.find("\"schemaVersion\"").expect("the schema version is present");
    let ok_at = stdout.find("\"ok\"").expect("the outcome is present");
    assert!(version_at < ok_at, "the schema version is the first key: {stdout}");
    assert!(
        !stderr.contains("Version drift detected"),
        "the machine path does not render the human block: {stderr}"
    );
}

/// No drift: the same object, `ok: true`, every declared check ran and says so.
#[test]
fn doctor_json_reports_a_clean_run_as_ok() {
    assert!(
        git_available(),
        "git CLI is required for e2e tests but was not found in PATH"
    );
    let guard = FixtureGuard::new("doctor-json-clean");
    let fixture = guard.fixture();
    let cutver_toml = r#"[version]
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"
"#;
    fixture.write("cutver.toml", cutver_toml);
    fixture.write("package.json", r#"{"version": "1.1.0"}"#);
    init_git_repo(fixture);
    initial_commit(fixture);
    run_git_ok(&fixture.dir, &["tag", "v1.1.0"]);
    fixture.write("CHANGELOG.md", "## [1.1.0] - 2026-02-01\n- v1.1.0\n");

    let output = cutver_command(&fixture.dir)
        .args(["doctor", "--json", "--check-changelog"])
        .output()
        .expect("failed to execute cutver binary");

    assert_eq!(
        output.status.code(),
        Some(0),
        "a clean run exits 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|err| {
        panic!("stdout must be one JSON object, got {stdout:?}: {err}");
    });
    assert_eq!(parsed["ok"], true, "{stdout}");
    assert_eq!(parsed["checks"][0]["status"], "ok", "{stdout}");
    assert_eq!(parsed["checks"][1]["name"], "changelog", "{stdout}");
    assert_eq!(parsed["checks"][1]["status"], "ok", "{stdout}");
    assert_eq!(parsed["version_drift"].as_array().map(Vec::len), Some(0), "{stdout}");
    assert!(
        !stdout.contains("cutver.toml is valid"),
        "the machine path does not render the dashboard: {stdout}"
    );
}
