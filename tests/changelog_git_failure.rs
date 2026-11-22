//! #266 S8, S9, S10 and the `.ok()` half of #267 (S11): the reads behind `changelog latest|show
//! --json` used to answer a failure with an empty value and exit `0`.
//!
//! Two different reads are pinned here, because both were converted into "nothing to report":
//!
//! - the git range behind the requested version, which an unreadable repository or a missing tag
//!   turned into `"commits": []` with exit `0`, so a consumer could not tell an empty range from a
//!   failed read;
//! - the `cutver.toml` the read resolves its tag prefix from, which a document that fails to load
//!   turned into "this workspace has no configuration", after which the read carried on with
//!   defaults instead of naming what was wrong.
//!
//! Every failure case asserts the same three things: a non-zero exit, a message naming the read that
//! failed, and **no** JSON document on stdout. The last test is the control: a workspace that really
//! has no configuration, and a range that really is empty, still read and still exit `0` — an empty
//! result stays reachable, which is what makes failing closed meaningful.

mod common;

use std::path::Path;
use std::process::Output;

use common::{FixtureGuard, cutver_command, git_available, init_git_repo, initial_commit, run_git_ok};

const CONFIG: &str = r#"[version]
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[changelog]
path = "CHANGELOG.md"
"#;

const CHANGELOG: &str = "# Changelog\n\n## [1.2.0] - 2026-03-01\n- new work\n\n## [1.1.0] - 2026-02-01\n- older work\n";

/// Three released versions, so `show 1.1.0` has a previous version and asks git for a real range
/// (`v1.0.0..v1.1.0`) instead of a single revision.
const THREE_RELEASES: &str = "# Changelog\n\n## [1.2.0] - 2026-03-01\n- new\n\n## [1.1.0] - 2026-02-01\n- middle\n\n## [1.0.0] - 2026-01-01\n- first\n";

/// A `cutver.toml` that cannot be parsed: the read used to treat this exactly like a workspace that
/// declares no configuration at all.
const BROKEN_CONFIG: &str = "[version\ncurrent_source = \"package.json\"\n";

/// The sentence the fail-closed git path carries, so a reader knows which read stopped.
const GIT_FAILURE: &str = "failed to read the release commit range from git:";

fn workspace(name: &str, config: Option<&str>) -> FixtureGuard {
    let guard = FixtureGuard::new(name);
    let fixture = guard.fixture();
    fixture.write("package.json", "{\n  \"version\": \"1.2.0\"\n}\n");
    fixture.write("CHANGELOG.md", CHANGELOG);
    if let Some(config) = config {
        fixture.write("cutver.toml", config);
    }
    guard
}

/// A repository whose changelog carries two released versions, tagged so the read has something to
/// answer — the failures below come from what is *missing*, not from the fixture.
fn repository(name: &str, config: Option<&str>) -> FixtureGuard {
    assert!(
        git_available(),
        "the git CLI is required for these fixtures but was not found in PATH"
    );
    let guard = workspace(name, config);
    let fixture = guard.fixture();
    init_git_repo(fixture);
    initial_commit(fixture);
    run_git_ok(&fixture.dir, &["tag", "v1.1.0"]);
    run_git_ok(&fixture.dir, &["tag", "1.1.0"]);
    guard
}

fn run(dir: &Path, args: &[&str]) -> Output {
    cutver_command(dir)
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

/// A failed read: exit `1`, the message that names it, and no document claiming a result.
fn assert_failed_read(output: &Output, read: &str) -> String {
    let stdout = stdout_of(output);
    let stderr = stderr_of(output);
    assert_eq!(
        output.status.code(),
        Some(1),
        "an unreadable {read} must not report success; stdout: {stdout:?}, stderr: {stderr:?}"
    );
    assert!(
        stdout.trim().is_empty(),
        "a failed read prints no document on stdout, so `\"commits\": []` can only ever mean the \
         range is empty: {stdout:?}"
    );
    stderr
}

/// S8, S10: outside a repository the range cannot be read at all. This used to be an empty commit
/// list with exit `0`.
#[test]
fn latest_json_fails_closed_when_there_is_no_repository_to_read() {
    let guard = workspace("changelog-latest-no-repo", Some(CONFIG));

    let output = run(&guard.fixture().dir, &["changelog", "latest", "--json"]);

    let stderr = assert_failed_read(&output, "commit range");
    assert!(stderr.contains(GIT_FAILURE), "stderr: {stderr}");
    assert!(
        stderr.contains("Fix:"),
        "the message says what to do about it, not only that it failed: {stderr}"
    );
}

/// S8, S10: the range names tags the repository does not have, so the read fails the same way rather
/// than answering with the commits it could guess.
#[test]
fn show_json_fails_closed_when_the_range_names_a_tag_that_does_not_exist() {
    let guard = workspace("changelog-show-missing-tag", Some(CONFIG));
    let fixture = guard.fixture();
    fixture.write("CHANGELOG.md", THREE_RELEASES);
    assert!(
        git_available(),
        "the git CLI is required for this fixture but was not found in PATH"
    );
    init_git_repo(fixture);
    initial_commit(fixture);

    let output = run(&fixture.dir, &["changelog", "show", "1.1.0", "--json"]);

    let stderr = assert_failed_read(&output, "commit range");
    assert!(stderr.contains(GIT_FAILURE), "stderr: {stderr}");
    assert!(
        stderr.contains("v1.0.0..v1.1.0"),
        "the message names the range that could not be read: {stderr}"
    );
}

/// S9, S11: `config::load(..).ok()` turned a broken document into "no configuration", and the read
/// then answered from defaults. `--path` is passed on purpose: without it the changelog path
/// resolver loads the configuration first, so the discarded error lived on this path exactly.
#[test]
fn latest_json_fails_closed_when_the_configuration_does_not_load() {
    let guard = workspace("changelog-broken-config", Some(BROKEN_CONFIG));

    let output = run(
        &guard.fixture().dir,
        &["changelog", "latest", "--path", "CHANGELOG.md", "--json"],
    );

    let stderr = assert_failed_read(&output, "configuration");
    assert!(
        stderr.contains("failed to parse config file"),
        "the message names what is wrong with the document: {stderr}"
    );
}

/// The control: a workspace that declares no configuration at all is not a failure, and a range that
/// really is empty still reports an empty list with exit `0`.
#[test]
fn a_workspace_with_no_configuration_still_reads_an_empty_range() {
    let guard = repository("changelog-no-config", None);

    let output = run(
        &guard.fixture().dir,
        &["changelog", "latest", "--path", "CHANGELOG.md", "--json"],
    );

    assert_eq!(
        output.status.code(),
        Some(0),
        "no `cutver.toml` is the domain answer \"this workspace declares none\", not an error; \
         stderr: {}",
        stderr_of(&output)
    );
    let document: serde_json::Value =
        serde_json::from_str(stdout_of(&output).trim()).expect("stdout is one JSON object");
    assert_eq!(
        document["commits"],
        serde_json::json!([]),
        "an empty range is still an empty array: {document}"
    );
}
