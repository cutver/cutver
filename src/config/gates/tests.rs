//! Unit tests for the parts `tests/` cannot reach: the table's own validation and the store's
//! round trip. The behaviour a caller sees is covered end to end in `tests/e2e_gates*.rs`.

use std::fs;
use std::path::PathBuf;

use super::*;
use crate::config::types::{Config, ConfigError};

/// A workspace with `[gates] dir` set, rooted in its own temporary directory.
fn workspace(name: &str, dir: &str) -> (tempfile::TempDir, Config) {
    let tmp = tempfile::Builder::new()
        .prefix(&format!("cutver-gates-{name}-"))
        .tempdir()
        .expect("a scratch workspace");
    let config = Config {
        root_dir: PathBuf::from(tmp.path()),
        gates: Gates {
            dir: dir.to_string(),
            ..Gates::default()
        },
        ..Config::default()
    };
    (tmp, config)
}

/// S8: a verdict written by `record` is the document `read` returns, and the file is one JSON object
/// whose keys are in the documented order.
#[test]
fn a_recorded_verdict_round_trips_through_the_store() {
    let (_tmp, config) = workspace("round-trip", DEFAULT_DIR);
    let gate = GateName::parse("ci").expect("a gate name");
    let commit = "4f2ac1b1c6c7d2d3e4f5a6b7c8d9e0f1a2b3c4d5";

    let recorded = record(&config, &gate, commit, Conclusion::Failure, Some("https://ci/1")).expect("it records");

    assert_eq!(recorded.path, ".cutver/attestations/ci.json");
    let raw = fs::read_to_string(path_of(&config, &gate)).expect("the file exists");
    assert!(
        raw.starts_with(&format!(
            r#"{{"schemaVersion":1,"gate":"ci","commit":"{commit}","conclusion":"failure","verified_at":""#
        )),
        "key order is the file's contract, and the version opens it: {raw}"
    );
    let read_back = read(&config, &gate).expect("it reads back");
    assert_eq!(read_back, recorded.attestation);
    assert!(
        read_back.covers(&commit.to_ascii_uppercase()),
        "hex ids compare without case"
    );
    assert!(!read_back.covers("0000000000000000000000000000000000000000"));
    assert_eq!(read_back.conclusion, Conclusion::Failure);
    assert_eq!(read_back.run.as_deref(), Some("https://ci/1"));
    assert!(
        crate::changelog::format_date(std::time::SystemTime::now()).len() == read_back.verified_at.len(),
        "`when` is written in the changelog's own date format"
    );
}

