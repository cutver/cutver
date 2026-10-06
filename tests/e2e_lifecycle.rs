#![cfg(unix)]

mod common;

use common::*;
use cutver::bump::run as bump_run;
use cutver::config;
use cutver::semver_bump::Bump;

#[test]
fn post_bump_runs_in_dry_run_and_real_bump() {
    assert!(git_available());
    let guard = FixtureGuard::new("post-bump-runs");
    let fixture = guard.fixture();
    let toml = r#"[version]
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"

[changelog]
path = "CHANGELOG.md"

[git]
require_clean_tree = false

[hooks]
post_bump = "touch post_bump_ran.txt"
"#;
    fixture.write("package.json", PACKAGE_JSON);
    fixture.write("Cargo.toml", CARGO_TOML);
    fixture.write("CHANGELOG.md", CHANGELOG_MD);
    fixture.write("cutver.toml", toml);
    init_git_repo(fixture);
    initial_commit(fixture);

    let cfg = config::load("cutver.toml").unwrap();

    // Dry run: summary records post_bump, command NOT executed
    let dry_summary = bump_run(&cfg, Bump::Minor, true, &[]).unwrap();
    assert!(dry_summary.dry_run);
    assert_eq!(dry_summary.post_bump.as_deref(), Some("touch post_bump_ran.txt"));
    assert!(!fixture.dir.join("post_bump_ran.txt").exists());

    // Real run: post_bump executes and file is created
    let real_summary = bump_run(&cfg, Bump::Minor, false, &[]).unwrap();
    assert!(!real_summary.dry_run);
    assert_eq!(real_summary.post_bump.as_deref(), Some("touch post_bump_ran.txt"));
    assert!(fixture.dir.join("post_bump_ran.txt").exists());
}

#[test]
fn post_bump_modifies_file_and_file_is_staged_and_committed() {
    assert!(git_available());
    let guard = FixtureGuard::new("post-bump-modify");
    let fixture = guard.fixture();
    let toml = r#"[version]
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"

[changelog]
path = "CHANGELOG.md"

[git]
require_clean_tree = true

[hooks]
post_bump = "echo \"lockfile-version-1.3.0\" > Cargo.lock"
"#;
    fixture.write("package.json", PACKAGE_JSON);
    fixture.write("Cargo.toml", CARGO_TOML);
    fixture.write("Cargo.lock", "lockfile-version-1.2.3\n");
    fixture.write("CHANGELOG.md", CHANGELOG_MD);
    fixture.write("cutver.toml", toml);
    init_git_repo(fixture);
    initial_commit(fixture);

    let cfg = config::load("cutver.toml").unwrap();
    let summary = bump_run(&cfg, Bump::Minor, false, &[]).unwrap();
    assert_eq!(summary.next.to_string(), "1.3.0");

    // Check that Cargo.lock was updated and included in the release commit
    assert_eq!(fixture.read("Cargo.lock").trim(), "lockfile-version-1.3.0");
    let commit_files = head_commit_files(fixture);
    assert!(
        commit_files.contains(&"Cargo.lock".to_string()),
        "commit_files: {commit_files:?}"
    );

    // Tree should be clean after bump
    let status_out = run_git(&fixture.dir, &["status", "--porcelain"]);
    assert!(String::from_utf8_lossy(&status_out.stdout).trim().is_empty());
}

