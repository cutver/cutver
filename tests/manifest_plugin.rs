#![cfg(unix)]

mod common;

use common::*;
use cutver::bump::run as bump_run;
use cutver::config;
use cutver::semver_bump::Bump;
use std::os::unix::fs::PermissionsExt;

const MOCK_PLUGIN_SCRIPT: &str = r#"#!/bin/sh
INPUT=$(cat)

case "$INPUT" in
  *"99.99.99"*)
    echo "Simulated manifest write failure" >&2
    exit 1
    ;;
  *"current_version"*)
    cat << 'JSON'
{"content": "apiVersion: v2\nname: mychart\nversion: 1.3.0\n"}
JSON
    ;;
  *)
    cat << 'JSON'
{"version": "1.2.3"}
JSON
    ;;
esac
"#;

#[test]
fn test_plugin_manifest_read_write_e2e_happy_path() {
    assert!(git_available());
    let guard = FixtureGuard::new("plugin-manifest-happy");
    let fixture = guard.fixture();

    let plugin_script = fixture.dir.join("helm_plugin.sh");
    fixture.write("helm_plugin.sh", MOCK_PLUGIN_SCRIPT);
    let mut perms = std::fs::metadata(&plugin_script).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&plugin_script, perms).unwrap();

    let script_path = plugin_script.to_string_lossy().replace('\\', "/");
    let toml = format!(
        r#"[version]
current_source = "Chart.yaml"

[[manifest]]
path = "Chart.yaml"
kind = "plugin"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[changelog]
path = "CHANGELOG.md"

[git]
require_clean_tree = true

[plugins.helm-adapter]
runtime = "process"
command = "{script_path}"
capabilities = ["manifest.v1"]
manifest_match = ["Chart.yaml", "charts/**/Chart.yaml"]
"#
    );

    let chart_yaml = "apiVersion: v2\nname: mychart\nversion: 1.2.3\n";
    fixture.write("Chart.yaml", chart_yaml);
    fixture.write("package.json", PACKAGE_JSON);
    fixture.write("CHANGELOG.md", CHANGELOG_MD);
    fixture.write("cutver.toml", &toml);
    init_git_repo(fixture);
    initial_commit(fixture);

    let cfg = config::load("cutver.toml").unwrap();

    // Verify doctor discovers and reads the plugin manifest version
    let doctor_drifts = cutver::bump::doctor(&cfg).unwrap();
    assert!(doctor_drifts.is_empty(), "doctor should report no drift");

    // Execute bump minor
    let summary = bump_run(&cfg, Bump::Minor, false, &[]).unwrap();
    assert_eq!(summary.next.to_string(), "1.3.0");
    assert_eq!(summary.tag, "v1.3.0");

    // Verify both plugin manifest and builtin json manifest mutated
    assert!(fixture.read("Chart.yaml").contains("version: 1.3.0"));
    assert!(fixture.read("package.json").contains("\"version\": \"1.3.0\""));
    assert!(fixture.read("CHANGELOG.md").contains("1.3.0"));

    assert_eq!(commit_count(fixture), 2);
    assert!(tag_exists(fixture, "v1.3.0"));
}

#[test]
fn test_plugin_manifest_write_failure_rolls_back_atomically() {
    assert!(git_available());
    let guard = FixtureGuard::new("plugin-manifest-rollback");
    let fixture = guard.fixture();

    let plugin_script = fixture.dir.join("helm_plugin.sh");
    fixture.write("helm_plugin.sh", MOCK_PLUGIN_SCRIPT);
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

[[manifest]]
path = "Chart.yaml"
kind = "plugin"
plugin = "helm-adapter"

[changelog]
path = "CHANGELOG.md"

[git]
require_clean_tree = true

[plugins.helm-adapter]
runtime = "process"
command = "{script_path}"
capabilities = ["manifest.v1"]
manifest_match = ["Chart.yaml"]
"#
    );

    let chart_yaml = "apiVersion: v2\nname: mychart\nversion: 1.2.3\n";
    fixture.write("Chart.yaml", chart_yaml);
    fixture.write("package.json", PACKAGE_JSON);
    fixture.write("CHANGELOG.md", CHANGELOG_MD);
    fixture.write("cutver.toml", &toml);
    init_git_repo(fixture);
    initial_commit(fixture);

    let _cfg = config::load("cutver.toml").unwrap();

    // Failing script on write error:
    // If the plugin exits non-zero during compute/write, the pipeline aborts before phase 2
    let failing_script = fixture.dir.join("fail_plugin.sh");
    fixture.write("fail_plugin.sh", "#!/bin/sh\nINPUT=$(cat)\ncase \"$INPUT\" in *\"current_version\"*) exit 1;; *) echo '{\"version\": \"1.2.3\"}';; esac\n");
    let mut fperms = std::fs::metadata(&failing_script).unwrap().permissions();
    fperms.set_mode(0o755);
    std::fs::set_permissions(&failing_script, fperms).unwrap();

    let fail_script_path = failing_script.to_string_lossy().replace('\\', "/");
    let failing_toml = toml.replace(&script_path, &fail_script_path);
    fixture.write("cutver.toml", &failing_toml);

    let cfg_fail = config::load("cutver.toml").unwrap();
    let res = bump_run(&cfg_fail, Bump::Minor, false, &[]);
    assert!(res.is_err(), "bump should fail when plugin manifest write fails");

    // Zero disk mutation occurred
    assert_eq!(fixture.read("Chart.yaml"), chart_yaml);
    assert_eq!(fixture.read("package.json"), PACKAGE_JSON);
    assert_eq!(fixture.read("CHANGELOG.md"), CHANGELOG_MD);

    assert_eq!(commit_count(fixture), 1);
    assert!(!tag_exists(fixture, "v1.3.0"));
}
