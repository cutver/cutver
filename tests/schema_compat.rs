//! Version `1` of every JSON surface only grows (#346): no key inside it is removed, renamed or
//! re-typed.
//!
//! `src/cli/json.rs` pins the envelope and two toy payloads, and `tests/machine_error_contract.rs`
//! pins the failure half, but neither states the **key set** a consumer parses, so until now a
//! surface could lose a key and every test stayed green. This suite is that statement: one constant
//! per surface, derived from what the binary emits, naming the page or the struct that owns it.
//!
//! The assertion is `emitted ⊇ pinned`, never equality, because the rule is additive
//! (`docs/internals/machine-contract.md`): a new key is not a new version. What fails here is a key
//! that disappears or is renamed — a consumer that already reads it silently starts getting `null`,
//! which is the change that would require `schemaVersion` 2.
//!
//! Every spawn goes through `common::cutver_command`, the hermetic helper phase 4A added: it removes
//! `GITHUB_ACTIONS`, `GITHUB_OUTPUT` and `GITHUB_STEP_SUMMARY`, so a runner cannot switch the CI
//! channel on under a test that asserts the exact bytes of a surface.

mod common;

use std::collections::BTreeSet;

use common::*;
use serde_json::Value;

/// The first bytes of a compact surface: the version is the first key of the object, so a consumer
/// reads it before the payload it may not understand.
const COMPACT_PREFIX: &str = r#"{"schemaVersion":1,"#;

/// The first bytes of a pretty-printed surface. Same promise, same position, two leading spaces.
const PRETTY_PREFIX: &str = "{\n  \"schemaVersion\": 1,";