#[test]
fn post_bump_stages_both_root_and_nested_lockfiles() {
    assert!(git_available());
    let guard = FixtureGuard::new("post-bump-root-and-nested-lockfiles");
    let fixture = guard.fixture();
    let toml = r#"[version]
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"

[changelog]
path = "CHANGELOG.md"

[git]
require_clean_tree = true

[hooks]
post_bump = "echo \"lockfile-root-1.3.0\" > Cargo.lock && echo \"lockfile-nested-1.3.0\" > crates/core/Cargo.lock && echo \"lockfile-app-1.3.0\" > App/Cargo.lock"
"#;
    fixture.write("package.json", PACKAGE_JSON);
    fixture.write("Cargo.toml", CARGO_TOML);
    fixture.write("Cargo.lock", "lockfile-root-1.2.3\n");
    fixture.write("crates/core/Cargo.lock", "lockfile-nested-1.2.3\n");
    fixture.write("App/Cargo.lock", "lockfile-app-1.2.3\n");
    fixture.write("CHANGELOG.md", CHANGELOG_MD);
    fixture.write("cutver.toml", toml);
    init_git_repo(fixture);
    initial_commit(fixture);

    let cfg = config::load("cutver.toml").unwrap();
    let summary = bump_run(&cfg, Bump::Minor, false, &[]).unwrap();
    assert_eq!(summary.next.to_string(), "1.3.0");

    // All lockfiles were updated
    assert_eq!(fixture.read("Cargo.lock").trim(), "lockfile-root-1.3.0");
    assert_eq!(fixture.read("crates/core/Cargo.lock").trim(), "lockfile-nested-1.3.0");
    assert_eq!(fixture.read("App/Cargo.lock").trim(), "lockfile-app-1.3.0");

    // Both root and nested lockfiles must be included in the release commit
    let commit_files = head_commit_files(fixture);
    assert!(
        commit_files.contains(&"Cargo.lock".to_string()),
        "commit_files should contain root Cargo.lock: {commit_files:?}"
    );
    assert!(
        commit_files.contains(&"crates/core/Cargo.lock".to_string()),
        "commit_files should contain crates/core/Cargo.lock: {commit_files:?}"
    );
    assert!(
        commit_files.contains(&"App/Cargo.lock".to_string()),
        "commit_files should contain App/Cargo.lock: {commit_files:?}"
    );

    // Tree should be clean after bump
    let status_out = run_git(&fixture.dir, &["status", "--porcelain"]);
    assert!(
        String::from_utf8_lossy(&status_out.stdout).trim().is_empty(),
        "working tree must be clean after bump"
    );
}

#[test]
fn post_bump_restricts_staging_to_known_lockfiles_and_leaves_arbitrary_files_unstaged() {
    assert!(git_available());
    let guard = FixtureGuard::new("post-bump-lockfile-only");
    let fixture = guard.fixture();
    let toml = r#"[version]
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"

[changelog]
path = "CHANGELOG.md"

[git]
require_clean_tree = true

[hooks]
post_bump = "echo \"lockfile-version-1.3.0\" > Cargo.lock && echo \"arbitrary-modification\" > unrelated.txt"
"#;
    fixture.write("package.json", PACKAGE_JSON);
    fixture.write("Cargo.toml", CARGO_TOML);
    fixture.write("Cargo.lock", "lockfile-version-1.2.3\n");
    fixture.write("unrelated.txt", "unrelated-initial\n");
    fixture.write("CHANGELOG.md", CHANGELOG_MD);
    fixture.write("cutver.toml", toml);
    init_git_repo(fixture);
    initial_commit(fixture);

    let cfg = config::load("cutver.toml").unwrap();
    let summary = bump_run(&cfg, Bump::Minor, false, &[]).unwrap();
    assert_eq!(summary.next.to_string(), "1.3.0");

    // Both files were modified by post_bump hook
    assert_eq!(fixture.read("Cargo.lock").trim(), "lockfile-version-1.3.0");
    assert_eq!(fixture.read("unrelated.txt").trim(), "arbitrary-modification");

    // ONLY Cargo.lock was staged and committed; unrelated.txt was not committed
    let commit_files = head_commit_files(fixture);
    assert!(
        commit_files.contains(&"Cargo.lock".to_string()),
        "commit_files should contain Cargo.lock: {commit_files:?}"
    );
    assert!(
        !commit_files.contains(&"unrelated.txt".to_string()),
        "commit_files should NOT contain unrelated.txt: {commit_files:?}"
    );

    // unrelated.txt remains unstaged in working tree
    let status_out = run_git(&fixture.dir, &["status", "--porcelain"]);
    let status_str = String::from_utf8_lossy(&status_out.stdout);
    assert!(
        status_str.contains("unrelated.txt"),
        "git status should report unrelated.txt: {status_str}"
    );
    assert!(
        status_str.lines().any(|l| l.starts_with(" M unrelated.txt")),
        "unrelated.txt must be unstaged (not in index): {status_str}"
    );

    // No files should remain staged in the index
    let diff_cached = run_git(&fixture.dir, &["diff", "--cached", "--name-only"]);
    assert!(
        String::from_utf8_lossy(&diff_cached.stdout).trim().is_empty(),
        "no files should remain staged in the index"
    );

    // Working tree diff should only show unrelated.txt
    let diff_unstaged = run_git(&fixture.dir, &["diff", "--name-only"]);
    let diff_unstaged_str = String::from_utf8_lossy(&diff_unstaged.stdout);
    let unstaged_files: Vec<&str> = diff_unstaged_str
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    assert_eq!(unstaged_files, vec!["unrelated.txt"]);
}

