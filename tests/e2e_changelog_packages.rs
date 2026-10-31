#![cfg(unix)]

mod common;

use common::*;
use std::process::Command;
use std::time::SystemTime;

const CHANGELOG_INITIAL: &str = "# Changelog\n\n## [1.0.0] - 2026-01-01\n\n- Initial release\n";

struct Workspace<'a> {
    guard: &'a FixtureGuard,
}

impl Workspace<'_> {
    fn fixture(&self) -> &Fixture {
        self.guard.fixture()
    }

    fn bump(&self) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_cutver"))
            .current_dir(&self.fixture().dir)
            .args(["bump", "minor"])
            .output()
            .expect("failed to execute cutver bump")
    }
}

/// Writes a three-package workspace: `alpha` (Cargo), `beta` and `gamma` (JSON).
///
/// `package_keys = false` removes every `package` key, which is the backward-compatibility shape.
fn write_workspace(guard: &FixtureGuard, package_keys: bool, extra_changelog: &str) {
    let fixture = guard.fixture();
    let alpha_key = if package_keys { "package = \"alpha\"\n" } else { "" };
    let beta_key = if package_keys { "package = \"beta\"\n" } else { "" };
    let gamma_key = if package_keys { "package = \"gamma\"\n" } else { "" };

    fixture.write(
        "cutver.toml",
        &format!(
            r#"[[manifest]]
path = "crates/alpha/Cargo.toml"
kind = "cargo-package"
{alpha_key}
[[manifest]]
path = "crates/beta/package.json"
kind = "json"
field = "version"
{beta_key}
[[manifest]]
path = "crates/gamma/package.json"
kind = "json"
field = "version"
{gamma_key}
[changelog]
path = "CHANGELOG.md"
{extra_changelog}
[git]
tag_prefix = "v"
require_clean_tree = true
"#
        ),
    );
    fixture.write(
        "crates/alpha/Cargo.toml",
        r#"[package]
name = "alpha"
version = "1.0.0"
edition = "2021"
"#,
    );
    fixture.write("crates/alpha/src/lib.rs", "// alpha\n");
    fixture.write(
        "crates/beta/package.json",
        r#"{
  "name": "beta",
  "version": "1.0.0"
}
"#,
    );
    fixture.write(
        "crates/gamma/package.json",
        r#"{
  "name": "gamma",
  "version": "1.0.0"
}
"#,
    );
    fixture.write("CHANGELOG.md", CHANGELOG_INITIAL);

    init_git_repo(fixture);
    initial_commit(fixture);
    run_git_ok(&fixture.dir, &["tag", "v1.0.0"]);

    // `beta` commits after `alpha` on purpose: declaration order, not history order, must win.
    fixture.write("crates/alpha/src/lib.rs", "// alpha feature\n");
    run_git_ok(&fixture.dir, &["add", "."]);
    run_git_ok(&fixture.dir, &["commit", "-m", "feat(alpha): alpha feature"]);

    fixture.write("crates/beta/index.js", "// beta fix\n");
    run_git_ok(&fixture.dir, &["add", "."]);
    run_git_ok(&fixture.dir, &["commit", "-m", "fix(beta): beta fix"]);

    fixture.write(".github/workflows/ci.yml", "name: ci\n");
    run_git_ok(&fixture.dir, &["add", "."]);
    run_git_ok(&fixture.dir, &["commit", "-m", "ci: tune pipeline"]);
}

fn assert_bump_succeeded(output: &std::process::Output) {
    assert!(
        output.status.success(),
        "bump failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn package_sections_render_in_declaration_order_and_group_unowned_commits() {
    assert!(
        git_available(),
        "git CLI is required for e2e tests but was not found in PATH"
    );
    let guard = FixtureGuard::new("changelog-packages-order");
    write_workspace(&guard, true, "");
    let workspace = Workspace { guard: &guard };

    let output = workspace.bump();
    assert_bump_succeeded(&output);

    let changelog = workspace.fixture().read("CHANGELOG.md");
    let date = cutver::changelog::format_date(SystemTime::now());
    let expected_section = format!(
        "## [v1.1.0] - {date}\n\n\
### alpha\n\
#### Features\n\
- **alpha**: alpha feature\n\n\
### beta\n\
#### Bug Fixes\n\
- **beta**: beta fix\n\n\
### Other changes\n\
- tune pipeline\n"
    );
    assert!(
        changelog.contains(&expected_section),
        "package sections are not rendered as designed: {changelog}"
    );
    assert!(
        !changelog.contains("### gamma"),
        "a package with no commit in the range must render no heading: {changelog}"
    );
    let alpha = changelog.find("### alpha").expect("alpha heading");
    let beta = changelog.find("### beta").expect("beta heading");
    let other = changelog.find("### Other changes").expect("other heading");
    assert!(
        alpha < beta && beta < other,
        "packages must render in declaration order before unowned commits: {changelog}"
    );
}

#[test]
fn changelog_without_package_keys_is_byte_identical_to_the_flat_render() {
    assert!(
        git_available(),
        "git CLI is required for e2e tests but was not found in PATH"
    );
    let guard = FixtureGuard::new("changelog-packages-backcompat");
    write_workspace(&guard, false, "");
    let workspace = Workspace { guard: &guard };

    let output = workspace.bump();
    assert_bump_succeeded(&output);

    let changelog = workspace.fixture().read("CHANGELOG.md");
    let date = cutver::changelog::format_date(SystemTime::now());
    let expected = format!(
        "# Changelog\n\n\
## [v1.1.0] - {date}\n\n\
### Features\n\
- **alpha**: alpha feature\n\n\
### Bug Fixes\n\
- **beta**: beta fix\n\n\
### Maintenance\n\
- tune pipeline\n\
## [1.0.0] - 2026-01-01\n\n\
- Initial release\n"
    );
    assert_eq!(
        changelog, expected,
        "a config with no package keys must render exactly the flat changelog"
    );
}

#[test]
fn template_mode_keeps_todays_output_when_packages_are_declared() {
    assert!(
        git_available(),
        "git CLI is required for e2e tests but was not found in PATH"
    );
    let guard = FixtureGuard::new("changelog-packages-template");
    write_workspace(
        &guard,
        true,
        "mode = \"template\"\nentry_template = \"Static release notes.\"\n",
    );
    let workspace = Workspace { guard: &guard };

    let output = workspace.bump();
    assert_bump_succeeded(&output);

    let changelog = workspace.fixture().read("CHANGELOG.md");
    assert!(
        changelog.contains("Static release notes."),
        "template mode must keep rendering the entry template: {changelog}"
    );
    assert!(
        !changelog.contains("### alpha"),
        "template mode must not grow package sections: {changelog}"
    );
}
