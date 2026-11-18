//! `cutver plan`, end to end: the release the workspace is about to cut, reported as data (#326).
//!
//! The command's whole promise is that it changes nothing, so the central test snapshots every file
//! in the workspace before and after a plan over a dirty tree — with a `[publish] commands` entry and
//! a `pre_bump` hook whose command would write a file — and compares the bytes. Everything else pins
//! the document (`schemaVersion` first, the keys in order, `notes` really rendered) and the blocker
//! semantics (data, exit `0`).
//!
//! Not `#![cfg(unix)]`: the shell commands a plan must *not* run never run, so their portability is
//! irrelevant here — which is itself the property under test.

mod common;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use common::*;
use serde_json::Value;

/// The `[hooks]`/`[preflight]`/`[publish]` commands that would leave a file behind, named so a test
/// can assert each one's marker is absent afterwards.
const PRE_BUMP_MARKER: &str = "pre_bump_ran.txt";
const PREFLIGHT_MARKER: &str = "preflight_ran.txt";
const PUBLISH_MARKER: &str = "published.txt";

/// A configuration where every mutating step is armed: a preflight, both kinds of hook-shaped work and
/// a publish command. A plan must dispatch none of them.
fn armed_config() -> String {
    format!(
        r#"[version]
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"

[preflight]
check = "touch {PREFLIGHT_MARKER}"

[changelog]
path = "CHANGELOG.md"
entry_template = "Maintenance and updates."

[git]
tag_prefix = "v"
require_clean_tree = true
require_branch = "main"

[hooks]
pre_bump = "touch {PRE_BUMP_MARKER}"

[publish]
push = true
commands = ["touch {PUBLISH_MARKER}"]
"#
    )
}

fn fixture(name: &str, config: &str) -> FixtureGuard {
    assert!(
        git_available(),
        "git CLI is required for e2e tests but was not found in PATH"
    );
    let guard = FixtureGuard::new(name);
    let fixture = guard.fixture();
    fixture.write("package.json", PACKAGE_JSON);
    fixture.write("Cargo.toml", CARGO_TOML);
    fixture.write("CHANGELOG.md", CHANGELOG_MD);
    fixture.write("cutver.toml", config);
    init_git_repo(fixture);
    initial_commit(fixture);
    guard
}

/// Runs the cutver binary with the fixture as its working directory.
fn cutver_in(fixture: &Fixture, args: &[&str]) -> Output {
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

/// Every file the workspace holds, keyed by its path relative to the root.
///
/// `.git/index` and `.git/logs` are the two entries a *read* of the repository may rewrite on its own
/// (stat refresh and reflog bookkeeping); the refs, `HEAD` and every worktree file are compared byte
/// for byte, so a written file, a created tag or a created commit cannot slip through.
fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut files = BTreeMap::new();
    collect(root, root, &mut files);
    files
}

fn collect(root: &Path, dir: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>) {
    for entry in fs::read_dir(dir).expect("the workspace is readable") {
        let entry = entry.expect("the directory entry is readable");
        let path = entry.path();
        let relative = path.strip_prefix(root).expect("the path is inside the workspace");
        if relative == Path::new(".git/index") || relative.starts_with(".git/logs") {
            continue;
        }
        if entry.file_type().expect("the entry type is readable").is_dir() {
            collect(root, &path, files);
        } else {
            files.insert(relative.to_path_buf(), fs::read(&path).expect("the file is readable"));
        }
    }
}

fn porcelain(fixture: &Fixture) -> String {
    String::from_utf8_lossy(&run_git(&fixture.dir, &["status", "--porcelain"]).stdout).to_string()
}

fn head(fixture: &Fixture) -> String {
    String::from_utf8_lossy(&run_git(&fixture.dir, &["rev-parse", "HEAD"]).stdout)
        .trim()
        .to_string()
}

fn branch(fixture: &Fixture) -> String {
    String::from_utf8_lossy(&run_git(&fixture.dir, &["symbolic-ref", "--short", "HEAD"]).stdout)
        .trim()
        .to_string()
}

fn local_tags(fixture: &Fixture) -> String {
    String::from_utf8_lossy(&run_git(&fixture.dir, &["tag", "-l"]).stdout).to_string()
}

fn parse(output: &Output) -> Value {
    let text = stdout_of(output);
    serde_json::from_str(text.trim()).unwrap_or_else(|err| panic!("stdout must be one JSON object: {text:?}: {err}"))
}