#[test]
fn post_bump_failure_triggers_rollback_and_aborts() {
    assert!(git_available());
    let guard = FixtureGuard::new("post-bump-fail");
    let fixture = guard.fixture();
    let toml = r#"[version]
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"

[changelog]
path = "CHANGELOG.md"

[git]
require_clean_tree = true

[hooks]
post_bump = "exit 1"
"#;
    fixture.write("package.json", PACKAGE_JSON);
    fixture.write("Cargo.toml", CARGO_TOML);
    fixture.write("CHANGELOG.md", CHANGELOG_MD);
    fixture.write("cutver.toml", toml);
    init_git_repo(fixture);
    initial_commit(fixture);

    let cfg = config::load("cutver.toml").unwrap();
    let res = bump_run(&cfg, Bump::Minor, false, &[]);
    assert!(res.is_err(), "expected bump to fail due to post_bump exit 1");

    // Manifests rolled back to 1.2.3
    assert!(fixture.read("package.json").contains("\"version\": \"1.2.3\""));
    assert!(fixture.read("Cargo.toml").contains("version = \"1.2.3\""));
    assert!(!fixture.read("CHANGELOG.md").contains("1.3.0"));

    // No commit or tag created
    assert_eq!(commit_count(fixture), 1);
    assert!(!tag_exists(fixture, "v1.3.0"));
}

#[test]
fn publish_push_and_commands_reported_in_dry_run_and_executed_in_real_run() {
    assert!(git_available());
    let guard = FixtureGuard::new("publish-push-cmds");
    let fixture = guard.fixture();
    let remote = Fixture::new("remote-target");
    run_git_ok(&remote.dir, &["init", "--bare"]);

    let toml = r#"[version]
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"

[changelog]
path = "CHANGELOG.md"

[git]
require_clean_tree = true
require_branch = "main"

[publish]
push = true
commands = ["echo published {version} > published.txt"]
"#;
    fixture.write("package.json", PACKAGE_JSON);
    fixture.write("Cargo.toml", CARGO_TOML);
    fixture.write("CHANGELOG.md", CHANGELOG_MD);
    fixture.write("cutver.toml", toml);
    init_git_repo(fixture);
    run_git_ok(&fixture.dir, &["checkout", "-B", "main"]);
    initial_commit(fixture);
    run_git_ok(&fixture.dir, &["remote", "add", "origin", remote.dir.to_str().unwrap()]);

    let cfg = config::load("cutver.toml").unwrap();

    // Dry run: reported in summary, neither push nor commands executed
    let dry_summary = bump_run(&cfg, Bump::Minor, true, &[]).unwrap();
    assert!(dry_summary.dry_run);
    assert!(dry_summary.publish_push);
    assert_eq!(
        dry_summary.publish_push_command.as_deref(),
        Some("git push origin main --tags")
    );
    assert_eq!(
        dry_summary.publish_commands,
        vec!["echo published 1.3.0 > published.txt"]
    );
    assert!(!fixture.dir.join("published.txt").exists());
    let remote_tags = run_git(&remote.dir, &["tag", "-l"]);
    assert!(String::from_utf8_lossy(&remote_tags.stdout).trim().is_empty());

    // Real run: push and commands executed
    let real_summary = bump_run(&cfg, Bump::Minor, false, &[]).unwrap();
    assert!(!real_summary.dry_run);
    assert!(real_summary.publish_push);
    assert_eq!(
        real_summary.publish_commands,
        vec!["echo published 1.3.0 > published.txt"]
    );
    assert!(fixture.dir.join("published.txt").exists());
    assert_eq!(fixture.read("published.txt").trim(), "published 1.3.0");

    // Verify tag pushed to remote
    let remote_tags = run_git(&remote.dir, &["tag", "-l"]);
    assert!(String::from_utf8_lossy(&remote_tags.stdout).contains("v1.3.0"));
}

