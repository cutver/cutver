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
use std::process::Output;

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
    // Every configuration here names `main` under `[git] require_branch`, and `git init`'s default
    // branch belongs to the host: CI runners still create `master`. The fixture names it instead of
    // inheriting it, so a plan reports the release and not its own checkout.
    run_git_ok(&fixture.dir, &["branch", "-M", "main"]);
    guard
}

/// Runs the cutver binary with the fixture as its working directory.
///
/// The CI channel's three variables are removed: a runner that sets them for its own steps would
/// annotate every expected failure and fill its sinks from this test, and `tests/ci_output.rs` owns
/// the channel's own assertions.
fn cutver_in(fixture: &Fixture, args: &[&str]) -> Output {
    cutver_command(&fixture.dir)
        .env_remove("GITHUB_ACTIONS")
        .env_remove("GITHUB_OUTPUT")
        .env_remove("GITHUB_STEP_SUMMARY")
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
/// entry, and `artifacts`/`gates` present and empty for a workspace that declares neither.
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
        "present and empty: this workspace declares no `[[artifact]]`"
    );
    assert_eq!(
        plan["gates"],
        serde_json::json!([]),
        "present and empty: this workspace requires no `[gates] require` entry"
    );
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

/// #307: the level a first release reports does not change. An explicitly named level is reported as
/// named, and only `auto` reports `initial`; the compatibility guard exempts a first release on its
/// missing previous tag instead of on the level's name.
#[test]
fn a_first_release_reports_the_level_it_was_given() {
    let guard = fixture("plan-first-release-level", &armed_config());
    let fixture = guard.fixture();

    let named = parse(&cutver_in(fixture, &["plan", "--json", "--first-release", "minor"]));
    assert_eq!(named["level"], "minor", "a named level stays named: {named}");
    assert_eq!(
        named["tag"], "v1.2.3",
        "a first release tags the version it has: {named}"
    );

    let deduced = parse(&cutver_in(fixture, &["plan", "--json", "--first-release", "auto"]));
    assert_eq!(
        deduced["level"], "initial",
        "only `auto` reports the promotion: {deduced}"
    );
}

/// S3: a plan cannot claim notes it did not render, so rendering that cannot happen fails closed
/// instead of printing `notes: null`.
///
/// The unrenderable state is a changelog that already carries the release the plan is computing:
/// `compute_update` returns no entry to prepend, which the error itself names as a cause ("remove
/// any existing '## [v1.3.0]' entry, which makes the prepend a no-op"). An *absent* changelog is no
/// longer one of these states: #289 S1b makes a declared-but-absent changelog render against empty
/// content, because that is what the bump it plans would do.
#[test]
fn a_plan_that_cannot_render_its_entry_fails_closed() {
    let guard = fixture("plan-no-entry", &armed_config());
    let fixture = guard.fixture();
    fixture.write(
        "CHANGELOG.md",
        "# Changelog\n\n## [v1.3.0] - 2026-01-01\n\n- already released\n",
    );
    run_git_ok(&fixture.dir, &["add", "."]);
    run_git_ok(&fixture.dir, &["commit", "-m", "chore: record the 1.3.0 entry"]);

    let output = cutver_in(fixture, &["plan", "--json", "minor"]);

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
    assert!(
        message.contains("CHANGELOG.md") && message.contains("v1.3.0"),
        "the failure names the file and the tag it could not render: {message}"
    );
    // The "never `notes: null`" half of the promise is the empty stdout asserted above, not a
    // substring of the message: this error's own `Where:` line quotes the phrase it refuses to print.
    assert!(
        message.contains("renders no entry"),
        "the plan says why it has no entry to report: {message}"
    );

    let human = cutver_in(fixture, &["plan", "minor"]);
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

/// S4: `artifacts[]` carries each declaration plus the count it observes now. A count, not a
/// verdict: at plan time the assets are not built yet (L7).
#[test]
fn the_plan_reports_each_declared_artifact_and_the_files_it_matches() {
    let artifacts = r#"
[[artifact]]
glob = "dist/*.tar.gz"
at_least = 2
description = "lean binaries"

[[artifact]]
glob = "artifacts/**/*.whl"
"#;
    let config = format!("{}{artifacts}", armed_config());
    let guard = fixture("plan-artifacts", &config);
    let fixture = guard.fixture();
    fixture.write("dist/linux.tar.gz", "a");
    fixture.write("dist/macos.tar.gz", "b");
    fixture.write("artifacts/linux/x86_64/pkg.whl", "c");

    let output = cutver_in(fixture, &["plan", "--json", "minor"]);

    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr_of(&output));
    let plan = parse(&output);
    assert_eq!(
        plan["artifacts"],
        serde_json::json!([
            { "glob": "dist/*.tar.gz", "at_least": 2, "description": "lean binaries", "matched": 2 },
            { "glob": "artifacts/**/*.whl", "at_least": 1, "matched": 1 },
        ]),
        "{plan}"
    );
    let blockers = plan["blockers"].as_array().expect("blockers is an array");
    assert!(
        blockers
            .iter()
            .all(|blocker| !blocker.as_str().unwrap().contains("declared artifact")),
        "a plan never blocks on an artifact: {blockers:?}"
    );
}

