//! #307 — a `minor` or `major` release must declare what it does about compatibility.
//!
//! `v0.19.0` shipped a breaking plugin-environment contract (`env_clear()` plus grants) and its notes
//! said nothing, because no commit in `v0.18.2..v0.19.0` carried a `BREAKING CHANGE:` footer: silence
//! was read as "compatible". The rule as the issue words it ("level is minor or major and no commit
//! declares a breaking change → fail") would block *every* feature release, because a normal minor is
//! by definition a range with a `feat` and no declaration. The maintainer chose the two-direction
//! declaration instead (#307 L2): the range declares `BREAKING CHANGE:` / `!` when it breaks, and the
//! reviewed-none footer `NO-BREAKING-CHANGE: <why>` when it does not.
//!
//! Every fixture here is a real repository: an explicit `--level` never parses commits on its own, so
//! the guard reads `<previous_tag>..HEAD` itself and the range is part of what is under test.

mod common;

use std::collections::BTreeMap;
use std::fs;
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
"#;

/// The migrated plugin-environment contract `0.19.0` shipped: the declaration this issue is about.
const MIGRATION: &str = "BREAKING CHANGE: grants are now required; `env_clear()` leaves only the allowlist";
/// The reviewed-none declaration: what a feature release that changes nothing writes instead.
const REVIEWED_NONE: &str = "NO-BREAKING-CHANGE: the endpoint is additive and the plugin environment is unchanged";

/// A released workspace at `1.2.3`: `v1.2.3` exists, and the range stays empty until a test commits.
fn released(name: &str) -> FixtureGuard {
    let guard = unreleased(name);
    run_git_ok(&guard.fixture().dir, &["tag", "v1.2.3"]);
    guard
}

/// The same workspace before its first release: no tag, so there is no range to compare yet.
fn unreleased(name: &str) -> FixtureGuard {
    assert!(
        git_available(),
        "git CLI is required for e2e tests but was not found in PATH"
    );
    let guard = FixtureGuard::new(name);
    let fixture = guard.fixture();
    fixture.write("package.json", PACKAGE_JSON);
    fixture.write("Cargo.toml", CARGO_TOML);
    fixture.write("CHANGELOG.md", CHANGELOG_MD);
    fixture.write("cutver.toml", TOML);
    init_git_repo(fixture);
    initial_commit(fixture);
    guard
}

/// One empty commit inside the range, so a test can build the range it needs.
fn commit(fixture: &Fixture, message: &str) {
    run_git_ok(&fixture.dir, &["commit", "--allow-empty", "-m", message]);
}

fn cutver_in(fixture: &Fixture, args: &[&str]) -> Output {
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

fn head(fixture: &Fixture) -> String {
    String::from_utf8_lossy(&run_git(&fixture.dir, &["rev-parse", "HEAD"]).stdout)
        .trim()
        .to_string()
}

fn tags(fixture: &Fixture) -> String {
    String::from_utf8_lossy(&run_git(&fixture.dir, &["tag", "-l"]).stdout).to_string()
}

/// Every file in the worktree, byte for byte, excluding `.git`: a read may rewrite the index, so the
/// repository's own bookkeeping is not what "wrote nothing" is about.
fn snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(dir: &Path, root: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(dir).expect("the worktree is readable") {
            let entry = entry.expect("the directory entry is readable");
            let path = entry.path();
            if entry.file_name() == ".git" {
                continue;
            }
            if entry.file_type().expect("the entry type is readable").is_dir() {
                walk(&path, root, out);
            } else {
                let relative = path
                    .strip_prefix(root)
                    .expect("the path is inside the worktree")
                    .to_string_lossy()
                    .replace('\\', "/");
                out.insert(relative, fs::read(&path).expect("the file is readable"));
            }
        }
    }
    let mut files = BTreeMap::new();
    walk(root, root, &mut files);
    files
}

fn plan_json(fixture: &Fixture, args: &[&str]) -> Value {
    let output = cutver_in(fixture, args);
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr_of(&output));
    let text = stdout_of(&output);
    serde_json::from_str(text.trim()).unwrap_or_else(|err| panic!("stdout must be one JSON object: {text:?}: {err}"))
}

