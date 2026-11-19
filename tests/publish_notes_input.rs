//! The one file `cutver publish --notes-file` may name, end to end (#302, #306).
//!
//! The notes are the closing phase's own input, so the clean-tree guard exempts them — and since
//! #326 S10 the exemption is decided over directories as well, which makes the *shape* of the
//! declared path the request's business: a path that names a directory is refused before the guard
//! runs, with the release untouched. A notes file keeps being exempt exactly as it was.

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

[changelog]
path = "CHANGELOG.md"

[git]
tag_prefix = "v"
require_clean_tree = true

[publish]
push = true
commands = ["echo published {version} > published.txt"]
"#;

/// A repository at 1.2.3 on `main`, pushed to a bare remote.
fn prepared_repo(name: &str) -> (FixtureGuard, Fixture) {
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
    fixture.write(".gitignore", "published.txt\n");
    fixture.write("cutver.toml", TOML);
    fixture.write("docs/README.md", "the docs directory\n");
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

fn remote_refs(remote: &Path) -> String {
    String::from_utf8_lossy(&run_git(remote, &["for-each-ref", "--format=%(refname)"]).stdout).to_string()
}

fn porcelain(fixture: &Fixture) -> String {
    String::from_utf8_lossy(&run_git(&fixture.dir, &["status", "--porcelain"]).stdout)
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
    rev(&fixture.dir, "HEAD")
}

fn publish(fixture: &Fixture, commit: &str, notes: &str) -> Output {
    cutver(
        fixture,
        &["publish", "--tag", "v1.3.0", "--commit", commit, "--notes-file", notes],
    )
}

fn exists(fixture: &Fixture, relative: &str) -> bool {
    fixture.dir.join(relative).exists()
}

/// `--notes-file` names a file. A path that names a directory is refused before the clean-tree guard
/// runs, because the guard exempts a declared directory from the tree and a directory can never be
/// notes (#302, #326 S10).
#[test]
fn a_notes_file_that_names_a_directory_is_refused_before_the_guard() {
    let (guard, remote) = prepared_repo("notes-directory");
    let fixture = guard.fixture();
    let head = prepare(fixture);
    // A change the guard would report, so the refusal is provably reached first: tracked content is
    // never hidden by git's untracked cache.
    fixture.write("docs/README.md", "the docs directory, edited\n");
    let status_before = porcelain(fixture);
    assert_eq!(
        status_before, " M docs/README.md",
        "the dirty tree the clean-tree guard would refuse is the state this test starts from"
    );

    let out = publish(fixture, &head, "docs");

    assert_eq!(out.status.code(), Some(1), "stderr:\n{}", stderr_of(&out));
    let message = stderr_of(&out);
    for expected in [
        "names a directory",
        "'docs'",
        "--notes-file",
        "Nothing was tagged, pushed or executed",
    ] {
        assert!(message.contains(expected), "{message} does not name {expected}");
    }
    assert!(
        !message.contains("uncommitted changes"),
        "the refusal precedes the clean-tree guard, which would have named the tree: {message}"
    );
    assert!(!tag_exists(fixture, "v1.3.0"), "no tag may be created locally");
    assert!(
        !remote_refs(&remote.dir).contains("refs/tags/"),
        "no tag may reach the remote"
    );
    assert!(!exists(fixture, "published.txt"), "no publish command may run");
    assert_eq!(porcelain(fixture), status_before, "the worktree is unchanged");
}

/// #302, preserved: the rendered notes file is the run's own input, so the guard exempts it and the
/// closing phase publishes. The file must survive byte for byte.
#[test]
fn a_notes_file_that_names_a_file_is_still_exempt_and_publishes() {
    let (guard, remote) = prepared_repo("notes-file-input");
    let fixture = guard.fixture();
    let head = prepare(fixture);
    let notes = "# v1.3.0\n\n- accepted\n";
    fixture.write("RELEASE_NOTES.md", notes);
    assert_eq!(porcelain(fixture), "?? RELEASE_NOTES.md");

    let out = publish(fixture, &head, "RELEASE_NOTES.md");

    assert!(out.status.success(), "stderr:\n{}", stderr_of(&out));
    assert_eq!(
        rev(&fixture.dir, "v1.3.0^{commit}"),
        head,
        "the closing phase tags the release commit"
    );
    assert_eq!(rev(&remote.dir, "refs/tags/v1.3.0^{commit}"), head);
    assert_eq!(fixture.read("published.txt").trim(), "published 1.3.0");
    assert_eq!(fixture.read("RELEASE_NOTES.md"), notes, "the input is untouched");
}

/// The exemption is the notes file, not the tree around it: another untracked file still aborts, so
/// the directory refusal above cannot be confused with a general looseness (#302, preserved).
#[test]
fn a_second_untracked_file_still_aborts_a_publish_that_declares_its_notes() {
    let (guard, _remote) = prepared_repo("notes-file-plus-one");
    let fixture = guard.fixture();
    let head = prepare(fixture);
    fixture.write("RELEASE_NOTES.md", "# v1.3.0\n");
    fixture.write("scratch.txt", "not declared\n");

    let out = publish(fixture, &head, "RELEASE_NOTES.md");

    assert_eq!(out.status.code(), Some(1), "stderr:\n{}", stderr_of(&out));
    let message = stderr_of(&out);
    assert!(message.contains("uncommitted changes"), "{message}");
    assert!(message.contains("scratch.txt"), "{message}");
    assert!(!tag_exists(fixture, "v1.3.0"), "no tag may be created locally");
}
