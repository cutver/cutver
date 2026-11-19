//! The clean-tree guard's declared-input exemption, through its public API (#302, #326 S10).
//!
//! `cutver publish` declares two inputs it reads rather than changes: the `--notes-file` it was
//! handed, and the `[gates]` attestation directory. Both are exempt; nothing else is. These tests
//! drive `git::status::require_clean_tree_except` directly so the rule can be asserted for the shapes
//! git's porcelain really prints, including the two collapsed descriptions of an untracked tree:
//! `?? .cutver/` on a fresh checkout and `?? .cutver/attestations/` once the parent holds a tracked
//! file.

mod common;

use std::path::PathBuf;

use common::*;

/// A repository with one committed file, and nothing else.
fn repo(name: &str) -> FixtureGuard {
    assert!(git_available(), "git CLI is required for these tests");
    let guard = FixtureGuard::new(name);
    let fixture = guard.fixture();
    fixture.write("README.md", "the workspace\n");
    init_git_repo(fixture);
    initial_commit(fixture);
    guard
}

fn porcelain(fixture: &Fixture) -> String {
    let out = run_git(&fixture.dir, &["status", "--porcelain"]);
    String::from_utf8_lossy(&out.stdout)
        .replace("\r\n", "\n")
        .trim_end()
        .to_string()
}

fn check(fixture: &Fixture, declared: &[PathBuf]) -> Result<(), String> {
    cutver::git::status::require_clean_tree_except(&fixture.dir, true, declared).map_err(|error| error.to_string())
}

fn inside(fixture: &Fixture, relative: &str) -> PathBuf {
    fixture.dir.join(relative)
}

/// #302, preserved: one declared file is exempt, and a tracked change never is.
#[test]
fn a_declared_file_is_exempt_and_a_tracked_change_never_is() {
    let guard = repo("exempt-file");
    let fixture = guard.fixture();
    fixture.write("RELEASE_NOTES.md", "## 1.3.0\n");

    assert_eq!(porcelain(fixture), "?? RELEASE_NOTES.md");
    let notes = vec![inside(fixture, "RELEASE_NOTES.md")];
    assert!(check(fixture, &[]).is_err(), "undeclared, the file aborts the command");
    assert!(
        check(fixture, &notes).is_ok(),
        "a declared input is not a change in the tree"
    );

    // A tracked file is a change in the tree even when it is declared: the exemption is for an
    // input this invocation produced, not for edits to the release commit.
    run_git_ok(&fixture.dir, &["add", "RELEASE_NOTES.md"]);
    run_git_ok(&fixture.dir, &["commit", "-m", "track the notes", "-q"]);
    fixture.write("RELEASE_NOTES.md", "## 1.3.0\n\nedited\n");
    assert!(
        check(fixture, &notes).is_err(),
        "a modified tracked file must still abort"
    );
}

/// S10: a declared directory covers both shapes git collapses an untracked tree into.
#[test]
fn a_declared_directory_covers_both_collapsed_shapes() {
    let guard = repo("exempt-directory");
    let fixture = guard.fixture();
    fixture.write(".cutver/attestations/ci.json", "{\"gate\":\"ci\"}\n");
    let declared = vec![inside(fixture, ".cutver/attestations")];

    // The shape a fresh CI checkout produces: git reports the topmost untracked directory.
    assert_eq!(porcelain(fixture), "?? .cutver/");
    assert!(check(fixture, &[]).is_err(), "undeclared, the directory aborts");
    assert!(
        check(fixture, &declared).is_ok(),
        "the attestation directory is an input, not a change"
    );

    // The second shape: with a tracked file in `.cutver/`, git descends one level.
    fixture.write(".cutver/README.md", "what this directory holds\n");
    run_git_ok(&fixture.dir, &["add", ".cutver/README.md"]);
    run_git_ok(&fixture.dir, &["commit", "-m", "document the directory", "-q"]);
    assert_eq!(porcelain(fixture), "?? .cutver/attestations/");
    assert!(
        check(fixture, &declared).is_ok(),
        "the collapsed entry named by the declaration is exempt too"
    );
}

/// S10: the exemption is the directory alone. Anything else in the tree git collapsed into the same
/// entry, and any tracked change beneath it, still aborts.
#[test]
fn a_declared_directory_exempts_nothing_else() {
    let guard = repo("exempt-directory-negative");
    let fixture = guard.fixture();
    fixture.write(".cutver/attestations/ci.json", "{\"gate\":\"ci\"}\n");
    let declared = vec![inside(fixture, ".cutver/attestations")];

    fixture.write(".cutver/scratch.txt", "work in progress\n");
    let error = check(fixture, &declared).expect_err("a stray file keeps the entry undeclared");
    assert!(error.contains(".cutver"), "{error}");

    fixture.write(".cutver/scratch.txt", "");
    let scratch = inside(fixture, ".cutver/scratch.txt");
    std::fs::remove_file(&scratch).expect("the fixture removes its own scratch file");
    run_git_ok(&fixture.dir, &["add", ".cutver/attestations/ci.json"]);
    run_git_ok(&fixture.dir, &["commit", "-m", "track the attestation", "-q"]);
    fixture.write(".cutver/attestations/ci.json", "{\"gate\":\"ci\",\"tampered\":true}\n");
    let error = check(fixture, &declared).expect_err("a tracked change always aborts");
    assert!(error.contains("ci.json"), "{error}");
}