/// S1, S2 and S3: an undeclared `minor` range fails before anything is written, and the failure names
/// the range, the level and both footers to choose between.
#[test]
fn an_undeclared_minor_range_fails_before_anything_is_written() {
    let guard = released("declaration-undeclared-minor");
    let fixture = guard.fixture();
    // The shape of `v0.18.2..v0.19.0`: features, and no declaration anywhere.
    commit(fixture, "feat(api): add the gallery endpoint");

    let before = snapshot(&fixture.dir);
    let head_before = head(fixture);
    let tags_before = tags(fixture);

    let output = cutver_in(fixture, &["bump", "minor"]);

    assert_eq!(
        output.status.code(),
        Some(1),
        "an undeclared minor must fail: stdout: {}",
        stdout_of(&output)
    );
    let stderr = stderr_of(&output);
    let first_line = stderr
        .lines()
        .find(|line| line.starts_with("Error:"))
        .unwrap_or_else(|| panic!("the failure is the crate's human line: {stderr:?}"));
    assert!(
        first_line.contains("minor"),
        "the first line names the level: {first_line}"
    );
    assert!(
        first_line.contains("v1.2.3..HEAD"),
        "the first line names the range it compared: {first_line}"
    );
    assert!(stderr.contains("\n  Where: "), "the failure says where: {stderr:?}");
    assert!(stderr.contains("\n  Fix: "), "the failure says what to add: {stderr:?}");
    assert!(
        stderr.contains("BREAKING CHANGE: <migration>"),
        "the Fix names the breaking footer: {stderr:?}"
    );
    assert!(
        stderr.contains("NO-BREAKING-CHANGE: <why this release stays compatible>"),
        "the Fix names the reviewed-none footer: {stderr:?}"
    );
    assert!(
        stderr.contains("feat(api)!: rewrite the parser"),
        "the Fix carries one worked example: {stderr:?}"
    );
    assert!(
        stderr.contains("Nothing was written"),
        "the failure states that nothing was written: {stderr:?}"
    );

    assert_eq!(snapshot(&fixture.dir), before, "no file was written");
    assert_eq!(head(fixture), head_before, "no commit was created");
    assert_eq!(tags(fixture), tags_before, "no tag was created");
}

/// S2: the same range releases once it carries the reviewed-none footer.
#[test]
fn the_reviewed_none_footer_lets_the_same_range_release() {
    let guard = released("declaration-reviewed-none");
    let fixture = guard.fixture();
    commit(fixture, "feat(api): add the gallery endpoint");
    commit(
        fixture,
        &format!("docs(api): record the compatibility review\n\n{REVIEWED_NONE}"),
    );

    let output = cutver_in(fixture, &["bump", "minor"]);

    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr_of(&output));
    assert!(
        fixture.read("package.json").contains("\"version\": \"1.3.0\""),
        "the release was cut: {}",
        fixture.read("package.json")
    );
    assert!(
        tags(fixture).contains("v1.3.0"),
        "the release was tagged: {}",
        tags(fixture)
    );
}

/// S2: `BREAKING CHANGE:` in a footer declares the range, and an explicit level honours it.
#[test]
fn a_breaking_change_footer_declares_an_explicitly_named_level() {
    let guard = released("declaration-breaking-footer");
    let fixture = guard.fixture();
    commit(fixture, "feat(api): add the gallery endpoint");
    commit(
        fixture,
        &format!("refactor(plugin): rewrite the handshake\n\n{MIGRATION}"),
    );

    let output = cutver_in(fixture, &["bump", "minor"]);

    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr_of(&output));
    assert!(
        fixture.read("package.json").contains("\"version\": \"1.3.0\""),
        "the named level still wins over deduction: {}",
        fixture.read("package.json")
    );
}

/// S2: the `!` header the parser already understands declares the range too.
#[test]
fn a_breaking_header_declares_the_range() {
    let guard = released("declaration-breaking-header");
    let fixture = guard.fixture();
    commit(fixture, "feat(api)!: replace the plugin handshake");

    let output = cutver_in(fixture, &["bump", "minor"]);

    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr_of(&output));
    assert!(fixture.read("package.json").contains("\"version\": \"1.3.0\""));
}

/// S1: `patch` is exempt — an ordinary fix release never needs a declaration.
#[test]
fn a_patch_release_is_exempt_from_the_declaration() {
    let guard = released("declaration-patch-exempt");
    let fixture = guard.fixture();
    commit(fixture, "feat(api): add the gallery endpoint");

    let output = cutver_in(fixture, &["bump", "patch"]);

    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr_of(&output));
    assert!(fixture.read("package.json").contains("\"version\": \"1.2.4\""));
}

/// S1: an empty range is a version move with no commits, so it has nothing to declare — the same
/// reason a promotion is not this check's object.
#[test]
fn a_minor_release_over_an_empty_range_has_nothing_to_declare() {
    let guard = released("declaration-empty-range");
    let fixture = guard.fixture();

    let output = cutver_in(fixture, &["bump", "minor"]);

    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr_of(&output));
    assert!(fixture.read("package.json").contains("\"version\": \"1.3.0\""));
}

/// S1: a workspace with no release tag has shipped nothing yet, so it has no prior contract to
/// declare against and the check does not apply.
#[test]
fn a_first_release_with_no_previous_tag_has_nothing_to_declare() {
    let guard = unreleased("declaration-no-tag");
    let fixture = guard.fixture();
    commit(fixture, "feat(api): add the gallery endpoint");

    let output = cutver_in(fixture, &["bump", "minor"]);

    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr_of(&output));
    assert!(fixture.read("package.json").contains("\"version\": \"1.3.0\""));
}

