//! `cutver attest` and `publish --require-gate`, end to end (#326 slice 3B, S8 and S9).
//!
//! `attest` records one verdict per gate; `publish --require-gate` refuses when the required verdict
//! is missing, failed or recorded for another commit. Every refusal is checked against the state it
//! must leave behind: no tag locally, no tag on the remote, no `[publish] commands`, an unchanged
//! tree. The plan and clean-tree halves of the channel live in `e2e_gates_channel.rs`.
//!
//! Not `#![cfg(unix)]`: the commands that must not run are shell lines, and the assertions are about
//! their absence and about git state, both portable.

mod common;

use std::path::Path;
use std::process::Output;

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
    cutver_command(&fixture.dir)
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
    rev(&fixture.dir, "HEAD")
}

fn attest(fixture: &Fixture, gate: &str, commit: &str, conclusion: &str, run: Option<&str>) -> Output {
    let mut args = vec!["attest", gate, "--commit", commit, "--conclusion", conclusion];
    if let Some(run) = run {
        args.push("--run");
        args.push(run);
    }
    cutver(fixture, &args)
}

fn attest_ok(fixture: &Fixture, gate: &str, commit: &str, conclusion: &str) -> Output {
    let out = attest(fixture, gate, commit, conclusion, None);
    assert_eq!(out.status.code(), Some(0), "stderr:\n{}", stderr_of(&out));
    out
}

fn publish(fixture: &Fixture, commit: &str, extra: &[&str]) -> Output {
    let mut args = vec!["publish", "--tag", "v1.3.0", "--commit", commit];
    args.extend(extra.iter().copied());
    cutver(fixture, &args)
}

/// Everything a refusal must leave untouched: no tag anywhere, no publish command, same tree.
fn assert_nothing_happened(fixture: &Fixture, remote: &Fixture, head: &str, status_before: &str) {
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
    assert_eq!(head, rev(&fixture.dir, "HEAD"), "the release commit is HEAD");
    assert_eq!(porcelain(fixture), status_before, "the worktree is unchanged");
}