/// The document's own keys, in the order the object prints them.
fn top_level_keys(raw: &str) -> Vec<String> {
    const DOCUMENT_KEYS: [&str; 17] = [
        "read_only",
        "level",
        "current_version",
        "next_version",
        "tag",
        "floating_tag",
        "commit",
        "branch",
        "commit_message",
        "notes",
        "changelog",
        "manifests",
        "preflight",
        "phases",
        "artifacts",
        "gates",
        "blockers",
    ];
    let mut found: Vec<(usize, String)> = DOCUMENT_KEYS
        .iter()
        .map(|key| {
            let needle = format!("\"{key}\":");
            let at = raw
                .find(&needle)
                .unwrap_or_else(|| panic!("the document carries `{key}`: {raw}"));
            (at, (*key).to_string())
        })
        .collect();
    found.sort();
    found.into_iter().map(|(_, key)| key).collect()
}

/// The acceptance criterion (#326 criterion 3): a plan over a dirty tree changes no file.
///
/// Every mutating surface is armed — a preflight, a `pre_bump` hook and a `[publish] commands` entry —
/// and every one of them would leave a marker file on disk if it ran.
#[test]
fn a_plan_over_a_dirty_tree_changes_no_file() {
    let guard = fixture("plan-dirty-tree", &armed_config());
    let fixture = guard.fixture();
    // The dirty tree a CI job plans in: an untracked file and a modified tracked one.
    fixture.write("wip.txt", "uncommitted work\n");
    fixture.write("package.json", &PACKAGE_JSON.replace("cutver-e2e", "cutver-e2e-wip"));

    let before = snapshot(&fixture.dir);
    let status_before = porcelain(fixture);
    let head_before = head(fixture);
    let tags_before = local_tags(fixture);

    let output = cutver_in(fixture, &["plan", "--json", "minor"]);

    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr_of(&output));
    let plan = parse(&output);
    assert_eq!(plan["next_version"], "1.3.0", "{plan}");
    let notes = plan["notes"].as_str().expect("notes is a string");
    assert!(
        !notes.trim().is_empty(),
        "the plan renders the entry it reports: {plan}"
    );
    assert!(notes.contains("1.3.0"), "the entry is the release's own: {notes}");

    let after = snapshot(&fixture.dir);
    assert_eq!(
        before, after,
        "a plan must leave every byte in the workspace as it found it"
    );
    assert_eq!(porcelain(fixture), status_before, "`git status` is unchanged");
    assert_eq!(head(fixture), head_before, "no commit was created");
    assert_eq!(local_tags(fixture), tags_before, "no tag was created");
    assert!(
        !fixture.dir.join(PRE_BUMP_MARKER).exists(),
        "`[hooks] pre_bump` did not run"
    );
    assert!(
        !fixture.dir.join(PREFLIGHT_MARKER).exists(),
        "`[preflight]` did not run"
    );
    assert!(
        !fixture.dir.join(PUBLISH_MARKER).exists(),
        "`[publish] commands` did not run"
    );
    let changelog = fixture.read("CHANGELOG.md");
    assert_eq!(
        changelog, CHANGELOG_MD,
        "the rendered entry was never written to the changelog"
    );
    assert!(
        !changelog.contains("[1.3.0]"),
        "the notes the plan reports are not in the file: {changelog}"
    );
}

/// S2 and S6: one versioned object, the documented keys in the documented order, the shape of every
/// entry, and `artifacts`/`gates` present and empty.
#[test]
fn the_plan_document_has_the_documented_keys_in_order() {
    let guard = fixture("plan-document", &armed_config());
    let fixture = guard.fixture();

    let output = cutver_in(fixture, &["plan", "--json", "minor"]);

    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr_of(&output));
    let raw = stdout_of(&output);
    assert!(
        raw.starts_with(r#"{"schemaVersion":1,"read_only":true,"level":"minor","#),
        "the versioned object opens schemaVersion first, then the plan: {raw}"
    );
    assert_eq!(
        top_level_keys(&raw),
        vec![
            "read_only",
            "level",
            "current_version",
            "next_version",
            "tag",
            "floating_tag",
            "commit",
            "branch",
            "commit_message",
            "notes",
            "changelog",
            "manifests",
            "preflight",
            "phases",
            "artifacts",
            "gates",
            "blockers",
        ],
        "{raw}"
    );

    let plan = parse(&output);
    assert_eq!(plan["read_only"], true);
    assert_eq!(plan["level"], "minor");
    assert_eq!(plan["current_version"], "1.2.3");
    assert_eq!(plan["next_version"], "1.3.0");
    assert_eq!(plan["tag"], "v1.3.0");
    assert_eq!(plan["commit"], head(fixture));
    assert_eq!(plan["branch"], branch(fixture));
    assert!(plan["commit_message"].as_str().expect("a string").contains("1.3.0"));
    assert_eq!(
        plan["changelog"],
        serde_json::json!({ "path": "CHANGELOG.md", "prepends": "1.3.0" })
    );
    assert_eq!(
        plan["manifests"],
        serde_json::json!([
            { "path": "package.json", "from": "1.2.3", "to": "1.3.0" },
            { "path": "Cargo.toml", "from": "1.2.3", "to": "1.3.0" },
        ])
    );
    assert_eq!(
        plan["preflight"],
        serde_json::json!([
            { "name": "check", "command": "touch preflight_ran.txt", "skipped": false },
        ])
    );

    let phases = plan["phases"].as_array().expect("phases is an array");
    let names: Vec<&str> = phases.iter().map(|phase| phase["name"].as_str().unwrap()).collect();
    assert_eq!(names, vec!["validate", "mutate", "publish"]);
    assert_eq!(
        phases
            .iter()
            .map(|phase| phase["mutates"].as_bool().unwrap())
            .collect::<Vec<_>>(),
        vec![false, true, true],
        "validate writes nothing; mutate and publish do"
    );
    for phase in phases {
        let steps = phase["steps"].as_array().expect("every phase lists its steps");
        assert!(!steps.is_empty(), "a phase lists what it would do: {phase}");
        for step in steps {
            assert!(step.is_string(), "a step is one line of text: {step}");
        }
    }
    assert!(
        phases[1]["steps"]
            .as_array()
            .unwrap()
            .iter()
            .any(|step| step.as_str().unwrap().contains("Cargo.toml")),
        "the mutate phase names the manifests the plan computes: {phases:?}"
    );

    assert_eq!(
        plan["artifacts"],
        serde_json::json!([]),
        "present and empty until phase 3"
    );
    assert_eq!(plan["gates"], serde_json::json!([]), "present and empty until phase 3");
    assert!(
        plan["notes"].as_str().expect("a string").contains("1.3.0"),
        "notes are rendered, never null: {plan}"
    );
}

