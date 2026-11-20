//! The deprecation channel (#272, v1.0 block 2 of #346).
//!
//! A deprecated spelling keeps working, warns on stderr in exactly one sentence that names both
//! spellings and the release that removes it, reaches a detected CI as one annotation, and never
//! touches stdout, the exit code or the difference between "deprecated" and "unknown".

mod common;

use std::fs;
use std::path::Path;
use std::process::Output;

use common::ci::Sinks;
use common::cutver_command;
use tempfile::TempDir;

/// The sentence `[version].source` must warn with, exactly as the maintainer approved its shape:
/// `warning: <where> is deprecated, use <canonical>; it will be removed in <version>`.
const SOURCE_MESSAGE: &str = "[version].source is deprecated, use current_source; it will be removed in 1.0.0";

/// The same sentence for the `kind` values, which name a value rather than a table key.
const KIND_TOML_MESSAGE: &str =
    r#"[[manifest]].kind = "toml" is deprecated, use "cargo-package"; it will be removed in 1.0.0"#;
const KIND_PYPROJECT_MESSAGE: &str =
    r#"[[manifest]].kind = "pyproject-toml" is deprecated, use "pyproject"; it will be removed in 1.0.0"#;

const PACKAGE_JSON: &str = "{\n  \"name\": \"deprecations\",\n  \"version\": \"1.2.3\"\n}\n";
const CARGO_TOML: &str = "[package]\nname = \"deprecations\"\nversion = \"1.2.3\"\n";
const PYPROJECT_TOML: &str = "[project]\nname = \"deprecations\"\nversion = \"1.2.3\"\n";

