//! `cutver publish --json`: the closing phase as one machine object (#404 S1, S2, S5, S6).
//!
//! The plan tree a human reads names the tag, the floating major tag, the pushes and the
//! `[publish] commands`; a wrapper that drives the release tail from a subprocess can read none of
//! it, because the tree is prose and the exit code says only "worked" or "did not". This suite pins
//! the object that carries the same facts: `schemaVersion` first, the refs actually pushed as a list,
//! the exit code of every command, and the `on_post_release` dispatch outcome.
//!
//! Two properties are asserted here that no single-object golden can state:
//!
//! - a refusal writes **nothing** on stdout and one envelope on stderr, so a consumer never reads
//!   half a document (S2, S9);
//! - `--dry-run` changes the values and never the keys (S5), and the human path without the flag is
//!   byte-for-byte the tree it always printed (S6).
//!
//! Every spawn goes through `common::cutver_command`, which removes the CI channel's variables, and
//! the style variables are removed too: the bytes compared below are the plain ones, whatever a
//! runner exports for its own shell.

mod common;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Output;

use common::ci::{TOML, head_commit, repo};
use common::{
    CARGO_TOML, CHANGELOG_MD, Fixture, FixtureGuard, PACKAGE_JSON, cutver_command, init_git_repo, initial_commit,
    run_git_ok, tag_exists,
};
use serde_json::Value;

/// The first bytes of a compact surface: the version is the first key, so a consumer reads it before
/// the payload it may not understand (`tests/schema_compat.rs` states the same rule for every
/// surface).
const COMPACT_PREFIX: &str = r#"{"schemaVersion":1,"#;

/// The keys `cutver publish --json` must carry: the tag and the commit published, whether the run was
/// a dry run, whether it skipped a tag that already existed, the refs it pushed, the floating major
/// tag and whether it moved, every `[publish] commands` result, and the `on_post_release` outcome.
///
/// The assertion is `emitted ⊇ pinned`, never equality: a key added later is not a new schema
/// version, while a key that disappears or is renamed leaves a consumer reading `null`.
const PUBLISH_KEYS: [&str; 9] = [
    "tag",
    "commit",
    "dry_run",
    "tag_skipped",
    "pushed_refs",
    "floating_tag",
    "floating_tag_moved",
    "publish_commands",
    "post_release",
];

/// The one `[publish] command` every fixture declares, in its config spelling.
const COMMAND_TEMPLATE: &str = "echo published {version} > published.txt";

/// The same command after interpolation, which is what a run reports.
const COMMAND_RENDERED: &str = "echo published 1.3.0 > published.txt";

/// The same command for a workspace still at the version the manifest declares.
const COMMAND_RENDERED_AT_HEAD: &str = "echo published 1.2.3 > published.txt";

/// The CI fixture's config, plus what the closing phase has to report: a floating major tag it moves
/// and one `[publish] command` it runs.
fn closing_config(push: bool) -> String {
    let mut config = TOML.replace(
        "[git]\ntag_prefix = \"v\"",
        "[git]\ntag_prefix = \"v\"\nfloating_major_tag = true",
    );
    config.push_str(&format!(
        "\n[publish]\npush = {push}\ncommands = [\"{COMMAND_TEMPLATE}\"]\n"
    ));
    config
}