/// The keys `cutver plan --json` must carry, in the order it documents them.
///
/// Source of truth: `docs/internals/machine-contract-surfaces.md` § `cutver plan --json` (every key
/// is spelled there, in that order), owned by `ReleasePlan` (`src/bump/plan.rs:77`), whose own doc
/// comment says the keys and their order are the contract. This is the richest surface: it carries
/// `read_only`, the version pair, the phases, the artifacts and the gates.
const PLAN_KEYS: [&str; 17] = [
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

/// The keys the release result carries on `cutver bump --json` and in `--report`.
///
/// Source of truth: `docs/reference/cli.md` under `cutver bump` (the field order is spelled out
/// there), owned by `SummaryJson` (`src/cli/runner/bump.rs`). The list is the same for a real and a
/// `--dry-run` run: only the values differ, so a dry run pins the keys without cutting a release.
const BUMP_KEYS: [&str; 8] = [
    "released",
    "deferred",
    "version",
    "tag",
    "commit",
    "floating_tag",
    "publish_push_command",
    "publish_commands",
];

/// The keys the release context carries on `cutver changelog latest|show --json`.
///
/// Source of truth: `ReleaseContext`'s hand-written `Serialize` (`src/changelog/context/release.rs`),
/// which registers every key; `docs/templates/release-notes.md` documents the same fields as template
/// variables, and `docs/reference/cli-tools.md` names the surface ("the object opens with
/// `schemaVersion` and then the release context keys").
///
/// Three pairs are deliberate aliases for templates (`owner`/`repo_owner`, `repo`/`repo_name`,
/// `project`/`project_name`), so `owner`, `repo` and `project` are pinned even though they duplicate
/// a value: a template that reads them breaks if they go.
///
/// The keys below are emitted but not enumerated in `docs/internals/machine-contract.md` nor in
/// `docs/templates/release-notes.md` — they are owned by the struct only: `first_time_contributors`
/// (documented in `docs/adr/0003-context-derived-changelog-sections.md`), `major`, `minor`, `patch`,
/// `is_prerelease`, `prerelease`, `build`, `year`, `month`, `day`, `repo_owner`, `owner`, `repo_name`,
/// `repo`, `forge`, `project_name`, `project`. They are pinned anyway — an undocumented key is still
/// a key a consumer reads — and the documentation gap is reported rather than assumed away.
const CHANGELOG_KEYS: [&str; 36] = [
    "version",
    "previous_version",
    "tag",
    "previous_tag",
    "date",
    "compare_url",
    "repository",
    "features",
    "fixes",
    "breaking",
    "perf",
    "refactor",
    "docs",
    "maintenance",
    "other",
    "all_changes",
    "commits",
    "sections",
    "contributors",
    "first_time_contributors",
    "major",
    "minor",
    "patch",
    "is_prerelease",
    "prerelease",
    "build",
    "year",
    "month",
    "day",
    "repo_owner",
    "owner",
    "repo_name",
    "repo",
    "forge",
    "project_name",
    "project",
];

/// The keys `cutver doctor --json` must carry.
///
/// Source of truth: `docs/internals/machine-contract-surfaces.md` § `doctor --json` (the example
/// object and its bullets), owned by `DoctorReport` (`src/cli/doctor.rs:20`). Drift is data on
/// stdout with exit `2`, so these keys are the whole machine-readable outcome of the diagnosis.
/// `warnings` is the deprecation channel `CONTRACT.md` Pillar IV.2 names `doctor` for (#272): a
/// runner that reads the diagnosis has to be able to act on a rename without scraping stderr.
const DOCTOR_KEYS: [&str; 5] = ["ok", "checks", "version_drift", "changelog_drift", "warnings"];

/// The keys `cutver plugin list --json` must carry.
///
/// Source of truth: `docs/internals/machine-contract.md` § Surfaces (`{"schemaVersion", "plugins":
/// [...]}`) and `docs/reference/cli-tools.md`, owned by the `PluginListJson` struct in
/// `src/cli/runner/plugin.rs`. `plugins` stays an object key rather than a bare array so the version
/// has somewhere to live.
#[cfg(feature = "plugins")]
const PLUGIN_LIST_KEYS: [&str; 1] = ["plugins"];

/// The keys the machine error envelope must carry.
///
/// Source of truth: `docs/internals/machine-contract.md` § Errors, owned by `FailureJson` /
/// `FailureBody` (`src/cli/machine.rs`). A consumer branches on `code`, reads the outcome from
/// `exitCode` and shows `message`; losing one leaves a CI job holding an object it cannot classify.
const ERROR_KEYS: [&str; 1] = ["error"];

/// The keys inside the envelope's `error` object.
const ERROR_BODY_KEYS: [&str; 3] = ["code", "exitCode", "message"];

/// A workspace all five success surfaces can describe: two manifests that agree, a changelog with
/// one released entry, the tag for it, and one `feat` commit after it, so `plan` has something to
/// release and `changelog latest` has a commit range it can read.
fn workspace(name: &str) -> FixtureGuard {
    assert!(
        git_available(),
        "the git CLI is required to build the surface fixtures but was not found in PATH"
    );
    let cutver_toml = r#"[version]
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
entry_template = "Maintenance and updates."

[git]
tag_prefix = "v"
require_clean_tree = true
"#;
    let guard = FixtureGuard::new(name);
    let fixture = guard.fixture();
    fixture.write("package.json", PACKAGE_JSON);
    fixture.write("Cargo.toml", CARGO_TOML);
    fixture.write(
        "CHANGELOG.md",
        "# Changelog\n\n## [1.1.0] - 2026-01-01\n\n### Features\n- older work\n",
    );
    fixture.write("cutver.toml", cutver_toml);
    init_git_repo(fixture);
    initial_commit(fixture);
    run_git_ok(&fixture.dir, &["tag", "v1.1.0"]);
    fixture.write("after-release.txt", "work done after the tagged release\n");
    run_git_ok(&fixture.dir, &["add", "-A"]);
    run_git_ok(&fixture.dir, &["commit", "-m", "feat: work after the release", "-q"]);
    guard
}

/// Runs the binary in `fixture` and returns its stdout, failing on a non-zero exit with the stderr
/// that explains it.
fn stdout_of_success(fixture: &Fixture, args: &[&str], surface: &str) -> String {
    let output = cutver_command(&fixture.dir)
        .args(args)
        .output()
        .expect("failed to execute the cutver binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "`{surface}` must succeed for its key set to be observable, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).to_string()
}

/// The properties this suite exists for, asserted the same way on every surface: the version is `1`,
/// the object opens with it at the exact documented bytes, and no pinned key is gone.
fn assert_surface(surface: &str, document: &str, baseline: &[&str], prefix: &str) {
    assert!(
        document.starts_with(prefix),
        "{surface} must keep `schemaVersion` as its first key, at the bytes `{prefix}`: a consumer \
         that reads the version before the payload (and refuses a version it does not know) would \
         otherwise have to parse the whole object before it can decide. Got: {document:?}"
    );

    let parsed: Value = serde_json::from_str(document.trim())
        .unwrap_or_else(|err| panic!("{surface} must be one JSON object, got {document:?}: {err}"));

    assert_eq!(
        parsed["schemaVersion"], 1,
        "{surface} changed the schema version. Version 1 only grows additively; a change to `2` \
         breaks every consumer pinned to 1 and must come with a migration note in \
         docs/internals/machine-contract.md. Got: {document}"
    );

    let emitted: BTreeSet<String> = parsed
        .as_object()
        .unwrap_or_else(|| panic!("{surface} must be a JSON object, got {document:?}"))
        .keys()
        .cloned()
        .collect();
    let missing: Vec<&str> = baseline.iter().copied().filter(|key| !emitted.contains(*key)).collect();
    assert!(
        missing.is_empty(),
        "{surface} no longer emits {missing:?}. A consumer that already reads {} silently starts \
         getting `null` instead of a value, and nothing in the release failed to say so — removing \
         or renaming a key inside schema version 1 is a breaking change, so either keep the key or \
         bump the version and document the migration. Emitted keys: {emitted:?}",
        if missing.len() == 1 { "it" } else { "them" }
    );
}

/// The richest surface: `plan` carries `read_only`, the version pair, the phases, artifacts and
/// gates, and a machine that branches on any of them is the reason this baseline exists.
#[test]
fn plan_keeps_every_documented_key_in_schema_version_one() {
    let guard = workspace("schema-plan");
    let fixture = guard.fixture();

    let document = stdout_of_success(fixture, &["plan", "--json"], "cutver plan --json");

    assert_surface("`cutver plan --json`", &document, &PLAN_KEYS, COMPACT_PREFIX);

    // The baseline above is only meaningful if the objects the pinned keys name are really there.
    let plan: Value = serde_json::from_str(document.trim()).expect("the plan is one JSON object");
    assert_eq!(
        plan["read_only"], true,
        "`read_only` is a property of the command, not of the invocation: a plan mutates nothing. \
         Got: {document}"
    );
    for (key, expected) in [("phases", 3), ("artifacts", 0), ("gates", 0)] {
        assert_eq!(
            plan[key].as_array().map(Vec::len),
            Some(expected),
            "`{key}` must still be the array the docs describe, or a consumer iterating it breaks. \
             Got: {document}"
        );
    }
}

/// The release result: `--dry-run` reports the same nine keys a real run does, so the key set is
/// pinned without cutting a release in the test.
#[test]
fn bump_dry_run_keeps_every_documented_key_in_schema_version_one() {
    let guard = workspace("schema-bump");
    let fixture = guard.fixture();

    let document = stdout_of_success(
        fixture,
        &["bump", "--json", "--dry-run"],
        "cutver bump --json --dry-run",
    );

    assert_surface("`cutver bump --json --dry-run`", &document, &BUMP_KEYS, COMPACT_PREFIX);

    let result: Value = serde_json::from_str(document.trim()).expect("the release result is one JSON object");
    assert_eq!(
        result["released"], false,
        "a dry run must keep reporting `released: false`, so a CI job reading it never claims a \
         release it did not cut. Got: {document}"
    );
}

/// The release context a CI job renders release notes from. This is the prettiest surface and the
/// one with the widest key set, so a template reading a field must keep finding it.
#[test]
fn changelog_latest_keeps_every_release_context_key_in_schema_version_one() {
    let guard = workspace("schema-changelog");
    let fixture = guard.fixture();

    let document = stdout_of_success(
        fixture,
        &["changelog", "latest", "--json"],
        "cutver changelog latest --json",
    );

    assert_surface(
        "`cutver changelog latest --json`",
        &document,
        &CHANGELOG_KEYS,
        PRETTY_PREFIX,
    );

    // `schemaVersion` is the contract version and `version` the release version: a consumer that
    // reads the wrong one gets a plausible number, so the split is pinned on the real surface too.
    let context: Value = serde_json::from_str(document.trim()).expect("the release context is one JSON object");
    assert_eq!(
        context["version"], "1.1.0",
        "`version` is the release the entry describes, never the schema version. Got: {document}"
    );
}

/// The diagnosis. Drift is data on stdout with exit `2`, so these keys are the entire machine
/// surface of `doctor`.
#[test]
fn doctor_keeps_every_documented_key_in_schema_version_one() {
    let guard = workspace("schema-doctor");
    let fixture = guard.fixture();

    let document = stdout_of_success(fixture, &["doctor", "--json"], "cutver doctor --json");

    assert_surface("`cutver doctor --json`", &document, &DOCTOR_KEYS, PRETTY_PREFIX);
}

/// The plugin list, which wraps its entries in an object so the version has somewhere to live. A
/// lean binary does not build the subcommand at all, so the test only exists with the feature.
#[cfg(feature = "plugins")]
#[test]
fn plugin_list_keeps_its_plugins_key_in_schema_version_one() {
    let guard = workspace("schema-plugin-list");
    let fixture = guard.fixture();
    // The plugin cache is resolved from the environment; keep it inside the fixture so the test
    // never reads or writes the ambient cache of whoever runs it.
    let cache = fixture.dir.join("plugin-cache");

    let output = cutver_command(&fixture.dir)
        .env("CUTVER_PLUGIN_CACHE_DIR", &cache)
        .args(["plugin", "list", "--json"])
        .output()
        .expect("failed to execute the cutver binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "`cutver plugin list --json` must succeed for its key set to be observable, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let document = String::from_utf8_lossy(&output.stdout).to_string();

    assert_surface(
        "`cutver plugin list --json`",
        &document,
        &PLUGIN_LIST_KEYS,
        PRETTY_PREFIX,
    );

    let list: Value = serde_json::from_str(document.trim()).expect("the plugin list is one JSON object");
    assert!(
        list["plugins"].is_array(),
        "`plugins` stays the array a consumer iterates, never a bare array standing in for the whole \
         document. Got: {document}"
    );
}

/// The failure half: a command that asked for machine output reports its failure as one object on
/// stderr, and that object keeps the three keys a consumer branches on.
#[test]
fn the_error_envelope_keeps_every_documented_key_in_schema_version_one() {
    // An empty workspace: no `cutver.toml` anywhere, so `bump --json` fails before it does anything.
    let guard = FixtureGuard::new("schema-error");
    let fixture = guard.fixture();

    let output = cutver_command(&fixture.dir)
        .args(["bump", "--json"])
        .output()
        .expect("failed to execute the cutver binary");
    assert_eq!(
        output.status.code(),
        Some(1),
        "the failure needs a machine channel to be observable, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stdout.is_empty(),
        "a failure prints the envelope on stderr and nothing on stdout, or a consumer reads half a \
         document: {:?}",
        String::from_utf8_lossy(&output.stdout)
    );
    let envelope = String::from_utf8_lossy(&output.stderr).to_string();

    assert_surface("the machine error envelope", &envelope, &ERROR_KEYS, COMPACT_PREFIX);

    let parsed: Value = serde_json::from_str(envelope.trim()).expect("stderr is the failure envelope");
    let body = parsed["error"]
        .as_object()
        .unwrap_or_else(|| panic!("the envelope's `error` is an object, got {envelope:?}"));
    let body_keys: BTreeSet<&str> = body.keys().map(String::as_str).collect();
    let missing: Vec<&str> = ERROR_BODY_KEYS
        .iter()
        .copied()
        .filter(|key| !body_keys.contains(key))
        .collect();
    assert!(
        missing.is_empty(),
        "the envelope's `error` no longer carries {missing:?}: a consumer branches on `code`, reads \
         the outcome from `exitCode` and shows `message`, so losing one leaves a CI job with an \
         object it cannot classify. Emitted keys: {body_keys:?}"
    );
    assert_eq!(
        parsed["error"]["exitCode"],
        output.status.code().unwrap(),
        "`exitCode` must equal the process exit code, or the object and the status disagree: \
         {envelope}"
    );
}
