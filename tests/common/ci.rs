//! Fixtures for the CI output channel (`tests/ci_output.rs`, #326 phase 4A).
//!
//! They live next to the other shared fixtures so the suite stays inside its line budget, and so
//! `with_ci` is the only place that sets the channel's three variables: a second suite describing a
//! detected CI differently could not be caught by the suite that owns the behaviour.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

use super::{
    CARGO_TOML, CHANGELOG_MD, Fixture, FixtureGuard, PACKAGE_JSON, cutver_command, git_available, init_git_repo,
    initial_commit, run_git, run_git_ok,
};
use tempfile::TempDir;

/// The configuration every CI fixture starts from: one JSON manifest, one Cargo manifest, a
/// changelog and the clean-tree guard a real release runs under.
pub const TOML: &str = r#"[version]
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

/// A repository at 1.2.3 on `main` whose `cutver.toml` is `config`.
pub fn repo(name: &str, config: &str) -> FixtureGuard {
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
    run_git_ok(&fixture.dir, &["checkout", "-B", "main"]);
    initial_commit(fixture);
    guard
}

/// The two files a runner hands the tool, in a directory of their own: they are outside the
/// workspace, so filling them can never dirty the tree a release runs over.
pub struct Sinks {
    dir: TempDir,
}

impl Sinks {
    pub fn new(name: &str) -> Self {
        let dir = tempfile::Builder::new()
            .prefix(&format!("cutver-ci-{name}-"))
            .tempdir()
            .expect("a temporary sinks directory");
        Self { dir }
    }

    pub fn output(&self) -> PathBuf {
        self.dir.path().join("github_output")
    }

    pub fn summary(&self) -> PathBuf {
        self.dir.path().join("github_step_summary")
    }

    pub fn output_text(&self) -> String {
        fs::read_to_string(self.output()).expect("the run wrote `$GITHUB_OUTPUT`")
    }

    pub fn summary_text(&self) -> String {
        fs::read_to_string(self.summary()).expect("the run wrote `$GITHUB_STEP_SUMMARY`")
    }
}

/// Runs as GitHub Actions does: `GITHUB_ACTIONS=true` and both files named.
pub fn with_ci(dir: &Path, args: &[&str], sinks: &Sinks) -> Output {
    cutver_command(dir)
        .env("GITHUB_ACTIONS", "true")
        .env("GITHUB_OUTPUT", sinks.output())
        .env("GITHUB_STEP_SUMMARY", sinks.summary())
        .args(args)
        .output()
        .expect("failed to execute the cutver binary")
}

/// Runs with the two file variables named and `GITHUB_ACTIONS` absent: the state `--ci` exists for.
pub fn without_github_actions(dir: &Path, args: &[&str], sinks: &Sinks) -> Output {
    cutver_command(dir)
        .env("GITHUB_OUTPUT", sinks.output())
        .env("GITHUB_STEP_SUMMARY", sinks.summary())
        .args(args)
        .output()
        .expect("failed to execute the cutver binary")
}

/// The commit `HEAD` points at, after a run.
pub fn head_commit(fixture: &Fixture) -> String {
    let out = run_git(&fixture.dir, &["rev-parse", "HEAD"]);
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// Every file the workspace holds, keyed by its path relative to the root. `.git/index` and
/// `.git/logs` are the two entries a *read* may rewrite on its own; everything else is compared.
pub fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut files = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(&dir).expect("the workspace is readable") {
            let path = entry.expect("the directory entry is readable").path();
            let relative = path.strip_prefix(root).expect("inside the workspace").to_path_buf();
            if relative == Path::new(".git/index") || relative.starts_with(".git/logs") {
                continue;
            }
            match path.is_dir() {
                true => pending.push(path),
                false => {
                    files.insert(relative, fs::read(&path).expect("the file is readable"));
                }
            }
        }
    }
    files
}
