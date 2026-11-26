//! The CI output channel, end to end (#326 phase 4A: S1-S6): detection, `$GITHUB_OUTPUT`, the step
//! summary and the failure annotation, asserted through the binary as exact bytes. Every command is
//! spawned through `common::cutver_command`, which removes the channel's three variables, so a runner
//! that exports `GITHUB_*` for its own steps cannot change an answer (S6); this suite opts back in
//! with `with_ci` and friends from `tests/common/ci.rs`. The context itself is built directly in the
//! module's unit tests rather than by mutating this process's environment.

mod common;

use std::fs;
use std::path::Path;
use std::process::Output;

use common::ci::{Sinks, TOML, head_commit, repo, snapshot, with_ci, without_github_actions};
use common::cutver_command;
use common::{Fixture, FixtureGuard, run_git_ok};
use tempfile::TempDir;

/// A `cutver` process with the channel's variables removed and nothing else set.
fn plain(dir: &Path, args: &[&str]) -> Output {
    cutver_command(dir)
        .args(args)
        .output()
        .expect("failed to execute the cutver binary")
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

/// A workspace whose two manifests disagree, which `doctor` reports as drift with exit `2`.
fn drifting_manifests() -> TempDir {
    let dir = TempDir::new().expect("a temporary workspace");
    let config = r#"[version]
current_source = "a.json"

[[manifest]]
path = "a.json"
kind = "json"
field = "version"

[[manifest]]
path = "b.json"
kind = "json"
field = "version"
"#;
    fs::write(dir.path().join("cutver.toml"), config).expect("the config is written");
    fs::write(dir.path().join("a.json"), r#"{"version": "1.1.0"}"#).expect("a manifest is written");
    fs::write(dir.path().join("b.json"), r#"{"version": "1.0.0"}"#).expect("a manifest is written");
    dir
}

/// S1: `GITHUB_ACTIONS=true` alone detects nothing, a named file alone detects nothing either, and
/// both together detect CI — which is what annotates the failure.
#[test]
fn ci_needs_github_actions_and_a_named_file_together() {
    let workspace = TempDir::new().expect("a temporary workspace");
    let sinks = Sinks::new("detect");
    let failing = ["bump", "--json"];

    let actions_only = cutver_command(workspace.path())
        .env("GITHUB_ACTIONS", "true")
        .args(failing)
        .output()
        .expect("failed to execute the cutver binary");
    assert_eq!(actions_only.status.code(), Some(1));
    assert!(
        !stderr_of(&actions_only).contains("::error::"),
        "no named file means no channel"
    );

    let file_only = cutver_command(workspace.path())
        .env("GITHUB_OUTPUT", sinks.output())
        .args(failing)
        .output()
        .expect("failed to execute the cutver binary");
    assert_eq!(file_only.status.code(), Some(1));
    assert!(
        !stderr_of(&file_only).contains("::error::"),
        "without `GITHUB_ACTIONS` a named file detects nothing"
    );

    let both = with_ci(workspace.path(), &failing, &sinks);
    assert_eq!(both.status.code(), Some(1));
    assert!(
        stderr_of(&both).starts_with("::error::"),
        "both halves detect CI, so the failure is annotated"
    );
    assert!(!sinks.output().exists(), "a failed run writes no output file");
    assert!(!sinks.summary().exists(), "a failed run writes no summary");
}

/// S1: `--ci` forces the channel on without `GITHUB_ACTIONS`, `--no-ci` forces it off even when the
/// runner detected one.
#[test]
fn the_flags_force_the_channel_on_and_off() {
    let guard = repo("ci-flags", TOML);
    let fixture = guard.fixture();
    let forced = Sinks::new("ci-flags-forced");

    let on = without_github_actions(&fixture.dir, &["plan", "minor", "--ci"], &forced);
    assert_eq!(on.status.code(), Some(0), "stderr:\n{}", stderr_of(&on));
    assert!(forced.output().exists(), "`--ci` writes `$GITHUB_OUTPUT` on its own");
    assert!(forced.summary().exists(), "`--ci` writes the step summary too");

    let suppressed = Sinks::new("ci-flags-suppressed");
    let off = with_ci(&fixture.dir, &["plan", "minor", "--no-ci"], &suppressed);
    assert_eq!(off.status.code(), Some(0), "stderr:\n{}", stderr_of(&off));
    assert!(!suppressed.output().exists(), "`--no-ci` opens no output file");
    assert!(!suppressed.summary().exists(), "`--no-ci` writes no summary");

    let failing = Sinks::new("ci-flags-failing");
    let workspace = TempDir::new().expect("a temporary workspace");
    let annotated = cutver_command(workspace.path())
        .env("GITHUB_ACTIONS", "true")
        .env("GITHUB_OUTPUT", failing.output())
        .env("GITHUB_STEP_SUMMARY", failing.summary())
        .args(["bump", "--no-ci"])
        .output()
        .expect("failed to execute the cutver binary");
    assert_eq!(annotated.status.code(), Some(1));
    assert!(
        !stderr_of(&annotated).contains("::error::"),
        "`--no-ci` forces the annotation off too: {}",
        stderr_of(&annotated)
    );
}

/// S1, S4: `--ci` alone — no `GITHUB_ACTIONS`, no sink named — still annotates a failure, on a
/// command that has no file sink of its own, and the flag is accepted before the subcommand.
#[test]
fn ci_alone_annotates_a_failure_with_no_sink_and_no_environment() {
    let workspace = TempDir::new().expect("a temporary workspace");

    let out = plain(workspace.path(), &["doctor", "--ci"]);
    assert_eq!(out.status.code(), Some(1), "stderr:\n{}", stderr_of(&out));
    assert!(
        stderr_of(&out).starts_with("::error::Error loading config:"),
        "`--ci` opens the annotation channel by itself: {}",
        stderr_of(&out)
    );

    let before = plain(workspace.path(), &["--ci", "doctor"]);
    assert!(
        stderr_of(&before).starts_with("::error::"),
        "a global flag is accepted before the subcommand: {}",
        stderr_of(&before)
    );
    assert!(!workspace.path().join("github_output").exists(), "no sink is created");
}

/// S4: `--no-ci` is accepted on a command with no file sink, suppresses the annotation, and leaves
/// the command's own failure and exit code untouched.
#[test]
fn no_ci_is_accepted_on_a_command_with_no_file_sink() {
    let workspace = TempDir::new().expect("a temporary workspace");
    let sinks = Sinks::new("no-ci-doctor");

    let out = cutver_command(workspace.path())
        .env("GITHUB_ACTIONS", "true")
        .env("GITHUB_OUTPUT", sinks.output())
        .env("GITHUB_STEP_SUMMARY", sinks.summary())
        .args(["doctor", "--json", "--no-ci"])
        .output()
        .expect("failed to execute the cutver binary");

    assert_ne!(
        out.status.code(),
        Some(2),
        "`--no-ci` is not a usage error here: {}",
        stderr_of(&out)
    );
    assert_eq!(out.status.code(), Some(1), "the missing config is doctor's own failure");
    let stderr = stderr_of(&out);
    assert!(!stderr.contains("::error::"), "no annotation under `--no-ci`: {stderr}");
    let envelope: serde_json::Value = serde_json::from_str(stderr.trim())
        .unwrap_or_else(|e| panic!("the machine failure is one line without an annotation, got {stderr:?}: {e}"));
    assert_eq!(envelope["error"]["code"], "config");
}

/// S4: `--no-ci` keeps doctor's own exit code on a run that found drift — `2` — and writes nothing.
#[test]
fn no_ci_keeps_doctors_drift_exit_code_and_fills_no_sink() {
    let workspace = drifting_manifests();
    let sinks = Sinks::new("no-ci-drift");

    let out = cutver_command(workspace.path())
        .env("GITHUB_ACTIONS", "true")
        .env("GITHUB_OUTPUT", sinks.output())
        .env("GITHUB_STEP_SUMMARY", sinks.summary())
        .args(["doctor", "--json", "--no-ci"])
        .output()
        .expect("failed to execute the cutver binary");

    assert_eq!(out.status.code(), Some(2), "drift is doctor's own outcome");
    assert_eq!(stderr_of(&out), "", "drift is not an error: no annotation, no stderr");
    assert!(!sinks.output().exists(), "`--no-ci` writes no output file");
    assert!(!sinks.summary().exists(), "`--no-ci` writes no summary");
}

/// S2, S3: a finished `bump` feeds both sinks: the keys the workflow extracts today, `channel`
/// included, and a bounded block saying what was released.
#[test]
fn bump_fills_both_sinks_with_what_it_released() {
    let guard = repo("sinks-bump", TOML);
    let fixture = guard.fixture();
    let sinks = Sinks::new("sinks-bump");

    let out = with_ci(&fixture.dir, &["bump", "minor"], &sinks);

    assert_eq!(out.status.code(), Some(0), "stderr:\n{}", stderr_of(&out));
    let commit = head_commit(fixture);
    assert_eq!(
        sinks.output_text(),
        format!("released=true\ndeferred=false\nversion=1.3.0\nchannel=stable\ntag=v1.3.0\ncommit={commit}\n")
    );
    assert_eq!(
        sinks.summary_text(),
        format!(
            "### cutver bump\n\n- released: `true`\n- deferred: `false`\n- version: `1.3.0`\n- channel: `stable`\n- tag: `v1.3.0`\n- commit: `{commit}`\n"
        )
    );
}

/// S6: a prerelease reports the channel its version carries, so a downstream job reads `channel`
/// instead of splitting `version` apart. This is the alpha the ladder's first rung cuts.
#[test]
fn a_prerelease_bump_reports_the_channel_its_version_carries() {
    let guard = repo("sinks-bump-alpha", TOML);
    let fixture = guard.fixture();
    let sinks = Sinks::new("sinks-bump-alpha");

    let out = with_ci(&fixture.dir, &["bump", "minor", "--channel", "alpha"], &sinks);

    assert_eq!(out.status.code(), Some(0), "stderr:\n{}", stderr_of(&out));
    let commit = head_commit(fixture);
    assert_eq!(
        sinks.output_text(),
        format!(
            "released=true\ndeferred=false\nversion=1.3.0-alpha.1\nchannel=alpha\ntag=v1.3.0-alpha.1\ncommit={commit}\n"
        ),
        "the version carries its own channel: `channel=alpha` is the field that keeps a consumer from \
         re-parsing the suffix it reads beside it"
    );
    assert_eq!(
        sinks.summary_text(),
        format!(
            "### cutver bump\n\n- released: `true`\n- deferred: `false`\n- version: `1.3.0-alpha.1`\n- channel: `alpha`\n- tag: `v1.3.0-alpha.1`\n- commit: `{commit}`\n"
        )
    );
}

/// #326 D1a: a request that yields the version the fixture already released releases nothing, and the
/// channel says so on both sinks — `released=false`, no `commit` line because there is no commit, and
/// a brief that never claims a release. `bump --channel stable` with no level named is the promotion
/// re-run a channel matrix does, so a workflow keying on `released` must not be told it published.
#[test]
fn a_no_op_bump_fills_both_sinks_with_no_release() {
    let guard = repo("sinks-no-op", TOML);
    let fixture = guard.fixture();
    let sinks = Sinks::new("sinks-no-op");
    let head_before = head_commit(fixture);

    let out = with_ci(&fixture.dir, &["bump", "--channel", "stable"], &sinks);

    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr:
{}",
        stderr_of(&out)
    );
    assert_eq!(
        sinks.summary_text(),
        "### cutver bump

- released: `false`
- deferred: `false`
- version: `1.2.3`
- channel: `stable`
- tag: `v1.2.3`
"
    );
    assert_eq!(
        sinks.output_text(),
        "released=false
deferred=false
version=1.2.3
channel=stable
tag=v1.2.3
"
    );
    assert_eq!(
        head_commit(fixture),
        head_before,
        "the run it reports as not released created no commit"
    );
}

/// S2: a value the run does not have is omitted rather than written empty — a `--dry-run` bump
/// created no commit, so there is no `commit` line at all.
#[test]
fn a_dry_run_omits_the_commit_it_does_not_have() {
    let guard = repo("outputs-dry-run", TOML);
    let fixture = guard.fixture();
    let sinks = Sinks::new("outputs-dry-run");

    let out = with_ci(&fixture.dir, &["bump", "minor", "--dry-run"], &sinks);

    assert_eq!(out.status.code(), Some(0), "stderr:\n{}", stderr_of(&out));
    assert_eq!(
        sinks.output_text(),
        "released=false\ndeferred=false\nversion=1.3.0\nchannel=stable\ntag=v1.3.0\n"
    );
}

/// S2, S3: `plan` appends its keys after what the file already held, and writes the brief.
#[test]
fn plan_appends_its_four_keys_and_writes_the_brief() {
    let guard = repo("sinks-plan", TOML);
    let fixture = guard.fixture();
    let sinks = Sinks::new("sinks-plan");
    fs::write(sinks.output(), "earlier=kept\n").expect("the sink is seeded");

    let out = with_ci(&fixture.dir, &["plan", "minor"], &sinks);

    assert_eq!(out.status.code(), Some(0), "stderr:\n{}", stderr_of(&out));
    let commit = head_commit(fixture);
    assert_eq!(
        sinks.output_text(),
        format!("earlier=kept\nnext_version=1.3.0\nchannel=stable\ntag=v1.3.0\ncommit={commit}\n")
    );
    assert_eq!(
        sinks.summary_text(),
        format!(
            "### cutver plan\n\n- level: `minor`\n- next_version: `1.3.0`\n- channel: `stable`\n- tag: `v1.3.0`\n- commit: `{commit}`\n- blockers: `0`\n"
        )
    );
}

/// The fixture config plus what the closing phase needs to have something to report: a floating
/// major tag that moves, and one `[publish] command` that runs.
fn publishing_config(push: bool) -> String {
    let mut config = TOML.replace(
        "[git]\ntag_prefix = \"v\"",
        "[git]\ntag_prefix = \"v\"\nfloating_major_tag = true",
    );
    config.push_str(&format!(
        "\n[publish]\npush = {push}\ncommands = [\"echo published {{version}} > published.txt\"]\n"
    ));
    config
}

/// A repository with a bare `origin`, so a closing phase that `[publish] push` asks to push has
/// somewhere to push to. The remote owns its own temp directory, so the caller keeps it alive.
fn publishing_repo(name: &str) -> (FixtureGuard, Fixture) {
    let remote = Fixture::new(&format!("{name}-remote"));
    run_git_ok(&remote.dir, &["init", "--bare", "-q"]);
    let guard = repo(name, &publishing_config(true));
    run_git_ok(
        &guard.fixture().dir,
        &["remote", "add", "origin", remote.dir.to_str().expect("a utf-8 path")],
    );
    (guard, remote)
}

/// The preparing bump, outside the channel: these tests are about the closing phase.
fn prepare(fixture: &Fixture) {
    let out = plain(&fixture.dir, &["bump", "minor", "--defer-publish"]);
    assert_eq!(out.status.code(), Some(0), "stderr:\n{}", stderr_of(&out));
}

/// S2, S3: `publish` — the irreversible phase — appends what it did to the same file the preparing
/// `bump` wrote, and adds a block of its own to the summary.
#[test]
fn publish_fills_both_sinks_with_what_it_published() {
    let guard = repo("sinks-publish", &publishing_config(false));
    let fixture = guard.fixture();
    let sinks = Sinks::new("sinks-publish");

    let prepared = with_ci(&fixture.dir, &["bump", "minor", "--defer-publish"], &sinks);
    assert_eq!(prepared.status.code(), Some(0), "stderr:\n{}", stderr_of(&prepared));
    let out = with_ci(&fixture.dir, &["publish", "--tag", "v1.3.0"], &sinks);
    assert_eq!(out.status.code(), Some(0), "stderr:\n{}", stderr_of(&out));

    let commit = head_commit(fixture);
    assert_eq!(
        sinks.output_text(),
        format!(
            "released=true\ndeferred=true\nversion=1.3.0\nchannel=stable\ntag=v1.3.0\ncommit={commit}\n\
             tag=v1.3.0\ncommit={commit}\ntag_pushed=false\nfloating_tag_moved=true\npublish_commands_run=1\n"
        ),
        "the closing phase appends its own lines after the preparing bump's, never truncating them"
    );
    assert_eq!(
        sinks.summary_text(),
        format!(
            "### cutver bump\n\n- released: `true`\n- deferred: `true`\n- version: `1.3.0`\n- channel: `stable`\n- tag: `v1.3.0`\n- commit: `{commit}`\n\
             ### cutver publish\n\n- tag: `v1.3.0`\n- commit: `{commit}`\n- tag_pushed: `false`\n- floating_tag_moved: `true`\n- publish_commands_run: `1`\n"
        )
    );
}

/// S2: a `--dry-run` published nothing, so every field that reports an action says so — even though
/// the run it printed planned the tag push, the floating tag move and the command. Stdout is
/// byte-for-byte the same with the channel on and off.
#[test]
fn a_dry_run_publishes_nothing_and_reports_nothing_done() {
    let (guard, _remote) = publishing_repo("publish-dry-run");
    let fixture = guard.fixture();
    let sinks = Sinks::new("publish-dry-run");
    prepare(fixture);

    let off = plain(&fixture.dir, &["publish", "--tag", "v1.3.0", "--dry-run"]);
    assert_eq!(off.status.code(), Some(0), "stderr:\n{}", stderr_of(&off));
    let on = with_ci(&fixture.dir, &["publish", "--tag", "v1.3.0", "--dry-run"], &sinks);
    assert_eq!(on.status.code(), Some(0), "stderr:\n{}", stderr_of(&on));
    assert!(!off.stdout.is_empty(), "the dry run printed something to compare");
    assert_eq!(off.stdout, on.stdout, "CI adds no byte to `publish --dry-run`");

    let commit = head_commit(fixture);
    assert_eq!(
        sinks.output_text(),
        format!("tag=v1.3.0\ncommit={commit}\ntag_pushed=false\nfloating_tag_moved=false\npublish_commands_run=0\n"),
        "a dry run pushed nothing, moved nothing and ran no command, whatever the config declares"
    );
}

/// S2: the same fields on the run that really pushed the tag say `true`, so the pair is a fact about
/// the run and not a constant.
#[test]
fn a_real_publish_reports_the_tag_it_pushed() {
    let (guard, _remote) = publishing_repo("publish-pushed");
    let fixture = guard.fixture();
    let sinks = Sinks::new("publish-pushed");
    prepare(fixture);

    let out = with_ci(&fixture.dir, &["publish", "--tag", "v1.3.0"], &sinks);
    assert_eq!(out.status.code(), Some(0), "stderr:\n{}", stderr_of(&out));

    let commit = head_commit(fixture);
    assert_eq!(
        sinks.output_text(),
        format!("tag=v1.3.0\ncommit={commit}\ntag_pushed=true\nfloating_tag_moved=true\npublish_commands_run=1\n")
    );
}

/// S3: the release has already happened when the channel is written, so a sink that cannot be
/// opened warns and the run keeps its exit code — and its other sink.
#[test]
fn an_unwritable_sink_does_not_fail_a_finished_publish() {
    let guard = repo("publish-degraded", &publishing_config(false));
    let fixture = guard.fixture();
    let sinks = Sinks::new("publish-degraded");
    prepare(fixture);
    let unwritable = sinks.output().join("missing");

    let out = cutver_command(&fixture.dir)
        .env("GITHUB_ACTIONS", "true")
        .env("GITHUB_OUTPUT", &unwritable)
        .env("GITHUB_STEP_SUMMARY", sinks.summary())
        .args(["publish", "--tag", "v1.3.0"])
        .output()
        .expect("failed to execute the cutver binary");

    assert_eq!(
        out.status.code(),
        Some(0),
        "the release finished; a sink is bookkeeping: {}",
        stderr_of(&out)
    );
    assert!(
        stdout_of(&out).contains("v1.3.0"),
        "the human summary is still printed: {}",
        stdout_of(&out)
    );
    assert!(
        stderr_of(&out).contains("could not write"),
        "the sink degrades to a warning: {}",
        stderr_of(&out)
    );
    assert!(!unwritable.exists(), "the sink that cannot be opened stays absent");
    assert!(
        sinks.summary_text().contains("### cutver publish"),
        "the sink that can be opened is still filled: {}",
        sinks.summary_text()
    );
}

/// S2: only a closing phase that succeeded speaks. A refused publish writes neither sink, so a
/// consumer never reads a field describing a release that did not happen.
#[test]
fn a_refused_publish_writes_no_output_and_no_summary() {
    let guard = repo("publish-refused", &publishing_config(false));
    let fixture = guard.fixture();
    let sinks = Sinks::new("publish-refused");
    prepare(fixture);

    let out = with_ci(&fixture.dir, &["publish", "--tag", "v9.9.9"], &sinks);

    assert_eq!(out.status.code(), Some(1), "stdout:\n{}", stdout_of(&out));
    assert!(!sinks.output().exists(), "a refused run writes no output file");
    assert!(!sinks.summary().exists(), "a refused run writes no summary");
}

/// S3: the block is bounded and shaped — a heading and one bullet per field, never anything a value
/// brought with it. `src/cli/ci/tests.rs` pins the escaping rules themselves, because a successful
/// run has no field that user content could restructure.
#[test]
fn the_summary_is_a_heading_and_bullets_over_a_bounded_block() {
    let guard = repo("summary-bound", TOML);
    let fixture = guard.fixture();
    let sinks = Sinks::new("summary-bound");
    let mut changelog = String::from("# Changelog\n\n## [Unreleased]\n\n");
    for line in 0..200 {
        changelog.push_str(&format!("- evidence line {line}\n## injected heading {line}\n"));
    }
    fixture.write("CHANGELOG.md", &changelog);

    let out = with_ci(&fixture.dir, &["plan", "minor"], &sinks);

    assert_eq!(out.status.code(), Some(0), "stderr:\n{}", stderr_of(&out));
    let summary = sinks.summary_text();
    assert!(
        summary.len() < 1024,
        "the block is bounded, found {} bytes: {summary}",
        summary.len()
    );
    for line in summary.lines() {
        let shaped = line.is_empty() || line.starts_with("- ") || line.starts_with("### ");
        assert!(shaped, "nothing but a field may open a line: {line:?} in {summary}");
    }
    assert!(
        !summary.contains("evidence line") && !summary.contains("injected heading"),
        "no changelog text reaches the summary: {summary}"
    );
}

/// S2, S3: a run that cannot compute its result writes neither file, and the failure is reported by
/// the annotation instead, on one escaped line.
#[test]
fn a_failed_plan_writes_no_output_and_no_summary() {
    let config = TOML.replace(
        "tag_prefix = \"v\"",
        "tag_prefix = \"v` [click](https://example.test)\"",
    );
    let guard = repo("summary-failed", &config);
    let fixture = guard.fixture();
    let sinks = Sinks::new("summary-failed");

    let out = with_ci(&fixture.dir, &["plan", "minor"], &sinks);

    assert_eq!(out.status.code(), Some(1), "stdout:\n{}", stdout_of(&out));
    assert!(!sinks.output().exists(), "a failed run writes no output file");
    assert!(!sinks.summary().exists(), "a failed run writes no summary");
    let stderr = stderr_of(&out);
    let annotation = stderr.lines().next().expect("the failure is annotated");
    assert!(
        annotation.starts_with("::error::cutver plan cannot report the notes for"),
        "the annotation carries the human text: {annotation}"
    );
    assert!(
        annotation.contains("%0A") && annotation.contains("[click]"),
        "the text stays on one line and keeps its characters: {annotation}"
    );
}

/// S4: one `::error::` annotation on stderr carries the human text on one escaped line.
#[test]
fn a_failure_annotates_stderr_with_the_human_text() {
    let workspace = TempDir::new().expect("a temporary workspace");
    let sinks = Sinks::new("annotate-human");

    let out = with_ci(workspace.path(), &["bump"], &sinks);

    assert_eq!(out.status.code(), Some(1));
    let stderr = stderr_of(&out);
    let annotation = stderr.lines().next().expect("the annotation is the first line");
    assert!(
        annotation.starts_with("::error::Error loading config: no cutver.toml found"),
        "the annotation carries the human text: {annotation}"
    );
    assert!(
        annotation.contains("%0A"),
        "newlines are escaped so the annotation is one line: {annotation}"
    );
    assert_eq!(
        stderr.matches("::error::").count(),
        1,
        "exactly one annotation per failure: {stderr}"
    );
    assert!(
        stderr.contains("Error loading config: no cutver.toml found"),
        "the human line stays in the log: {stderr}"
    );
}

/// S4: the annotation precedes the machine envelope on its own line, and nothing is added to stdout.
#[test]
fn the_annotation_precedes_the_machine_envelope_and_never_touches_stdout() {
    let workspace = TempDir::new().expect("a temporary workspace");
    let sinks = Sinks::new("annotate-envelope");

    let out = with_ci(workspace.path(), &["bump", "--json"], &sinks);

    assert_eq!(out.status.code(), Some(1));
    assert!(
        out.stdout.is_empty(),
        "the CI channel never writes stdout: {:?}",
        stdout_of(&out)
    );
    let stderr = stderr_of(&out);
    let mut lines = stderr.lines();
    let annotation = lines.next().expect("the annotation is first");
    assert!(annotation.starts_with("::error::"), "{annotation}");
    let envelope: serde_json::Value =
        serde_json::from_str(lines.next().expect("the envelope follows")).expect("the envelope is one JSON object");
    assert_eq!(envelope["schemaVersion"], 1);
    assert_eq!(envelope["error"]["code"], "config");
    assert_eq!(envelope["error"]["exitCode"], 1);
}

/// S4, PRESERVE: only a failure on a detected CI annotates. A successful run says nothing, and
/// without a single CI variable the human failure is exactly what it always was.
#[test]
fn only_a_failure_on_a_detected_ci_annotates() {
    let guard = repo("annotate-clean", TOML);
    let fixture = guard.fixture();
    let sinks = Sinks::new("annotate-clean");

    let clean = with_ci(&fixture.dir, &["plan", "minor"], &sinks);
    assert_eq!(clean.status.code(), Some(0), "stderr:\n{}", stderr_of(&clean));
    assert!(stderr_of(&clean).is_empty(), "a clean run has nothing to annotate");

    let workspace = TempDir::new().expect("a temporary workspace");
    let human = plain(workspace.path(), &["bump"]);
    assert_eq!(human.status.code(), Some(1));
    let stderr = stderr_of(&human);
    assert!(
        stderr.starts_with("Error loading config:"),
        "the rendered error keeps its marker: {stderr:?}"
    );
    assert!(!stderr.contains("::error::"), "no annotation outside CI: {stderr}");
    assert!(human.stdout.is_empty(), "a config failure prints no stdout");
}

/// S5: stdout is byte-for-byte identical with CI detected and without it — `plan --json`,
/// `bump --json` and a failing command.
#[test]
fn stdout_is_byte_identical_with_and_without_ci() {
    let guard = repo("stdout-bytes", TOML);
    let fixture = guard.fixture();
    let sinks = Sinks::new("stdout-bytes");

    let off = plain(&fixture.dir, &["plan", "minor", "--json"]);
    let on = with_ci(&fixture.dir, &["plan", "minor", "--json"], &sinks);
    assert_eq!(off.status.code(), on.status.code(), "the exit code is unchanged");
    assert!(!off.stdout.is_empty(), "the plan printed something to compare");
    assert_eq!(off.stdout, on.stdout, "CI adds no byte to `plan --json`");

    // `--dry-run` mutates nothing, so the same workspace can answer twice.
    let off = plain(&fixture.dir, &["bump", "minor", "--dry-run", "--json"]);
    let on = with_ci(&fixture.dir, &["bump", "minor", "--dry-run", "--json"], &sinks);
    assert_eq!(off.status.code(), on.status.code(), "the exit code is unchanged");
    assert!(!off.stdout.is_empty(), "the bump printed something to compare");
    assert_eq!(off.stdout, on.stdout, "CI adds no byte to `bump --json`");

    let workspace = TempDir::new().expect("a temporary workspace");
    let off = plain(workspace.path(), &["changelog", "latest", "--json"]);
    let on = with_ci(workspace.path(), &["changelog", "latest", "--json"], &sinks);
    assert_eq!(off.status.code(), Some(1), "the failure still exits 1");
    assert_eq!(off.stdout, on.stdout, "CI adds no byte to a failing run either");
}

/// S6: with `GITHUB_ACTIONS` absent the channel is closed — no file opened, no annotation, no path
/// created — even when the two file variables name paths that do not exist yet.
#[test]
fn outside_ci_nothing_is_created_opened_or_annotated() {
    let guard = repo("no-ci-side-effects", TOML);
    let fixture = guard.fixture();
    let sinks = Sinks::new("no-ci-side-effects");
    let before = snapshot(&fixture.dir);

    let out = without_github_actions(&fixture.dir, &["plan", "minor"], &sinks);
    assert_eq!(out.status.code(), Some(0), "stderr:\n{}", stderr_of(&out));
    assert!(!stderr_of(&out).contains("::error::"), "a success annotates nothing");

    let workspace = TempDir::new().expect("a temporary workspace");
    let failing = without_github_actions(workspace.path(), &["bump"], &sinks);
    assert_eq!(failing.status.code(), Some(1));
    assert!(
        !stderr_of(&failing).contains("::error::"),
        "a failure outside CI prints the human line only"
    );

    assert!(!sinks.output().exists(), "`$GITHUB_OUTPUT` may not be created");
    assert!(!sinks.summary().exists(), "`$GITHUB_STEP_SUMMARY` may not be created");
    assert_eq!(snapshot(&fixture.dir), before, "a run outside CI touches no path");
}