/// S4 and L7: a plan over a dirty tree with no assets still exits `0`, reports `matched: 0` and
/// raises no artifact blocker — the assets are built after the plan, so a blocker would be a lie.
#[test]
fn a_plan_over_a_dirty_tree_without_assets_never_blocks_on_artifacts() {
    let artifacts = "\n[[artifact]]\nglob = \"dist/*.tar.gz\"\ndescription = \"release tarball\"\n";
    let config = format!("{}{artifacts}", armed_config());
    let guard = fixture("plan-artifacts-dirty", &config);
    let fixture = guard.fixture();
    fixture.write("wip.txt", "uncommitted work\n");

    let output = cutver_in(fixture, &["plan", "--json", "minor"]);

    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr_of(&output));
    let plan = parse(&output);
    assert_eq!(
        plan["artifacts"],
        serde_json::json!([
            { "glob": "dist/*.tar.gz", "at_least": 1, "description": "release tarball", "matched": 0 },
        ]),
        "{plan}"
    );
    let blockers: Vec<&str> = plan["blockers"]
        .as_array()
        .expect("blockers is an array")
        .iter()
        .map(|blocker| blocker.as_str().expect("a blocker is one line of text"))
        .collect();
    assert!(!blockers.is_empty(), "the dirty tree is still reported: {plan}");
    assert!(
        blockers.iter().all(|blocker| !blocker.contains("declared artifact")),
        "no blocker may be about artifacts: {blockers:?}"
    );
}

/// S1: `--channel` on `plan` is the prerelease ladder, and it starts the counter on the version the
/// level cut — `1.2.0` bumped a minor becomes `1.3.0-alpha.1`, not an increment of the last channel.
#[test]
fn a_channel_turns_the_bumped_version_into_a_prerelease() {
    let guard = fixture("plan-channel-alpha", &armed_config());
    let fixture = guard.fixture();

    let output = cutver_in(fixture, &["plan", "--json", "minor", "--channel", "alpha"]);

    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr_of(&output));
    let plan = parse(&output);
    assert_eq!(plan["next_version"], "1.3.0-alpha.1", "{plan}");
    assert_eq!(plan["tag"], "v1.3.0-alpha.1", "{plan}");
    assert_eq!(plan["level"], "minor", "the level keeps its own meaning: {plan}");
}

/// S1 and D1: every value of the ladder is a channel, and `stable` is the release itself, so it adds
/// no prerelease segment to the version the level cut.
#[test]
fn the_stable_channel_releases_the_version_it_cuts() {
    let guard = fixture("plan-channel-stable", &armed_config());
    let fixture = guard.fixture();

    let plan = parse(&cutver_in(fixture, &["plan", "--json", "minor", "--channel", "stable"]));

    assert_eq!(plan["next_version"], "1.3.0", "{plan}");
    assert_eq!(plan["tag"], "v1.3.0", "{plan}");
}

