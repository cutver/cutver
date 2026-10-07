#![cfg(unix)]

mod common;

use common::*;
use cutver::bump::run as bump_run;
use cutver::config;
use cutver::semver_bump::Bump;
use std::os::unix::fs::PermissionsExt;

const MOCK_CHANGELOG_SCRIPT: &str = r####"#!/bin/sh
INPUT=$(cat)

case "$INPUT" in
  *"FAIL_PLUGIN"*)
    echo "Simulated changelog plugin failure" >&2
    exit 1
    ;;
  *)
    cat << 'JSON'
{"body": "### Formatted by Custom Plugin\n- Custom notes line"}
JSON
    ;;
esac
"####;

#[test]
fn test_changelog_plugin_explicit_happy_path() {
    assert!(git_available());
    let guard = FixtureGuard::new("changelog-plugin-explicit");
    let fixture = guard.fixture();

    let plugin_script = fixture.dir.join("cl_plugin.sh");
    fixture.write("cl_plugin.sh", MOCK_CHANGELOG_SCRIPT);
    let mut perms = std::fs::metadata(&plugin_script).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&plugin_script, perms).unwrap();

    let script_path = plugin_script.to_string_lossy().replace('\\', "/");
    let toml = format!(
        r#"[version]
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[changelog]
path = "CHANGELOG.md"
format = "plugin"
plugin = "my-changelog-plugin"

[git]
require_clean_tree = true

[plugins.my-changelog-plugin]
runtime = "process"
command = "{script_path}"
capabilities = ["changelog.v1"]
"#
    );
    fixture.write("cutver.toml", &toml);
    fixture.write("package.json", r#"{"version": "1.0.0"}"#);
    fixture.write("CHANGELOG.md", "# Changelog\n");

    init_git_repo(fixture);
    initial_commit(fixture);

    // Add commit to trigger bump
    fixture.write("file.txt", "content");
    run_git_ok(&fixture.dir, &["add", "."]);
    run_git_ok(&fixture.dir, &["commit", "-m", "feat: cool new feature"]);

    let cfg = config::load("cutver.toml").unwrap();
    let res = bump_run(&cfg, Bump::Minor, false, &[]);

    assert!(res.is_ok(), "bump should succeed with changelog plugin: {res:?}");
    let changelog_content = fixture.read("CHANGELOG.md");
    assert!(
        changelog_content.contains("### Formatted by Custom Plugin\n- Custom notes line"),
        "changelog should contain plugin-rendered body: {changelog_content}"
    );
}

#[test]
fn test_changelog_plugin_automatic_resolution() {
    assert!(git_available());
    let guard = FixtureGuard::new("changelog-plugin-auto");
    let fixture = guard.fixture();

    let plugin_script = fixture.dir.join("cl_plugin.sh");
    fixture.write("cl_plugin.sh", MOCK_CHANGELOG_SCRIPT);
    let mut perms = std::fs::metadata(&plugin_script).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&plugin_script, perms).unwrap();

    let script_path = plugin_script.to_string_lossy().replace('\\', "/");
    let toml = format!(
        r#"[version]
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[changelog]
path = "CHANGELOG.md"
format = "plugin"

[git]
require_clean_tree = true

[plugins.auto-changelog-plugin]
runtime = "process"
command = "{script_path}"
capabilities = ["changelog.v1"]
"#
    );
    fixture.write("cutver.toml", &toml);
    fixture.write("package.json", r#"{"version": "1.0.0"}"#);
    fixture.write("CHANGELOG.md", "# Changelog\n");

    init_git_repo(fixture);
    initial_commit(fixture);

    // Add commit to trigger bump
    fixture.write("file.txt", "content");
    run_git_ok(&fixture.dir, &["add", "."]);
    run_git_ok(&fixture.dir, &["commit", "-m", "feat: cool new feature"]);

    let cfg = config::load("cutver.toml").unwrap();
    let res = bump_run(&cfg, Bump::Minor, false, &[]);

    assert!(
        res.is_ok(),
        "bump should succeed with auto-resolved changelog plugin: {res:?}"
    );
    let changelog_content = fixture.read("CHANGELOG.md");
    assert!(
        changelog_content.contains("### Formatted by Custom Plugin\n- Custom notes line"),
        "changelog should contain plugin-rendered body: {changelog_content}"
    );
}

#[test]
fn test_changelog_plugin_failure_aborts_before_mutation() {
    assert!(git_available());
    let guard = FixtureGuard::new("changelog-plugin-fail");
    let fixture = guard.fixture();

    let failing_script = fixture.dir.join("fail_plugin.sh");
    fixture.write(
        "fail_plugin.sh",
        r#"#!/bin/sh
INPUT=$(cat)
echo "Intentional failure in changelog plugin" >&2
exit 1
"#,
    );
    let mut perms = std::fs::metadata(&failing_script).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&failing_script, perms).unwrap();

    let script_path = failing_script.to_string_lossy().replace('\\', "/");
    let toml = format!(
        r#"[version]
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[changelog]
path = "CHANGELOG.md"
format = "plugin"
plugin = "failing-plugin"

[git]
require_clean_tree = true

[plugins.failing-plugin]
runtime = "process"
command = "{script_path}"
capabilities = ["changelog.v1"]
"#
    );
    fixture.write("cutver.toml", &toml);
    let initial_pkg = r#"{"version": "1.0.0"}"#;
    let initial_cl = "# Changelog\n";
    fixture.write("package.json", initial_pkg);
    fixture.write("CHANGELOG.md", initial_cl);

    init_git_repo(fixture);
    initial_commit(fixture);

    // Add commit to trigger bump
    fixture.write("file.txt", "content");
    run_git_ok(&fixture.dir, &["add", "."]);
    run_git_ok(&fixture.dir, &["commit", "-m", "feat: cool new feature"]);

    let cfg = config::load("cutver.toml").unwrap();
    let res = bump_run(&cfg, Bump::Minor, false, &[]);

    assert!(res.is_err(), "bump should fail when changelog plugin fails");

    // Verify files remain untouched
    assert_eq!(fixture.read("package.json"), initial_pkg);
    assert_eq!(fixture.read("CHANGELOG.md"), initial_cl);
}