/// S8: one JSON object per gate, recording what was verified, for which commit, with which
/// conclusion, and when.
#[test]
fn attest_records_the_gate_commit_conclusion_and_day() {
    let (guard, _remote) = prepared_repo("attest-record", "");
    let fixture = guard.fixture();
    let head = rev(&fixture.dir, "HEAD");

    let out = attest(fixture, "ci", &head, "success", Some("https://ci.example/run/7"));

    assert_eq!(out.status.code(), Some(0), "stderr:\n{}", stderr_of(&out));
    let stdout = stdout_of(&out);
    assert_eq!(stdout.lines().count(), 1, "one line of output: {stdout}");
    for expected in ["ci", head.as_str(), ".cutver/attestations/ci.json"] {
        assert!(stdout.contains(expected), "{stdout} does not name {expected}");
    }

    let raw = fixture.read(".cutver/attestations/ci.json");
    // Key order is read from the rendered bytes: `serde_json::Value` sorts keys and would hide it.
    let prefix = format!(r#"{{"schemaVersion":1,"gate":"ci","commit":"{head}","conclusion":"success","verified_at":""#);
    assert!(raw.starts_with(&prefix), "{raw}");
    let recorded: Value = serde_json::from_str(raw.trim()).expect("the attestation is one JSON object");
    assert_eq!(recorded["gate"], "ci");
    assert_eq!(recorded["commit"], head);
    assert_eq!(recorded["conclusion"], "success");
    assert_eq!(recorded["run"], "https://ci.example/run/7");
    let verified_at = recorded["verified_at"].as_str().expect("verified_at is a string");
    assert!(is_iso_date(verified_at), "`when` is an ISO date, found {verified_at:?}");

    // `--run` is optional: the key is absent, not null, when the gate names no run.
    let second = attest_ok(fixture, "e2e", &head, "failure");
    assert!(stdout_of(&second).contains("e2e"), "{}", stdout_of(&second));
    let raw = fixture.read(".cutver/attestations/e2e.json");
    assert!(raw.contains(r#""conclusion":"failure""#), "{raw}");
    assert!(!raw.contains("\"run\""), "{raw}");
}

/// S8: `[gates] dir` sets where the attestations live, and the default directory stays untouched.
#[test]
fn attest_uses_the_configured_directory() {
    let (guard, _remote) = prepared_repo("attest-dir", "\n[gates]\ndir = \"ci/verdicts\"\n");
    let fixture = guard.fixture();
    let head = rev(&fixture.dir, "HEAD");

    let out = attest(fixture, "ci", &head, "success", None);

    assert_eq!(out.status.code(), Some(0), "stderr:\n{}", stderr_of(&out));
    assert!(stdout_of(&out).contains("ci/verdicts/ci.json"), "{}", stdout_of(&out));
    let recorded: Value = serde_json::from_str(fixture.read("ci/verdicts/ci.json").trim()).expect("one JSON object");
    assert_eq!(recorded["gate"], "ci");
    assert!(
        !fixture.dir.join(".cutver").exists(),
        "the default directory must not be created when `[gates] dir` names another one"
    );
}

/// S8: a gate name becomes a file name, so one that could leave the directory is refused, and a
/// `--commit` that is not a sha is refused too — each before anything is written.
#[test]
fn attest_refuses_a_gate_name_or_a_commit_it_cannot_record() {
    let (guard, _remote) = prepared_repo("attest-invalid", "");
    let fixture = guard.fixture();
    let head = rev(&fixture.dir, "HEAD");
    let status_before = porcelain(fixture);

    for gate in ["../escape", "a/b", "..", ".hidden", "with space"] {
        let out = attest(fixture, gate, &head, "success", None);
        assert_eq!(out.status.code(), Some(1), "gate {gate:?}: {}", stderr_of(&out));
        let message = stderr_of(&out);
        assert!(message.contains("gate name"), "gate {gate:?}: {message}");
    }
    assert!(!fixture.dir.join(".cutver").exists(), "no attestation may be written");
    assert!(
        !fixture.dir.join("escape.json").exists(),
        "nothing may escape the directory"
    );
    assert_eq!(porcelain(fixture), status_before, "the worktree is unchanged");

    let out = attest(fixture, "ci", "HEAD", "success", None);
    assert_eq!(out.status.code(), Some(1), "stderr:\n{}", stderr_of(&out));
    let message = stderr_of(&out);
    assert!(message.contains("HEAD"), "the refusal names the value: {message}");
    assert!(message.contains("sha"), "the refusal says what it needs: {message}");
    assert!(!fixture.dir.join(".cutver").exists(), "no attestation may be written");
}

/// S8: the conclusion is a closed set, refused by the parser before the command runs.
#[test]
fn attest_refuses_a_conclusion_outside_the_closed_set() {
    let (guard, _remote) = prepared_repo("attest-conclusion", "");
    let fixture = guard.fixture();
    let head = rev(&fixture.dir, "HEAD");

    let out = attest(fixture, "ci", &head, "maybe", None);

    assert_eq!(
        out.status.code(),
        Some(2),
        "a usage error is clap's exit 2; stderr:\n{}",
        stderr_of(&out)
    );
    let message = stderr_of(&out);
    for expected in ["possible values", "success", "failure"] {
        assert!(message.contains(expected), "{message} does not name {expected}");
    }
    assert!(!fixture.dir.join(".cutver").exists(), "no attestation may be written");
}

/// S9: a required gate with no attestation refuses before anything irreversible, naming the gate
/// and the commit.
#[test]
fn a_required_gate_without_an_attestation_refuses_naming_gate_and_commit() {
    let (guard, remote) = prepared_repo("gates-missing", "");
    let fixture = guard.fixture();
    let head = prepare(fixture);
    let status_before = porcelain(fixture);

    let out = publish(fixture, &head, &["--require-gate", "ci"]);

    assert_eq!(out.status.code(), Some(1), "stderr:\n{}", stderr_of(&out));
    let message = stderr_of(&out);
    assert!(message.contains("gate 'ci'"), "{message}");
    assert!(message.contains(&head), "the refusal names the commit: {message}");
    assert!(message.contains(".cutver/attestations"), "{message}");
    assert!(message.contains("Nothing was tagged, pushed or executed"), "{message}");
    assert_nothing_happened(fixture, &remote, &head, &status_before);
}

/// S9: a `failure` conclusion refuses too, in the same voice.
#[test]
fn a_failed_conclusion_refuses_in_the_same_voice() {
    let (guard, remote) = prepared_repo("gates-failed", "");
    let fixture = guard.fixture();
    let head = prepare(fixture);
    let attested = attest(fixture, "ci", &head, "failure", Some("https://ci.example/run/9"));
    assert_eq!(attested.status.code(), Some(0), "stderr:\n{}", stderr_of(&attested));
    let status_before = porcelain(fixture);

    let out = publish(fixture, &head, &["--require-gate", "ci"]);

    assert_eq!(out.status.code(), Some(1), "stderr:\n{}", stderr_of(&out));
    let message = stderr_of(&out);
    assert!(message.contains("gate 'ci'"), "{message}");
    assert!(message.contains(&head), "{message}");
    assert!(message.contains("failure"), "the refusal names the verdict: {message}");
    assert!(
        message.contains("https://ci.example/run/9"),
        "the refusal names the run that recorded it: {message}"
    );
    assert!(message.contains("Nothing was tagged, pushed or executed"), "{message}");
    assert_nothing_happened(fixture, &remote, &head, &status_before);
}

/// S9: an attestation is only this release's when it names the commit being published.
#[test]
fn an_attestation_for_another_commit_does_not_satisfy_the_gate() {
    let (guard, remote) = prepared_repo("gates-other-commit", "");
    let fixture = guard.fixture();
    let other = rev(&fixture.dir, "HEAD");
    let head = prepare(fixture);
    attest_ok(fixture, "ci", &other, "success");
    let status_before = porcelain(fixture);

    let out = publish(fixture, &head, &["--require-gate", "ci"]);

    assert_eq!(out.status.code(), Some(1), "stderr:\n{}", stderr_of(&out));
    let message = stderr_of(&out);
    assert!(message.contains("gate 'ci'"), "{message}");
    assert!(
        message.contains(&head),
        "the refusal names the published commit: {message}"
    );
    assert!(
        message.contains(&other),
        "the refusal names the commit it did find: {message}"
    );
    assert_nothing_happened(fixture, &remote, &head, &status_before);
}

/// S9: every `--require-gate` is checked, and one missing gate refuses the release.
#[test]
fn every_required_gate_is_checked_and_one_missing_gate_refuses() {
    let (guard, remote) = prepared_repo("gates-each", "");
    let fixture = guard.fixture();
    let head = prepare(fixture);
    attest_ok(fixture, "ci", &head, "success");
    let status_before = porcelain(fixture);

    let out = publish(fixture, &head, &["--require-gate", "ci", "--require-gate", "security"]);

    assert_eq!(out.status.code(), Some(1), "stderr:\n{}", stderr_of(&out));
    let message = stderr_of(&out);
    assert!(message.contains("gate 'security'"), "{message}");
    assert!(
        !message.contains("gate 'ci'"),
        "a satisfied gate is not refused: {message}"
    );
    assert_nothing_happened(fixture, &remote, &head, &status_before);
}

/// S9: a required gate that is a valid name but cannot be a file name is refused the same way.
#[test]
fn a_required_gate_name_that_cannot_name_a_file_is_refused() {
    let (guard, remote) = prepared_repo("gates-bad-name", "");
    let fixture = guard.fixture();
    let head = prepare(fixture);
    let status_before = porcelain(fixture);

    let out = publish(fixture, &head, &["--require-gate", "../escape"]);

    assert_eq!(out.status.code(), Some(1), "stderr:\n{}", stderr_of(&out));
    let message = stderr_of(&out);
    assert!(message.contains("gate name"), "{message}");
    assert!(message.contains("../escape"), "{message}");
    assert_nothing_happened(fixture, &remote, &head, &status_before);
}

/// S9: once every required gate attests the commit, the closing phase publishes.
#[test]
fn a_satisfied_gate_lets_the_release_publish() {
    let (guard, remote) = prepared_repo("gates-satisfied", "");
    let fixture = guard.fixture();
    let head = prepare(fixture);
    attest_ok(fixture, "ci", &head, "success");
    attest_ok(fixture, "security", &head, "success");

    let out = publish(fixture, &head, &["--require-gate", "ci", "--require-gate", "security"]);

    assert!(out.status.success(), "stderr:\n{}", stderr_of(&out));
    assert_eq!(rev(&fixture.dir, "v1.3.0^{commit}"), head, "the release was tagged");
    assert_eq!(rev(&remote.dir, "refs/tags/v1.3.0^{commit}"), head);
    assert_eq!(fixture.read("published.txt").trim(), "published 1.3.0");
}

/// S3, D1: a record written before `schemaVersion` existed still satisfies the gate. This is the half
/// that matters: the reader is total, so adding the key cannot turn a verdict that did vouch for the
/// commit into "no attestation" and disable a gate that had passed.
#[test]
fn a_legacy_attestation_without_the_version_key_still_satisfies_the_gate() {
    let (guard, remote) = prepared_repo("gates-legacy", "");
    let fixture = guard.fixture();
    let head = prepare(fixture);
    // Hand-written on purpose: no `cutver` of this version can produce the legacy shape any more.
    fixture.write(
        ".cutver/attestations/ci.json",
        &format!(r#"{{"gate":"ci","commit":"{head}","conclusion":"success","verified_at":"2026-11-26"}}"#),
    );

    let out = publish(fixture, &head, &["--require-gate", "ci"]);

    assert!(out.status.success(), "stderr:\n{}", stderr_of(&out));
    assert_eq!(rev(&fixture.dir, "v1.3.0^{commit}"), head, "the release was tagged");
    assert_eq!(rev(&remote.dir, "refs/tags/v1.3.0^{commit}"), head);
    assert_eq!(fixture.read("published.txt").trim(), "published 1.3.0");
}

/// PRESERVE: a project that requires no gate publishes exactly as it did before the gate channel.
#[test]
fn a_release_that_requires_no_gate_is_unchanged() {
    let (guard, remote) = prepared_repo("gates-none", "");
    let fixture = guard.fixture();
    let head = prepare(fixture);

    let out = publish(fixture, &head, &[]);

    assert!(out.status.success(), "stderr:\n{}", stderr_of(&out));
    assert_eq!(rev(&remote.dir, "refs/tags/v1.3.0^{commit}"), head);
    assert_eq!(fixture.read("published.txt").trim(), "published 1.3.0");
    assert!(
        !fixture.dir.join(".cutver").exists(),
        "no attestation directory is created by a release that reads no gate"
    );
}

/// L13/S9: a damaged attestation is "no attestation" — named as unreadable, never an error, and the
/// refusal still precedes the tag.
#[test]
fn a_damaged_attestation_is_named_and_refuses_as_missing() {
    let (guard, remote) = prepared_repo("gates-damaged", "");
    let fixture = guard.fixture();
    let head = prepare(fixture);
    fixture.write(
        ".cutver/attestations/ci.json",
        &format!("{{\"gate\":\"ci\",\"commit\":\"{head}\",\"conclusion\":\"suc"),
    );
    // The baseline includes the damaged file: the refusal must add nothing to it.
    let status_before = porcelain(fixture);

    let out = publish(fixture, &head, &["--require-gate", "ci"]);

    assert_eq!(
        out.status.code(),
        Some(1),
        "stderr:
{}",
        stderr_of(&out)
    );
    let message = stderr_of(&out);
    assert!(message.contains("gate 'ci'"), "{message}");
    assert!(message.contains(&head), "{message}");
    assert!(
        message.contains("does not exist or does not read as an attestation"),
        "{message}"
    );
    assert_nothing_happened(fixture, &remote, &head, &status_before);
}
