//! Declared `[[artifact]]` tables, end to end (#326 slice 3A).
//!
//! `cutver publish` verifies every declared artifact after validation and before the first
//! irreversible step, so a missing artifact must leave the repository untouched: no tag locally, no
//! tag on the remote, no `[publish] commands`, no change in the worktree. `--dry-run` promises the
//! same verdict, so it fails the same way.
//!
//! Not `#![cfg(unix)]`: the commands that must not run are shell lines, and the assertions are about
//! their absence and about git state, both portable.

mod common;

use std::path::Path;
use std::process::Output;

use common::*;

const TOML: &str = r#"[version]
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
require_clean_tree = true

[publish]
push = true
commands = ["echo published {version} > published.txt"]
"#;

/// A repository at 1.2.3 on `main`, published to a bare remote, with `extra_artifacts` appended as
/// `[[artifact]]` declarations. Built assets are ignored so the test is about the artifact check and
/// not about the clean-tree guard.
fn prepared_repo(name: &str, extra_artifacts: &str) -> (FixtureGuard, Fixture) {
    assert!(
        git_available(),
        "git CLI is required for e2e tests but was not found in PATH"
    );
    let remote = Fixture::new(&format!("{name}-remote"));
    run_git_ok(&remote.dir, &["init", "--bare", "-q"]);

    let guard = FixtureGuard::new(name);
    let fixture = guard.fixture();
    fixture.write("package.json", PACKAGE_JSON);
    fixture.write("Cargo.toml", CARGO_TOML);
    fixture.write("CHANGELOG.md", CHANGELOG_MD);
    fixture.write(".gitignore", "dist/\nartifacts/\npublished.txt\n");
    fixture.write("cutver.toml", &format!("{TOML}{extra_artifacts}"));
    init_git_repo(fixture);
    run_git_ok(&fixture.dir, &["checkout", "-B", "main"]);
    initial_commit(fixture);
    run_git_ok(&fixture.dir, &["remote", "add", "origin", remote.dir.to_str().unwrap()]);
    (guard, remote)
}