/// `cutver publish` with the style variables removed, so the tree it prints is compared as bytes.
fn publish(dir: &Path, args: &[&str]) -> Output {
    cutver_command(dir)
        .env_remove("NO_COLOR")
        .env_remove("CLICOLOR")
        .env_remove("CLICOLOR_FORCE")
        .arg("publish")
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

/// The object `text` holds, or a failure naming the surface that was supposed to be one.
fn object(text: &str, surface: &str) -> Value {
    serde_json::from_str(text.trim())
        .unwrap_or_else(|err| panic!("{surface} must be one JSON object, got {text:?}: {err}"))
}

/// The properties every publish object shares: one line, the version first at the documented bytes,
/// and no pinned key missing.
fn assert_publish_object(document: &str, surface: &str) -> Value {
    assert_eq!(
        document.lines().count(),
        1,
        "{surface} prints one compact object on stdout, got: {document:?}"
    );
    assert!(
        document.starts_with(COMPACT_PREFIX),
        "{surface} must open with `schemaVersion` at the bytes `{COMPACT_PREFIX}`: a consumer that \
         reads the version before the payload would otherwise have to parse the whole object first. \
         Got: {document:?}"
    );

    let parsed = object(document, surface);
    assert_eq!(parsed["schemaVersion"], 1, "{surface}: {document}");

    let emitted: BTreeSet<String> = parsed
        .as_object()
        .unwrap_or_else(|| panic!("{surface} must be a JSON object, got {document:?}"))
        .keys()
        .cloned()
        .collect();
    let missing: Vec<&str> = PUBLISH_KEYS
        .iter()
        .copied()
        .filter(|key| !emitted.contains(*key))
        .collect();
    assert!(
        missing.is_empty(),
        "{surface} no longer emits {missing:?}. A consumer that already reads a missing key silently \
         starts getting `null` instead of a value, and nothing in the release failed to say so. \
         Emitted keys: {emitted:?}"
    );

    // Field order is part of the contract too, so it is pinned by position rather than by equality:
    // a key added later does not fail here, a reordered one does.
    assert_keys_in_order(document, &PUBLISH_KEYS, surface);
    assert_keys_in_order(document, &["position", "command", "exit_code"], surface);
    parsed
}

/// Asserts every key of `keys` appears in `document` after the one before it, in one pass.
fn assert_keys_in_order(document: &str, keys: &[&str], surface: &str) {
    let mut cursor = 0;
    for key in keys {
        let needle = format!("\"{key}\":");
        match document[cursor..].find(&needle) {
            Some(index) => cursor += index,
            None => panic!("{surface} must carry `{key}` after every key before it, got: {document:?}"),
        }
    }
}

/// A workspace whose `[publish] command` has already run against a release cut by `bump`, and a bare
/// `origin` for a run that `[publish] push` asks to push.
fn released_workspace(name: &str, push: bool) -> (FixtureGuard, Fixture) {
    assert!(
        common::git_available(),
        "the git CLI is required to close a release but was not found in PATH"
    );
    let remote = Fixture::new(&format!("{name}-remote"));
    run_git_ok(&remote.dir, &["init", "--bare", "-q"]);
    let guard = repo(name, &closing_config(push));
    run_git_ok(
        &guard.fixture().dir,
        &["remote", "add", "origin", remote.dir.to_str().expect("a utf-8 path")],
    );
    let prepared = cutver_command(&guard.fixture().dir)
        .args(["bump", "minor", "--defer-publish"])
        .output()
        .expect("failed to execute cutver bump");
    assert_eq!(
        prepared.status.code(),
        Some(0),
        "the preparing bump must succeed, stderr:\n{}",
        stderr_of(&prepared)
    );
    (guard, remote)
}

/// S1, S5: a `--dry-run` reports the same keys a real run does, and the values say it published
/// nothing — no ref, no floating-tag move, no command run.
#[test]
fn a_dry_run_prints_one_object_with_every_key_the_rule_names() {
    let guard = repo("publish-json-dry-run", &closing_config(false));
    let fixture = guard.fixture();
    let commit = head_commit(fixture);

    let out = publish(&fixture.dir, &["--tag", "v1.2.3", "--dry-run", "--json"]);

    assert_eq!(out.status.code(), Some(0), "stderr:\n{}", stderr_of(&out));
    let document = stdout_of(&out);
    let parsed = assert_publish_object(&document, "`cutver publish --dry-run --json`");

    assert!(
        document.starts_with(&format!(
            r#"{{"schemaVersion":1,"tag":"v1.2.3","commit":"{commit}","dry_run":true,"#
        )),
        "the keys a consumer branches on come first and in that order, got: {document:?}"
    );
    assert_eq!(parsed["tag"], "v1.2.3");
    assert_eq!(parsed["commit"], commit.as_str());
    assert_eq!(parsed["dry_run"], true);
    assert_eq!(
        parsed["tag_skipped"], false,
        "a dry run creates no tag, but it skipped none either: `tag_skipped` is a real run's answer \
         about a tag that already existed"
    );

    assert_eq!(parsed["pushed_refs"], serde_json::json!([]), "a dry run pushes nothing");
    assert_eq!(parsed["floating_tag"], "v1");
    assert_eq!(
        parsed["floating_tag_moved"], false,
        "a dry run moves no floating major tag, so it must not report one moved"
    );

    let commands = parsed["publish_commands"]
        .as_array()
        .unwrap_or_else(|| panic!("`publish_commands` is the list of results: {document}"));
    assert_eq!(commands.len(), 1, "the one declared command: {document}");
    assert_eq!(commands[0]["position"], 1, "the position is 1-based: {document}");
    assert_eq!(commands[0]["command"], COMMAND_RENDERED_AT_HEAD);
    assert!(
        commands[0].get("exit_code").is_some_and(Value::is_null),
        "a command that did not run has no exit code, and the key is still there so an adapter \
         parses one shape: {document}"
    );

    let post_release = parsed["post_release"]
        .as_object()
        .unwrap_or_else(|| panic!("`post_release` is the dispatch outcome: {document}"));
    assert!(post_release.contains_key("dispatched"), "{document}");
    assert!(post_release.contains_key("left_alone"), "{document}");

    assert!(
        !fixture.dir.join("published.txt").exists(),
        "the object must describe the plan without running it"
    );
    assert!(!tag_exists(fixture, "v1.2.3"), "a dry run creates no tag");
}

/// S1: a real run reports the refs `git` really received, the floating major tag it moved, and the
/// code each command's child exited with.
#[test]
fn a_real_publish_reports_the_refs_it_pushed_and_the_code_each_command_exited_with() {
    let (guard, _remote) = released_workspace("publish-json-real", true);
    let fixture = guard.fixture();
    let commit = head_commit(fixture);

    let out = publish(&fixture.dir, &["--tag", "v1.3.0", "--json"]);

    assert_eq!(out.status.code(), Some(0), "stderr:\n{}", stderr_of(&out));
    let document = stdout_of(&out);
    let parsed = assert_publish_object(&document, "`cutver publish --json`");

    assert_eq!(parsed["tag"], "v1.3.0");
    assert_eq!(parsed["commit"], commit.as_str());
    assert_eq!(parsed["dry_run"], false);
    assert_eq!(
        parsed["tag_skipped"], false,
        "a fresh release creates its tag: {document}"
    );

    assert_eq!(
        parsed["pushed_refs"],
        serde_json::json!(["refs/tags/v1.3.0", "+refs/tags/v1:refs/tags/v1"]),
        "the refs are the refspecs `git` received, in push order, never the ` && `-joined command \
         line: {document}"
    );
    assert_eq!(parsed["floating_tag"], "v1");
    assert_eq!(parsed["floating_tag_moved"], true);

    let commands = parsed["publish_commands"]
        .as_array()
        .unwrap_or_else(|| panic!("`publish_commands` is the list of results: {document}"));
    assert_eq!(commands.len(), 1, "{document}");
    assert_eq!(commands[0]["position"], 1);
    assert_eq!(commands[0]["command"], COMMAND_RENDERED);
    assert_eq!(
        commands[0]["exit_code"], 0,
        "the command ran and exited 0, and the object reports that code: {document}"
    );

    assert_eq!(
        fixture.read("published.txt").trim(),
        "published 1.3.0",
        "the reported command really ran"
    );
}

/// The `lifecycle.v1` probe the dispatch fixture declares: it drains its envelope and answers one
/// JSON object, which is all the host reads back. It is written for both platforms so the assertion
/// runs wherever the suite runs.
const PROBE_UNIX: &str = "#!/bin/sh\ncat >/dev/null\nprintf '{}'\n";
const PROBE_WINDOWS: &str = "@echo off\r\nfindstr . >nul\r\necho {}\r\n";

fn probe_script() -> &'static str {
    if cfg!(windows) { PROBE_WINDOWS } else { PROBE_UNIX }
}

