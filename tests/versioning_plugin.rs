#![cfg(unix)]

mod common;

use common::*;
use cutver::bump::run as bump_run;
use cutver::cli::BumpLevel;
use cutver::config;
use std::os::unix::fs::PermissionsExt;

const MOCK_VERSIONING_SCRIPT: &str = r####"#!/bin/sh
INPUT=$(cat)

case "$INPUT" in
  *"INVALID_SEMVER"*)
    cat << 'JSON'
{"next_version": "not-semver", "rationale": "invalid semver output"}
JSON
    ;;
  *"FAIL_PLUGIN"*)
    echo "Simulated versioning plugin failure" >&2
    exit 1
    ;;
  *)
    cat << 'JSON'
{"next_version": "2.5.0", "rationale": "computed by custom plugin"}
JSON
    ;;
esac
"####;

#[test]
fn test_versioning_plugin_explicit_happy_path() {
    assert!(git_available());
    let guard = FixtureGuard::new("versioning-plugin-explicit");
    let fixture = guard.fixture();

    let plugin_script = fixture.dir.join("ver_plugin.sh");
    fixture.write("ver_plugin.sh", MOCK_VERSIONING_SCRIPT);
    let mut perms = std::fs::metadata(&plugin_script).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&plugin_script, perms).unwrap();

    let script_path = plugin_script.to_string_lossy().replace('\\', "/");
    let toml = format!(
        r#"[version]
strategy = "plugin"
plugin = "my-versioning-plugin"
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[git]
require_clean_tree = true

[plugins.my-versioning-plugin]
runtime = "process"
command = "{script_path}"
capabilities = ["versioning.v1"]
"#
    );
    fixture.write("cutver.toml", &toml);
    fixture.write("package.json", r#"{"version": "1.0.0"}"#);

    init_git_repo(fixture);
    initial_commit(fixture);

    let cfg = config::load("cutver.toml").unwrap();
    let res = bump_run(&cfg, BumpLevel::Auto, false, &[]);

    assert!(
        res.is_ok(),
        "bump should succeed with explicit versioning plugin: {res:?}"
    );
    let pkg_content = fixture.read("package.json");
    assert!(
        pkg_content.contains(r#""version": "2.5.0""#),
        "package.json should contain plugin-computed version 2.5.0: {pkg_content}"
    );
}

#[test]
fn test_versioning_plugin_automatic_resolution() {
    assert!(git_available());
    let guard = FixtureGuard::new("versioning-plugin-auto");
    let fixture = guard.fixture();

    let plugin_script = fixture.dir.join("ver_plugin.sh");
    fixture.write("ver_plugin.sh", MOCK_VERSIONING_SCRIPT);
    let mut perms = std::fs::metadata(&plugin_script).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&plugin_script, perms).unwrap();

    let script_path = plugin_script.to_string_lossy().replace('\\', "/");
    let toml = format!(
        r#"[version]
strategy = "plugin"
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[git]
require_clean_tree = true

[plugins.auto-versioning-plugin]
runtime = "process"
command = "{script_path}"
capabilities = ["versioning.v1"]
"#
    );
    fixture.write("cutver.toml", &toml);
    fixture.write("package.json", r#"{"version": "1.0.0"}"#);

    init_git_repo(fixture);
    initial_commit(fixture);

    let cfg = config::load("cutver.toml").unwrap();
    let res = bump_run(&cfg, BumpLevel::Auto, false, &[]);

    assert!(
        res.is_ok(),
        "bump should succeed with auto-resolved versioning plugin: {res:?}"
    );
    let pkg_content = fixture.read("package.json");
    assert!(
        pkg_content.contains(r#""version": "2.5.0""#),
        "package.json should contain plugin-computed version 2.5.0: {pkg_content}"
    );
}

#[test]
fn test_versioning_plugin_invalid_semver_fails_closed() {
    assert!(git_available());
    let guard = FixtureGuard::new("versioning-plugin-invalid-semver");
    let fixture = guard.fixture();

    let failing_script = fixture.dir.join("invalid_plugin.sh");
    fixture.write(
        "invalid_plugin.sh",
        r#"#!/bin/sh
cat << 'JSON'
{"next_version": "not-semver", "rationale": "malformed"}
JSON
"#,
    );
    let mut perms = std::fs::metadata(&failing_script).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&failing_script, perms).unwrap();

    let script_path = failing_script.to_string_lossy().replace('\\', "/");
    let toml = format!(
        r#"[version]
strategy = "plugin"
plugin = "invalid-plugin"
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[git]
require_clean_tree = true

[plugins.invalid-plugin]
runtime = "process"
command = "{script_path}"
capabilities = ["versioning.v1"]
"#
    );
    fixture.write("cutver.toml", &toml);
    let initial_pkg = r#"{"version": "1.0.0"}"#;
    fixture.write("package.json", initial_pkg);

    init_git_repo(fixture);
    initial_commit(fixture);

    let cfg = config::load("cutver.toml").unwrap();
    let res = bump_run(&cfg, BumpLevel::Auto, false, &[]);

    assert!(
        res.is_err(),
        "bump should fail when versioning plugin returns invalid SemVer"
    );
    let err_str = res.unwrap_err().to_string();
    assert!(
        err_str.contains("not-semver"),
        "error message should indicate invalid semver: {err_str}"
    );

    // Verify files remain untouched
    assert_eq!(fixture.read("package.json"), initial_pkg);
}

#[test]
fn test_versioning_plugin_process_exit_failure_fails_closed() {
    assert!(git_available());
    let guard = FixtureGuard::new("versioning-plugin-exit-fail");
    let fixture = guard.fixture();

    let failing_script = fixture.dir.join("fail_plugin.sh");
    fixture.write(
        "fail_plugin.sh",
        r#"#!/bin/sh
echo "Intentional failure in versioning plugin" >&2
exit 1
"#,
    );
    let mut perms = std::fs::metadata(&failing_script).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&failing_script, perms).unwrap();

    let script_path = failing_script.to_string_lossy().replace('\\', "/");
    let toml = format!(
        r#"[version]
strategy = "plugin"
plugin = "failing-plugin"
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[git]
require_clean_tree = true

[plugins.failing-plugin]
runtime = "process"
command = "{script_path}"
capabilities = ["versioning.v1"]
"#
    );
    fixture.write("cutver.toml", &toml);
    let initial_pkg = r#"{"version": "1.0.0"}"#;
    fixture.write("package.json", initial_pkg);

    init_git_repo(fixture);
    initial_commit(fixture);

    let cfg = config::load("cutver.toml").unwrap();
    let res = bump_run(&cfg, BumpLevel::Auto, false, &[]);

    assert!(res.is_err(), "bump should fail when versioning plugin exits non-zero");

    // Verify files remain untouched
    assert_eq!(fixture.read("package.json"), initial_pkg);
}
