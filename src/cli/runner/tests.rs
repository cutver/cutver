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
    let script_path = dir.join("cutver-mock");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
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