fn probe_name() -> &'static str {
    if cfg!(windows) { "probe.cmd" } else { "probe.sh" }
}

/// Marks the POSIX probe executable. A batch file needs no mode bit.
fn make_executable(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(path)
            .expect("the fixture script exists")
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(path, perms).expect("the fixture script is executable");
    }
    #[cfg(not(unix))]
    let _ = path;
}

/// A workspace declaring one `on_post_release` plugin, so the dispatch outcome is observable.
fn plugin_workspace(name: &str) -> FixtureGuard {
    let guard = FixtureGuard::new(name);
    let fixture = guard.fixture();
    let script: PathBuf = fixture.dir.join(probe_name());
    fixture.write(probe_name(), probe_script());
    make_executable(&script);

    let command = script.to_string_lossy().replace('\\', "/");
    fixture.write("package.json", PACKAGE_JSON);
    fixture.write("Cargo.toml", CARGO_TOML);
    fixture.write("CHANGELOG.md", CHANGELOG_MD);
    fixture.write(
        "cutver.toml",
        &format!(
            "{}\n[plugins.probe]\nruntime = \"process\"\ncommand = \"{command}\"\ncapabilities = [\"lifecycle.v1\"]\nevents = [\"on_post_release\"]\n\n[plugins.probe.permissions]\nenv = [\"PATH\"]\n",
            closing_config(false)
        ),
    );
    init_git_repo(fixture);
    run_git_ok(&fixture.dir, &["checkout", "-B", "main"]);
    initial_commit(fixture);
    guard
}