/// S10: a declaration that would cover the whole workspace, or that names a path outside it, exempts
/// nothing at all.
#[test]
fn a_dangerous_declaration_exempts_nothing() {
    let guard = repo("exempt-dangerous");
    let fixture = guard.fixture();
    fixture.write("wip.txt", "uncommitted work\n");

    let root = std::fs::canonicalize(&fixture.dir).expect("the fixture is canonicalizable");
    for dangerous in [root, inside(fixture, "."), inside(fixture, "wip.txt/..")] {
        assert!(
            check(fixture, std::slice::from_ref(&dangerous)).is_err(),
            "declaring {} must not exempt the worktree",
            dangerous.display()
        );
    }

    let outside = tempfile::tempdir().expect("a scratch directory");
    std::fs::write(outside.path().join("wip.txt"), "outside the workspace\n").expect("it is writable");
    assert!(
        check(fixture, &[outside.path().to_path_buf()]).is_err(),
        "a path outside the workspace exempts nothing"
    );
}

/// S10: an untracked attestation is exempt even when git lists it individually, which happens
/// whenever the declared directory also holds a tracked file.
#[test]
fn an_individual_untracked_attestation_inside_the_declared_directory_is_exempt() {
    let guard = repo("exempt-file-entry");
    let fixture = guard.fixture();
    fixture.write(".cutver/attestations/.gitkeep", "");
    run_git_ok(&fixture.dir, &["add", ".cutver/attestations/.gitkeep"]);
    run_git_ok(&fixture.dir, &["commit", "-m", "keep the directory", "-q"]);
    fixture.write(
        ".cutver/attestations/ci.json",
        "{\"gate\":\"ci\"}
",
    );
    let declared = vec![inside(fixture, ".cutver/attestations")];

    assert_eq!(porcelain(fixture), "?? .cutver/attestations/ci.json");
    assert!(
        check(fixture, &declared).is_ok(),
        "an attestation is an input, not a change"
    );
    // Still the directory alone: a file beside it, in the same tracked parent, is not exempt.
    fixture.write(
        "wip.txt",
        "uncommitted work
",
    );
    assert!(
        check(fixture, &declared).is_err(),
        "only the declared directory is exempt"
    );
}

/// S10: a symlink inside the declared directory is not followed, so the exemption cannot be used to
/// reach a path the declaration never named.
#[cfg(unix)]
#[test]
fn a_symlink_inside_the_declared_directory_is_not_followed() {
    let guard = repo("exempt-symlink");
    let fixture = guard.fixture();
    fixture.write(
        ".cutver/attestations/ci.json",
        "{\"gate\":\"ci\"}
",
    );
    let outside = tempfile::tempdir().expect("a scratch directory");
    std::fs::write(
        outside.path().join("secret"),
        "outside
",
    )
    .expect("it is writable");
    std::os::unix::fs::symlink(
        outside.path().join("secret"),
        fixture.dir.join(".cutver/attestations/link"),
    )
    .expect("the fixture creates a symlink");

    assert!(check(fixture, &[inside(fixture, ".cutver/attestations")]).is_err());
}

/// S10/D1: both shapes agree on symlinks. A link inside the declared directory is refused even when
/// git lists it individually, because a link is a path the declaration never named — and the
/// collapsed shape already refuses it. A link whose target is *inside* the directory is the case the
/// two shapes disagreed on; it escapes nothing, but it is not a file the declaration covers either.
#[cfg(unix)]
#[test]
fn a_symlink_inside_the_declared_directory_is_refused_when_listed_individually() {
    let guard = repo("exempt-symlink-listed");
    let fixture = guard.fixture();
    fixture.write(".cutver/attestations/.gitkeep", "");
    run_git_ok(&fixture.dir, &["add", ".cutver/attestations/.gitkeep"]);
    run_git_ok(&fixture.dir, &["commit", "-m", "keep the directory", "-q"]);
    fixture.write(
        ".cutver/attestations/ci.json",
        "{\"gate\":\"ci\"}
",
    );
    std::os::unix::fs::symlink(
        fixture.dir.join(".cutver/attestations/ci.json"),
        fixture.dir.join(".cutver/attestations/inlink.txt"),
    )
    .expect("the fixture creates a symlink");
    let declared = vec![inside(fixture, ".cutver/attestations")];

    assert_eq!(
        porcelain(fixture).lines().collect::<Vec<_>>(),
        vec!["?? .cutver/attestations/ci.json", "?? .cutver/attestations/inlink.txt"],
        "the tracked `.gitkeep` is what makes git list the tree file by file"
    );
    let error = check(fixture, &declared).expect_err("a link inside the directory fails closed");
    assert!(
        error.contains("inlink.txt"),
        "the refusal names the link, not the attestation beside it: {error}"
    );
}