/// S4: the guards a real bump aborts on are data, and a blocked plan still exits `0` (L6).
#[test]
fn blockers_are_data_and_a_blocked_plan_still_exits_zero() {
    let config = armed_config().replace("require_branch = \"main\"", "require_branch = \"release-only\"");
    let guard = fixture("plan-blockers", &config);
    let fixture = guard.fixture();
    fixture.write("wip.txt", "uncommitted work\n");
    let on_branch = branch(fixture);
    assert_ne!(on_branch, "release-only", "the fixture is not on the required branch");

    let json = cutver_in(fixture, &["plan", "--json"]);
    assert_eq!(json.status.code(), Some(0), "stderr: {}", stderr_of(&json));

    let plan = parse(&json);
    let blockers: Vec<&str> = plan["blockers"]
        .as_array()
        .expect("blockers is an array")
        .iter()
        .map(|blocker| blocker.as_str().expect("a blocker is one line of text"))
        .collect();
    assert_eq!(blockers.len(), 2, "one per guard: {blockers:?}");
    for blocker in &blockers {
        assert!(!blocker.contains('\n'), "a blocker is one line: {blocker:?}");
    }
    assert!(
        blockers.iter().any(|blocker| blocker.contains("require_clean_tree")),
        "the dirty tree is reported with its remedy: {blockers:?}"
    );
    assert!(
        blockers
            .iter()
            .any(|blocker| blocker.contains("'release-only'") && blocker.contains(&on_branch)),
        "the required branch is reported with its remedy: {blockers:?}"
    );

    let human = cutver_in(fixture, &["plan"]);
    assert_eq!(human.status.code(), Some(0), "the human path exits 0 too");
    let rendered = stdout_of(&human);
    assert!(
        rendered.contains("A real bump would abort on 2 blocker(s)"),
        "the human path prints the blockers: {rendered}"
    );
}

/// S8 and the parity the plan owes `bump`: a blocker-free plan reports the version the equivalent
/// dry run reports, and `bump --dry-run --json` keeps its own nine keys with `schemaVersion` first.
#[test]
fn a_blocker_free_plan_agrees_with_the_dry_run() {
    let config = armed_config().replace("\nrequire_branch = \"main\"", "");
    let guard = fixture("plan-parity", &config);
    let fixture = guard.fixture();

    let plan_output = cutver_in(fixture, &["plan", "--json", "minor"]);
    assert_eq!(
        plan_output.status.code(),
        Some(0),
        "stderr: {}",
        stderr_of(&plan_output)
    );
    let plan = parse(&plan_output);
    assert_eq!(
        plan["blockers"],
        serde_json::json!([]),
        "a clean tree on any branch is a blocker-free plan: {plan}"
    );

    let dry_run = cutver_in(fixture, &["bump", "--dry-run", "--json", "minor"]);
    assert_eq!(dry_run.status.code(), Some(0), "stderr: {}", stderr_of(&dry_run));
    let release = parse(&dry_run);
    assert_eq!(
        plan["next_version"], release["version"],
        "the plan reports the version the dry run would cut"
    );
    assert_eq!(plan["tag"], release["tag"]);
    assert_eq!(release["released"], false, "a dry run releases nothing: {release}");
    assert_eq!(
        release["commit"], "",
        "a dry run commits nothing, so it names no commit: {release}"
    );
    assert_eq!(
        plan["commit"],
        head(fixture),
        "the plan names the commit the release would be cut from: {plan}"
    );

    let raw = stdout_of(&dry_run);
    const BUMP_KEYS: [&str; 9] = [
        "schemaVersion",
        "released",
        "deferred",
        "version",
        "tag",
        "commit",
        "floating_tag",
        "publish_push_command",
        "publish_commands",
    ];
    let mut positions: Vec<(usize, &str)> = BUMP_KEYS
        .iter()
        .map(|key| {
            let at = raw
                .find(&format!("\"{key}\":"))
                .unwrap_or_else(|| panic!("the bump result carries `{key}`: {raw}"));
            (at, *key)
        })
        .collect();
    positions.sort();
    assert_eq!(
        positions.into_iter().map(|(_, key)| key).collect::<Vec<_>>(),
        BUMP_KEYS.to_vec(),
        "the bump result keeps its keys and their order: {raw}"
    );
}