fn cutver(fixture: &Fixture, args: &[&str]) -> Output {
    cutver_command(&fixture.dir)
        .args(args)
        .output()
        .expect("failed to execute the cutver binary")
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

fn rev(repo: &Path, reference: &str) -> String {
    let out = run_git(repo, &["rev-parse", reference]);
    assert!(
        out.status.success(),
        "git rev-parse {reference} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn head(fixture: &Fixture) -> String {
    rev(&fixture.dir, "HEAD")
}

fn remote_refs(remote: &Path) -> String {
    let out = run_git(remote, &["for-each-ref", "--format=%(refname)"]);
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn porcelain(fixture: &Fixture) -> String {
    let out = run_git(&fixture.dir, &["status", "--porcelain"]);
    String::from_utf8_lossy(&out.stdout)
        .replace("\r\n", "\n")
        .trim_end()
        .to_string()
}

/// The prepare phase: commit and push the branch, leave the tag to `publish`.
fn prepare(fixture: &Fixture) -> String {
    let out = cutver(fixture, &["bump", "minor", "--defer-publish"]);
    assert!(
        out.status.success(),
        "the prepare phase must succeed; stderr:\n{}",
        stderr_of(&out)
    );
    head(fixture)
}

fn publish(fixture: &Fixture, commit: &str, dry_run: bool) -> Output {
    let mut args = vec!["publish", "--tag", "v1.3.0", "--commit", commit];
    if dry_run {
        args.push("--dry-run");
    }
    cutver(fixture, &args)
}

/// S2: a declared artifact whose glob matches lets the closing phase proceed.
#[test]
fn a_declared_artifact_that_matches_lets_publish_proceed() {
    let (guard, remote) = prepared_repo(
        "artifacts-match",
        "\n[[artifact]]\nglob = \"dist/*.tar.gz\"\ndescription = \"release tarball\"\n",
    );
    let fixture = guard.fixture();
    let head = prepare(fixture);
    fixture.write("dist/cutver-1.3.0.tar.gz", "tarball");
    assert_eq!(porcelain(fixture), "", "the built asset is ignored, not a tree change");

    let out = publish(fixture, &head, false);

    assert!(out.status.success(), "stderr:\n{}", stderr_of(&out));
    assert_eq!(
        rev(&fixture.dir, "v1.3.0^{commit}"),
        head,
        "the release was tagged locally"
    );
    assert_eq!(
        rev(&remote.dir, "refs/tags/v1.3.0^{commit}"),
        head,
        "the release tag reached the remote"
    );
    assert_eq!(
        fixture.read("published.txt").trim(),
        "published 1.3.0",
        "the publish commands ran after the verification passed"
    );
}

/// S2: no match fails closed before the first irreversible step, and says exactly what it counted.
#[test]
fn a_declared_artifact_with_no_match_refuses_before_anything_irreversible() {
    let (guard, remote) = prepared_repo(
        "artifacts-missing",
        "\n[[artifact]]\nglob = \"dist/*.tar.gz\"\ndescription = \"release tarball\"\n",
    );
    let fixture = guard.fixture();
    let head = prepare(fixture);
    let status_before = porcelain(fixture);
    assert_eq!(status_before, "", "the fixture starts from a clean tree");

    let dry_run = publish(fixture, &head, true);
    assert_eq!(
        dry_run.status.code(),
        Some(1),
        "a dry run promises the real run's verdict; stderr:\n{}",
        stderr_of(&dry_run)
    );
    let dry_message = stderr_of(&dry_run);
    assert!(dry_message.contains("release tarball"), "{dry_message}");
    assert!(dry_message.contains("dist/*.tar.gz"), "{dry_message}");
    assert!(dry_message.contains("0 file(s)"), "{dry_message}");
    assert!(dry_message.contains("1 required"), "{dry_message}");
    assert!(
        dry_message.contains("Nothing was tagged, pushed or executed"),
        "{dry_message}"
    );

    let out = publish(fixture, &head, false);
    assert_eq!(out.status.code(), Some(1), "stderr:\n{}", stderr_of(&out));
    let message = stderr_of(&out);
    assert!(message.contains("release tarball"), "{message}");
    assert!(message.contains("dist/*.tar.gz"), "{message}");
    assert!(message.contains("0 file(s)"), "{message}");
    assert!(message.contains("1 required"), "{message}");
    assert!(!tag_exists(fixture, "v1.3.0"), "no tag may be created locally");
    assert!(
        !remote_refs(&remote.dir).contains("refs/tags/"),
        "no tag may reach the remote: {}",
        remote_refs(&remote.dir)
    );
    assert!(
        !fixture.dir.join("published.txt").exists(),
        "no publish command may run"
    );
    assert_eq!(
        head,
        rev(&fixture.dir, "HEAD"),
        "the release commit is the committed HEAD"
    );
    assert_eq!(porcelain(fixture), status_before, "the worktree is unchanged");
}

/// S3: `**` is what reaches into a subdirectory, and it does reach there.
#[test]
fn a_recursive_glob_reaches_into_a_subdirectory() {
    let (guard, remote) = prepared_repo(
        "artifacts-recursive",
        "\n[[artifact]]\nglob = \"artifacts/**/*.whl\"\ndescription = \"python wheels\"\n",
    );
    let fixture = guard.fixture();
    let head = prepare(fixture);
    fixture.write("artifacts/linux/x86_64/cutver-1.3.0.whl", "wheel");

    let out = publish(fixture, &head, false);

    assert!(out.status.success(), "stderr:\n{}", stderr_of(&out));
    assert_eq!(rev(&fixture.dir, "v1.3.0^{commit}"), head);
    assert_eq!(rev(&remote.dir, "refs/tags/v1.3.0^{commit}"), head);
}

/// S3: a glob that would walk out of the workspace is refused, not silently treated as empty.
#[test]
fn a_glob_that_escapes_the_workspace_is_refused_not_silently_empty() {
    let (guard, remote) = prepared_repo("artifacts-escape", "\n[[artifact]]\nglob = \"../*\"\n");
    let fixture = guard.fixture();
    let head = prepare(fixture);
    let status_before = porcelain(fixture);

    let out = publish(fixture, &head, false);

    assert_eq!(out.status.code(), Some(1), "stderr:\n{}", stderr_of(&out));
    let message = stderr_of(&out);
    assert!(message.contains("escapes the workspace"), "{message}");
    assert!(message.contains("../*"), "{message}");
    assert!(!tag_exists(fixture, "v1.3.0"), "no tag may be created locally");
    assert!(
        !remote_refs(&remote.dir).contains("refs/tags/"),
        "no tag may reach the remote: {}",
        remote_refs(&remote.dir)
    );
    assert!(
        !fixture.dir.join("published.txt").exists(),
        "no publish command may run"
    );
    assert_eq!(porcelain(fixture), status_before, "the worktree is unchanged");
}
