//! The rest of the gate channel, end to end (#326 slice 3B, S10 and S11).
//!
//! `attest` and `publish --require-gate` live in `e2e_gates.rs`. This suite holds the two halves that
//! are about what the tool *observes*: the `gates[]` array `cutver plan --json` fills for the commit
//! it reports, and the exemption that keeps the attestation directory — an input to the command, not a
//! change in the tree — from tripping `publish`'s clean-tree guard.

mod common;

use std::path::Path;
use std::process::{Command, Output};

use common::*;
use serde_json::Value;

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

/// A repository at 1.2.3 on `main`, pushed to a bare remote, with `extra` appended to the config.
///
/// `.gitignore` ignores `published.txt` but **not** `.cutver/`: the attestation directory is untracked
/// in this fixture exactly as it is in a fresh CI checkout, which is what S10 is about.
fn prepared_repo(name: &str, extra: &str) -> (FixtureGuard, Fixture) {
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
    fixture.write("cutver.toml", &format!("{TOML}{extra}"));
    init_git_repo(fixture);
    run_git_ok(&fixture.dir, &["checkout", "-B", "main"]);
    initial_commit(fixture);
    run_git_ok(&fixture.dir, &["remote", "add", "origin", remote.dir.to_str().unwrap()]);
    (guard, remote)
}

fn cutver(fixture: &Fixture, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cutver"))
        .current_dir(&fixture.dir)
        .args(args)
        .output()
        .expect("failed to execute the cutver binary")
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
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
    rev(&fixture.dir, "HEAD")
}

fn attest_ok(fixture: &Fixture, gate: &str, commit: &str, conclusion: &str) {
    let out = cutver(
        fixture,
        &["attest", gate, "--commit", commit, "--conclusion", conclusion],
    );
    assert_eq!(out.status.code(), Some(0), "stderr:\n{}", stderr_of(&out));
}

fn publish(fixture: &Fixture, commit: &str) -> Output {
    publish_with(fixture, commit, &[])
}

fn publish_with(fixture: &Fixture, commit: &str, extra: &[&str]) -> Output {
    let mut args = vec!["publish", "--tag", "v1.3.0", "--commit", commit];
    args.extend(extra.iter().copied());
    cutver(fixture, &args)
}

fn remote_refs(remote: &Path) -> String {
    let out = run_git(remote, &["for-each-ref", "--format=%(refname)"]);
    String::from_utf8_lossy(&out.stdout).to_string()
}

/// S10: the attestation directory is an input to the command, not a change in the tree — including
/// in the shape a fresh CI checkout produces, where git collapses it into one untracked entry.
#[test]
fn the_attestation_directory_does_not_trip_the_clean_tree_guard() {
    let (guard, remote) = prepared_repo("gates-clean-tree", "");
    let fixture = guard.fixture();
    let head = prepare(fixture);
    attest_ok(fixture, "ci", &head, "success");
    assert_eq!(
        porcelain(fixture).trim(),
        "?? .cutver/",
        "the fixture reproduces the collapsed entry git reports for a fresh attestation directory"
    );

    let out = publish(fixture, &head);

    assert!(out.status.success(), "stderr:\n{}", stderr_of(&out));
    assert_eq!(rev(&fixture.dir, "v1.3.0^{commit}"), head, "the release was tagged");
    assert_eq!(rev(&remote.dir, "refs/tags/v1.3.0^{commit}"), head);
}