/// S3: a plan cannot claim notes it did not render, so rendering that cannot happen fails closed
/// instead of printing `notes: null`.
#[test]
fn a_plan_that_cannot_render_its_entry_fails_closed() {
    let config = armed_config().replace("path = \"CHANGELOG.md\"", "path = \"MISSING.md\"");
    let guard = fixture("plan-no-changelog", &config);
    let fixture = guard.fixture();

    let output = cutver_in(fixture, &["plan", "--json"]);

    assert_eq!(output.status.code(), Some(1), "stdout: {}", stdout_of(&output));
    assert!(
        output.stdout.is_empty(),
        "a plan that cannot be computed prints no plan: {:?}",
        stdout_of(&output)
    );
    let stderr = stderr_of(&output);
    assert_eq!(stderr.lines().count(), 1, "the machine failure is one line: {stderr:?}");
    let envelope: Value = serde_json::from_str(stderr.trim()).expect("stderr is the failure envelope");
    assert_eq!(envelope["schemaVersion"], 1);
    assert_eq!(envelope["error"]["code"], "plan");
    assert_eq!(envelope["error"]["exitCode"], 1);
    let message = envelope["error"]["message"].as_str().expect("a string");
    assert!(message.contains("MISSING.md"), "the failure names the file: {message}");
    assert!(
        !message.contains("notes: null"),
        "the plan never reports notes it did not render: {message}"
    );

    let human = cutver_in(fixture, &["plan"]);
    assert_eq!(human.status.code(), Some(1));
    assert!(
        stderr_of(&human).starts_with("Error:"),
        "without `--json` the failure is the human line: {}",
        stderr_of(&human)
    );
}

/// S1 and L5: the plan dispatches no plugin, so a configuration that needs one is refused by name
/// instead of being run behind a promise that nothing changes.
#[test]
fn a_plugin_driven_version_is_refused_instead_of_dispatched() {
    let config = armed_config().replace(
        "current_source = \"package.json\"",
        "current_source = \"package.json\"\nstrategy = \"plugin\"",
    );
    let guard = fixture("plan-plugin-strategy", &config);
    let fixture = guard.fixture();

    let output = cutver_in(fixture, &["plan"]);

    assert_eq!(output.status.code(), Some(1), "stdout: {}", stdout_of(&output));
    let stderr = stderr_of(&output);
    assert!(stderr.contains("strategy = \"plugin\""), "{stderr}");
    assert!(stderr.contains("cutver bump --dry-run"), "{stderr}");
    assert_eq!(local_tags(fixture), "", "nothing was tagged");
}

/// The snapshot above is only honest if the fixture is quiescent: `git commit` triggers automatic
/// maintenance, and git detaches it on macOS (`maintenance.autoDetach` defaults to true there), so a
/// background process can hold `.git/objects/maintenance.lock` when `before` is taken and release it
/// before `after` — a removal no plan can cause. `tests/common::init_git_repo` disables it; this pins
/// that, so the acceptance criterion keeps failing for real mutations and never for git's bookkeeping.
#[test]
fn the_fixture_never_carries_a_background_maintenance_lock() {
    let guard = fixture("plan-quiescent", &armed_config());
    let fixture = guard.fixture();

    let config = fixture.read(".git/config");
    assert!(
        config.contains("autoDetach = false"),
        "background maintenance must be off in the fixture or the snapshot races it: {config}"
    );

    for _ in 0..50 {
        assert!(
            !fixture.dir.join(".git/objects/maintenance.lock").exists(),
            "`git commit` left a background `git maintenance` lock in the fixture"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
