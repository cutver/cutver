//! Two-phase release: `bump --defer-publish` prepares, `cutver publish` closes (issue #259).
//!
//! A release runs in two invocations so the irreversible steps can wait for verification: the
//! deferred bump commits, pushes the branch and stops; `cutver publish` then creates the tag, pushes
//! it, moves the floating major tag and runs `[publish].commands` — but only when the tag matches
//! what the manifests at HEAD declare.

mod common;

use std::path::Path;
use std::process::Command;

use common::*;
use cutver::bump::{PublishRequest, run as bump_run, run_deferred_publish, run_publish};
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
    prepared_repo_with(name, TOML)
}

/// The same prepared state with its own `cutver.toml`, so a test can configure the
/// `[publish].commands` line it needs instead of sharing the fixture's one command.
fn prepared_repo_with(name: &str, toml: &str) -> (FixtureGuard, Fixture, config::Config) {
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
    fixture.write("cutver.toml", toml);
    init_git_repo(fixture);
    run_git_ok(&fixture.dir, &["checkout", "-B", "main"]);
    initial_commit(fixture);
    run_git_ok(&fixture.dir, &["remote", "add", "origin", remote.dir.to_str().unwrap()]);
    run_git_ok(&fixture.dir, &["push", "origin", "refs/heads/main"]);
    run_git_ok(&fixture.dir, &["tag", "-a", "v1", "-m", "v1"]);

    let cfg = common::load_config(guard.fixture());
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

/// The invocation the release workflow runs, from inside the worktree: `--notes-file` is spelled the
/// way the job spells it, relative to the repository it runs in (#302).
fn publish_from_worktree(fixture: &Fixture, notes: &str, commit: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_cutver"))
        .current_dir(&fixture.dir)
        .args(["publish", "--tag", "v1.3.0", "--commit", commit, "--notes-file", notes])
        .output()
        .expect("failed to execute cutver publish")
}

/// The streamed invocation (#306): `--notes-file -` hands the notes over stdin, so nothing transient
/// has to exist in the worktree the clean-tree guard is checking.
fn publish_from_stdin(fixture: &Fixture, notes: &str, commit: &str) -> std::process::Output {
    use std::io::Write;
    use std::process::Stdio;
    let mut child = Command::new(env!("CARGO_BIN_EXE_cutver"))
        .current_dir(&fixture.dir)
        .args(["publish", "--tag", "v1.3.0", "--commit", commit, "--notes-file", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to execute cutver publish");
    child
        .stdin
        .as_mut()
        .expect("stdin must be piped to the child")
        .write_all(notes.as_bytes())
        .expect("failed to write the notes to the child");
    child.wait_with_output().expect("failed to wait for cutver publish")
}

/// What `git status --porcelain` reports, newline-normalised and without the trailing newline.
fn porcelain(fixture: &Fixture) -> String {
    let out = run_git(&fixture.dir, &["status", "--porcelain"]);
    String::from_utf8_lossy(&out.stdout)
        .replace("\r\n", "\n")
        .trim_end()
        .to_string()
}

/// The baseline order, restored for issue #259's review (finding R4-001): the release tag is created
/// before the branch is pushed, so a tag that cannot be created never leaves the release commit on
/// the remote without one.
#[test]
fn an_inline_run_creates_the_tag_before_it_pushes_the_branch() {
    let (guard, remote, cfg) = prepared_repo("deferred-publish-inline-order");
    let fixture = guard.fixture();
    let before = rev(&fixture.dir, "HEAD");
    // A ref under the release tag's own path: `git tag -a v1.3.0` cannot lock a ref whose child
    // already exists, and `check_tag_collisions` does not see it because `v1.3.0` itself does not
    // exist. That makes tag creation fail after the release commit, which is the case the order
    // has to survive.
    run_git_ok(&fixture.dir, &["tag", "v1.3.0/blocker"]);

    let err = bump_run(&cfg, Bump::Minor, false, &[]).expect_err("the release tag cannot be created");

    let head = rev(&fixture.dir, "HEAD");
    match &err {
        cutver::bump::Error::Tag { tag, commit, .. } => {
            assert_eq!(tag, "v1.3.0");
            assert_eq!(commit, &head, "the failure must name the release commit");
        }
        other => panic!("expected a tag failure, got: {other:?}"),
    }
    assert_ne!(head, before, "the release commit must have been created");
    assert!(
        err.to_string().contains("nothing was pushed"),
        "an inline tag failure keeps its original wording, which only holds while the tag comes \
         before every push: {err}"
    );
    assert_eq!(
        rev(&remote.dir, "refs/heads/main"),
        before,
        "the branch must not be pushed when the tag could not be created"
    );
    assert!(!tag_exists(fixture, "v1.3.0"), "no tag may be created");
    assert!(
        !fixture.dir.join("published.txt").exists(),
        "no publish command may run"
    );
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

/// `{{ previous_version }}` must render the same value in both entry points: the inline phase derives
/// it from the manifests it bumped, the closing phase from the previous release tag. Rendering the
/// new version there broke the one-authority invariant the two-phase split is built on (#259). The
/// token is the Jinja field `[publish].commands` interpolate against, not the legacy `{version}`
/// form, which `normalize_legacy_tokens` only accepts for `version` and `tag`.
#[test]
fn publish_renders_the_previous_version_from_the_previous_tag() {
    let toml = TOML.replace(
        r#"commands = ["echo published {version} > published.txt"]"#,
        r#"commands = ["echo {{ previous_version }} > previous.txt"]"#,
    );
    let (guard, _remote, cfg) = prepared_repo_with("deferred-publish-previous-version", &toml);
    let fixture = guard.fixture();
    // The shared fixture cuts no release tag, so this test cuts the previous release itself: `v1.2.3`
    // is the name of the released 1.2.3 state, and the closing phase has to read it back.
    run_git_ok(&fixture.dir, &["tag", "-a", "v1.2.3", "-m", "v1.2.3"]);
    run_deferred_publish(&cfg, Bump::Minor, false, &[], false).expect("the prepare phase succeeds");
    let head = rev(&fixture.dir, "HEAD");

    run_publish(&cfg, &publish_request("v1.3.0", Some(head))).expect("the closing phase succeeds");

    let rendered = fixture.read("previous.txt");
    assert_eq!(
        rendered.trim(),
        "1.2.3",
        "the closing phase must render the version the previous tag names, not the one it publishes"
    );
    assert!(
        !rendered.contains("1.3.0"),
        "the closing phase must not report the new version as the previous one: {rendered}"
    );
}

#[test]
fn publish_refuses_a_dirty_working_tree() {
    let (guard, remote, cfg) = prepared_repo("deferred-publish-dirty-tree");
    let fixture = guard.fixture();
    run_deferred_publish(&cfg, Bump::Minor, false, &[], false).expect("the prepare phase succeeds");
    let head = rev(&fixture.dir, "HEAD");
    // An uncommitted manifest edit: the declared version is read from the working tree, so the tree
    // must be the one the tag names before anything is tagged, pushed or published.
    fixture.write("package.json", &fixture.read("package.json").replace("1.3.0", "1.9.9"));

    let err = run_publish(&cfg, &publish_request("v1.3.0", Some(head)))
        .expect_err("a working tree that is not the tagged one must be refused");

    match &err {
        cutver::bump::Error::Git(cutver::git::Error::Guard { detail }) => {
            assert!(
                detail.contains("package.json"),
                "the guard must name the uncommitted file: {detail}"
            );
        }
        other => panic!("expected the clean-tree guard, got: {other:?}"),
    }
    assert!(!tag_exists(fixture, "v1.3.0"), "no tag may be created");
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

/// The release workflow renders the notes inside the checkout and hands that file to
/// `cutver publish` (#302). Refusing it aborted release run 43564101364 before anything was written:
/// the command rejected its own declared input.
#[test]
fn publish_accepts_the_notes_file_the_workflow_hands_it() {
    let (guard, remote, cfg) = prepared_repo("deferred-publish-notes-file");
    let fixture = guard.fixture();
    run_deferred_publish(&cfg, Bump::Minor, false, &[], false).expect("the prepare phase succeeds");
    let head = rev(&fixture.dir, "HEAD");
    // Exactly the state the release workflow leaves: the rendered notes are a new file in the
    // checkout that nothing has committed.
    fixture.write("RELEASE_NOTES.md", "# v1.3.0\n\n- accepted\n");
    assert_eq!(
        porcelain(fixture),
        "?? RELEASE_NOTES.md",
        "the notes file must be the untracked input this test is about"
    );

    let out = publish_from_worktree(fixture, "RELEASE_NOTES.md", &head);

    assert!(
        out.status.success(),
        "the declared notes file is this run's own input, so the guard must accept it; stderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
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
        fixture.read("published.txt").trim(),
        "published 1.3.0",
        "the publish commands must run"
    );
    assert_eq!(
        fixture.read("RELEASE_NOTES.md"),
        "# v1.3.0\n\n- accepted\n",
        "the guard must not touch the input it accepted"
    );
}

/// The same file spelled several ways: the exemption is decided on the resolved path, not on the
/// string the caller typed, so `./RELEASE_NOTES.md` and an absolute path are exempt as well (#302).
#[test]
fn publish_accepts_a_dot_prefixed_and_an_absolute_spelling_of_the_notes_file() {
    for case in ["dot-prefixed", "absolute"] {
        let (guard, _remote, cfg) = prepared_repo(&format!("deferred-publish-notes-spelling-{case}"));
        let fixture = guard.fixture();
        run_deferred_publish(&cfg, Bump::Minor, false, &[], false).expect("the prepare phase succeeds");
        let head = rev(&fixture.dir, "HEAD");
        fixture.write("RELEASE_NOTES.md", "# v1.3.0\n");
        let notes = if case == "absolute" {
            fixture.dir.join("RELEASE_NOTES.md").to_string_lossy().to_string()
        } else {
            "./RELEASE_NOTES.md".to_string()
        };

        let out = publish_from_worktree(fixture, &notes, &head);

        assert!(
            out.status.success(),
            "`{notes}` names the declared notes file, so it must be exempt; stderr:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(
            rev(&fixture.dir, "v1.3.0^{commit}"),
            head,
            "`{notes}` must close the release, not just pass the guard"
        );
    }
}

/// A path is a file, not a string. The notes are reached through a tracked symlinked directory, so
/// git reports `release/RELEASE_NOTES.md` while the caller typed `notes/RELEASE_NOTES.md`, and only
/// a canonical comparison accepts the same file under both spellings (#302).
#[cfg(unix)]
#[test]
fn publish_accepts_a_notes_file_reached_through_a_symlinked_directory() {
    let (guard, _remote, cfg) = prepared_repo("deferred-publish-notes-symlink");
    let fixture = guard.fixture();
    fixture.write("release/.gitkeep", "");
    std::os::unix::fs::symlink("release", fixture.dir.join("notes")).expect("symlink the release directory");
    run_git_ok(&fixture.dir, &["add", "release/.gitkeep", "notes"]);
    run_git_ok(
        &fixture.dir,
        &[
            "commit",
            "-q",
            "-m",
            "chore: keep the rendered notes in a linked directory",
        ],
    );
    run_deferred_publish(&cfg, Bump::Minor, false, &[], false).expect("the prepare phase succeeds");
    let head = rev(&fixture.dir, "HEAD");
    fixture.write("release/RELEASE_NOTES.md", "# v1.3.0\n");
    assert_eq!(
        porcelain(fixture),
        "?? release/RELEASE_NOTES.md",
        "git must report the real path, not the symlinked spelling"
    );

    let out = publish_from_worktree(fixture, "notes/RELEASE_NOTES.md", &head);

    assert!(
        out.status.success(),
        "the symlinked spelling names the declared notes file; stderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        rev(&fixture.dir, "v1.3.0^{commit}"),
        head,
        "the closing phase must tag the release commit"
    );
}

/// Everything else stays strict: the exemption is the declared notes file alone, and the failure
/// names the untracked entry nobody declared.
#[test]
fn publish_still_refuses_a_second_untracked_file_when_a_notes_file_is_declared() {
    let (guard, remote, cfg) = prepared_repo("deferred-publish-notes-file-plus-one");
    let fixture = guard.fixture();
    run_deferred_publish(&cfg, Bump::Minor, false, &[], false).expect("the prepare phase succeeds");
    let head = rev(&fixture.dir, "HEAD");
    fixture.write("RELEASE_NOTES.md", "# v1.3.0\n");
    fixture.write("leftover.txt", "scratch\n");

    let out = publish_from_worktree(fixture, "RELEASE_NOTES.md", &head);

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !out.status.success(),
        "an undeclared untracked file must still abort; stdout:\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(
        stderr.contains("leftover.txt"),
        "the guard must name the file that was never declared:\n{stderr}"
    );
    assert!(
        !stderr.contains("?? RELEASE_NOTES.md"),
        "the declared notes file is not an error and must not be listed:\n{stderr}"
    );
    assert!(!tag_exists(fixture, "v1.3.0"), "no tag may be created");
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

/// Only an untracked entry can be exempted. Naming a tracked file as the notes input excuses no
/// change to it: the tree must still be the one the tag names.
#[test]
fn publish_still_refuses_a_modified_tracked_file_named_as_its_notes_input() {
    let (guard, remote, cfg) = prepared_repo("deferred-publish-notes-file-tracked");
    let fixture = guard.fixture();
    run_deferred_publish(&cfg, Bump::Minor, false, &[], false).expect("the prepare phase succeeds");
    let head = rev(&fixture.dir, "HEAD");
    fixture.write(
        "CHANGELOG.md",
        &format!("{}\n## Unreleased\n\n- edited by hand\n", fixture.read("CHANGELOG.md")),
    );
    assert_eq!(
        porcelain(fixture),
        " M CHANGELOG.md",
        "the notes input must be the modified tracked file this test is about"
    );

    let out = publish_from_worktree(fixture, "CHANGELOG.md", &head);

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !out.status.success(),
        "a modified tracked file must still abort; stdout:\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(
        stderr.contains("git guard failed") && stderr.contains("CHANGELOG.md"),
        "the guard must name the modified tracked file:\n{stderr}"
    );
    assert!(!tag_exists(fixture, "v1.3.0"), "no tag may be created");
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

#[test]
fn publish_refuses_a_commit_that_is_not_head() {
    let (guard, remote, cfg) = prepared_repo("deferred-publish-stale-commit");
    let fixture = guard.fixture();
    let verified = rev(&fixture.dir, "HEAD");
    run_deferred_publish(&cfg, Bump::Minor, false, &[], false).expect("the prepare phase succeeds");
    let head = rev(&fixture.dir, "HEAD");
    assert_ne!(head, verified, "the prepare phase must move HEAD");

    let err = run_publish(&cfg, &publish_request("v1.3.0", Some(verified.clone())))
        .expect_err("a commit the caller verified earlier is not HEAD any more");

    match &err {
        cutver::bump::Error::PublishCommitMismatch { expected, actual } => {
            assert_eq!(expected, &verified, "the failure must name the requested commit");
            assert_eq!(actual, &head, "the failure must name the actual HEAD");
        }
        other => panic!("expected a commit mismatch, got: {other:?}"),
    }
    let text = err.to_string();
    assert!(
        text.contains("Where:") && text.contains("Fix:") && text.contains("Nothing was written"),
        "the mismatch must be actionable: {text}"
    );
    assert!(!tag_exists(fixture, "v1.3.0"), "no tag may be created");
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

#[test]
fn publish_refuses_a_tag_that_already_exists_locally() {
    let (guard, remote, cfg) = prepared_repo("deferred-publish-local-tag");
    let fixture = guard.fixture();
    let verified = rev(&fixture.dir, "HEAD");
    run_deferred_publish(&cfg, Bump::Minor, false, &[], false).expect("the prepare phase succeeds");
    // A release tag cut by hand on an earlier commit: the retry must not move it, and must not
    // report a remote collision it never checked.
    run_git_ok(&fixture.dir, &["tag", "-a", "v1.3.0", "-m", "already cut", &verified]);

    let err = run_publish(&cfg, &publish_request("v1.3.0", None)).expect_err("the tag exists locally");

    match &err {
        cutver::bump::Error::TagExists { tag, commit } => {
            assert_eq!(tag, "v1.3.0");
            assert_eq!(commit, &verified, "the failure must name the commit the tag points at");
        }
        other => panic!("expected an existing tag, got: {other:?}"),
    }
    assert!(
        !remote_refs(&remote.dir).contains("refs/tags/"),
        "an existing local tag must fail before any push: {}",
        remote_refs(&remote.dir)
    );
    assert!(
        !fixture.dir.join("published.txt").exists(),
        "no publish command may run"
    );
}

#[test]
fn publish_refuses_a_tag_that_only_exists_remotely() {
    let (guard, remote, cfg) = prepared_repo("deferred-publish-remote-tag");
    let fixture = guard.fixture();
    run_deferred_publish(&cfg, Bump::Minor, false, &[], false).expect("the prepare phase succeeds");
    let head = rev(&fixture.dir, "HEAD");
    // The tag a previous, half-finished run pushed before it failed: it is not in this clone, so
    // only the remote check can see it, and a local-only check would tag over it. A plain ref
    // rather than `git tag -a`: the bare remote has no identity, and this test must not depend on
    // whatever `tag.gpgsign` the ambient git config asks for.
    run_git_ok(&remote.dir, &["update-ref", "refs/tags/v1.3.0", &head]);

    let err = run_publish(&cfg, &publish_request("v1.3.0", None)).expect_err("the tag exists remotely");

    match &err {
        cutver::bump::Error::RemoteTagExists {
            tag,
            remote: name,
            commit,
        } => {
            assert_eq!(tag, "v1.3.0");
            assert_eq!(name, "origin");
            assert_eq!(commit, &head, "the failure must name the remote commit");
        }
        other => panic!("expected an existing remote tag, got: {other:?}"),
    }
    assert!(!tag_exists(fixture, "v1.3.0"), "no local tag may be created");
    assert!(
        !fixture.dir.join("published.txt").exists(),
        "no publish command may run"
    );
}

#[test]
fn a_publish_dry_run_writes_nothing() {
    let (guard, remote, cfg) = prepared_repo("deferred-publish-dry-run");
    let fixture = guard.fixture();
    run_deferred_publish(&cfg, Bump::Minor, false, &[], false).expect("the prepare phase succeeds");
    let head = rev(&fixture.dir, "HEAD");
    let floating_before = rev(&fixture.dir, "v1^{commit}");

    let request = PublishRequest {
        dry_run: true,
        ..publish_request("v1.3.0", Some(head.clone()))
    };
    let summary = run_publish(&cfg, &request).expect("a valid plan passes the dry run");

    assert!(summary.dry_run, "the summary must report a dry run");
    assert_eq!(
        summary.publish_commands,
        vec!["echo published 1.3.0 > published.txt"],
        "the plan must name the commands it would run"
    );
    assert!(!tag_exists(fixture, "v1.3.0"), "a dry run must not create the tag");
    assert_eq!(
        rev(&fixture.dir, "v1^{commit}"),
        floating_before,
        "a dry run must not move the floating major tag"
    );
    assert!(
        !remote_refs(&remote.dir).contains("refs/tags/"),
        "a dry run must push nothing: {}",
        remote_refs(&remote.dir)
    );
    assert!(
        !fixture.dir.join("published.txt").exists(),
        "a dry run must not run the publish commands"
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

/// #306: the notes travel as a stream, so the invocation that carries them declares no worktree
/// input at all — and leaves none behind.
#[test]
fn publish_takes_its_notes_from_a_stream_and_leaves_no_file_behind() {
    let (guard, remote, cfg) = prepared_repo("deferred-publish-notes-stream");
    let fixture = guard.fixture();
    run_deferred_publish(&cfg, Bump::Minor, false, &[], false).expect("the prepare phase succeeds");
    let head = rev(&fixture.dir, "HEAD");
    assert_eq!(
        porcelain(fixture),
        "",
        "nothing may be untracked before the streamed run"
    );

    let out = publish_from_stdin(fixture, "# v1.3.0\n\n- streamed\n", &head);

    assert!(
        out.status.success(),
        "a non-empty stream is this run's notes input; stderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
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
        fixture.read("published.txt").trim(),
        "published 1.3.0",
        "the publish commands must run"
    );
    assert_eq!(
        porcelain(fixture),
        "?? published.txt",
        "a streamed run must not create a notes file in the tree"
    );
}

/// An empty stream must fail closed before the tag, the pushes and the publish commands: the
/// alternative is a published release with no notes.
#[test]
fn publish_refuses_an_empty_notes_stream_before_it_writes_anything() {
    let (guard, remote, cfg) = prepared_repo("deferred-publish-notes-stream-empty");
    let fixture = guard.fixture();
    run_deferred_publish(&cfg, Bump::Minor, false, &[], false).expect("the prepare phase succeeds");
    let head = rev(&fixture.dir, "HEAD");

    let out = publish_from_stdin(fixture, "", &head);

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !out.status.success(),
        "an empty stream must not publish; stdout:\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(
        stderr.contains("release notes on stdin were empty"),
        "the refusal must name the empty stream; stderr:\n{stderr}"
    );
    let tags = run_git(&fixture.dir, &["tag", "-l", "v1.3.0"]);
    assert!(
        String::from_utf8_lossy(&tags.stdout).trim().is_empty(),
        "no release tag may be created"
    );
    assert!(
        !remote_refs(&remote.dir).contains("refs/tags/v1.3.0"),
        "the release tag must not reach the remote"
    );
    assert!(
        !fixture.dir.join("published.txt").exists(),
        "no publish command may run"
    );
}

/// Whitespace is not notes: a stream of blanks is empty for this rule.
#[test]
fn publish_refuses_a_whitespace_only_notes_stream() {
    let (guard, _remote, cfg) = prepared_repo("deferred-publish-notes-stream-blank");
    let fixture = guard.fixture();
    run_deferred_publish(&cfg, Bump::Minor, false, &[], false).expect("the prepare phase succeeds");
    let head = rev(&fixture.dir, "HEAD");

    let out = publish_from_stdin(fixture, "   \n\n", &head);

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "a blank stream must not publish");
    assert!(
        stderr.contains("release notes on stdin were empty"),
        "a blank stream is an empty stream; stderr:\n{stderr}"
    );
}

/// The streamed path exempts nothing: another untracked file still aborts, exactly as it does when
/// the notes come from a declared file.
#[test]
fn publish_still_refuses_a_stray_untracked_file_when_the_notes_come_from_stdin() {
    let (guard, _remote, cfg) = prepared_repo("deferred-publish-notes-stream-stray");
    let fixture = guard.fixture();
    run_deferred_publish(&cfg, Bump::Minor, false, &[], false).expect("the prepare phase succeeds");
    let head = rev(&fixture.dir, "HEAD");
    fixture.write("stray.txt", "not this run's input\n");

    let out = publish_from_stdin(fixture, "# v1.3.0\n", &head);

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "a stray untracked file must still abort");
    assert!(
        stderr.contains("uncommitted changes") && stderr.contains("stray.txt"),
        "the refusal must name the change it found; stderr:\n{stderr}"
    );
}
