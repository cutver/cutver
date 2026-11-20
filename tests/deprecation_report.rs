//! The deprecation channel as data (#272, v1.0 block 2 of #346).
//!
//! `tests/config_deprecations.rs` owns the channel that reaches a reader: the sentence, the CI
//! annotation, the exit code that does not move. This file owns the second half of the same
//! obligation — what a machine that never reads stderr can act on, and the two surfaces
//! `CONTRACT.md` names as migration warnings.

mod common;

use std::fs;
use std::path::Path;
use std::process::Output;

use common::cutver_command;
use tempfile::TempDir;

const SOURCE_MESSAGE: &str = "[version].source is deprecated, use current_source; it will be removed in 1.0.0";

const PACKAGE_JSON: &str = "{\n  \"name\": \"report\",\n  \"version\": \"1.2.3\"\n}\n";

const DEPRECATED: &str = r#"[version]
source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"
"#;

const CANONICAL: &str = r#"[version]
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"
"#;

fn workspace(toml: &str) -> TempDir {
    let dir = TempDir::new().expect("a temporary workspace");
    fs::write(dir.path().join("cutver.toml"), toml).expect("writing cutver.toml");
    fs::write(dir.path().join("package.json"), PACKAGE_JSON).expect("writing package.json");
    dir
}

fn plain(dir: &Path, args: &[&str]) -> Output {
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

/// A machine run keeps the human sentence off stdout, and carries it where the contract says.
#[test]
fn a_machine_run_carries_the_deprecation_as_data_and_keeps_stdout_clean() {
    let dir = workspace(DEPRECATED);

    let output = plain(dir.path(), &["doctor", "--json"]);

    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr_of(&output));
    let stdout = stdout_of(&output);
    let document: serde_json::Value = serde_json::from_str(&stdout).expect("one JSON object on stdout");
    assert_eq!(document["schemaVersion"], 1, "the version does not move: {stdout}");
    assert_eq!(
        document["warnings"],
        serde_json::json!([SOURCE_MESSAGE]),
        "the deprecation must travel as data naming both spellings; stdout: {stdout}"
    );
    assert!(
        !stdout.contains("warning: "),
        "the human sentence stays on stderr; stdout: {stdout}"
    );
    assert!(
        stderr_of(&output).contains(&format!("warning: {SOURCE_MESSAGE}")),
        "the human line is emitted either way; stderr: {}",
        stderr_of(&output)
    );
}

/// Nothing degenerate: a canonical document reports an empty array, never a missing key.
#[test]
fn a_canonical_document_reports_an_empty_array_and_warns_nothing() {
    let dir = workspace(CANONICAL);

    let output = plain(dir.path(), &["doctor", "--json"]);

    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr_of(&output));
    assert!(
        !stderr_of(&output).contains("deprecated"),
        "stderr: {}",
        stderr_of(&output)
    );
    let document: serde_json::Value = serde_json::from_str(&stdout_of(&output)).expect("one JSON object on stdout");
    assert_eq!(
        document["warnings"],
        serde_json::json!([]),
        "an absent warning is an empty array; stdout: {}",
        stdout_of(&output)
    );
}

/// A deprecation is not a failure: drift still reports `ok: false` and exit `2`.
#[test]
fn a_deprecation_does_not_move_the_drift_exit_code() {
    let dir = workspace(
        r#"[version]
source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"
"#,
    );
    fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"report\"\nversion = \"9.9.9\"\n",
    )
    .expect("writing Cargo.toml");

    let output = plain(dir.path(), &["doctor", "--json"]);

    assert_eq!(
        output.status.code(),
        Some(2),
        "drift is the outcome, the deprecation is not; stderr: {}",
        stderr_of(&output)
    );
    let document: serde_json::Value = serde_json::from_str(&stdout_of(&output)).expect("one JSON object on stdout");
    assert_eq!(document["ok"], false, "stdout: {}", stdout_of(&output));
    assert_eq!(
        document["warnings"],
        serde_json::json!([SOURCE_MESSAGE]),
        "stdout: {}",
        stdout_of(&output)
    );
}

/// `CONTRACT.md` names `cutver init --update` as the second migration-warning surface.
#[test]
fn init_update_names_both_spellings_for_a_deprecated_key() {
    let dir = workspace(DEPRECATED);

    let output = plain(dir.path(), &["init", "--update"]);

    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr_of(&output));
    assert!(
        stderr_of(&output).contains(&format!("warning: {SOURCE_MESSAGE}")),
        "stderr: {}",
        stderr_of(&output)
    );
    assert!(
        !stdout_of(&output).contains("deprecated"),
        "stdout: {}",
        stdout_of(&output)
    );
}

/// The warning is not a one-shot: an update with nothing to add still reports the rename.
#[test]
fn init_update_warns_when_there_is_nothing_to_update() {
    let dir = workspace(DEPRECATED);

    let output = plain(dir.path(), &["init", "--update"]);

    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr_of(&output));
    assert!(
        stdout_of(&output).contains("already up to date"),
        "the fixture has no new manifest to add; stdout: {}",
        stdout_of(&output)
    );
    assert_eq!(
        stderr_of(&output)
            .lines()
            .filter(|line| *line == format!("warning: {SOURCE_MESSAGE}"))
            .count(),
        1,
        "stderr: {}",
        stderr_of(&output)
    );
}