/// S1: the object names which `on_post_release` plugins the run dispatched, not only the ones it
/// left alone.
#[test]
fn a_declared_on_post_release_plugin_is_reported_as_dispatched() {
    let guard = plugin_workspace("publish-json-dispatch");
    let fixture = guard.fixture();

    let out = publish(&fixture.dir, &["--tag", "v1.2.3", "--dry-run", "--json"]);

    assert_eq!(out.status.code(), Some(0), "stderr:\n{}", stderr_of(&out));
    let document = stdout_of(&out);
    let parsed = assert_publish_object(&document, "`cutver publish --dry-run --json`");

    assert_eq!(
        parsed["post_release"]["dispatched"],
        serde_json::json!(["probe"]),
        "the plugin the workspace declares for the event is named as dispatched: {document}"
    );
    assert_eq!(
        parsed["post_release"]["left_alone"],
        serde_json::json!([]),
        "a plain run leaves no plugin alone: {document}"
    );
}

/// S2, S9: a refusal writes one object on **stderr** with the `publish` failure class in it, and adds
/// nothing at all to stdout — no tree, no partial object.
#[test]
fn a_refused_publish_prints_the_envelope_on_stderr_and_leaves_stdout_empty() {
    let guard = repo("publish-json-refusal", &closing_config(false));
    let fixture = guard.fixture();

    let out = publish(&fixture.dir, &["--tag", "v9.9.9", "--json"]);

    assert_eq!(out.status.code(), Some(1), "stderr:\n{}", stderr_of(&out));
    assert!(
        out.stdout.is_empty(),
        "a failure prints nothing on stdout, or a consumer reads half a document: {:?}",
        stdout_of(&out)
    );

    let stderr = stderr_of(&out);
    assert_eq!(
        stderr.lines().count(),
        1,
        "the failure is one compact object and no prose: {stderr:?}"
    );
    assert!(
        stderr.starts_with(COMPACT_PREFIX),
        "the envelope opens with `schemaVersion` too: {stderr:?}"
    );
    let envelope = object(&stderr, "the failure envelope");
    assert_eq!(envelope["error"]["code"], "publish", "{stderr}");
    assert_eq!(envelope["error"]["exitCode"], 1, "{stderr}");
    assert!(
        !stderr.contains("Publish Plan") && !stderr.contains("Error: "),
        "the machine path replaces the tree and the rendered line, it does not accompany them: \
         {stderr:?}"
    );
    assert!(
        !fixture.dir.join("published.txt").exists() && !tag_exists(fixture, "v9.9.9"),
        "a refused run publishes nothing"
    );
}

/// S6: without the flag the closing phase prints exactly the tree it always printed, so `--json` is
/// the only difference between the two channels.
#[test]
fn a_dry_run_without_the_flag_still_prints_the_plan_tree() {
    let guard = repo("publish-json-human-dry-run", &closing_config(false));
    let fixture = guard.fixture();
    let commit = head_commit(fixture);

    let human = publish(&fixture.dir, &["--tag", "v1.2.3", "--dry-run"]);
    assert_eq!(human.status.code(), Some(0), "stderr:\n{}", stderr_of(&human));
    assert_eq!(
        stdout_of(&human),
        format!(
            "┌─ SIMULATION MODE ──────────────────────────────────────────────────┐\n\
             │ No files, commits, or git tags will be modified.                   │\n\
             └────────────────────────────────────────────────────────────────────┘\n\
             \n\
             ℹ Publish Plan: v1.2.3 at {commit}\n\
             ├── Tag\n\
             │   └── v1.2.3\n\
             ├── Floating tag\n\
             │   └── v1\n\
             └── Commands\n\
             \x20   └── {COMMAND_RENDERED_AT_HEAD}\n"
        ),
        "the human path is byte-for-byte the tree it was"
    );

    let machine = publish(&fixture.dir, &["--tag", "v1.2.3", "--dry-run", "--json"]);
    assert_eq!(machine.status.code(), Some(0), "stderr:\n{}", stderr_of(&machine));
    let machine_stdout = stdout_of(&machine);
    for line in stdout_of(&human).lines().filter(|line| !line.is_empty()) {
        assert!(
            !machine_stdout.contains(line),
            "`--json` replaces the tree with one object and keeps none of its lines, found {line:?} in \
             {machine_stdout:?}"
        );
    }
}

/// S6: the same on the real path, where the header claims what the run published.
#[test]
fn a_real_publish_without_the_flag_still_prints_the_published_tree() {
    let (guard, _remote) = released_workspace("publish-json-human-real", false);
    let fixture = guard.fixture();
    let commit = head_commit(fixture);

    let out = publish(&fixture.dir, &["--tag", "v1.3.0"]);

    assert_eq!(out.status.code(), Some(0), "stderr:\n{}", stderr_of(&out));
    assert_eq!(
        stdout_of(&out),
        format!(
            "✔ Published: v1.3.0 at {commit}\n\
             ├── Tag\n\
             │   └── v1.3.0\n\
             ├── Floating tag\n\
             │   └── v1\n\
             └── Commands\n\
             \x20   └── {COMMAND_RENDERED}\n"
        ),
        "a real publish keeps its tree byte for byte"
    );
}
