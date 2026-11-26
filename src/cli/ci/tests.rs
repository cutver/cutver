//! Unit tests for the CI channel's pure parts: detection, the escaping rules and the block shape.
//!
//! None of this touches the process environment. Every input arrives through the injected lookup, so
//! the tests are deterministic under a parallel test binary and never depend on whether the machine
//! running them is itself a CI runner. The end-to-end behaviour — the real files, the real stderr —
//! is pinned by `tests/ci_output.rs`.

use std::collections::BTreeMap;

use crate::bump::{PublishSummary, Summary};

use super::fields::Field;
use super::text::{MAX_VALUE_CHARS, code_span, one_line};
use super::{CiContext, CiMode, escape_command_data, summary};

/// A lookup over a fixed table, which is what the boundary resolver binds to `std::env::var`.
fn lookup(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
    let table: BTreeMap<String, String> = pairs
        .iter()
        .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
        .collect();
    move |key| table.get(key).cloned()
}

const FULL: [(&str, &str); 3] = [
    ("GITHUB_ACTIONS", "true"),
    ("GITHUB_OUTPUT", "/tmp/github_output"),
    ("GITHUB_STEP_SUMMARY", "/tmp/github_step_summary"),
];

fn detected(pairs: &[(&str, &str)], mode: CiMode) -> bool {
    CiContext::from_env_lookup(lookup(pairs), mode).detected()
}

/// S1: `GITHUB_ACTIONS=true` and one named file, both.
#[test]
fn detection_needs_the_flag_and_a_named_file() {
    assert!(detected(&FULL, CiMode::Detect), "both halves detect CI");
    assert!(detected(&FULL[..2], CiMode::Detect), "one named file is enough");
    assert!(
        !detected(&[("GITHUB_ACTIONS", "true")], CiMode::Detect),
        "no named file means nothing to write to, so nothing is detected"
    );
    assert!(
        !detected(&FULL[1..], CiMode::Detect),
        "the two files without the flag are not a detected CI"
    );
    assert!(
        !detected(&[("GITHUB_ACTIONS", "false"), FULL[1], FULL[2]], CiMode::Detect),
        "only `true` detects, never `false`"
    );
    assert!(
        !detected(&[("GITHUB_ACTIONS", "1"), FULL[1], FULL[2]], CiMode::Detect),
        "no other spelling detects, so a heuristic can never turn the channel on by accident"
    );
    assert!(
        detected(&[("GITHUB_ACTIONS", " TRUE "), FULL[1], FULL[2]], CiMode::Detect),
        "the value is read case-insensitively and trimmed"
    );
}

/// S1: `--ci` turns the channel on without `GITHUB_ACTIONS`; `--no-ci` turns it off whatever the
/// environment says.
#[test]
fn the_flags_override_detection() {
    assert!(
        detected(&FULL[1..], CiMode::Force),
        "`--ci` needs no `GITHUB_ACTIONS`, only a file to write"
    );
    assert!(
        detected(&[("GITHUB_ACTIONS", "true")], CiMode::Force),
        "`--ci` is the caller's own declaration: the annotation follows it without a named sink"
    );
    assert!(
        detected(&[], CiMode::Force),
        "`--ci` on a bare environment still annotates a failure"
    );
    assert!(!detected(&FULL, CiMode::Off), "`--no-ci` wins over detection");
    assert!(
        !detected(&[], CiMode::Off),
        "`--no-ci` closes the channel even with nothing to detect"
    );
    assert_eq!(CiMode::from_flags(false, false), CiMode::Detect);
    assert_eq!(CiMode::from_flags(true, false), CiMode::Force);
    assert_eq!(CiMode::from_flags(false, true), CiMode::Off);
}

/// S6: a variable that holds nothing names no path, so resolution opens nothing.
#[test]
fn a_blank_variable_names_no_file() {
    assert!(!detected(
        &[("GITHUB_ACTIONS", "true"), ("GITHUB_OUTPUT", "  ")],
        CiMode::Detect
    ));
    assert!(!detected(
        &[("GITHUB_ACTIONS", "true"), ("GITHUB_OUTPUT", "")],
        CiMode::Detect
    ));
}

/// S2: the output file is one line per field, a missing value is omitted, and no value can add a
/// line of its own.
#[test]
fn a_field_is_one_line_and_an_absent_value_is_omitted() {
    let fields: Vec<Field> = vec![
        ("released", Some("true".to_string())),
        ("commit", Some(String::new())),
        ("deferred", None),
        ("tag", Some("v1.3.0".to_string())),
        ("version", Some("1.3.0\nreleased=false".to_string())),
    ];

    assert_eq!(
        super::fields::render(&fields),
        "released=true\ntag=v1.3.0\nversion=1.3.0 released=false\n"
    );
}

/// S3: the block is a heading and a bullet per field, every value inside a code span.
#[test]
fn the_summary_block_is_a_heading_and_one_bullet_per_field() {
    let block = summary::render(
        "cutver bump",
        &[
            ("released", Some("true".to_string())),
            ("commit", None),
            ("tag", Some("v1.3.0".to_string())),
        ],
    );

    assert_eq!(block, "### cutver bump\n\n- released: `true`\n- tag: `v1.3.0`\n");
}

