//! Two-phase release: `bump --defer-publish` prepares, `cutver publish` closes (issue #259).
//!
//! A release runs in two invocations so the irreversible steps can wait for verification: the
//! deferred bump commits, pushes the branch and stops; `cutver publish` then creates the tag, pushes
//! it, moves the floating major tag and runs `[publish].commands` — but only when the tag matches
//! what the manifests at HEAD declare.

mod common;

use std::path::Path;

use common::*;
use cutver::bump::{PublishRequest, run_deferred_publish, run_publish};
use cutver::config;
use cutver::semver_bump::Bump;

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
floating_major_tag = true
require_clean_tree = true
require_branch = "main"

[publish]
push = true
commands = ["echo published {version} > published.txt"]
"#;

fn rev(repo: &Path, reff: &str) -> String {
    let out = run_git(repo, &["rev-parse", reff]);
    assert!(
        out.status.success(),
        "git rev-parse {reff} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn remote_refs(remote: &Path) -> String {
    let out = run_git(remote, &["for-each-ref", "--format=%(refname)"]);
    String::from_utf8_lossy(&out.stdout).to_string()
}

/// A repository at 1.2.3 on `main`, published to a bare remote, with the floating major tag `v1`
/// already pointing at the released commit — the state `cutver publish` closes from.
fn prepared_repo(name: &str) -> (FixtureGuard, Fixture, config::Config) {
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
    fixture.write("cutver.toml", TOML);
    init_git_repo(fixture);
    run_git_ok(&fixture.dir, &["checkout", "-B", "main"]);
    initial_commit(fixture);
    run_git_ok(&fixture.dir, &["remote", "add", "origin", remote.dir.to_str().unwrap()]);
    run_git_ok(&fixture.dir, &["push", "origin", "refs/heads/main"]);
    run_git_ok(&fixture.dir, &["tag", "-a", "v1", "-m", "v1"]);

    let cfg = config::load("cutver.toml").unwrap();
    (guard, remote, cfg)
}

fn publish_request(tag: &str, commit: Option<String>) -> PublishRequest {
    PublishRequest {
        tag: tag.to_string(),
        commit,
        notes_file: None,
        dry_run: false,
    }
}

#[test]
fn a_deferred_run_publishes_the_branch_and_leaves_the_tag_to_publish() {
    let (guard, remote, cfg) = prepared_repo("deferred-publish-prepare");
    let fixture = guard.fixture();
    let before = rev(&fixture.dir, "HEAD");

    let summary = run_deferred_publish(&cfg, Bump::Minor, false, &[], false).expect("the prepare phase succeeds");

    let head = rev(&fixture.dir, "HEAD");
    assert_ne!(head, before, "the prepare phase must create the release commit");
    assert_eq!(
        rev(&remote.dir, "refs/heads/main"),
        head,
        "the prepare phase must publish the release commit so the build can smoke it"
    );
    assert!(
        !remote_refs(&remote.dir).contains("refs/tags/"),
        "the tag must wait for the closing phase: {}",
        remote_refs(&remote.dir)
    );
    assert!(!tag_exists(fixture, "v1.3.0"), "no release tag is created yet");
    assert!(
        !fixture.dir.join("published.txt").exists(),
        "publish commands must wait for the closing phase"
    );
    assert!(summary.deferred, "the summary must report the deferral");
    assert!(
        summary.publish_commands.is_empty(),
        "a deferred run runs no publish command: {:?}",
        summary.publish_commands
    );
    assert_eq!(summary.tag, "v1.3.0");
    assert_eq!(summary.commit, head, "the summary must name the release commit");
}

#[test]
fn publish_creates_the_tag_pushes_it_and_runs_the_publish_commands() {
    let (guard, remote, cfg) = prepared_repo("deferred-publish-close");
    let fixture = guard.fixture();
    run_deferred_publish(&cfg, Bump::Minor, false, &[], false).expect("the prepare phase succeeds");
    let head = rev(&fixture.dir, "HEAD");

    let summary =
        run_publish(&cfg, &publish_request("v1.3.0", Some(head.clone()))).expect("the closing phase succeeds");

    assert_eq!(
        rev(&fixture.dir, "v1.3.0^{commit}"),
        head,
        "the closing phase must tag the release commit"
    );
    assert_eq!(
        rev(&remote.dir, "refs/tags/v1.3.0^{commit}"),
        head,
        "the release tag must reach the remote"
    );
    assert_eq!(
        rev(&remote.dir, "refs/tags/v1^{commit}"),
        head,
        "the floating major tag must move onto the release commit"
    );
    assert_eq!(
        fixture.read("published.txt").trim(),
        "published 1.3.0",
        "the publish commands must run with the release context"
    );
    assert_eq!(summary.commit, head);
    assert_eq!(summary.tag, "v1.3.0");
    assert_eq!(summary.floating_tag.as_deref(), Some("v1"));
    assert_eq!(summary.publish_commands, vec!["echo published 1.3.0 > published.txt"]);

    let command = summary
        .publish_push_command
        .as_deref()
        .expect("the summary must name the pushes it ran");
    assert_eq!(
        command,
        "git push origin refs/tags/v1.3.0 && git push origin +refs/tags/v1:refs/tags/v1"
    );
    let tag_push = command.find("refs/tags/v1.3.0").expect("the release tag push is named");
    let floating_push = command
        .find("+refs/tags/v1:refs/tags/v1")
        .expect("the forced floating refspec is named");
    assert!(
        tag_push < floating_push,
        "the release tag push must come before the floating refspec: {command}"
    );
}

#[test]
fn publish_refuses_a_tag_the_manifests_do_not_declare() {
    let (guard, remote, cfg) = prepared_repo("deferred-publish-mismatch");
    let fixture = guard.fixture();
    run_deferred_publish(&cfg, Bump::Minor, false, &[], false).expect("the prepare phase succeeds");
    let head = rev(&fixture.dir, "HEAD");

    let err = run_publish(&cfg, &publish_request("v9.9.9", Some(head))).expect_err("the tag is not declared");

    match &err {
        cutver::bump::Error::PublishTagMismatch { tag, declared } => {
            assert_eq!(tag, "v9.9.9");
            assert_eq!(declared, "v1.3.0", "the failure must name the declared tag");
        }
        other => panic!("expected a tag mismatch, got: {other:?}"),
    }
    let text = err.to_string();
    assert!(
        text.contains("Where:") && text.contains("Fix:") && text.contains("Nothing was written"),
        "the mismatch must be actionable: {text}"
    );
    assert!(!tag_exists(fixture, "v9.9.9"), "no tag may be created");
    assert!(
        !remote_refs(&remote.dir).contains("refs/tags/"),
        "nothing may be pushed: {}",
        remote_refs(&remote.dir)
    );
    assert!(
        !fixture.dir.join("published.txt").exists(),
        "no publish command may run"
    );
}