#[test]
fn publish_push_with_floating_major_tag_pushes_floating_tag_to_remote() {
    assert!(git_available());
    let guard = FixtureGuard::new("publish-push-floating-tag");
    let fixture = guard.fixture();
    let remote = Fixture::new("remote-target-floating-tag");
    run_git_ok(&remote.dir, &["init", "--bare"]);

    let toml = r#"[version]
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"

[changelog]
path = "CHANGELOG.md"

[git]
tag_prefix = "v"
floating_major_tag = true
require_clean_tree = true
require_branch = "main"

[publish]
push = true
"#;
    fixture.write("package.json", PACKAGE_JSON);
    fixture.write("Cargo.toml", CARGO_TOML);
    fixture.write("CHANGELOG.md", CHANGELOG_MD);
    fixture.write("cutver.toml", toml);
    init_git_repo(fixture);
    run_git_ok(&fixture.dir, &["checkout", "-B", "main"]);
    initial_commit(fixture);
    run_git_ok(&fixture.dir, &["remote", "add", "origin", remote.dir.to_str().unwrap()]);

    let cfg = config::load("cutver.toml").unwrap();

    // Dry run
    let dry_summary = bump_run(&cfg, Bump::Minor, true, &[]).unwrap();
    assert!(dry_summary.dry_run);
    assert!(dry_summary.publish_push);
    assert_eq!(
        dry_summary.publish_push_command.as_deref(),
        Some("git push origin +refs/tags/v1:refs/tags/v1 && git push origin main --tags")
    );

    // Real run: both v1.3.0 and v1 pushed to remote
    let real_summary = bump_run(&cfg, Bump::Minor, false, &[]).unwrap();
    assert!(!real_summary.dry_run);
    assert_eq!(real_summary.floating_tag.as_deref(), Some("v1"));

    let remote_tags = run_git(&remote.dir, &["tag", "-l"]);
    let remote_tags_str = String::from_utf8_lossy(&remote_tags.stdout);
    assert!(remote_tags_str.contains("v1.3.0"));
    assert!(remote_tags_str.contains("v1"));
}

#[test]
fn publish_push_resolves_active_branch_when_require_branch_is_omitted() {
    assert!(git_available());
    let guard = FixtureGuard::new("publish-push-no-require-branch");
    let fixture = guard.fixture();
    let remote = Fixture::new("remote-target-no-req-branch");
    run_git_ok(&remote.dir, &["init", "--bare"]);

    let toml = r#"[version]
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"

[changelog]
path = "CHANGELOG.md"

[git]
require_clean_tree = true

[publish]
push = true
"#;
    fixture.write("package.json", PACKAGE_JSON);
    fixture.write("Cargo.toml", CARGO_TOML);
    fixture.write("CHANGELOG.md", CHANGELOG_MD);
    fixture.write("cutver.toml", toml);
    init_git_repo(fixture);
    run_git_ok(&fixture.dir, &["checkout", "-B", "release-branch"]);
    initial_commit(fixture);
    run_git_ok(&fixture.dir, &["remote", "add", "origin", remote.dir.to_str().unwrap()]);

    let cfg = config::load("cutver.toml").unwrap();

    // Dry run dynamically resolves active branch "release-branch"
    let dry_summary = bump_run(&cfg, Bump::Minor, true, &[]).unwrap();
    assert_eq!(
        dry_summary.publish_push_command.as_deref(),
        Some("git push origin release-branch --tags")
    );

    // Real run pushes release-branch and tag to remote
    let real_summary = bump_run(&cfg, Bump::Minor, false, &[]).unwrap();
    assert_eq!(real_summary.publish_push_command.as_deref(), None);

    // Verify tag pushed to remote
    let remote_tags = run_git(&remote.dir, &["tag", "-l"]);
    assert!(String::from_utf8_lossy(&remote_tags.stdout).contains("v1.3.0"));

    // Verify release-branch pushed to remote
    let remote_branches = run_git(&remote.dir, &["branch", "-l"]);
    assert!(String::from_utf8_lossy(&remote_branches.stdout).contains("release-branch"));
}

