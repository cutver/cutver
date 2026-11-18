//! The failure half of the machine contract: when a command asked for machine output, its failure is
//! one JSON object on stderr and the exit code is unchanged; when it did not, the rendered human
//! error is byte-for-byte what it always was.
//!
//! Not `cfg(unix)`: none of this needs a git repository, so the contract is proven on every platform.

use std::process::{Command, Output};

use tempfile::TempDir;

/// A workspace with a `cutver.toml` that cannot be loaded.
fn unreadable_config() -> TempDir {
    let dir = TempDir::new().expect("a temporary workspace");
    std::fs::write(dir.path().join("cutver.toml"), "[version]\ncurrent_source = \"\n").expect("the config is written");
    dir
}

/// A workspace with two manifests that disagree, and no `cutver.toml` problem.
fn drifting_manifests() -> TempDir {
    let dir = TempDir::new().expect("a temporary workspace");
    let config = r#"[version]
current_source = "a.json"

[[manifest]]
path = "a.json"
kind = "json"
field = "version"

[[manifest]]
path = "b.json"
kind = "json"
field = "version"
"#;
    std::fs::write(dir.path().join("cutver.toml"), config).expect("the config is written");
    std::fs::write(dir.path().join("a.json"), r#"{"version": "1.1.0"}"#).expect("a manifest is written");
    std::fs::write(dir.path().join("b.json"), r#"{"version": "1.0.0"}"#).expect("a manifest is written");
    dir
}

fn run_in(dir: &TempDir, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cutver"))
        .current_dir(dir.path())
        .args(args)
        .output()
        .expect("failed to execute the cutver binary")
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

#[test]
fn a_machine_failure_is_one_json_object_on_stderr_with_the_exit_code_unchanged() {
    let dir = TempDir::new().expect("a temporary workspace");

    let output = run_in(&dir, &["bump", "--json"]);

    assert_eq!(output.status.code(), Some(1), "stderr: {}", stderr_of(&output));
    assert!(
        output.stdout.is_empty(),
        "a failure prints nothing on stdout: {:?}",
        String::from_utf8_lossy(&output.stdout)
    );
    let stderr = stderr_of(&output);
    assert_eq!(stderr.lines().count(), 1, "the failure is one line: {stderr:?}");

    let parsed: serde_json::Value = serde_json::from_str(stderr.trim()).unwrap_or_else(|err| {
        panic!("stderr must be one JSON object, got {stderr:?}: {err}");
    });
    assert_eq!(parsed["schemaVersion"], 1, "{stderr}");
    assert_eq!(parsed["error"]["code"], "config", "{stderr}");
    assert_eq!(parsed["error"]["exitCode"], 1, "{stderr}");
    let message = parsed["error"]["message"].as_str().expect("the message is a string");
    assert!(
        message.contains("no cutver.toml found"),
        "the message carries the diagnosis: {message}"
    );
    assert!(
        message.contains("cutver init"),
        "the message keeps the remedy the human path prints: {message}"
    );

    let version_at = stderr.find("\"schemaVersion\"").expect("the schema version is present");
    let error_at = stderr.find("\"error\"").expect("the error is present");
    assert!(version_at < error_at, "the schema version is the first key: {stderr}");
}

#[test]
fn the_human_path_keeps_the_rendered_error_and_prints_no_json() {
    let dir = TempDir::new().expect("a temporary workspace");

    let output = run_in(&dir, &["bump"]);

    assert_eq!(output.status.code(), Some(1));
    let stderr = stderr_of(&output);
    assert!(
        stderr.starts_with("Error loading config:"),
        "the human path keeps its rendered error, prefix included: {stderr:?}"
    );
    assert!(
        !stderr.trim_start().starts_with('{'),
        "the human path never prints an envelope: {stderr:?}"
    );
    assert!(
        stderr.contains("no cutver.toml found"),
        "the human text is unchanged: {stderr:?}"
    );
}

#[test]
fn the_report_flag_alone_selects_the_machine_failure() {
    let dir = TempDir::new().expect("a temporary workspace");

    let output = run_in(&dir, &["bump", "--report", "release.json"]);

    assert_eq!(output.status.code(), Some(1));
    let stderr = stderr_of(&output);
    let parsed: serde_json::Value = serde_json::from_str(stderr.trim()).unwrap_or_else(|err| {
        panic!("--report asks for machine output, so the failure is JSON: {stderr:?}: {err}");
    });
    assert_eq!(parsed["error"]["code"], "config", "{stderr}");
    assert!(
        !dir.path().join("release.json").exists(),
        "a failed run writes no report"
    );
}

#[test]
fn a_configuration_error_names_itself_in_the_envelope() {
    let dir = unreadable_config();

    let output = run_in(&dir, &["doctor", "--json"]);

    assert_eq!(output.status.code(), Some(1));
    let stderr = stderr_of(&output);
    let parsed: serde_json::Value = serde_json::from_str(stderr.trim())
        .unwrap_or_else(|err| panic!("stderr must be one JSON object, got {stderr:?}: {err}"));
    assert_eq!(parsed["error"]["code"], "config", "{stderr}");
    assert!(
        parsed["error"]["message"].as_str().is_some_and(|m| !m.is_empty()),
        "the message is never empty: {stderr}"
    );
    assert!(
        output.stdout.is_empty(),
        "no half-printed report before the failure: {:?}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn a_changelog_failure_keeps_its_own_code() {
    let dir = TempDir::new().expect("a temporary workspace");

    let output = run_in(&dir, &["changelog", "latest", "--json"]);

    assert_eq!(output.status.code(), Some(1), "stderr: {}", stderr_of(&output));
    let stderr = stderr_of(&output);
    let parsed: serde_json::Value = serde_json::from_str(stderr.trim())
        .unwrap_or_else(|err| panic!("stderr must be one JSON object, got {stderr:?}: {err}"));
    assert_eq!(parsed["error"]["code"], "changelog", "{stderr}");
}

/// Drift is an outcome, not a failure: `doctor --json` keeps reporting it on stdout with exit 2 and
/// leaves stderr empty, so a consumer never confuses the two channels.
#[test]
fn drift_stays_data_on_stdout_and_never_becomes_an_error_envelope() {
    let dir = drifting_manifests();

    let output = run_in(&dir, &["doctor", "--json"]);

    assert_eq!(output.status.code(), Some(2), "stderr: {}", stderr_of(&output));
    assert_eq!(stderr_of(&output), "", "drift is not an error: no stderr");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|err| panic!("stdout must be one JSON object, got {stdout:?}: {err}"));
    assert_eq!(parsed["ok"], false, "{stdout}");
    assert_eq!(parsed["schemaVersion"], 1, "{stdout}");
}
