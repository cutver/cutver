//! `examples/cutver.toml` is loaded here, not trusted: every promise it makes to a reader is asserted.
//!
//! An example nobody loads rots into a lie. Nothing else in the suite would notice the shipped file
//! drifting from the loader — the docs link to it, no code reads it, and it is not compiled — so a key
//! the schema rejects, a manifest kind that stopped existing, or a regex that cannot parse its own
//! target would ship as prose in TOML clothing and be believed.
//!
//! The `regex` manifest is the one that rots silently. `RegexEditor::read_version` parses capture
//! group 1 and falls back to the whole match only when the pattern has no group at all
//! (`src/manifest/regex.rs:20-27`), so a pattern without a group hands `VERSION="1.4.2"` to the SemVer
//! parser and fails with `invalid version string` — the old documentation example did exactly that.
//! These tests read the version through the same editor the release pipeline builds, so the documented
//! contract is executed rather than described.

use std::path::PathBuf;

use cutver::config;
use cutver::manifest::editor_for;
use semver::Version;

/// The version the scratch project holds, and the one the example's regex has to find.
const VERSION: &str = "1.4.2";

fn repo_file(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

/// A project shaped like the one `examples/cutver.toml` describes, with the example itself copied in
/// and loaded through `cutver::config::load` — the entry point the CLI uses, so the schema pass, the
/// preflight parse and the path resolution all run against the file the docs point readers at.
fn scratch_project() -> (tempfile::TempDir, config::Config) {
    let scratch = tempfile::tempdir().expect("a scratch project must be creatable in the system temp directory");
    let root = scratch.path();

    let cargo = format!("[package]\nname = \"acme-cli\"\nversion = \"{VERSION}\"\n");
    let npm = format!("{{\n  \"name\": \"acme-cli\",\n  \"version\": \"{VERSION}\"\n}}\n");
    let install = format!("#!/bin/sh\nVERSION=\"{VERSION}\"\n");

    std::fs::write(root.join("Cargo.toml"), cargo).expect("the scratch Cargo.toml must be writable");
    std::fs::write(root.join("package.json"), npm).expect("the scratch package.json must be writable");
    std::fs::create_dir_all(root.join("scripts")).expect("the scratch scripts directory must be creatable");
    std::fs::write(root.join("scripts/install.sh"), install).expect("the scratch install script must be writable");
    std::fs::write(root.join("CHANGELOG.md"), "# Changelog\n").expect("the scratch changelog must be writable");

    let example = repo_file("examples/cutver.toml");
    let text = std::fs::read_to_string(&example).unwrap_or_else(|err| {
        panic!(
            "examples/cutver.toml must be readable, {} failed: {err}",
            example.display()
        )
    });
    std::fs::write(root.join("cutver.toml"), text).expect("the copied example must be writable");

    let config = config::load(root.join("cutver.toml")).unwrap_or_else(|err| {
        panic!(
            "examples/cutver.toml must load: an example the loader rejects teaches a configuration that \
             fails at the first `cutver` command — {err}"
        )
    });
    (scratch, config)
}

fn kind_of(manifest: &config::Manifest) -> &'static str {
    match &manifest.kind {
        config::ManifestKind::CargoPackage => "cargo-package",
        config::ManifestKind::Json { .. } => "json",
        config::ManifestKind::Pyproject { .. } => "pyproject",
        config::ManifestKind::Gradle { .. } => "gradle",
        config::ManifestKind::Regex { .. } => "regex",
        config::ManifestKind::Plugin { .. } => "plugin",
        config::ManifestKind::Template { .. } => "template",
    }
}

#[test]
fn the_example_shows_one_version_across_three_manifest_kinds() {
    let (_scratch, config) = scratch_project();
    let kinds: Vec<&str> = config.manifest.iter().map(kind_of).collect();
    assert_eq!(
        kinds,
        ["cargo-package", "json", "regex"],
        "the example exists to show one version rewritten in three file formats; if it no longer declares a \
         Cargo manifest, an npm manifest and a regex-matched script, the documented claim is gone"
    );
}