/// D3: the human tree names the transition in words, so approving a release on a channel is not
/// reading suffixes by eye. `plan --json` gains no field: `next_version` already carries it.
#[test]
fn the_plan_tree_names_the_channel_the_release_lands_on() {
    let guard = fixture("plan-channel-tree", &armed_config());
    let fixture = guard.fixture();

    let human = stdout_of(&cutver_in(fixture, &["plan", "minor", "--channel", "alpha"]));

    assert!(
        human.contains("Release Plan: 1.2.3 -> 1.3.0-alpha.1"),
        "the tree reports the version it would cut: {human}"
    );
    assert!(
        human.contains("channel: stable -> alpha"),
        "the tree names the channel the run leaves and the one it lands on: {human}"
    );
}

/// S1: the ladder is a closed set, refused by the parser before a plan is computed — the same exit `2`
/// and the same `possible values` line `attest --conclusion` already answers with.
#[test]
fn an_unknown_channel_is_refused_before_a_plan_is_computed() {
    let guard = fixture("plan-channel-unknown", &armed_config());
    let fixture = guard.fixture();

    let output = cutver_in(fixture, &["plan", "minor", "--channel", "gamma"]);

    assert_eq!(
        output.status.code(),
        Some(2),
        "a usage error is clap's exit 2; stderr:\n{}",
        stderr_of(&output)
    );
    let message = stderr_of(&output);
    for expected in ["possible values", "alpha", "beta", "rc", "stable"] {
        assert!(message.contains(expected), "{message} does not name {expected}");
    }
}

/// Pillar I.2 and S1: a channel is a computation, never a mutation. The plan still changes no file,
/// creates no commit and no tag, even on the tree where every mutating step is armed.
#[test]
fn a_channel_plan_still_writes_nothing() {
    let guard = fixture("plan-channel-dry-tree", &armed_config());
    let fixture = guard.fixture();
    fixture.write("wip.txt", "uncommitted work\n");
    let before = snapshot(&fixture.dir);
    let head_before = head(fixture);

    let output = cutver_in(fixture, &["plan", "--json", "minor", "--channel", "beta"]);

    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr_of(&output));
    assert_eq!(
        snapshot(&fixture.dir),
        before,
        "a plan on a channel changes no file, exactly like a plan on the ladder"
    );
    assert_eq!(head(fixture), head_before, "no commit is created");
    assert!(
        local_tags(fixture).trim().is_empty(),
        "no tag is created: {}",
        local_tags(fixture)
    );
    for marker in [PRE_BUMP_MARKER, PREFLIGHT_MARKER, PUBLISH_MARKER] {
        assert!(
            !fixture.dir.join(marker).exists(),
            "{marker} proves a step a plan must not run did run"
        );
    }
}

/// A fixture whose workspace is on a version it already published, the state a promotion starts from
/// (#326 D1). The changelog carries the entry that version already has, so an entry the promotion
/// renders for a different version cannot be confused with it.
fn released_fixture(name: &str, version: &str, config: &str) -> FixtureGuard {
    assert!(
        git_available(),
        "git CLI is required for e2e tests but was not found in PATH"
    );
    let guard = FixtureGuard::new(name);
    let fixture = guard.fixture();
    fixture.write("package.json", &PACKAGE_JSON.replace("1.2.3", version));
    fixture.write("Cargo.toml", &CARGO_TOML.replace("1.2.3", version));
    fixture.write(
        "CHANGELOG.md",
        &format!("# Changelog\n\n## [{version}]\n\n- candidate\n"),
    );
    fixture.write("cutver.toml", config);
    init_git_repo(fixture);
    initial_commit(fixture);
    // The configurations below require `main`, and `git init`'s default branch belongs to the host:
    // CI runners still create `master`, so the fixture names the branch instead of inheriting it.
    run_git_ok(&fixture.dir, &["branch", "-M", "main"]);
    guard
}

