//! Guards around the point of no return in the release pipeline (issue #265, finding F-09).
//!
//! Two failures have to be cheap, and today neither is:
//!
//! * An invalid tag name — a `tag_prefix` that cannot form a git ref — was only discovered when
//!   `git tag` ran, which is *after* the release commit exists. The workspace was then modified and
//!   committed with no tag and no compensation.
//! * Any failure after the commit (a publish command, for instance) said nothing about the commit
//!   and tag that had already landed, so the operator had to reconstruct the state by hand.
//!
//! Both tests run on every supported platform: the publish command is `exit 1`, which both `sh -c`
//! and `cmd /C` reject, and the bare remote is built with plain `git` commands.

mod common;

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use common::*;
use cutver::bump::run as bump_run;
use cutver::semver_bump::Bump;

const INVALID_PREFIX_TOML: &str = r#"[version]
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
tag_prefix = "{prefix}"
require_clean_tree = true
require_branch = "main"

[publish]
push = true
commands = ["echo published {version} > published.txt"]
"#;

const FAILING_PUBLISH_TOML: &str = r#"[version]
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
require_branch = "main"

[publish]
push = true
commands = ["exit 1"]
"#;

/// Every file in the worktree, byte for byte, excluding `.git`: `git status` refreshes the index,
/// so `.git/index` is allowed to change while the worktree is untouched.
fn snapshot_worktree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(dir: &Path, root: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        let entries = fs::read_dir(dir).unwrap();
        for entry in entries {
            let entry = entry.unwrap();
            let path = entry.path();
            let name = entry.file_name();
            if name == ".git" {
                continue;
            }
            if path.is_dir() {
                walk(&path, root, out);
            } else {
                let rel = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
                out.insert(rel, fs::read(&path).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

fn fixture_with_config(name: &str, toml: &str) -> common::FixtureGuard {
    let guard = FixtureGuard::new(name);
    let fixture = guard.fixture();
    fixture.write("package.json", PACKAGE_JSON);
    fixture.write("Cargo.toml", CARGO_TOML);
    fixture.write("CHANGELOG.md", CHANGELOG_MD);
    fixture.write("cutver.toml", toml);
    init_git_repo(fixture);
    run_git_ok(&fixture.dir, &["checkout", "-B", "main"]);
    initial_commit(fixture);
    guard
}

#[test]
fn an_invalid_tag_prefix_fails_before_the_workspace_is_touched() {
    assert!(
        git_available(),
        "git CLI is required for e2e tests but was not found in PATH"
    );

    // Each of these cannot form a git ref, so `git tag` would refuse the name — but only after the
    // bump had been written and committed. The rehearsal must refuse it too.
    for prefix in ["v~", "v^", "v:", "v.."] {
        let guard = fixture_with_config("release-tag-prefix", &INVALID_PREFIX_TOML.replace("{prefix}", prefix));
        let fixture = guard.fixture();
        let before = snapshot_worktree(&fixture.dir);
        let head_before = rev(&fixture.dir, "HEAD");
        let commits_before = commit_count(fixture);

        let cfg = common::load_config(guard.fixture());

        let dry = bump_run(&cfg, Bump::Minor, true, &[]);
        let dry_text = dry
            .as_ref()
            .err()
            .map(|e| e.to_string())
            .unwrap_or_else(|| "a dry run must reject an invalid tag prefix".to_string());
        assert!(
            dry.is_err(),
            "prefix '{prefix}': a rehearsal must fail on an invalid tag name, got {dry:?}"
        );
        assert!(
            dry_text.contains(&format!("{prefix}1.3.0")),
            "prefix '{prefix}': the error must name the tag it refused, got: {dry_text}"
        );
        assert!(
            dry_text.contains("tag_prefix"),
            "prefix '{prefix}': the error must name the setting to change, got: {dry_text}"
        );
        assert!(
            dry_text.contains("Where:") && dry_text.contains("Fix:"),
            "prefix '{prefix}': the error must be actionable, got: {dry_text}"
        );

        let err = bump_run(&cfg, Bump::Minor, false, &[]).expect_err("an invalid tag prefix must fail the run");
        let text = err.to_string();
        assert!(
            matches!(err, cutver::bump::Error::InvalidTagName { .. }),
            "prefix '{prefix}': expected a typed tag-name error, got: {err:?}"
        );
        assert!(
            text.contains(&format!("{prefix}1.3.0")),
            "prefix '{prefix}': the error must name the tag it refused, got: {text}"
        );

        // Nothing was written, staged, committed or tagged.
        assert_eq!(
            snapshot_worktree(&fixture.dir),
            before,
            "prefix '{prefix}': every file must be byte-identical after the failure"
        );
        assert_eq!(
            rev(&fixture.dir, "HEAD"),
            head_before,
            "prefix '{prefix}': no commit may be created"
        );
        assert_eq!(
            commit_count(fixture),
            commits_before,
            "prefix '{prefix}': the history must not grow"
        );
        assert!(
            !tag_exists(fixture, &format!("{prefix}1.3.0")),
            "prefix '{prefix}': no tag may be created"
        );
        let porcelain = String::from_utf8_lossy(&run_git(&fixture.dir, &["status", "--porcelain"]).stdout).to_string();
        assert!(
            porcelain.trim().is_empty(),
            "prefix '{prefix}': the worktree must be clean, got: {porcelain}"
        );
    }
}

#[test]
fn a_failing_publish_command_names_the_commit_and_the_tag_that_already_landed() {
    assert!(
        git_available(),
        "git CLI is required for e2e tests but was not found in PATH"
    );
    let guard = FixtureGuard::new("release-post-commit-failure");
    let fixture = guard.fixture();
    let remote = Fixture::new("remote-accepts-all");
    run_git_ok(&remote.dir, &["init", "--bare", "-q"]);

    fixture.write("package.json", PACKAGE_JSON);
    fixture.write("Cargo.toml", CARGO_TOML);
    fixture.write("CHANGELOG.md", CHANGELOG_MD);
    fixture.write("cutver.toml", FAILING_PUBLISH_TOML);
    init_git_repo(fixture);
    run_git_ok(&fixture.dir, &["checkout", "-B", "main"]);
    initial_commit(fixture);
    run_git_ok(&fixture.dir, &["remote", "add", "origin", remote.dir.to_str().unwrap()]);
    run_git_ok(&fixture.dir, &["push", "origin", "refs/heads/main"]);

    let cfg = common::load_config(guard.fixture());
    let err = bump_run(&cfg, Bump::Minor, false, &[]).expect_err("the publish command exits non-zero");
    let text = err.to_string();

    // The commit and the tag already exist — locally and, because the pushes succeeded, remotely.
    let head = rev(&fixture.dir, "HEAD");
    assert_eq!(
        rev(&remote.dir, "refs/heads/main"),
        head,
        "the branch push must have succeeded before the publish command ran"
    );
    assert_eq!(
        rev(&fixture.dir, "v1.3.0^{commit}"),
        head,
        "the release tag must already point at the release commit"
    );
    assert_eq!(
        rev(&remote.dir, "refs/tags/v1.3.0^{commit}"),
        head,
        "the release tag must already be published"
    );

    // ...and the failure has to say so, instead of leaving the operator to work it out.
    assert!(
        text.contains("v1.3.0"),
        "the failure must name the release tag that already landed, got: {text}"
    );
    assert!(
        text.contains(&head),
        "the failure must name the release commit that already landed, got: {text}"
    );
    assert!(
        text.contains("Where:") && text.contains("Fix:"),
        "the failure must be actionable, got: {text}"
    );
    // The failure must not assert a remote state it cannot know: `[publish] push` may be off, and
    // the first push may have failed before anything landed. It has to send the operator to look,
    // not to delete (native review #265, R1-001/R1-002/R4-001/R4-002).
    assert!(
        text.contains("ls-remote"),
        "the fix must tell the operator to inspect the remote before removing anything, got: {text}"
    );
    for claim in ["is already published", "are already on the remote"] {
        assert!(
            !text.contains(claim),
            "the fix must not assert `{claim}`: it depends on [publish] push and on which push failed, got: {text}"
        );
    }
}

fn rev(repo: &Path, reff: &str) -> String {
    let out = run_git(repo, &["rev-parse", reff]);
    assert!(
        out.status.success(),
        "git rev-parse {reff} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}