/// S10: the exemption is the attestation directory alone. A file inside it that is not an
/// attestation, a tracked change under it, and an untracked file outside it all still abort.
#[test]
fn a_change_that_is_not_an_attestation_still_trips_the_clean_tree_guard() {
    let (guard, _remote) = prepared_repo("gates-clean-tree-inside", "");
    let fixture = guard.fixture();
    let head = prepare(fixture);
    attest_ok(fixture, "ci", &head, "success");
    fixture.write(".cutver/scratch.txt", "not an attestation");

    let out = publish(fixture, &head);

    assert_eq!(out.status.code(), Some(1), "stderr:\n{}", stderr_of(&out));
    let message = stderr_of(&out);
    assert!(message.contains("uncommitted changes"), "{message}");
    assert!(
        message.contains(".cutver"),
        "the entry carrying the stray file is reported: {message}"
    );
    assert!(!tag_exists(fixture, "v1.3.0"), "no tag may be created locally");

    let (guard, _remote) = prepared_repo("gates-clean-tree-tracked", "");
    let fixture = guard.fixture();
    let head = prepare(fixture);
    attest_ok(fixture, "ci", &head, "success");
    // A tracked file in the same directory: the exemption only ever covers untracked entries.
    run_git_ok(&fixture.dir, &["add", ".cutver/attestations/ci.json"]);
    run_git_ok(&fixture.dir, &["commit", "-m", "track the attestation", "-q"]);
    let tracked_head = rev(&fixture.dir, "HEAD");
    fixture.write(".cutver/attestations/ci.json", "{\"gate\":\"ci\"}\n");

    let out = publish(fixture, &tracked_head);

    assert_eq!(out.status.code(), Some(1), "stderr:\n{}", stderr_of(&out));
    let message = stderr_of(&out);
    assert!(message.contains("uncommitted changes"), "{message}");
    assert!(
        message.contains(".cutver/attestations/ci.json"),
        "a tracked change under the directory is a change in the tree: {message}"
    );

    let (guard, _remote) = prepared_repo("gates-clean-tree-outside", "");
    let fixture = guard.fixture();
    let head = prepare(fixture);
    attest_ok(fixture, "ci", &head, "success");
    fixture.write("wip.txt", "uncommitted work\n");

    let out = publish(fixture, &head);

    assert_eq!(out.status.code(), Some(1), "stderr:\n{}", stderr_of(&out));
    let message = stderr_of(&out);
    assert!(message.contains("uncommitted changes"), "{message}");
    assert!(message.contains("wip.txt"), "{message}");
}

/// S10: a `[gates] dir` that would whitelist the whole worktree is refused by the configuration
/// instead of exempting everything.
#[test]
fn a_gates_directory_that_names_the_workspace_root_is_refused() {
    for dir in ["", "./ci", ".", "..", "../outside", "/absolute", "ci/../../escape"] {
        let (guard, _remote) = prepared_repo("gates-dir-escape", &format!("\n[gates]\ndir = \"{dir}\"\n"));
        let fixture = guard.fixture();

        let out = cutver(fixture, &["doctor"]);

        assert_eq!(out.status.code(), Some(1), "dir {dir:?}: {}", stderr_of(&out));
        let message = stderr_of(&out);
        assert!(
            message.contains("invalid `[gates] dir"),
            "dir {dir:?} is refused for what it names, not for being unknown: {message}"
        );
        assert!(
            message.contains("clean-tree guard"),
            "dir {dir:?} says why it is refused: {message}"
        );
    }
}