/// The documented regex contract, executed: group 1 is the version, and the whole match is what gets
/// replaced. A pattern without a group reaches the SemVer parser as `VERSION="1.4.2"` and fails.
#[test]
fn the_regex_manifest_reads_the_version_through_capture_group_one() {
    let (_scratch, config) = scratch_project();
    let entry = config
        .manifest
        .iter()
        .find(|manifest| matches!(manifest.kind, config::ManifestKind::Regex { .. }))
        .expect("the example must declare a kind = \"regex\" manifest, which is the only kind that can rot unnoticed");
    let config::ManifestKind::Regex { pattern, .. } = &entry.kind else {
        unreachable!("the entry was selected as a regex manifest")
    };

    let content = std::fs::read_to_string(entry.path.as_str())
        .unwrap_or_else(|err| panic!("the example's regex target {} must be readable: {err}", entry.path));
    let editor = editor_for(entry).expect("dispatching the example's regex manifest must build an editor");
    let read = editor.read_version(&content).unwrap_or_else(|err| {
        panic!(
            "the example's regex must read {VERSION} out of {} with pattern {pattern}: {err}\n  \
             `read_version` parses capture group 1 and only falls back to the whole match when the pattern \
             has no group, so a pattern without a capture group hands the surrounding text to the SemVer \
             parser. That is the failure the old example taught; a reader copying it gets it back.",
            entry.path
        )
    });
    assert_eq!(
        read,
        Version::parse(VERSION).expect("the test's own version literal must be valid SemVer"),
        "the example's regex must read the version the scratch project holds"
    );
}

/// The example's own headline claim: `cutver bump` rewrites all three files, so all three must read
/// the same current version. One manifest pointing at a path that does not exist fails here.
#[test]
fn every_manifest_in_the_example_reads_the_same_version() {
    let (_scratch, config) = scratch_project();
    let expected = Version::parse(VERSION).expect("the test's own version literal must be valid SemVer");
    let mut read = Vec::new();
    for entry in &config.manifest {
        let content = std::fs::read_to_string(entry.path.as_str())
            .unwrap_or_else(|err| panic!("manifest {} must name a file in the scratch project: {err}", entry.path));
        let editor = editor_for(entry).expect("every manifest in the example must dispatch to an editor");
        let version = editor.read_version(&content).unwrap_or_else(|err| {
            panic!(
                "manifest {} ({}) must read its version: {err}",
                entry.path,
                kind_of(entry)
            )
        });
        read.push((kind_of(entry), version));
    }
    assert_eq!(
        read,
        vec![
            ("cargo-package", expected.clone()),
            ("json", expected.clone()),
            ("regex", expected),
        ],
        "the example promises one version across three files; a manifest that reads a different version, or \
         fails to read one, makes the copied example a broken release"
    );
}

/// An example is a working configuration, not prose in TOML clothing: the machinery it advertises has
/// to survive the loader in the shape the comments describe.
#[test]
fn the_example_configures_the_rest_of_its_schema() {
    let (_scratch, config) = scratch_project();

    let steps: Vec<&str> = config.preflight.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(
        steps,
        ["format", "tests", "clippy"],
        "the example advertises three preflight steps (a string, an inline table with its own timeout, and a \
         third string); losing one drops a documented form from the only file readers copy"
    );

    assert_eq!(
        config.artifact.len(),
        2,
        "the example declares two [[artifact]] globs; one surviving means the second declaration form was lost"
    );
    assert_eq!(
        config.gates.require.len(),
        2,
        "the example requires two attestation gates; publish would refuse the release the example describes"
    );
    assert_eq!(
        config.publish.commands.len(),
        1,
        "the example declares one publish command; an empty list would make the documented publish step a no-op"
    );
}