/// A workspace whose `cutver.toml` is `config`, with the manifests that document names.
fn workspace(toml: &str) -> TempDir {
    let dir = TempDir::new().expect("a temporary workspace");
    fs::write(dir.path().join("cutver.toml"), toml).expect("writing cutver.toml");
    fs::write(dir.path().join("package.json"), PACKAGE_JSON).expect("writing package.json");
    fs::write(dir.path().join("Cargo.toml"), CARGO_TOML).expect("writing Cargo.toml");
    fs::write(dir.path().join("pyproject.toml"), PYPROJECT_TOML).expect("writing pyproject.toml");
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

/// Lines of `text` that are exactly `line`, so "warned once" is a count and not a `contains`.
fn line_count(text: &str, line: &str) -> usize {
    text.lines().filter(|candidate| *candidate == line).count()
}

fn assert_exit(output: &Output, expected: i32) {
    assert_eq!(
        output.status.code(),
        Some(expected),
        "stdout: {}\nstderr: {}",
        stdout_of(output),
        stderr_of(output)
    );
}

/// The deprecated `[version].source` alias: accepted, and warned about naming both spellings.
#[test]
fn a_deprecated_version_key_still_works_and_warns_naming_both_spellings() {
    let dir = workspace(
        r#"[version]
source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"
"#,
    );

    let output = plain(dir.path(), &["doctor"]);

    assert_exit(&output, 0);
    let stderr = stderr_of(&output);
    assert!(
        stderr.contains(&format!("warning: {SOURCE_MESSAGE}")),
        "the warning must name both spellings and the removal release; stderr: {stderr}"
    );
    assert!(
        !stdout_of(&output).contains("deprecated"),
        "stdout carries the payload, never the warning; stdout: {}",
        stdout_of(&output)
    );
}

/// The legacy `kind = "toml"`, whose spelling is a value rather than a key.
#[test]
fn a_legacy_cargo_manifest_kind_still_works_and_warns() {
    let dir = workspace(
        r#"[version]
current_source = "Cargo.toml"

[[manifest]]
path = "Cargo.toml"
kind = "toml"
"#,
    );

    let output = plain(dir.path(), &["doctor"]);

    assert_exit(&output, 0);
    assert!(
        stderr_of(&output).contains(&format!("warning: {KIND_TOML_MESSAGE}")),
        "stderr: {}",
        stderr_of(&output)
    );
}

/// The legacy `kind = "pyproject-toml"`.
#[test]
fn a_legacy_pyproject_manifest_kind_still_works_and_warns() {
    let dir = workspace(
        r#"[version]
current_source = "pyproject.toml"

[[manifest]]
path = "pyproject.toml"
kind = "pyproject-toml"
"#,
    );

    let output = plain(dir.path(), &["doctor"]);

    assert_exit(&output, 0);
    assert!(
        stderr_of(&output).contains(&format!("warning: {KIND_PYPROJECT_MESSAGE}")),
        "stderr: {}",
        stderr_of(&output)
    );
}

/// Under a detected CI the same sentence is also one annotation, and the exit code does not move.
#[test]
fn under_ci_the_same_sentence_is_one_warning_annotation() {
    let dir = workspace(
        r#"[version]
source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"
"#,
    );
    let sinks = Sinks::new("config-deprecations");

    let output = common::ci::with_ci(dir.path(), &["doctor"], &sinks);

    assert_exit(&output, 0);
    let stderr = stderr_of(&output);
    assert!(
        stderr.contains(&format!("::warning::{SOURCE_MESSAGE}")),
        "a detected CI gets the annotation with the same text; stderr: {stderr}"
    );
    assert!(
        stderr.contains(&format!("warning: {SOURCE_MESSAGE}")),
        "the human line stays, because a reader of the log is not a runner; stderr: {stderr}"
    );
    assert!(
        !stdout_of(&output).contains("::warning::"),
        "the annotation never reaches stdout; stdout: {}",
        stdout_of(&output)
    );
}

/// The two cases stay distinguishable: a never-valid key still fails closed, and it does not warn.
#[test]
fn an_unknown_key_still_fails_closed_and_never_warns() {
    let dir = workspace(
        r#"[version]
source = "package.json"

[project]
colour = "deprecations"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"
"#,
    );

    let output = plain(dir.path(), &["doctor"]);

    assert_exit(&output, 1);
    let stderr = stderr_of(&output);
    assert!(
        stderr.contains("unknown configuration key 'colour' in [project] table in cutver.toml"),
        "the unknown-key message is unchanged; stderr: {stderr}"
    );
    assert!(
        !stderr.contains("deprecated"),
        "a rejected document warns about nothing; stderr: {stderr}"
    );
}

/// The legacy key inside an inline `[version]` table: same alias, same sentence, once.
#[test]
fn a_version_inline_table_loads_and_warns_once() {
    let dir = workspace(
        r#"version = { source = "package.json" }

[[manifest]]
path = "package.json"
kind = "json"
field = "version"
"#,
    );

    let output = plain(dir.path(), &["doctor"]);

    assert_exit(&output, 0);
    assert_eq!(
        line_count(&stderr_of(&output), &format!("warning: {SOURCE_MESSAGE}")),
        1,
        "stderr: {}",
        stderr_of(&output)
    );
}

/// The legacy `kind` inside a root-level inline array of tables.
#[test]
fn a_manifest_inline_array_loads_and_warns_once() {
    let dir = workspace(
        r#"manifest = [{ path = "Cargo.toml", kind = "toml" }]

[version]
current_source = "Cargo.toml"
"#,
    );

    let output = plain(dir.path(), &["doctor"]);

    assert_exit(&output, 0);
    assert_eq!(
        line_count(&stderr_of(&output), &format!("warning: {KIND_TOML_MESSAGE}")),
        1,
        "stderr: {}",
        stderr_of(&output)
    );
}

/// Two spellings in one document warn once each: not twice, not zero times.
#[test]
fn two_deprecated_spellings_warn_once_each() {
    let dir = workspace(
        r#"[version]
source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[[manifest]]
path = "Cargo.toml"
kind = "toml"
"#,
    );

    let output = plain(dir.path(), &["doctor"]);
    let stderr = stderr_of(&output);

    assert_exit(&output, 0);
    assert_eq!(
        line_count(&stderr, &format!("warning: {SOURCE_MESSAGE}")),
        1,
        "stderr: {stderr}"
    );
    assert_eq!(
        line_count(&stderr, &format!("warning: {KIND_TOML_MESSAGE}")),
        1,
        "stderr: {stderr}"
    );
    assert_eq!(stderr.matches("deprecated").count(), 2, "stderr: {stderr}");
}

/// No entry is decorative: each row of the table is detected by its own spelling, and the table
/// says which release removes it.
#[test]
fn every_table_entry_is_detected_and_names_its_replacement_and_removal() {
    let documents = [
        ("[version]\nsource = \"package.json\"\n", SOURCE_MESSAGE),
        (
            "[version]\ncurrent_source = \"Cargo.toml\"\n\n[[manifest]]\npath = \"Cargo.toml\"\nkind = \"toml\"\n",
            KIND_TOML_MESSAGE,
        ),
        (
            "[version]\ncurrent_source = \"pyproject.toml\"\n\n[[manifest]]\npath = \"pyproject.toml\"\nkind = \"pyproject-toml\"\n",
            KIND_PYPROJECT_MESSAGE,
        ),
    ];

    assert_eq!(
        documents.len(),
        cutver::config::DEPRECATIONS.len(),
        "a new table entry needs a document here that exercises it"
    );

    for (document, message) in documents {
        let found = cutver::config::deprecations_in(document).expect("a well-formed document scans");
        assert_eq!(
            found.iter().map(|entry| entry.message()).collect::<Vec<_>>(),
            vec![message.to_string()],
            "document: {document}"
        );
    }

    let canonical = r#"[version]
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"
"#;
    assert!(
        cutver::config::deprecations_in(canonical)
            .expect("a well-formed document scans")
            .is_empty(),
        "a canonical document carries no deprecation"
    );
}
