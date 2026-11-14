//! #262: the remote tag preflight must fail closed when it cannot reach the remote.
//!
//! A failed `git ls-remote` used to read as "no tag on the remote": the release wrote its
//! manifests, committed and tagged locally, and only `git push` failed, leaving a commit and a tag
//! behind for the operator to unwind by hand.

mod common;

use common::*;
use cutver::bump::run as bump_run;
use cutver::semver_bump::Bump;

/// Every non-git file under the fixture, so "nothing was written" is checkable byte for byte.
fn snapshot(fixture: &Fixture) -> Vec<(String, Vec<u8>)> {
    let mut entries = Vec::new();
    let mut stack = vec![fixture.dir.clone()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.file_name().is_some_and(|name| name == ".git") {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let rel = path
                .strip_prefix(&fixture.dir)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            entries.push((rel, std::fs::read(&path).unwrap()));
        }
    }
    entries.sort();
    entries
}

/// #262 AC1 + AC3 + AC4: a configured but unreachable remote is a typed error, and the aborted run
/// leaves the workspace byte-identical.
#[test]
fn an_unreachable_remote_aborts_before_any_write() {
    assert!(
        git_available(),
        "git CLI is required for e2e tests but was not found in PATH"
    );
    let guard = FixtureGuard::new("remote-unreachable");
    write_fixture(
        &guard,
        r#"[preflight]
check = "true""#,
    );
    let fixture = guard.fixture();
    let unreachable = fixture
        .dir
        .join("missing-remote.git")
        .to_string_lossy()
        .replace('\\', "/");
    run_git_ok(&fixture.dir, &["remote", "add", "origin", &unreachable]);
    let before = snapshot(fixture);

    let cfg = load_config(fixture);
    let err = bump_run(&cfg, Bump::Minor, false, &[]).unwrap_err();

    assert!(
        matches!(err, cutver::bump::Error::RemoteTagCheck { .. }),
        "expected a typed remote check error, got {err:?}"
    );
    let message = err.to_string();
    assert!(message.contains("origin"), "the error must name the remote: {message}");
    assert!(
        message.contains("does not appear to be a git repository"),
        "the error must quote git's stderr: {message}"
    );
    assert!(
        message.contains("git ls-remote"),
        "the error must suggest the next step: {message}"
    );
    assert_eq!(
        snapshot(fixture),
        before,
        "the aborted run must leave the workspace byte-identical"
    );
    assert_eq!(commit_count(fixture), 1, "no release commit may be created");
    assert!(!tag_exists(fixture, "v1.3.0"), "no tag may be created");
}

/// #262 AC2: no remote configured is not an error — the check answers "no remote tag" and the bump
/// runs exactly as it does today.
#[test]
fn a_repository_without_a_remote_still_bumps() {
    assert!(
        git_available(),
        "git CLI is required for e2e tests but was not found in PATH"
    );
    let guard = FixtureGuard::new("remote-absent");
    write_fixture(
        &guard,
        r#"[preflight]
check = "true""#,
    );
    let fixture = guard.fixture();

    let cfg = load_config(fixture);
    bump_run(&cfg, Bump::Minor, false, &[]).expect("a repository without a remote must still bump");

    assert_bumped(fixture);
    assert_eq!(commit_count(fixture), 2, "the release commit must exist");
    assert!(tag_exists(fixture, "v1.3.0"), "the release tag must exist");
}