/// #409 D1: a file written before the version key existed carries no `schemaVersion`, and it still
/// reads as the attestation it holds. The reader is total on purpose, so this change must not turn a
/// recorded verdict into "no attestation" — that would make `publish --require-gate` fail closed on a
/// gate that did vouch for the commit.
#[test]
fn a_file_written_before_the_version_key_still_reads() {
    let (_tmp, config) = workspace("legacy", DEFAULT_DIR);
    let gate = GateName::parse("ci").expect("a gate name");
    let commit = "4f2ac1b1c6c7d2d3e4f5a6b7c8d9e0f1a2b3c4d5";
    let path = path_of(&config, &gate);
    fs::create_dir_all(path.parent().expect("the gates directory is inside the workspace"))
        .expect("the gates directory is created");
    fs::write(
        &path,
        format!(r#"{{"gate":"ci","commit":"{commit}","conclusion":"failure","verified_at":"2026-11-26"}}"#),
    )
    .expect("the legacy file is written");

    let read_back = read(&config, &gate).expect("the legacy shape is still an attestation");

    assert_eq!(read_back.commit, commit);
    assert!(read_back.covers(commit), "it answers for the commit it names");
    assert_eq!(read_back.conclusion, Conclusion::Failure);
    assert_eq!(read_back.run, None, "a legacy file names no run");
}

/// S8: the commit is parsed before anything is written, so a tag or a short form cannot be recorded.
#[test]
fn a_commit_that_is_not_a_sha_is_refused_before_it_is_written() {
    let (_tmp, config) = workspace("invalid-commit", DEFAULT_DIR);
    let gate = GateName::parse("ci").expect("a gate name");

    for commit in ["HEAD", "4f2ac1b", "v1.3.0"] {
        let error = record(&config, &gate, commit, Conclusion::Success, None).expect_err("it is refused");
        assert!(error.to_string().contains(commit), "{error}");
    }
    assert!(!dir(&config).exists(), "nothing may be written");
}

/// S12: the channel is data. A damaged or misnamed file is not an attestation, and never an error.
#[test]
fn a_file_that_does_not_read_as_an_attestation_is_absent() {
    let (_tmp, config) = workspace("damaged", DEFAULT_DIR);
    let gate = GateName::parse("ci").expect("a gate name");
    let other = GateName::parse("security").expect("a gate name");
    let path = path_of(&config, &gate);
    fs::create_dir_all(path.parent().expect("the directory is named")).expect("it is created");

    assert!(read(&config, &gate).is_none(), "no file is no attestation");
    for damaged in ["", "{", r#"{"gate":"security","commit":"00"}"#, r#"{"gate":"../ci"}"#] {
        fs::write(&path, damaged).expect("the fixture file is writable");
        assert!(
            read(&config, &gate).is_none(),
            "{damaged:?} is not an attestation for 'ci'"
        );
    }
    assert!(
        read(&config, &other).is_none(),
        "a gate with no file of its own reads nothing"
    );
}

/// S10: the declared directory may not name the workspace root or walk upwards.
#[test]
fn the_gates_dir_must_stay_inside_the_workspace() {
    for dir in ["", "  ", ".", "..", "./attestations", "../attestations"] {
        let (_tmp, config) = workspace("dir-validation", dir);
        match config.gates.validate() {
            Err(ConfigError::GatesDirInvalid { dir: reported, .. }) => assert_eq!(reported, dir),
            other => panic!("`[gates] dir = {dir:?}` must be refused, got {other:?}"),
        }
    }
    // An absolute path is spelled per host, and only the spelling the host can represent as absolute
    // is the one under test: `C:/absolute` is a plain relative directory name on unix, so refusing it
    // there would be the bug, not the guarantee. `/absolute` has a root but no prefix on Windows.
    for dir in ["/absolute", "C:/absolute"] {
        if !std::path::Path::new(dir).is_absolute() {
            continue;
        }
        let (_tmp, config) = workspace("dir-validation-absolute", dir);
        match config.gates.validate() {
            Err(ConfigError::GatesDirInvalid { dir: reported, .. }) => assert_eq!(reported, dir),
            other => panic!("`[gates] dir = {dir:?}` must be refused, got {other:?}"),
        }
    }
    for dir in [DEFAULT_DIR, "ci/verdicts", "attestations"] {
        let (_tmp, config) = workspace("dir-accepted", dir);
        assert!(
            config.gates.validate().is_ok(),
            "`{dir}` is a workspace-relative directory"
        );
        assert_eq!(declared_input(&config), Some(config.root_dir.join(dir)));
    }
}

/// A gate name is a file name: everything that could leave the directory is unrepresentable.
#[test]
fn a_gate_name_cannot_leave_the_attestations_directory() {
    for name in ["", "../escape", "a/b", "..", ".hidden", "with space", "a\\b"] {
        let error = GateName::parse(name).expect_err("it is refused");
        assert_eq!(error.name, name);
    }
    let gate = GateName::parse("e2e-linux").expect("a gate name");
    assert_eq!(gate.as_str(), "e2e-linux");
    assert_eq!(gate.file_name(), "e2e-linux.json");
    assert_eq!(
        GateName::parse("ci.v2").expect("an inner dot is allowed").file_name(),
        "ci.v2.json"
    );
}