/// S3: a value carrying a backtick cannot close its own span, and one carrying a newline cannot open
/// a line of its own.
#[test]
fn a_value_can_neither_close_its_span_nor_open_a_line() {
    assert_eq!(code_span("v`1.3.0"), "``v`1.3.0``");
    assert_eq!(code_span("`run`"), "`` `run` ``");
    assert_eq!(one_line("v\n## injected"), "v ## injected");
    assert_eq!(one_line("a\t\tb\r\nc"), "a b c");
    assert_eq!(one_line("escape\u{1b}[31m"), "escape[31m");
    assert!(
        !summary::render("t", &[("tag", Some("v\n## injected".to_string()))]).contains("\n## "),
        "a heading cannot start a line"
    );
}

/// S3: a value is truncated, so the block cannot grow with the material behind it.
#[test]
fn a_long_value_is_truncated_and_marked() {
    let long = "x".repeat(MAX_VALUE_CHARS * 4);
    let span = code_span(&long);

    assert_eq!(span, format!("`{}…`", "x".repeat(MAX_VALUE_CHARS)));
    assert!(span.chars().count() <= MAX_VALUE_CHARS + 3);
}

/// S4: the annotation is one line, so the characters its syntax reserves are percent-escaped.
#[test]
fn the_annotation_escaping_is_one_line() {
    assert_eq!(
        escape_command_data("first line\n  Where: x\n  Fix: 100% of it\r\n"),
        "first line%0A  Where: x%0A  Fix: 100%25 of it%0D%0A"
    );
}

/// S2: `publish` reuses `bump`'s `tag` and `commit` and adds only what the closing phase alone
/// knows. Its three action fields read `dry_run`, because the summary of a dry run still carries the
/// plan it printed — a filled `publish_push_command` on a run that pushed nothing must not read as a
/// pushed tag.
#[test]
fn the_publish_fields_report_what_the_closing_phase_did() {
    let real = PublishSummary {
        tag: "v1.3.0".to_string(),
        commit: "abc1234".to_string(),
        // The two machine-only lists stay empty: this test is about the fields the sink writes, and
        // an empty list is the state a run with no refspec recorded and no plugin dispatched has.
        push_refs: Vec::new(),
        floating_tag: Some("v1".to_string()),
        publish_push_command: Some("git push origin refs/tags/v1.3.0".to_string()),
        publish_commands: vec!["cargo publish".to_string()],
        publish_command_results: Vec::new(),
        post_release_left_alone: Vec::new(),
        post_release_dispatched: Vec::new(),
        tag_skipped: false,
        dry_run: false,
    };

    assert_eq!(
        super::fields::render(&super::fields::publish(&real)),
        "tag=v1.3.0\ncommit=abc1234\ntag_pushed=true\nfloating_tag_moved=true\npublish_commands_run=1\n"
    );

    let dry = PublishSummary { dry_run: true, ..real };
    assert_eq!(
        super::fields::render(&super::fields::publish(&dry)),
        "tag=v1.3.0\ncommit=abc1234\ntag_pushed=false\nfloating_tag_moved=false\npublish_commands_run=0\n",
        "the planned pushes and the listed commands were not performed"
    );
}

/// A finished `bump` that released `next`, everything else filler: the field list reads the version,
/// the tag it names and the two release flags, never the tree behind them.
fn released(next: semver::Version) -> Summary {
    let tag = format!("v{next}");
    Summary {
        source: "Cargo.toml".into(),
        current: semver::Version::new(1, 2, 0),
        next,
        dry_run: false,
        preflight: Vec::new(),
        touched: Vec::new(),
        changelog: None,
        commit_message: "chore(release)".into(),
        commit: "abc1234".into(),
        tag,
        tag_skipped: false,
        floating_tag: None,
        pre_bump: None,
        post_bump: None,
        warnings: Vec::new(),
        publish_push: false,
        publish_push_command: None,
        publish_commands: Vec::new(),
        deferred: false,
        rationale: None,
        root_dir: None,
    }
}

/// S6: the run reports the channel the version it produced carries, so no downstream job re-parses a
/// semver suffix to learn it — `stable` for a release, the identifier for a prerelease. The version
/// decides, not the flag: `1.2.0-rc.1` is `rc` whether or not `--channel rc` was the request.
#[test]
fn the_bump_fields_name_the_channel_of_the_version_they_report() {
    for (next, channel) in [("1.3.0", "stable"), ("1.3.0-alpha.1", "alpha"), ("1.3.0-rc.1", "rc")] {
        let summary = released(next.parse().expect("a valid version"));
        let expected =
            format!("released=true\ndeferred=false\nversion={next}\nchannel={channel}\ntag=v{next}\ncommit=abc1234\n");

        assert_eq!(
            super::fields::render(&super::fields::bump(&summary)),
            expected,
            "`{next}` is on the `{channel}` channel and the run must say so: a consumer that has to \
             parse the suffix for it is the parser this field exists to remove"
        );
    }
}

/// S2, S3: a sink the runner named but cannot be opened writes nothing and never fails the run — it
/// degrades to the `warning:` line the human boundary owns, because bookkeeping cannot undo a
/// release that already finished.
#[test]
fn an_unwritable_sink_creates_nothing_and_does_not_fail_the_run() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let missing = dir.path().join("missing").join("github_output");
    let context = CiContext::from_env_lookup(
        lookup(&[
            ("GITHUB_ACTIONS", "true"),
            ("GITHUB_OUTPUT", missing.to_str().expect("a utf-8 path")),
        ]),
        CiMode::Detect,
    );

    context.append_outputs(&[("tag", Some("v1.3.0".to_string()))]);

    assert!(context.detected(), "the channel is on, only the sink is unusable");
    assert!(!missing.exists(), "the sink that cannot be opened stays absent");
}