/// D1, first case: the channel the version already carries moves only its counter, and the level is
/// ignored — `1.2.0-alpha.1` on `alpha` is `1.2.0-alpha.2`, never a patch cut on top of it.
#[test]
fn a_channel_on_its_own_prerelease_moves_only_the_counter() {
    let guard = released_fixture("plan-promote-counter", "1.2.0-alpha.1", &armed_config());
    let fixture = guard.fixture();

    let output = cutver_in(fixture, &["plan", "--json", "patch", "--channel", "alpha"]);

    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr_of(&output));
    let plan = parse(&output);
    assert_eq!(plan["next_version"], "1.2.0-alpha.2", "{plan}");
    assert_eq!(plan["tag"], "v1.2.0-alpha.2", "{plan}");
    assert_eq!(
        plan["level"], "patch",
        "the level is still reported, it just does not decide the version: {plan}"
    );
}

/// D1, second case: another channel on the same base version starts its own counter at `1`, and the
/// level is ignored exactly as it is above.
#[test]
fn another_channel_starts_its_own_counter_on_the_same_base_version() {
    let guard = released_fixture("plan-promote-switch", "1.2.0-alpha.3", &armed_config());
    let fixture = guard.fixture();

    let plan = parse(&cutver_in(fixture, &["plan", "--json", "patch", "--channel", "beta"]));

    assert_eq!(plan["next_version"], "1.2.0-beta.1", "{plan}");
    assert_eq!(plan["tag"], "v1.2.0-beta.1", "{plan}");
}

/// D1 and ADR-0005: any prerelease promotes, `rc` to `stable` included, with no rung to validate. The
/// release it promotes to is the stable release in every artifact: the tag carries no suffix, the
/// floating major tag moves onto it like any other stable release, and the changelog prepends
/// `## [<X.Y.Z>]` while the prerelease entry stays as its own history.
#[test]
fn stable_promotes_a_prerelease_to_its_own_release_version() {
    let config = armed_config().replace("tag_prefix = \"v\"", "tag_prefix = \"v\"\nfloating_major_tag = true");
    let guard = released_fixture("plan-promote-stable", "1.2.0-rc.1", &config);
    let fixture = guard.fixture();

    let output = cutver_in(fixture, &["plan", "--json", "patch", "--channel", "stable"]);

    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr_of(&output));
    let plan = parse(&output);
    assert_eq!(
        plan["next_version"], "1.2.0",
        "the suffix is dropped, nothing is incremented: {plan}"
    );
    assert_eq!(
        plan["tag"], "v1.2.0",
        "a promotion tags the release, with no suffix: {plan}"
    );
    assert_eq!(
        plan["floating_tag"], "v1",
        "the stable release moves the floating major tag: {plan}"
    );
    assert_eq!(
        plan["manifests"][0],
        serde_json::json!({ "path": "package.json", "from": "1.2.0-rc.1", "to": "1.2.0" }),
        "{plan}"
    );
    let notes = plan["notes"].as_str().expect("notes is a string");
    assert!(
        notes.starts_with("## [v1.2.0]"),
        "the entry the promotion prepends is the release's own, with no suffix: {notes}"
    );
    assert!(
        !notes.contains("rc"),
        "the entry is not the prerelease it promoted from: {notes}"
    );
    assert_eq!(
        plan["changelog"],
        serde_json::json!({ "path": "CHANGELOG.md", "prepends": "1.2.0" }),
        "{plan}"
    );
}

