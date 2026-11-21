//! The changelog sink's half of the fail-closed transaction (#289): an absent declared changelog is
//! a normal state the run creates, and rollback then removes it. Specs S1, S4 and S6 in
//! `odd/tasks/bump-changelog-created-rollback.md`.
//!
//! This suite is deliberately *not* part of `tests/e2e_lifecycle.rs`: that file is `#![cfg(unix)]`
//! and therefore runs zero tests on Windows, while the changelog sink has to behave identically on
//! all three hosts (S7). Everything here runs on linux, macOS and Windows.

mod common;

use common::*;
use cutver::bump::run as bump_run;
use cutver::semver_bump::Bump;

/// A `cutver.toml` that declares `CHANGELOG.md` and nothing else unusual, plus `extra` appended.
///
/// The changelog is *declared* and absent in every case below, which is the state the issue is
/// about: a config without `[changelog] path` declares no changelog and never reaches the sink.
fn write_config(fixture: &Fixture, extra: &str) {
    fixture.write(
        "cutver.toml",
        &format!(
            r#"[version]
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[changelog]
path = "CHANGELOG.md"

[git]
require_clean_tree = true

{extra}
"#
        ),
    );
}

/// The fixture every case starts from: one version source and whatever else the case adds.
fn write_fixture(fixture: &Fixture, extra: &str) {
    fixture.write("package.json", PACKAGE_JSON);
    write_config(fixture, extra);
}

/// Commits the fixture, so `[git] require_clean_tree` holds for every case.
fn commit_fixture(fixture: &Fixture) {
    init_git_repo(fixture);
    initial_commit(fixture);
}

/// S4: a run that creates the changelog removes it again when a *later* step fails.
///
/// The two assertions are ordered and both matter. The first one says the run reached
/// `[hooks] post_bump`, which is only possible if an absent declared changelog was treated as a
/// normal state and the entry was rendered against empty content — a changelog read failure here
/// means the run aborted before writing anything, and the second assertion would then pass for the
/// wrong reason. The second one says the file the run created is not left behind.
#[test]
fn a_changelog_the_run_created_is_removed_when_a_later_step_fails() {
    let guard = FixtureGuard::new("changelog-created-rollback");
    let fixture = guard.fixture();
    write_fixture(fixture, "[hooks]\npost_bump = \"exit 1\"\n");
    commit_fixture(fixture);
    assert!(
        !fixture.dir.join("CHANGELOG.md").exists(),
        "the fixture must start without the changelog this suite is about"
    );

    let cfg = load_config(fixture);
    let err = bump_run(&cfg, Bump::Minor, false, &[]).unwrap_err();

    assert!(
        matches!(err, cutver::bump::Error::PostBumpHookFailed { .. }),
        "the failure must be the post_bump hook's own exit, because reaching it proves the run \
         created the absent changelog (S1/S4). A read error here means a declared but absent \
         changelog still aborts the run before it writes anything, so nothing could be rolled \
         back. got: {err:?}"
    );
    assert!(
        !fixture.dir.join("CHANGELOG.md").exists(),
        "rollback must remove the changelog the failed run created instead of leaving a file the \
         tree did not have before the run started (S2/S4). got: {err:?}"
    );
    assert!(
        fixture.read("package.json").contains("\"version\": \"1.2.3\""),
        "the version source must still be restored byte-for-byte"
    );
}