/// S11: `gates[]` carries the gates the release would require, and the conclusion attested for the
/// commit the plan reports — `null` when there is none for that commit.
#[test]
fn the_plan_reports_the_required_gates_and_the_attested_conclusion() {
    let (guard, _remote) = prepared_repo(
        "plan-gates",
        "\n[gates]\ndir = \".cutver/attestations\"\nrequire = [\"ci\", \"security\"]\n",
    );
    let fixture = guard.fixture();
    let commit = rev(&fixture.dir, "HEAD");
    attest_ok(fixture, "ci", &commit, "success");

    let out = cutver(fixture, &["plan", "--json", "minor"]);

    assert_eq!(out.status.code(), Some(0), "stderr:\n{}", stderr_of(&out));
    let raw = stdout_of(&out);
    assert!(
        raw.contains(r#""gates":[{"name":"ci","conclusion":"success","commit":""#),
        "the entry's key order is part of the document: {raw}"
    );
    let plan: Value = serde_json::from_str(raw.trim()).expect("stdout is one JSON object");
    assert_eq!(plan["gates"].as_array().map(Vec::len), Some(2), "{plan}");
    assert_eq!(plan["gates"][0]["name"], "ci");
    assert_eq!(plan["gates"][0]["conclusion"], "success");
    assert_eq!(plan["gates"][0]["commit"], commit);
    assert!(
        is_iso_date(plan["gates"][0]["verified_at"].as_str().expect("a date")),
        "{plan}"
    );
    assert_eq!(plan["gates"][0]["run"], Value::Null);
    assert_eq!(plan["gates"][1]["name"], "security");
    assert_eq!(plan["gates"][1]["conclusion"], Value::Null, "{plan}");
    assert_eq!(plan["gates"][1]["commit"], Value::Null, "{plan}");

    // An attestation for another commit is not this release's: the plan reports what it knows
    // about the commit it reports.
    let (guard, _remote) = prepared_repo("plan-gates-stale", "\n[gates]\nrequire = [\"ci\"]\n");
    let fixture = guard.fixture();
    let other = rev(&fixture.dir, "HEAD");
    fixture.write("dist/notes.txt", "a release build\n");
    run_git_ok(&fixture.dir, &["add", "-A"]);
    run_git_ok(&fixture.dir, &["commit", "-m", "feat: build output", "-q"]);
    let commit = rev(&fixture.dir, "HEAD");
    attest_ok(fixture, "ci", &other, "success");

    let out = cutver(fixture, &["plan", "--json", "minor"]);

    assert_eq!(out.status.code(), Some(0), "stderr:\n{}", stderr_of(&out));
    let plan: Value = serde_json::from_str(stdout_of(&out).trim()).expect("one JSON object");
    assert_eq!(plan["commit"], commit);
    assert_eq!(plan["gates"][0]["name"], "ci");
    assert_eq!(plan["gates"][0]["conclusion"], Value::Null, "{plan}");
}

/// PRESERVE: a workspace that declares no `[gates]` table plans with `gates` present and empty, and
/// a `[gates]` typo or an unserializable gate name is rejected like a typo in `[publish]`.
#[test]
fn the_gates_table_is_a_registered_table_with_a_closed_key_set() {
    let (guard, _remote) = prepared_repo("gates-absent", "");
    let fixture = guard.fixture();

    let out = cutver(fixture, &["plan", "--json", "minor"]);

    assert_eq!(out.status.code(), Some(0), "stderr:\n{}", stderr_of(&out));
    let plan: Value = serde_json::from_str(stdout_of(&out).trim()).expect("one JSON object");
    assert_eq!(plan["gates"], serde_json::json!([]), "{plan}");

    let (guard, _remote) = prepared_repo("gates-typo", "\n[gates]\ndirr = \".cutver/gates\"\n");
    let fixture = guard.fixture();

    let out = cutver(fixture, &["doctor"]);

    assert_eq!(out.status.code(), Some(1), "stderr:\n{}", stderr_of(&out));
    let message = stderr_of(&out);
    assert!(message.contains("[gates]"), "{message}");
    assert!(message.contains("dirr"), "{message}");

    let (guard, _remote) = prepared_repo("gates-bad-name-config", "\n[gates]\nrequire = [\"../ci\"]\n");
    let fixture = guard.fixture();

    let out = cutver(fixture, &["doctor"]);

    assert_eq!(out.status.code(), Some(1), "stderr:\n{}", stderr_of(&out));
    assert!(stderr_of(&out).contains("../ci"), "{}", stderr_of(&out));
}

/// The `[gates]` fixture pins the key inventory: both keys, parsed through the loader the CLI uses.
#[test]
fn the_gates_fixture_pins_the_key_inventory() {
    let guard = FixtureGuard::new("gates-fixture");
    let fixture = guard.fixture();
    fixture.write("package.json", PACKAGE_JSON);
    fixture.write(
        "cutver.toml",
        &std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/gates/cutver.toml"))
            .expect("the gates fixture is readable"),
    );
    init_git_repo(fixture);
    initial_commit(fixture);

    let config = load_config(fixture);

    assert_eq!(config.gates.dir, "ci/verdicts");
    let required: Vec<&str> = config.gates.require.iter().map(|gate| gate.as_str()).collect();
    assert_eq!(required, vec!["ci", "security"]);

    let out = cutver(fixture, &["doctor"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "the fixture is a valid configuration; stderr:\n{}",
        stderr_of(&out)
    );
}

/// S10: a tracked file in the attestation directory — a `.gitkeep`, a `.gitignore`, a README — is
/// enough for git to list the attestations one by one instead of collapsing the tree, and the
/// release still publishes: the directory is the input, whichever way git spells it.
#[test]
fn a_tracked_file_in_the_attestation_directory_does_not_break_the_release() {
    let (guard, remote) = prepared_repo("gates-tracked-gitkeep", "");
    let fixture = guard.fixture();
    fixture.write(".cutver/attestations/.gitkeep", "");
    run_git_ok(&fixture.dir, &["add", "-A"]);
    run_git_ok(&fixture.dir, &["commit", "-m", "keep the attestation directory", "-q"]);
    let head = prepare(fixture);
    attest_ok(fixture, "ci", &head, "success");
    assert_eq!(
        porcelain(fixture).trim(),
        "?? .cutver/attestations/ci.json",
        "git lists the attestation individually, because the directory holds a tracked file"
    );

    let out = publish_with(fixture, &head, &["--require-gate", "ci"]);

    assert!(
        out.status.success(),
        "stderr:
{}",
        stderr_of(&out)
    );
    assert_eq!(rev(&fixture.dir, "v1.3.0^{commit}"), head, "the release was tagged");
    assert_eq!(rev(&remote.dir, "refs/tags/v1.3.0^{commit}"), head);
}

/// D5/S11: `[gates] require` is enforced, not reported. A declared gate with no attestation refuses
/// `publish` even without `--require-gate`, and a gate named twice is one requirement.
#[test]
fn the_declared_gates_are_required_by_publish_and_checked_once() {
    let (guard, remote) = prepared_repo(
        "gates-declared",
        "
[gates]
require = [\"ci\"]
",
    );
    let fixture = guard.fixture();
    let head = prepare(fixture);
    let status_before = porcelain(fixture);

    let out = publish(fixture, &head);

    assert_eq!(
        out.status.code(),
        Some(1),
        "stderr:
{}",
        stderr_of(&out)
    );
    let message = stderr_of(&out);
    assert!(message.contains("gate 'ci'"), "{message}");
    assert_eq!(
        message.matches("gate 'ci'").count(),
        1,
        "a required gate is reported once: {message}"
    );
    assert!(!tag_exists(fixture, "v1.3.0"), "no tag may be created locally");
    assert!(
        !remote_refs(&remote.dir).contains("refs/tags/"),
        "no tag may reach the remote"
    );
    assert_eq!(porcelain(fixture), status_before, "the worktree is unchanged");

    // The same gate, named again for this invocation, is still one requirement.
    let out = publish_with(fixture, &head, &["--require-gate", "ci"]);

    assert_eq!(
        out.status.code(),
        Some(1),
        "stderr:
{}",
        stderr_of(&out)
    );
    let message = stderr_of(&out);
    assert_eq!(
        message.matches("gate 'ci'").count(),
        1,
        "the union is deduplicated: {message}"
    );
    assert!(!tag_exists(fixture, "v1.3.0"), "no tag may be created locally");
    assert_eq!(porcelain(fixture), status_before, "the worktree is unchanged");
}

/// D5: a project that declares its gates once in `cutver.toml` publishes as soon as every one of
/// them attests the release commit — no flag needed.
#[test]
fn the_declared_gates_let_the_release_publish_once_they_attest() {
    let (guard, remote) = prepared_repo(
        "gates-declared-satisfied",
        "
[gates]
require = [\"ci\", \"security\"]
",
    );
    let fixture = guard.fixture();
    let head = prepare(fixture);
    attest_ok(fixture, "ci", &head, "success");
    attest_ok(fixture, "security", &head, "success");

    let out = publish(fixture, &head);

    assert!(
        out.status.success(),
        "stderr:
{}",
        stderr_of(&out)
    );
    assert_eq!(rev(&fixture.dir, "v1.3.0^{commit}"), head, "the release was tagged");
    assert_eq!(rev(&remote.dir, "refs/tags/v1.3.0^{commit}"), head);
    assert_eq!(fixture.read("published.txt").trim(), "published 1.3.0");
}