#[test]
fn publish_command_aborts_when_timing_out() {
    assert!(git_available());
    let guard = FixtureGuard::new("publish-cmd-timeout");
    let fixture = guard.fixture();

    let toml = r#"[version]
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"

[changelog]
path = "CHANGELOG.md"

[git]
require_clean_tree = true
require_branch = "main"

[publish]
default_timeout = 1
commands = ["sleep 5"]
"#;
    fixture.write("package.json", PACKAGE_JSON);
    fixture.write("Cargo.toml", CARGO_TOML);
    fixture.write("CHANGELOG.md", CHANGELOG_MD);
    fixture.write("cutver.toml", toml);
    init_git_repo(fixture);
    run_git_ok(&fixture.dir, &["checkout", "-B", "main"]);
    initial_commit(fixture);

    let cfg = config::load("cutver.toml").unwrap();

    // Dry run remains unaffected (commands are recorded but not executed)
    let dry_summary = bump_run(&cfg, Bump::Minor, true, &[]).unwrap();
    assert_eq!(dry_summary.publish_commands, vec!["sleep 5"]);

    // Real run: command times out and aborts without hanging
    let start = std::time::Instant::now();
    let err = bump_run(&cfg, Bump::Minor, false, &[]).unwrap_err();
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_secs() < 4,
        "command hung or did not abort promptly: took {:?}",
        elapsed
    );

    match err {
        cutver::bump::Error::PublishCommandTimeout {
            ref command,
            timeout,
            elapsed_ms,
        } => {
            assert_eq!(command, "sleep 5");
            assert_eq!(timeout, 1);
            assert!(elapsed_ms >= 1000);
        }
        other => panic!("expected PublishCommandTimeout, got: {:?}", other),
    }

    let err_str = err.to_string();
    assert!(
        err_str.contains("publish command 'sleep 5' timed out after"),
        "unexpected error message: {err_str}"
    );
    assert!(err_str.contains("(limit 1s)"), "unexpected error message: {err_str}");
}

#[test]
fn post_bump_stages_modern_lockfiles() {
    assert!(
        git_available(),
        "git CLI is required for e2e tests but was not found in PATH"
    );
    let guard = FixtureGuard::new("post-bump-modern-lockfiles");
    let fixture = guard.fixture();
    let toml = r#"[version]
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[hooks]
post_bump = "echo bun-lock >> bun.lock && echo uv-lock >> uv.lock && echo pdm-lock >> pdm.lock"
"#;
    fixture.write("package.json", PACKAGE_JSON);
    fixture.write("cutver.toml", toml);
    fixture.write("bun.lock", "bun-lock-initial\n");
    fixture.write("uv.lock", "uv-lock-initial\n");
    fixture.write("pdm.lock", "pdm-lock-initial\n");
    init_git_repo(fixture);
    initial_commit(fixture);

    let cfg = config::load("cutver.toml").unwrap();
    let _summary = bump_run(&cfg, Bump::Patch, false, &[]).unwrap();

    let commit_files = head_commit_files(fixture);
    assert!(
        commit_files.contains(&"bun.lock".to_string()),
        "commit_files should contain bun.lock: {commit_files:?}"
    );
    assert!(
        commit_files.contains(&"uv.lock".to_string()),
        "commit_files should contain uv.lock: {commit_files:?}"
    );
    assert!(
        commit_files.contains(&"pdm.lock".to_string()),
        "commit_files should contain pdm.lock: {commit_files:?}"
    );
}