/// S1: an *unreadable* changelog is not the same state as an absent one, and it still fails closed.
///
/// The bytes here are invalid UTF-8, which is unreadable on every host without depending on file
/// permissions (a `chmod 000` file is readable by an elevated runner on Windows and by root in a
/// container, so a permission-based case would prove nothing there). The reader's contract is that
/// only `NotFound` means "there was nothing here": backing this file up as empty and writing that
/// back would turn a real changelog into a zero-byte one.
#[test]
fn an_unreadable_changelog_fails_closed_instead_of_being_backed_up_as_empty() {
    let guard = FixtureGuard::new("changelog-unreadable");
    let fixture = guard.fixture();
    let original: &[u8] = &[0x2f, 0x2f, 0xff, 0xfe, 0x00, 0x41, 0x0a];
    write_fixture(fixture, "");
    std::fs::write(fixture.dir.join("CHANGELOG.md"), original).unwrap();
    commit_fixture(fixture);
    let package_before = fixture.read("package.json");

    let cfg = load_config(fixture);
    let err = bump_run(&cfg, Bump::Minor, false, &[]).unwrap_err();

    assert!(
        matches!(err, cutver::bump::Error::ReadUnreadable { .. }),
        "an unreadable changelog must fail closed as an unreadable backup, not be treated as \
         absent and overwritten: {err:?}"
    );
    assert!(
        err.to_string().contains("CHANGELOG.md"),
        "the error must name the file it could not back up: {err}"
    );
    assert_eq!(
        std::fs::read(fixture.dir.join("CHANGELOG.md")).unwrap(),
        original,
        "the unreadable changelog must keep its bytes"
    );
    assert_eq!(
        package_before,
        fixture.read("package.json"),
        "nothing may be written before the rollback snapshot exists"
    );
}

/// S6, preserved: a rehearsal records nothing and writes nothing.
///
/// `prepare_changelog` returns `original: None, updated: None` for a dry run, so `apply_changelog`
/// neither records a backup nor writes; this case shares both of those code paths with the change
/// above, so it is pinned here rather than assumed.
#[test]
fn a_rehearsal_neither_creates_nor_records_the_absent_changelog() {
    let guard = FixtureGuard::new("changelog-created-dry-run");
    let fixture = guard.fixture();
    write_fixture(fixture, "");
    commit_fixture(fixture);
    assert!(
        !fixture.dir.join("CHANGELOG.md").exists(),
        "the fixture must start without the changelog this suite is about"
    );

    let cfg = load_config(fixture);
    bump_run(&cfg, Bump::Minor, true, &[]).expect("a rehearsal of an absent changelog must not fail");

    assert!(
        !fixture.dir.join("CHANGELOG.md").exists(),
        "a rehearsal must not create the changelog"
    );
    let status = run_git(&fixture.dir, &["status", "--porcelain"]);
    let status = String::from_utf8_lossy(&status.stdout);
    assert!(
        status.trim().is_empty(),
        "a rehearsal must leave the tree exactly as it found it: {status}"
    );
}

/// S1b: `cutver plan` reads an absent declared changelog the same way the bump writes it.
///
/// A plan that refuses what a bump would do is a check applied a level away from its object: the
/// bump creates an absent `CHANGELOG.md` and rolls it back on failure (S1/S4), so `plan` — the
/// command whose whole promise is "this is the release you are about to cut" — has to render the
/// same notes from the same absent state through the same render authority, and still write
/// nothing.
#[test]
fn a_plan_renders_notes_for_an_absent_changelog_without_creating_it() {
    let guard = FixtureGuard::new("changelog-absent-plan");
    let fixture = guard.fixture();
    write_fixture(fixture, "");
    commit_fixture(fixture);
    assert!(
        !fixture.dir.join("CHANGELOG.md").exists(),
        "the fixture must start without the changelog this suite is about"
    );

    let output = cutver_command(&fixture.dir)
        .args(["plan", "--json", "minor"])
        .output()
        .expect("failed to execute the cutver binary");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    assert!(
        output.status.success(),
        "a plan must not fail where a bump would create the changelog (S1b): stdout={stdout} \
         stderr={stderr}"
    );

    let plan: serde_json::Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|err| panic!("`plan --json` prints one JSON object: {stdout:?}: {err}"));
    let notes = plan["notes"]
        .as_str()
        .unwrap_or_else(|| panic!("the plan carries `notes` as a string: {plan}"));
    assert!(
        !notes.trim().is_empty(),
        "the plan reports the notes the run would write: {plan}"
    );
    assert!(notes.contains("1.3.0"), "the notes are the release's own: {notes}");

    assert!(
        !fixture.dir.join("CHANGELOG.md").exists(),
        "a plan writes nothing, so the changelog it just rendered must still be absent"
    );
    let status = run_git(&fixture.dir, &["status", "--porcelain"]);
    let status = String::from_utf8_lossy(&status.stdout);
    assert!(
        status.trim().is_empty(),
        "a plan must leave the tree exactly as it found it: {status}"
    );
}
