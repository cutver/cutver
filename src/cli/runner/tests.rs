use super::*;
use crate::cli::args::{ChangelogCommands, Cli, Commands};
use crate::config;
use clap::Parser;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

static TMP_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn tmp_id(prefix: &str) -> String {
    let n = TMP_SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    format!("{}-{}-{}", prefix, std::process::id(), n)
}

fn temp_dir(prefix: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(tmp_id(prefix));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(dir: &Path, name: &str, text: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, text).unwrap();
    path
}

fn git_commit(dir: &Path, msg: &str) {
    let status = std::process::Command::new("git")
        .current_dir(dir)
        .args(["add", "."])
        .status()
        .unwrap();
    assert!(status.success(), "git add failed");
    let status = std::process::Command::new("git")
        .current_dir(dir)
        .args(["commit", "-m", msg, "-q"])
        .status()
        .unwrap();
    assert!(status.success(), "git commit failed");
}

fn git_tag(dir: &Path, tag: &str) {
    let status = std::process::Command::new("git")
        .current_dir(dir)
        .args(["tag", tag])
        .status()
        .unwrap();
    assert!(status.success(), "git tag failed: {tag}");
}

#[test]
fn doctor_reports_valid_config() {
    let dir = temp_dir("cutver-doc-ok");
    write(&dir, "package.json", r#"{"version": "1.0.0"}"#);
    write(
        &dir,
        "cutver.toml",
        r#"
[version]
current_source = "package.json"
[[manifest]]
path = "package.json"
kind = "json"
field = "version"
"#,
    );
    let args = Cli::try_parse_from(["cutver", "-c", &dir.join("cutver.toml").to_string_lossy(), "doctor"]).unwrap();
    assert_eq!(run(args), 0);
}

#[test]
fn doctor_fails_for_invalid_config() {
    let dir = temp_dir("cutver-doc-bad");
    write(&dir, "cutver.toml", "[version]\ncurrent_source = \"missing\"");
    let args = Cli::try_parse_from(["cutver", "-c", &dir.join("cutver.toml").to_string_lossy(), "doctor"]).unwrap();
    assert_eq!(run(args), 1);
}

#[test]
fn doctor_reports_drift_exit_code() {
    let dir = temp_dir("cutver-doc-drift-exit");
    write(&dir, "package.json", r#"{"version": "1.2.3"}"#);
    write(&dir, "Cargo.toml", "[package]\nversion = \"1.0.0\"\n");
    write(
        &dir,
        "cutver.toml",
        r#"
[version]
current_source = "package.json"
[[manifest]]
path = "package.json"
kind = "json"
field = "version"
[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"
"#,
    );
    let args = Cli::try_parse_from(["cutver", "-c", &dir.join("cutver.toml").to_string_lossy(), "doctor"]).unwrap();
    assert_eq!(run(args), 2);
}

#[test]
fn doctor_with_check_changelog_consistent() {
    let dir = temp_dir("cutver-doc-cl-main-ok");
    crate::git::init_test_repo(&dir);
    write(&dir, "package.json", r#"{"version": "1.0.0"}"#);
    write(
        &dir,
        "CHANGELOG.md",
        "# Changelog\n\n## [1.0.0] - 2026-01-01\n- initial\n",
    );
    write(
        &dir,
        "cutver.toml",
        r#"
[version]
current_source = "package.json"
[[manifest]]
path = "package.json"
kind = "json"
field = "version"
"#,
    );
    git_commit(&dir, "chore: initial");
    git_tag(&dir, "v1.0.0");

    let args = Cli::try_parse_from([
        "cutver",
        "-c",
        &dir.join("cutver.toml").to_string_lossy(),
        "doctor",
        "--check-changelog",
    ])
    .unwrap();
    assert_eq!(run(args), 0);
}

#[test]
fn doctor_with_check_changelog_drift_missing_in_changelog() {
    let dir = temp_dir("cutver-doc-cl-main-missing");
    crate::git::init_test_repo(&dir);
    write(&dir, "package.json", r#"{"version": "1.1.0"}"#);
    write(
        &dir,
        "CHANGELOG.md",
        "# Changelog\n\n## [1.0.0] - 2026-01-01\n- initial\n",
    );
    write(
        &dir,
        "cutver.toml",
        r#"
[version]
current_source = "package.json"
[[manifest]]
path = "package.json"
kind = "json"
field = "version"
"#,
    );
    git_commit(&dir, "chore: initial");
    git_tag(&dir, "v1.0.0");
    git_tag(&dir, "v1.1.0");

    let args = Cli::try_parse_from([
        "cutver",
        "-c",
        &dir.join("cutver.toml").to_string_lossy(),
        "doctor",
        "--check-changelog",
    ])
    .unwrap();
    assert_eq!(run(args), 2);
}

#[test]
fn doctor_with_check_changelog_orphan_section() {
    let dir = temp_dir("cutver-doc-cl-main-orphan");
    crate::git::init_test_repo(&dir);
    write(&dir, "package.json", r#"{"version": "1.0.0"}"#);
    write(
        &dir,
        "CHANGELOG.md",
        "# Changelog\n\n## [1.2.0] - 2026-01-02\n- unreleased\n\n## [1.0.0] - 2026-01-01\n- initial\n",
    );
    write(
        &dir,
        "cutver.toml",
        r#"
[version]
current_source = "package.json"
[[manifest]]
path = "package.json"
kind = "json"
field = "version"
"#,
    );
    git_commit(&dir, "chore: initial");
    git_tag(&dir, "v1.0.0");

    let args = Cli::try_parse_from([
        "cutver",
        "-c",
        &dir.join("cutver.toml").to_string_lossy(),
        "doctor",
        "--check-changelog",
    ])
    .unwrap();
    assert_eq!(run(args), 2);
}

#[test]
fn doctor_with_check_changelog_error() {
    let dir = temp_dir("cutver-doc-cl-main-err");
    crate::git::init_test_repo(&dir);
    write(&dir, "package.json", r#"{"version": "1.0.0"}"#);
    write(
        &dir,
        "cutver.toml",
        r#"
[version]
current_source = "package.json"
[changelog]
path = "NONEXISTENT.md"
[[manifest]]
path = "package.json"
kind = "json"
field = "version"
"#,
    );
    let args = Cli::try_parse_from([
        "cutver",
        "-c",
        &dir.join("cutver.toml").to_string_lossy(),
        "doctor",
        "--check-changelog",
    ])
    .unwrap();
    assert_eq!(run(args), 1);
}

#[test]
fn bump_dry_run_does_not_mutate() {
    let dir = temp_dir("cutver-bump-stub");
    crate::git::init_test_repo(&dir);
    write(&dir, "package.json", r#"{"version": "1.2.3"}"#);
    write(&dir, "Cargo.toml", "[package]\nversion = \"1.2.3\"\n");
    write(
        &dir,
        "cutver.toml",
        r#"
[version]
current_source = "package.json"
[[manifest]]
path = "package.json"
kind = "json"
field = "version"
[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"
[git]
require_clean_tree = false
[preflight]
tests = "cargo test"
"#,
    );
    let cargo_before = fs::read_to_string(dir.join("Cargo.toml")).unwrap();
    let args = Cli::try_parse_from([
        "cutver",
        "-c",
        &dir.join("cutver.toml").to_string_lossy(),
        "bump",
        "minor",
        "--dry-run",
        "--skip-preflight",
        "tests",
    ])
    .unwrap();
    assert_eq!(run(args), 0);
    assert_eq!(fs::read_to_string(dir.join("Cargo.toml")).unwrap(), cargo_before);
}

#[test]
fn bump_auto_dry_run_with_feat() {
    let dir = temp_dir("cutver-bump-auto-main");
    crate::git::init_test_repo(&dir);
    write(&dir, "package.json", r#"{"version": "1.2.3"}"#);
    write(&dir, "Cargo.toml", "[package]\nversion = \"1.2.3\"\n");
    write(
        &dir,
        "cutver.toml",
        r#"
[version]
current_source = "package.json"
[[manifest]]
path = "package.json"
kind = "json"
field = "version"
[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"
[git]
require_clean_tree = false
"#,
    );
    std::process::Command::new("git")
        .current_dir(&dir)
        .args(["add", "."])
        .status()
        .unwrap();
    std::process::Command::new("git")
        .current_dir(&dir)
        .args(["commit", "-m", "chore: initial commit", "-q"])
        .status()
        .unwrap();
    std::process::Command::new("git")
        .current_dir(&dir)
        .args(["commit", "--allow-empty", "-m", "feat: exciting new feature", "-q"])
        .status()
        .unwrap();

    let args = Cli::try_parse_from([
        "cutver",
        "-c",
        &dir.join("cutver.toml").to_string_lossy(),
        "bump",
        "auto",
        "--dry-run",
    ])
    .unwrap();
    assert_eq!(run(args), 0);
}

#[test]
fn config_defaults_and_preflight_order() {
    let dir = temp_dir("cutver-config-defaults");
    write(
        &dir,
        "cutver.toml",
        "[version]\ncurrent_source = \"a\"\n[[manifest]]\npath = \"a\"\nkind = \"cargo-package\"\n[preflight]\ntests = \"cargo test\"\nz = \"z\"\na = \"a\"\nm = \"m\"\n[changelog]\npath = \"CHANGELOG.md\"\n",
    );
    write(&dir, "a", "");
    let cfg = config::load(dir.join("cutver.toml")).unwrap();
    assert_eq!(Path::new(&cfg.version.current_source).file_name().unwrap(), "a");
    assert_eq!(
        (cfg.manifest.len(), cfg.preflight.len(), cfg.git.tag_prefix.as_str()),
        (1, 4, "v")
    );
    assert_eq!(
        (
            cfg.changelog.format.as_str(),
            cfg.changelog.entry_template.as_str(),
            cfg.git.commit_message.as_str()
        ),
        ("keep-a-changelog", "", "chore(release): v{version}")
    );
    assert!(cfg.git.require_clean_tree && cfg.git.require_branch.is_none());
    assert_eq!(
        cfg.preflight.iter().map(|(k, _)| k.as_str()).collect::<Vec<_>>(),
        vec!["tests", "z", "a", "m"]
    );
}

#[test]
fn changelog_latest_explicit_path() {
    let dir = temp_dir("cutver-cl-explicit");
    let cl = write(
        &dir,
        "MY_CHANGELOG.md",
        "# Changelog\n\n## [1.2.0] - 2026-03-01\n\n- Added feature X\n\n## [1.1.0] - 2026-02-01\n\n- Old feature\n",
    );
    let args = Cli::try_parse_from(["cutver", "changelog", "latest", "-p", &cl.to_string_lossy()]).unwrap();
    assert_eq!(run(args), 0);
}

#[test]
fn changelog_latest_explicit_path_with_header() {
    let dir = temp_dir("cutver-cl-header");
    let cl = write(
        &dir,
        "MY_CHANGELOG.md",
        "# Changelog\n\n## [1.2.0] - 2026-03-01\n\n- Added feature X\n",
    );
    let args = Cli::try_parse_from(["cutver", "changelog", "latest", "-H", "-p", &cl.to_string_lossy()]).unwrap();
    assert_eq!(run(args), 0);
}

#[test]
fn changelog_latest_from_config() {
    let dir = temp_dir("cutver-cl-cfg");
    write(
        &dir,
        "CUSTOM_CHANGELOG.md",
        "# Changelog\n\n## [2.0.0] - 2026-04-01\n\n- Breaking change\n",
    );
    write(&dir, "package.json", r#"{"version": "2.0.0"}"#);
    write(
        &dir,
        "cutver.toml",
        r#"
[version]
current_source = "package.json"
[[manifest]]
path = "package.json"
kind = "json"
field = "version"
[changelog]
path = "CUSTOM_CHANGELOG.md"
"#,
    );
    let args = Cli::try_parse_from([
        "cutver",
        "-c",
        &dir.join("cutver.toml").to_string_lossy(),
        "changelog",
        "latest",
    ])
    .unwrap();
    assert_eq!(run(args), 0);
}

#[test]
fn changelog_latest_missing_explicit_path_fails() {
    let args = Cli::try_parse_from([
        "cutver",
        "changelog",
        "latest",
        "-p",
        "this-file-does-not-exist-12345.md",
    ])
    .unwrap();
    assert_eq!(run(args), 1);
}

#[test]
fn changelog_latest_no_release_section_fails() {
    let dir = temp_dir("cutver-cl-no-rel");
    let cl = write(
        &dir,
        "CHANGELOG.md",
        "# Changelog\n\n## [Unreleased]\n\n- Work in progress\n",
    );
    let args = Cli::try_parse_from(["cutver", "changelog", "latest", "-p", &cl.to_string_lossy()]).unwrap();
    assert_eq!(run(args), 1);
}

#[test]
fn changelog_latest_config_missing_file_fails() {
    let dir = temp_dir("cutver-cl-cfg-missing");
    write(&dir, "package.json", r#"{"version": "1.0.0"}"#);
    write(
        &dir,
        "cutver.toml",
        r#"
[version]
current_source = "package.json"
[[manifest]]
path = "package.json"
kind = "json"
field = "version"
[changelog]
path = "NONEXISTENT.md"
"#,
    );
    let args = Cli::try_parse_from([
        "cutver",
        "-c",
        &dir.join("cutver.toml").to_string_lossy(),
        "changelog",
        "latest",
    ])
    .unwrap();
    assert_eq!(run(args), 1);
}

#[test]
fn changelog_latest_config_without_changelog_path_fallback() {
    let dir = temp_dir("cutver-cl-no-path");
    write(
        &dir,
        "CHANGELOG.md",
        "# Changelog\n\n## [1.0.0] - 2026-01-01\n\n- Initial release\n",
    );
    write(&dir, "package.json", r#"{"version": "1.0.0"}"#);
    write(
        &dir,
        "cutver.toml",
        r#"
[version]
current_source = "package.json"
[[manifest]]
path = "package.json"
kind = "json"
field = "version"
"#,
    );
    let args = Cli::try_parse_from([
        "cutver",
        "-c",
        &dir.join("cutver.toml").to_string_lossy(),
        "changelog",
        "latest",
    ])
    .unwrap();
    assert_eq!(run(args), 0);
}

#[test]
fn run_changelog_direct() {
    let dir = temp_dir("cutver-run-cl");
    let cl = write(
        &dir,
        "CHANGELOG.md",
        "# Changelog\n\n## [1.0.0] - 2026-01-01\n\n- Released\n",
    );
    let cmd = ChangelogCommands::Latest {
        include_header: false,
        path: Some(cl),
        template: None,
        json: false,
    };
    assert_eq!(run_changelog(None, cmd), 0);
}

#[test]
fn changelog_show_explicit_path() {
    let dir = temp_dir("cutver-cl-show-explicit");
    let cl = write(
        &dir,
        "MY_CHANGELOG.md",
        "# Changelog\n\n## [1.2.0] - 2026-03-01\n\n- Added feature X\n\n## [1.1.0] - 2026-02-01\n\n- Old feature\n",
    );
    let args = Cli::try_parse_from(["cutver", "changelog", "show", "1.1.0", "-p", &cl.to_string_lossy()]).unwrap();
    assert_eq!(run(args), 0);
}

#[test]
fn changelog_show_explicit_path_with_header() {
    let dir = temp_dir("cutver-cl-show-header");
    let cl = write(
        &dir,
        "MY_CHANGELOG.md",
        "# Changelog\n\n## [1.2.0] - 2026-03-01\n\n- Added feature X\n\n## [1.1.0] - 2026-02-01\n\n- Old feature\n",
    );
    let args = Cli::try_parse_from([
        "cutver",
        "changelog",
        "show",
        "1.1.0",
        "-H",
        "-p",
        &cl.to_string_lossy(),
    ])
    .unwrap();
    assert_eq!(run(args), 0);
}

#[test]
fn changelog_show_missing_version_fails() {
    let dir = temp_dir("cutver-cl-show-missing");
    let cl = write(
        &dir,
        "MY_CHANGELOG.md",
        "# Changelog\n\n## [1.2.0] - 2026-03-01\n\n- Added feature X\n",
    );
    let args = Cli::try_parse_from(["cutver", "changelog", "show", "0.9.0", "-p", &cl.to_string_lossy()]).unwrap();
    assert_eq!(run(args), 1);
}

#[test]
fn run_changelog_show_direct() {
    let dir = temp_dir("cutver-run-cl-show");
    let cl = write(
        &dir,
        "CHANGELOG.md",
        "# Changelog\n\n## [1.0.0] - 2026-01-01\n\n- Released\n",
    );
    let cmd = ChangelogCommands::Show {
        version: "1.0.0".to_string(),
        include_header: false,
        path: Some(cl),
        template: None,
        json: false,
    };
    assert_eq!(run_changelog(None, cmd), 0);
}

#[test]
fn run_init_success_and_fail_on_collision() {
    let dir = temp_dir("cutver-runner-init");
    write(&dir, "Cargo.toml", "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n");

    let args = Cli::try_parse_from(["cutver", "init", "-p", &dir.to_string_lossy()]).unwrap();
    assert_eq!(run(args), 0);
    assert!(dir.join("cutver.toml").is_file());

    // Second run without force should fail
    let args_fail = Cli::try_parse_from(["cutver", "init", "-p", &dir.to_string_lossy()]).unwrap();
    assert_eq!(run(args_fail), 1);

    // Third run with force should succeed
    let args_force = Cli::try_parse_from(["cutver", "init", "-p", &dir.to_string_lossy(), "--force"]).unwrap();
    assert_eq!(run(args_force), 0);
}

#[test]
fn test_external_subcommand_dispatch_and_exit_code() {
    let dir = temp_dir("cutver-runner-ext-ok");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let script_path = dir.join("cutver-mock");
        fs::write(&script_path, "#!/bin/sh\nexit 42\n").unwrap();
        let mut perms = fs::metadata(&script_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms).unwrap();
    }
    #[cfg(windows)]
    {
        fs::write(dir.join("cutver-mock.bat"), "@echo off\r\nexit /b 42\r\n").unwrap();
    }

    let orig_path = std::env::var_os("PATH").unwrap_or_default();
    let mut new_path = dir.clone().into_os_string();
    let sep = if cfg!(windows) { ";" } else { ":" };
    new_path.push(sep);
    new_path.push(&orig_path);

    unsafe {
        std::env::set_var("PATH", &new_path);
    }
    let code = run_external(&["mock".to_string(), "--foo".to_string()]);
    unsafe {
        std::env::set_var("PATH", orig_path);
    }

    assert_eq!(code, 42);
}

#[test]
fn test_external_subcommand_empty_args() {
    assert_eq!(run_external(&[]), 1);
}

#[test]
fn test_external_subcommand_not_found() {
    let code = run_external(&["nonexistent-plugin-command-xyz-123".to_string()]);
    assert_eq!(code, 1);
}

/// The exact release summary `bump --json` renders: no tree, no banner, one object.
fn release_summary() -> crate::bump::Summary {
    crate::bump::Summary {
        source: "Cargo.toml".into(),
        current: semver::Version::new(0, 18, 2),
        next: semver::Version::new(0, 19, 0),
        dry_run: false,
        preflight: Vec::new(),
        touched: Vec::new(),
        changelog: None,
        commit_message: "chore(release): v0.19.0".into(),
        tag: "v0.19.0".into(),
        tag_skipped: false,
        floating_tag: Some("v0".into()),
        commit: "abc1234".into(),
        pre_bump: None,
        post_bump: None,
        warnings: Vec::new(),
        publish_push: true,
        publish_push_command: Some("git push origin refs/heads/main".into()),
        publish_commands: vec!["cargo publish".into()],
        deferred: false,
        rationale: None,
        root_dir: None,
    }
}

#[test]
fn bump_json_renders_one_line_with_the_release_shape() {
    let json = render_summary_json(&release_summary()).expect("the summary serializes");

    assert_eq!(
        json,
        concat!(
            r#"{"released":true,"deferred":false,"version":"0.19.0","tag":"v0.19.0","commit":"abc1234","#,
            r#""floating_tag":"v0","publish_push_command":"git push origin refs/heads/main","#,
            r#""publish_commands":["cargo publish"]}"#
        )
    );
    assert!(!json.contains('\n'), "the object must be a single line: {json}");
}

#[test]
fn bump_json_reports_a_dry_run_as_not_released_without_a_commit() {
    let mut summary = release_summary();
    summary.dry_run = true;
    summary.commit = String::new();
    summary.floating_tag = None;
    summary.publish_push_command = None;

    let json = render_summary_json(&summary).expect("the summary serializes");
    let value: serde_json::Value = serde_json::from_str(&json).expect("the output is valid JSON");

    assert_eq!(value["released"], serde_json::Value::Bool(false), "{json}");
    assert_eq!(value["commit"], serde_json::Value::String(String::new()), "{json}");
    assert!(value["floating_tag"].is_null(), "optional values are null: {json}");
    assert!(value["publish_push_command"].is_null(), "{json}");
}

#[test]
fn report_file_receives_the_json_object_and_a_trailing_newline_only() {
    let dir = temp_dir("cutver-report-file");
    let path = dir.join("release.json");
    let summary = release_summary();

    write_report(&summary, &path).expect("the report is written");

    let object = render_summary_json(&summary).expect("the summary serializes");
    assert_eq!(
        fs::read(&path).expect("the report exists"),
        format!("{object}\n").into_bytes(),
        "the file must hold the JSON object plus one trailing newline and no other bytes"
    );
    let mut entries: Vec<String> = fs::read_dir(&dir)
        .expect("the report directory is readable")
        .map(|entry| {
            entry
                .expect("a readable entry")
                .file_name()
                .to_string_lossy()
                .to_string()
        })
        .collect();
    entries.sort();
    assert_eq!(entries, vec!["release.json"], "no atomic temp file may survive");
}

#[test]
fn report_file_from_a_dry_run_reports_not_released() {
    let dir = temp_dir("cutver-report-dry-run");
    let path = dir.join("release.json");
    let mut summary = release_summary();
    summary.dry_run = true;
    summary.commit = String::new();

    write_report(&summary, &path).expect("a dry run writes a report too");

    let text = fs::read_to_string(&path).expect("the report exists");
    let value: serde_json::Value = serde_json::from_str(&text).expect("the report is valid JSON");
    assert_eq!(value["released"], serde_json::Value::Bool(false), "{text}");
    assert_eq!(value["commit"], serde_json::Value::String(String::new()), "{text}");
    assert!(text.ends_with('\n'), "the report ends with one newline: {text:?}");
}

#[test]
fn report_write_failure_names_the_path_and_leaves_no_partial_file() {
    let dir = temp_dir("cutver-report-unwritable");
    let path = dir.join("missing-subdir").join("release.json");

    let err = write_report(&release_summary(), &path).expect_err("the parent directory does not exist");

    match &err {
        ReportError::Write {
            path: reported, tag, ..
        } => {
            assert_eq!(reported, &path.display().to_string());
            assert_eq!(tag, "v0.19.0");
        }
        other => panic!("expected a write failure, got: {other:?}"),
    }
    let text = err.to_string();
    assert!(
        text.contains("Where:") && text.contains("Fix:") && text.contains("atomically"),
        "the failure must be actionable: {text}"
    );
    assert!(!path.exists(), "no report may be left behind");
    let leftovers: Vec<_> = fs::read_dir(&dir)
        .expect("the report directory is readable")
        .map(|entry| {
            entry
                .expect("a readable entry")
                .file_name()
                .to_string_lossy()
                .to_string()
        })
        .collect();
    assert!(leftovers.is_empty(), "nothing may be written: {leftovers:?}");
}

#[test]
fn test_cli_parsing_external_subcommand() {
    let cli = Cli::try_parse_from(["cutver", "slack", "--channel", "general"]).unwrap();
    match cli.command {
        Commands::External(args) => {
            assert_eq!(args, vec!["slack", "--channel", "general"]);
        }
        _ => panic!("expected Commands::External variant"),
    }
}
