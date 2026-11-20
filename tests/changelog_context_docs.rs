//! The release context is documented, key by key (#346 block 1's open remainder, #272's neighbour).
//!
//! `tests/schema_compat.rs` pins the key set the binary emits, and `tests/cli_reference_parity.rs`
//! keeps flags and subcommands in step with the reference — but neither of them watches a JSON key,
//! so a surface could be pinned and unreadable at the same time. It was: 17 of the 37 keys of
//! `changelog latest --json` were emitted, read by templates, pinned by the baseline, and named in
//! no page a consumer would find.
//!
//! This test is the missing half. It reads the page that owns the surface and compares both
//! directions, because the two failures are different: a key nobody documents is a promise the
//! consumer cannot act on, and a documented key nobody emits is a promise the tool does not keep.

mod common;

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use common::CHANGELOG_CONTEXT_KEYS;

/// The page that owns the release context.
const PAGE: &str = "docs/internals/machine-contract-surfaces.md";

/// The heading that opens the section, and the prefix that closes it.
const HEADING: &str = "## `changelog latest --json`";

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn page() -> String {
    fs::read_to_string(repo_root().join(Path::new(PAGE))).expect("the surfaces page is readable")
}

/// The body of the release-context section: from its heading to the next top-level one.
fn section(text: &str) -> &str {
    let start = text
        .find(HEADING)
        .unwrap_or_else(|| panic!("the release context must have its own section in {PAGE}: no `{HEADING}`"));
    let rest = &text[start + HEADING.len()..];
    match rest.find("\n## ") {
        Some(end) => &rest[..end],
        None => rest,
    }
}

/// Every key the section's table names in its first column, in the order the table lists them.
fn documented_keys(section: &str) -> Vec<String> {
    section
        .lines()
        .filter(|line| line.starts_with("| ") && !line.starts_with("| Key |") && !line.starts_with("| :---"))
        .flat_map(|row| row.split('|').nth(1).unwrap_or_default().split('`').skip(1).step_by(2))
        .map(str::to_string)
        .collect()
}

#[test]
fn every_key_the_release_context_emits_is_documented() {
    let text = page();
    let documented: BTreeSet<String> = documented_keys(section(&text)).into_iter().collect();

    let mut expected: BTreeSet<String> = CHANGELOG_CONTEXT_KEYS.iter().map(|key| key.to_string()).collect();
    expected.insert("schemaVersion".to_string());

    let undocumented: Vec<&String> = expected.difference(&documented).collect();
    assert!(
        undocumented.is_empty(),
        "{PAGE} documents the release context but not {undocumented:?}: a key a consumer parses has to be \
         readable by the person who has to act on it"
    );

    let invented: Vec<&String> = documented.difference(&expected).collect();
    assert!(
        invented.is_empty(),
        "{PAGE} documents {invented:?}, which the release context does not emit: a documented key nobody \
         writes is a promise the tool does not keep"
    );
}

/// The table is the whole promise: a row that names no key would pass the comparison above by being
/// empty and silently document nothing.
#[test]
fn the_section_documents_the_whole_key_set_and_not_a_sample() {
    let text = page();
    let documented = documented_keys(section(&text));

    assert_eq!(
        documented.len(),
        CHANGELOG_CONTEXT_KEYS.len() + 1,
        "one row per key, `schemaVersion` included; this page had 37 keys to carry: {documented:?}"
    );

    let unique: BTreeSet<&String> = documented.iter().collect();
    assert_eq!(
        unique.len(),
        documented.len(),
        "a key listed twice is a table nobody maintains: {documented:?}"
    );
}