/// S1: `auto` derives the level from the commits, and the derived `minor` is held to the same rule.
#[test]
fn auto_that_deduces_minor_is_held_to_the_same_rule() {
    let guard = released("declaration-auto-minor");
    let fixture = guard.fixture();
    commit(fixture, "feat(api): add the gallery endpoint");

    let undeclared = cutver_in(fixture, &["bump", "auto"]);
    assert_eq!(undeclared.status.code(), Some(1), "stdout: {}", stdout_of(&undeclared));
    assert!(
        stderr_of(&undeclared).contains("the minor release over 'v1.2.3..HEAD'"),
        "the deduced level is the one reported: {}",
        stderr_of(&undeclared)
    );

    commit(
        fixture,
        &format!("docs(api): record the compatibility review\n\n{REVIEWED_NONE}"),
    );
    let declared = cutver_in(fixture, &["bump", "auto"]);
    assert_eq!(declared.status.code(), Some(0), "stderr: {}", stderr_of(&declared));
    assert!(fixture.read("package.json").contains("\"version\": \"1.3.0\""));
}

/// S1: `major` is the other level the rule covers, whether deduced or named.
#[test]
fn a_named_major_release_owes_the_same_declaration() {
    let guard = released("declaration-named-major");
    let fixture = guard.fixture();
    commit(fixture, "fix(api): stop dropping scoped commits");

    let output = cutver_in(fixture, &["bump", "major"]);

    assert_eq!(output.status.code(), Some(1), "stdout: {}", stdout_of(&output));
    assert!(
        stderr_of(&output).contains("the major release over 'v1.2.3..HEAD'"),
        "{}",
        stderr_of(&output)
    );
    assert!(
        fixture.read("package.json").contains("\"version\": \"1.2.3\""),
        "nothing was written"
    );
}

/// S4: `cutver plan` reports the guard as a blocker, exits `0` and writes nothing.
#[test]
fn the_plan_reports_the_undeclared_range_as_a_blocker() {
    let guard = released("declaration-plan-blocker");
    let fixture = guard.fixture();
    commit(fixture, "feat(api): add the gallery endpoint");

    let before = snapshot(&fixture.dir);
    let head_before = head(fixture);
    let tags_before = tags(fixture);

    let plan = plan_json(fixture, &["plan", "--json", "minor"]);

    let blockers: Vec<&str> = plan["blockers"]
        .as_array()
        .unwrap_or_else(|| panic!("blockers is an array: {plan}"))
        .iter()
        .map(|blocker| blocker.as_str().expect("a blocker is one line of text"))
        .collect();
    assert_eq!(
        blockers.len(),
        1,
        "exactly one guard is reported for a clean tree: {blockers:?}"
    );
    let blocker = blockers[0];
    assert!(!blocker.contains('\n'), "a blocker is one line: {blocker:?}");
    assert!(blocker.contains("v1.2.3..HEAD"), "{blocker}");
    assert!(blocker.contains("minor"), "{blocker}");
    assert!(blocker.contains("BREAKING CHANGE:"), "{blocker}");
    assert!(blocker.contains("NO-BREAKING-CHANGE:"), "{blocker}");
    assert_eq!(
        plan["next_version"], "1.3.0",
        "the plan still reports the release it computes"
    );

    assert_eq!(snapshot(&fixture.dir), before, "a plan writes nothing");
    assert_eq!(head(fixture), head_before, "a plan creates no commit");
    assert_eq!(tags(fixture), tags_before, "a plan creates no tag");

    let human = cutver_in(fixture, &["plan", "minor"]);
    assert_eq!(human.status.code(), Some(0), "the human path exits 0 too");
    assert!(
        stdout_of(&human).contains("A real bump would abort on 1 blocker(s)"),
        "the human path prints the blocker: {}",
        stdout_of(&human)
    );
}

/// S4 and S2: a declared plan carries no blocker at all, so the document separates the two states.
#[test]
fn a_declared_range_plans_without_a_blocker() {
    let guard = released("declaration-plan-clear");
    let fixture = guard.fixture();
    commit(
        fixture,
        &format!("refactor(plugin): rewrite the handshake\n\n{MIGRATION}"),
    );

    let plan = plan_json(fixture, &["plan", "--json", "minor"]);

    assert_eq!(
        plan["blockers"],
        serde_json::json!([]),
        "a declared range has nothing to report: {plan}"
    );
}

/// S6: a footer is a footer. Prose that names the sentinel in the commit body does not satisfy the
/// check, so the range is still silence.
#[test]
fn prose_naming_the_footer_does_not_satisfy_the_check() {
    let guard = released("declaration-prose-only");
    let fixture = guard.fixture();
    commit(fixture, "feat(api): add the gallery endpoint");
    commit(
        fixture,
        "docs(api): record the review\n\nThe review asked whether\nNO-BREAKING-CHANGE: would be the honest\ndeclaration here, and left it for the next release to write.",
    );

    let output = cutver_in(fixture, &["bump", "minor"]);

    assert_eq!(
        output.status.code(),
        Some(1),
        "prose is not a footer: stdout: {}",
        stdout_of(&output)
    );
    assert!(
        stderr_of(&output).contains("NO-BREAKING-CHANGE:"),
        "the failure still names the footer to add: {}",
        stderr_of(&output)
    );
}
