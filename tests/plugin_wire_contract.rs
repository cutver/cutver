//! Golden wire-format contract for the plugin boundary.
//!
//! The plugin ABI is a JSON contract implemented in two independent places: this
//! crate serializes it, and the `cutver-pdk` crate in the `cutver/plugins`
//! repository deserializes it. Nothing structurally couples the two, so this
//! test is the guard on the side that owns the contract.
//!
//! The fixture below is committed **byte-identical** to
//! `plugins/github-releases/tests/fixtures/changelog_request.json` in the plugins
//! repository, where the PDK decodes it and the plugin renders from it. Any
//! change to a field name, field order, optionality, or wire string fails here
//! first, forcing whoever changed the contract to reconcile both sides in the
//! same breath.
//!
//! ## Object key order on the wire
//!
//! Only the four top-level envelope fields keep their declaration order
//! (`protocol`, `capability`, `operation`, `payload`), because `PluginInvocation` is a struct.
//! The nested `payload` is a `serde_json::Value`, and `serde_json` backs a
//! `Value` map with a sorted map, so **payload keys are emitted alphabetically**,
//! not in `ChangelogRenderRequest` declaration order.
//!
//! Key order is therefore not part of the contract; field names and optionality
//! are. Do not tighten this test to demand declaration order inside `payload`.
//! Code that string-matches the envelope, such as the process-plugin test
//! scripts, may rely on the top-level order only.

use cutver::plugin::dto::{
    ChangelogRenderRequest, ChangelogRenderResponse, PROTOCOL_VERSION, PluginCommitEntry, PluginContributor,
    PluginInvocation, decode_answer,
};
use cutver::plugin::types::{PluginCall, PluginName};

/// Canonical envelope, exactly as the plugins repository commits it.
const CANONICAL_ENVELOPE: &str = include_str!("fixtures/plugin_wire_contract/changelog_request.json");

/// The request that must serialize to [`CANONICAL_ENVELOPE`].
fn canonical_request() -> ChangelogRenderRequest {
    ChangelogRenderRequest {
        root_dir: "/workspace".to_string(),
        version: "1.2.0".to_string(),
        tag_name: "v1.2.0".to_string(),
        previous_tag: Some("v1.1.0".to_string()),
        release_date: "2026-03-14".to_string(),
        commits: vec![
            PluginCommitEntry {
                sha: "a1b2c3d4e5f6a7b8c9d0".to_string(),
                message: "feat(cli): add tree view".to_string(),
                r#type: Some("feat".to_string()),
                scope: Some("cli".to_string()),
                author_name: Some("Alice".to_string()),
                pr_number: Some("#42".to_string()),
                is_breaking: false,
            },
            PluginCommitEntry {
                sha: "b2c3d4e5f6a7b8c9d0e1".to_string(),
                message: "fix: correct tag prefix".to_string(),
                r#type: Some("fix".to_string()),
                scope: None,
                author_name: Some("Bob".to_string()),
                pr_number: Some("#43".to_string()),
                is_breaking: false,
            },
            PluginCommitEntry {
                sha: "c3d4e5f6a7b8c9d0e1f2".to_string(),
                message: "feat!: drop legacy config".to_string(),
                r#type: Some("feat".to_string()),
                scope: None,
                author_name: Some("Carol".to_string()),
                pr_number: None,
                is_breaking: true,
            },
        ],
        repository: Some("https://github.com/cutver/cutver".to_string()),
        compare_url: Some("https://github.com/cutver/cutver/compare/v1.1.0...v1.2.0".to_string()),
        is_prerelease: false,
        contributors: vec![
            PluginContributor {
                name: "Alice".to_string(),
                is_first_contribution: false,
            },
            PluginContributor {
                name: "Bob".to_string(),
                is_first_contribution: true,
            },
            PluginContributor {
                name: "Carol".to_string(),
                is_first_contribution: true,
            },
        ],
    }
}

#[test]
fn envelope_serializes_to_the_canonical_wire_format() {
    let invocation =
        PluginInvocation::new(PluginCall::ChangelogRender, &canonical_request()).expect("request serializes");

    // The wire form has no line endings at all, so normalize before comparing.
    // `.gitattributes` marks the fixture `-text` to stop git rewriting it, but a
    // tree already checked out before that landed can still hand us CRLF, and
    // failing on a developer's git configuration rather than on the contract
    // would make this test a nuisance instead of a guard.
    let canonical = CANONICAL_ENVELOPE.replace("\r\n", "\n");

    assert_eq!(
        serde_json::to_string(&invocation).expect("envelope serializes"),
        canonical.trim_end_matches('\n'),
        "the core serialized an envelope that differs from the committed canonical fixture. \
         Update the fixture in BOTH this repository and cutver/plugins, or the plugin will drift."
    );
}

#[test]
fn canonical_envelope_decodes_back_to_the_same_request() {
    let invocation: PluginInvocation = serde_json::from_str(CANONICAL_ENVELOPE).expect("canonical envelope decodes");
    let request: ChangelogRenderRequest =
        serde_json::from_value(invocation.payload).expect("canonical payload decodes");

    assert_eq!(request, canonical_request());
}