#[test]
fn post_bump_and_publish_commands_with_minijinja_interpolation() {
    assert!(
        git_available(),
        "git CLI is required for e2e tests but was not found in PATH"
    );
    let guard = FixtureGuard::new("lifecycle-minijinja-interp");
    let fixture = guard.fixture();

    let toml = r#"[version]
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[hooks]
post_bump = "echo \"bump={{ bump_level }} tag={{ tag }} branch={{ branch }}\" > hook_out.txt"

[publish]
push = false
commands = [
    "echo \"published {{ version }} (tag {{ tag }}) [forge: {{ forge }} repo: {{ repo }} owner: {{ owner }}]\" > publish_out.txt"
]
"#;
    fixture.write("package.json", PACKAGE_JSON);
    fixture.write("cutver.toml", toml);
    init_git_repo(fixture);
    run_git_ok(&fixture.dir, &["checkout", "-B", "main"]);
    initial_commit(fixture);
    run_git_ok(
        &fixture.dir,
        &["remote", "add", "origin", "https://github.com/test-owner/test-repo.git"],
    );

    let cfg = config::load("cutver.toml").unwrap();

    let summary = bump_run(&cfg, Bump::Minor, false, &[]).unwrap();
    assert_eq!(summary.next.to_string(), "1.3.0");
    assert_eq!(
        summary.post_bump.as_deref(),
        Some("echo \"bump=minor tag=v1.3.0 branch=main\" > hook_out.txt")
    );
    assert_eq!(fixture.read("hook_out.txt").trim(), "bump=minor tag=v1.3.0 branch=main");

    assert_eq!(summary.publish_commands.len(), 1);
    assert_eq!(
        summary.publish_commands[0],
        "echo \"published 1.3.0 (tag v1.3.0) [forge: github repo: test-repo owner: test-owner]\" > publish_out.txt"
    );
    assert!(fixture.dir.join("publish_out.txt").exists());
    assert_eq!(
        fixture.read("publish_out.txt").trim(),
        "published 1.3.0 (tag v1.3.0) [forge: github repo: test-repo owner: test-owner]"
    );
}

#[test]
fn test_plugin_pre_bump_rejection_aborts_without_mutating_files() {
    assert!(git_available());
    let guard = FixtureGuard::new("plugin-pre-bump-reject");
    let fixture = guard.fixture();

    let reject_script = fixture.dir.join("reject.sh");
    fixture.write(
        "reject.sh",
        "#!/bin/sh\ncat >/dev/null\necho '{\"allow\": false, \"reason\": \"Pre-bump check failed\"}'\n",
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&reject_script).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&reject_script, perms).unwrap();
    }

    let script_path = reject_script.to_string_lossy().replace('\\', "/");
    let toml = format!(
        r#"[version]
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"

[changelog]
path = "CHANGELOG.md"

[git]
require_clean_tree = true

[plugins.verifier]
runtime = "process"
command = "{script_path}"
capabilities = ["lifecycle.v1"]
events = ["on_pre_bump"]
"#
    );

    fixture.write("package.json", PACKAGE_JSON);
    fixture.write("Cargo.toml", CARGO_TOML);
    fixture.write("CHANGELOG.md", CHANGELOG_MD);
    fixture.write("cutver.toml", &toml);
    init_git_repo(fixture);
    initial_commit(fixture);

    let cfg = config::load("cutver.toml").unwrap();
    let res = bump_run(&cfg, Bump::Minor, false, &[]);

    let err = res.expect_err("bump must fail due to pre-bump rejection");
    match err {
        cutver::bump::Error::PreBumpRejected { plugin, reason } => {
            assert_eq!(plugin.as_str(), "verifier");
            assert_eq!(reason, "Pre-bump check failed");
        }
        other => panic!("expected PreBumpRejected, got {other:?}"),
    }

    // Manifests and changelog remain untouched
    assert!(fixture.read("package.json").contains("\"version\": \"1.2.3\""));
    assert!(fixture.read("Cargo.toml").contains("version = \"1.2.3\""));
    assert!(!fixture.read("CHANGELOG.md").contains("1.3.0"));

    // No commit or tag created
    assert_eq!(commit_count(fixture), 1);
    assert!(!tag_exists(fixture, "v1.3.0"));
}