/// D1 and D1a: the reported no-op fires when the request yields the version the repository already
/// carries — here `stable` with no level named, on a version that is already a release — never merely
/// because `stable` was named. It exits `0` and reports the version it is already on, and the release
/// tag that exists is the state a promotion re-run finds, not a guard to trip over.
#[test]
fn a_request_that_yields_the_current_version_is_a_reported_no_op() {
    let guard = fixture("plan-promote-noop", &armed_config());
    let fixture = guard.fixture();
    run_git_ok(&fixture.dir, &["tag", "-a", "v1.2.3", "-m", "release 1.2.3"]);

    let output = cutver_in(fixture, &["plan", "--json", "--channel", "stable"]);

    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr_of(&output));
    let plan = parse(&output);
    assert_eq!(plan["current_version"], "1.2.3", "{plan}");
    assert_eq!(
        plan["next_version"], "1.2.3",
        "the release it reports is the one the workspace already carries: {plan}"
    );
    assert_eq!(plan["tag"], "v1.2.3", "{plan}");
    assert_eq!(plan["changelog"], Value::Null, "a no-op prepends no entry: {plan}");
    assert_eq!(
        plan["notes"], "",
        "a no-op prepends no entry, and `notes` is never `null`: {plan}"
    );
    assert_eq!(plan["manifests"], serde_json::json!([]), "no manifest changes: {plan}");
    assert_eq!(
        plan["phases"],
        serde_json::json!([]),
        "a no-op has no phase to run: {plan}"
    );
    let blockers: Vec<&str> = plan["blockers"]
        .as_array()
        .expect("blockers is an array")
        .iter()
        .map(|blocker| blocker.as_str().expect("a blocker is one line of text"))
        .collect();
    assert!(
        blockers.iter().all(|blocker| !blocker.contains("already exists")),
        "the release tag that exists is what a promotion re-run finds: {blockers:?}"
    );

    let human = stdout_of(&cutver_in(fixture, &["plan", "--channel", "stable"]));
    assert!(
        human.contains("Release Plan: 1.2.3 -> 1.2.3"),
        "the tree reports the version it would leave the workspace on: {human}"
    );
    assert!(
        human.contains("No-op") && human.contains("nothing"),
        "the tree says nothing was released instead of pretending it was: {human}"
    );
    // The rehearsal the plan is one computation with, and the only place this host executes the
    // release path of a no-op: `tests/e2e_bump.rs` is unix-gated, so it runs it in CI only.
    let before = snapshot(&fixture.dir);
    let head_before = head(fixture);
    let dry_run = cutver_in(fixture, &["bump", "--dry-run", "--channel", "stable"]);
    assert_eq!(dry_run.status.code(), Some(0), "stderr: {}", stderr_of(&dry_run));
    let rehearsed = stdout_of(&dry_run);
    assert!(
        rehearsed.contains("Release Plan: 1.2.3 -> 1.2.3") && rehearsed.contains("No-op"),
        "a rehearsal reports the same no-op the plan reports: {rehearsed}"
    );
    assert_eq!(
        snapshot(&fixture.dir),
        before,
        "a no-op rehearsal leaves every byte of the workspace as it found it"
    );
    assert_eq!(head(fixture), head_before, "no commit is created");
    assert!(
        human.contains("tag: v1.2.3 [SKIPPED]"),
        "the tag it did not create is reported as skipped: {human}"
    );
}

/// D1a: a named level still cuts the version it names with `--channel stable`, so the channel never
/// collides with a normal release — the `stable` entry of a `[alpha, beta, rc, stable]` CI matrix.
#[test]
fn a_named_level_still_cuts_with_the_stable_channel() {
    let guard = fixture("plan-stable-named-level", &armed_config());
    let fixture = guard.fixture();

    let plan = parse(&cutver_in(fixture, &["plan", "--json", "minor", "--channel", "stable"]));

    assert_eq!(plan["next_version"], "1.3.0", "{plan}");
    assert_eq!(plan["tag"], "v1.3.0", "{plan}");
}

/// D1, fifth case and unchanged behaviour: on a stable version a prerelease channel starts its
/// counter at `1` on the base the level cut.
#[test]
fn a_prerelease_channel_on_a_stable_version_starts_its_counter_on_the_bumped_base() {
    let guard = fixture("plan-stable-to-beta", &armed_config());
    let fixture = guard.fixture();

    let plan = parse(&cutver_in(fixture, &["plan", "--json", "minor", "--channel", "beta"]));

    assert_eq!(plan["next_version"], "1.3.0-beta.1", "{plan}");
    assert_eq!(plan["tag"], "v1.3.0-beta.1", "{plan}");
}