#[test]
fn an_answer_that_declares_another_protocol_is_refused_not_misdecoded() {
    // A plugin built against a future shape says so in its answer. Reporting that as
    // `InvalidPayload` sends the author hunting a schema bug that does not exist, which is the
    // whole point of the version travelling up (#369 S3).
    let answer = br#"{"protocol":"2","body":"Notes"}"#;
    let name = PluginName::new("future-shape").unwrap();

    let error = decode_answer::<ChangelogRenderResponse>(&name, "changelog.v1", answer)
        .expect_err("a plugin that speaks protocol 2 must not be read as if it spoke 1");

    let rendered = error.to_string();
    assert!(
        rendered.contains("protocol '2'"),
        "the error must name the version the plugin declared; it reads: {rendered}"
    );
    assert!(
        rendered.contains(PROTOCOL_VERSION),
        "the error must name the version the host speaks; it reads: {rendered}"
    );
    assert!(
        rendered.contains("Where:") && rendered.contains("Fix:"),
        "the house diagnosis convention is Where:/Fix:, and a version mismatch needs both: {rendered}"
    );
}

#[test]
fn a_protocol_this_host_cannot_read_is_refused_too() {
    // `"protocol": 2` is the numeric style this program itself uses for `schemaVersion`, and a
    // Python author is as likely to write it as the string. Reading it as "absent" would bless it.
    for answer in [
        &br#"{"protocol":2,"body":"Notes"}"#[..],
        &br#"{"protocol":null,"body":"Notes"}"#[..],
        &br#"{"protocol":true,"body":"Notes"}"#[..],
        &br#"{"protocol":["1"],"body":"Notes"}"#[..],
    ] {
        let declared = String::from_utf8_lossy(answer);
        let outcome = decode_answer::<ChangelogRenderResponse>(
            &PluginName::new("unreadable-shape").unwrap(),
            "changelog.v1",
            answer,
        );

        assert!(
            outcome.is_err(),
            "a declaration the host cannot read must be refused, never ignored: {declared}"
        );
    }
}

#[test]
fn an_answer_that_declares_nothing_is_read_as_the_host_protocol() {
    // Every plugin that exists today answers with the DTO alone. Absent has to keep meaning "the
    // shape it was built against", or the version field would break the whole installed base.
    let answer = br#"{"body":"Notes"}"#;
    let name = PluginName::new("built-against-1").unwrap();

    let decoded: ChangelogRenderResponse =
        decode_answer(&name, "changelog.v1", answer).expect("an answer without a protocol key is protocol 1");

    assert_eq!(decoded.body, "Notes");
}

#[test]
fn absent_optional_fields_are_omitted_from_the_wire() {
    let minimal = ChangelogRenderRequest {
        previous_tag: None,
        repository: None,
        compare_url: None,
        commits: Vec::new(),
        contributors: Vec::new(),
        ..canonical_request()
    };

    // `commits`, `is_prerelease` and `contributors` are always present; the three
    // Option fields disappear entirely rather than serializing as null.
    assert_eq!(
        serde_json::to_string(&minimal).expect("minimal request serializes"),
        r#"{"root_dir":"/workspace","version":"1.2.0","tag_name":"v1.2.0","release_date":"2026-03-14","commits":[],"is_prerelease":false,"contributors":[]}"#
    );
}

#[test]
fn absent_optional_fields_still_decode() {
    // A plugin written against the older, smaller contract must keep decoding.
    let payload = r#"{"root_dir":"/workspace","version":"1.0.0","tag_name":"v1.0.0","release_date":"2026-01-01"}"#;
    let request: ChangelogRenderRequest = serde_json::from_str(payload).expect("legacy payload decodes");

    assert_eq!(request.previous_tag, None);
    assert_eq!(request.repository, None);
    assert_eq!(request.compare_url, None);
    assert!(request.commits.is_empty());
    assert!(request.contributors.is_empty());
    assert!(!request.is_prerelease);
}

#[test]
fn commit_entry_optional_fields_are_omitted_from_the_wire() {
    let entry = PluginCommitEntry {
        sha: "a1b2c3d".to_string(),
        message: "fix: tighten parser".to_string(),
        r#type: None,
        scope: None,
        author_name: None,
        pr_number: None,
        is_breaking: false,
    };

    assert_eq!(
        serde_json::to_string(&entry).expect("entry serializes"),
        r#"{"sha":"a1b2c3d","message":"fix: tighten parser","is_breaking":false}"#
    );
}

#[test]
fn response_shape_is_stable() {
    let response = ChangelogRenderResponse {
        body: "## Features\n".to_string(),
    };

    // A plain escaped string, not a raw one: the JSON contains `"##`, and Rust 2024
    // reserves sequences of two or more `#` after a literal.
    assert_eq!(
        serde_json::to_string(&response).expect("response serializes"),
        "{\"body\":\"## Features\\n\"}"
    );
}