#[test]
fn test_plugin_post_bump_failure_triggers_transaction_rollback() {
    assert!(git_available());
    let guard = FixtureGuard::new("plugin-post-bump-rollback");
    let fixture = guard.fixture();

    let fail_script = fixture.dir.join("fail.sh");
    fixture.write("fail.sh", "#!/bin/sh\ncat >/dev/null\nexit 1\n");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&fail_script).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&fail_script, perms).unwrap();
    }

    let script_path = fail_script.to_string_lossy().replace('\\', "/");
    let toml = format!(
        r#"[version]
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"

[changelog]
path = "CHANGELOG.md"

[git]
require_clean_tree = true

[plugins.failing-hook]
runtime = "process"
command = "{script_path}"
capabilities = ["lifecycle.v1"]
events = ["on_post_bump"]
"#
    );

    fixture.write("package.json", PACKAGE_JSON);
    fixture.write("Cargo.toml", CARGO_TOML);
    fixture.write("CHANGELOG.md", CHANGELOG_MD);
    fixture.write("cutver.toml", &toml);
    init_git_repo(fixture);
    initial_commit(fixture);

    let cfg = config::load("cutver.toml").unwrap();
    let res = bump_run(&cfg, Bump::Minor, false, &[]);
    assert!(res.is_err(), "expected bump to fail due to post_bump plugin exit 1");

    // Manifests rolled back to 1.2.3 byte-for-byte by MutationTransaction
    assert_eq!(fixture.read("package.json"), PACKAGE_JSON);
    assert_eq!(fixture.read("Cargo.toml"), CARGO_TOML);
    assert_eq!(fixture.read("CHANGELOG.md"), CHANGELOG_MD);

    // No commit or tag created
    assert_eq!(commit_count(fixture), 1);
    assert!(!tag_exists(fixture, "v1.3.0"));
}

#[test]
fn test_plugin_lifecycle_happy_path() {
    assert!(git_available());
    let guard = FixtureGuard::new("plugin-lifecycle-happy");
    let fixture = guard.fixture();

    let hook_script = fixture.dir.join("hook.sh");
    fixture.write("hook.sh", "#!/bin/sh\nread -r line\necho '{\"allow\": true}'\n");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&hook_script).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&hook_script, perms).unwrap();
    }

    let script_path = hook_script.to_string_lossy().replace('\\', "/");
    let toml = format!(
        r#"[version]
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"

[changelog]
path = "CHANGELOG.md"

[git]
require_clean_tree = true

[plugins.lifecycle-orchestrator]
runtime = "process"
command = "{script_path}"
capabilities = ["lifecycle.v1"]
events = ["on_pre_bump", "on_post_bump", "on_post_release"]
"#
    );

    fixture.write("package.json", PACKAGE_JSON);
    fixture.write("Cargo.toml", CARGO_TOML);
    fixture.write("CHANGELOG.md", CHANGELOG_MD);
    fixture.write("cutver.toml", &toml);
    init_git_repo(fixture);
    initial_commit(fixture);

    let cfg = config::load("cutver.toml").unwrap();
    let summary = bump_run(&cfg, Bump::Minor, false, &[]).unwrap();

    assert_eq!(summary.next.to_string(), "1.3.0");
    assert_eq!(summary.tag, "v1.3.0");
    assert!(fixture.read("package.json").contains("\"version\": \"1.3.0\""));
    assert!(fixture.read("Cargo.toml").contains("version = \"1.3.0\""));
    assert!(fixture.read("CHANGELOG.md").contains("1.3.0"));

    assert_eq!(commit_count(fixture), 2);
    assert!(tag_exists(fixture, "v1.3.0"));
}
