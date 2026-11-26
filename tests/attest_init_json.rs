//! `cutver attest --json` and `cutver init --json`: the release tail's two bookkeeping commands on a
//! machine surface (#404 S3, S4, S9).
//!
//! `attest` printed one line and `init` printed a summary, and a program that had just run them could
//! read neither: the line names the file it wrote in prose, and the summary's primary badge is a
//! decoration. This suite pins the objects that carry the same facts — the record `attest` wrote with
//! the path `publish --require-gate` reads, and the manifests, source of truth and files `init`
//! produced — with `schemaVersion` first and the failure class in `error.code`, plus the human bytes
//! unchanged when the flag is absent.
//!
//! `init`'s human summary names the absolute target directory, so the byte-identity goldens below
//! build that one line from the fixture's own path instead of hard-coding a host.

mod common;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Output;

use common::{CARGO_TOML, Fixture, FixtureGuard, GRADLE_KTS, PACKAGE_JSON, cutver_command, load_config};
use serde_json::Value;

/// The first bytes of a compact surface: the version is the first key, so a consumer reads it before
/// the payload it may not understand (`tests/schema_compat.rs` states the same rule for every surface).
const COMPACT_PREFIX: &str = r#"{"schemaVersion":1,"#;

/// The commit every attestation below records. `attest` validates the shape of a commit and reads no
/// repository, so a fixed sha keeps the assertions free of git.
const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

/// The configuration `attest` needs: one manifest and the manifest it takes the version from.
const ATTEST_TOML: &str = r#"[version]
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"
"#;

/// The keys `cutver attest --json` must carry: what was verified, for which commit, with which
/// conclusion, when, the run when the gate names one, and the file it landed in.
const ATTEST_KEYS: [&str; 6] = ["gate", "commit", "conclusion", "verified_at", "run", "path"];

/// The keys `cutver init --json` must carry: the manifests detected, the primary source of truth and
/// the files written.
const INIT_KEYS: [&str; 3] = ["manifests", "primary", "files_written"];

/// The keys each entry of `init`'s `manifests` list carries.
const MANIFEST_KEYS: [&str; 3] = ["path", "is_root", "kind"];

/// `cutver` in `dir` with the style variables removed, so a human golden is compared as plain bytes
/// whatever a runner exports for its own shell.
fn cutver(dir: &Path, args: &[&str]) -> Output {
    cutver_command(dir)
        .env_remove("NO_COLOR")
        .env_remove("CLICOLOR")
        .env_remove("CLICOLOR_FORCE")
        .args(args)
        .output()
        .expect("failed to execute the cutver binary")
}

fn attest(fixture: &Fixture, args: &[&str]) -> Output {
    let mut full = vec!["attest"];
    full.extend(args.iter().copied());
    cutver(&fixture.dir, &full)
}

fn init(fixture: &Fixture, args: &[&str]) -> Output {
    let mut full = vec!["init"];
    full.extend(args.iter().copied());
    cutver(&fixture.dir, &full)
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

/// The object `text` holds, or a failure naming the surface that was supposed to be one.
fn object(text: &str, surface: &str) -> Value {
    serde_json::from_str(text.trim())
        .unwrap_or_else(|err| panic!("{surface} must be one JSON object, got {text:?}: {err}"))
}

/// Asserts every key of `keys` appears in `document` after the one before it, in one pass: field
/// order is part of the contract, so a reordered key fails where an added one does not.
fn assert_keys_in_order(document: &str, keys: &[&str], surface: &str) {
    let mut cursor = 0;
    for key in keys {
        let needle = format!("\"{key}\":");
        match document[cursor..].find(&needle) {
            Some(index) => cursor += index,
            None => panic!("{surface} must carry `{key}` after every key before it, got: {document:?}"),
        }
    }
}

/// The properties every surface here shares: one compact line, the documented version first, and no
/// pinned key missing. The assertion is `emitted ⊇ pinned`, never equality, because a key added later
/// is not a new schema version while a key that disappears leaves a consumer reading `null`.
fn assert_surface(document: &str, surface: &str, keys: &[&str]) -> Value {
    assert_eq!(
        document.lines().count(),
        1,
        "{surface} prints one compact object on stdout, got: {document:?}"
    );
    assert!(
        document.starts_with(COMPACT_PREFIX),
        "{surface} must open with `schemaVersion` at the bytes `{COMPACT_PREFIX}`: {document:?}"
    );

    let parsed = object(document, surface);
    assert_eq!(parsed["schemaVersion"], 1, "{surface}: {document}");

    let emitted: BTreeSet<String> = parsed
        .as_object()
        .unwrap_or_else(|| panic!("{surface} must be a JSON object, got {document:?}"))
        .keys()
        .cloned()
        .collect();
    let missing: Vec<&str> = keys.iter().copied().filter(|key| !emitted.contains(*key)).collect();
    assert!(
        missing.is_empty(),
        "{surface} no longer emits {missing:?}; emitted keys: {emitted:?}"
    );

    assert_keys_in_order(document, keys, surface);
    parsed
}

/// S3: the object names the record `attest` just wrote, including the file, and the flag changes the
/// channel and never the bytes of the file.
/// The path a comparison can hold.
///
/// One file has more than one honest spelling: macOS reaches `/var/...` through the `/private/var`
/// symlink, and a Windows runner reaches its temp directory through the 8.3 short name (`RUNNER~1`
/// for `runneradmin`). A raw `Path` comparison then fails on a path nobody disagrees about, so both
/// sides are canonicalized and the assertion is about the file, never about its spelling.
fn resolved(path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();
    std::fs::canonicalize(path).unwrap_or_else(|err| panic!("the path must exist, {} failed: {err}", path.display()))
}

#[test]
fn attest_with_the_flag_prints_the_record_it_wrote_and_the_path_publish_reads() {
    let guard = FixtureGuard::new("attest-json");
    let fixture = guard.fixture();
    fixture.write("cutver.toml", ATTEST_TOML);
    fixture.write("package.json", PACKAGE_JSON);

    let out = attest(fixture, &["ci", "--commit", SHA, "--conclusion", "success", "--json"]);
    assert_eq!(out.status.code(), Some(0), "stderr:\n{}", stderr_of(&out));

    let document = stdout_of(&out);
    let parsed = assert_surface(&document, "`cutver attest --json`", &ATTEST_KEYS);

    assert_eq!(parsed["gate"], "ci", "{document}");
    assert_eq!(parsed["commit"], SHA, "{document}");
    assert_eq!(parsed["conclusion"], "success", "{document}");
    let verified_at = parsed["verified_at"].as_str().expect("`verified_at` is a string");
    assert!(
        common::is_iso_date(verified_at),
        "`verified_at` is the day the record was written, found {verified_at:?}"
    );
    assert!(
        parsed.get("run").is_some_and(Value::is_null),
        "a gate that names no run still carries the key, as `null`, so an adapter parses one shape: {document}"
    );

    let path = parsed["path"].as_str().expect("`path` is a string");
    assert_eq!(
        path, ".cutver/attestations/ci.json",
        "the path is the file the record landed in, spelled relative to the workspace root with \
         forward slashes — the form a wrapper resolves and the form the human line prints: {document}"
    );
    assert!(
        fixture.dir.join(path).is_file(),
        "the reported path resolves to a file that exists: {document}"
    );

    // The wrapper's real question is "which file will `publish --require-gate` read", so it is
    // answered through the reader itself instead of by re-deriving the attestation directory.
    let config = load_config(fixture);
    let gate = cutver::config::gates::GateName::parse("ci").expect("`ci` is a gate name");
    let recorded = cutver::config::gates::read(&config, &gate).expect("the record publish reads");
    assert_eq!(recorded.commit, SHA, "{document}");
    assert_eq!(
        resolved(cutver::config::gates::path_of(&config, &gate)),
        resolved(fixture.dir.join(path)),
        "the path the object reports is the path publish reads"
    );

    let from_machine = fixture.read(path);
    let human = attest(fixture, &["ci", "--commit", SHA, "--conclusion", "success"]);
    assert_eq!(human.status.code(), Some(0), "stderr:\n{}", stderr_of(&human));
    assert_eq!(
        fixture.read(path),
        from_machine,
        "the flag changes the channel, never the bytes of the record it writes"
    );
}

/// S3, S9: a refusal writes one envelope on **stderr** with the `attest` class in it, adds nothing to
/// stdout, and records nothing.
#[test]
fn a_refused_attestation_prints_one_envelope_with_its_class_and_writes_nothing() {
    let guard = FixtureGuard::new("attest-json-refusal");
    let fixture = guard.fixture();
    fixture.write("cutver.toml", ATTEST_TOML);
    fixture.write("package.json", PACKAGE_JSON);

    let out = attest(
        fixture,
        &["../escape", "--commit", SHA, "--conclusion", "success", "--json"],
    );

    assert_eq!(out.status.code(), Some(1), "stderr:\n{}", stderr_of(&out));
    assert!(
        out.stdout.is_empty(),
        "a failure prints nothing on stdout, or a consumer reads half a document: {:?}",
        stdout_of(&out)
    );

    let stderr = stderr_of(&out);
    assert_eq!(
        stderr.lines().count(),
        1,
        "the failure is one compact object and no prose: {stderr:?}"
    );
    assert!(
        stderr.starts_with(COMPACT_PREFIX),
        "the envelope opens with `schemaVersion`: {stderr:?}"
    );
    let envelope = object(&stderr, "the failure envelope");
    assert_eq!(envelope["error"]["code"], "attest", "{stderr}");
    assert_eq!(
        envelope["error"]["exitCode"], 1,
        "the class is in the envelope and the exit code is the process code: {stderr}"
    );
    assert!(
        !stderr.contains("Error: "),
        "the machine path replaces the rendered line instead of accompanying it: {stderr:?}"
    );
    assert!(
        !fixture.dir.join(".cutver").exists(),
        "a refused attestation writes no file at all"
    );
}

/// S4: the object names every manifest detected, exactly one primary source of truth, and every file
/// the run wrote — which must be on disk.
#[test]
fn init_with_the_flag_names_every_manifest_the_primary_and_the_files_it_wrote() {
    let guard = FixtureGuard::new("init-json-polyglot");
    let fixture = guard.fixture();
    fixture.write("package.json", PACKAGE_JSON);
    fixture.write("Cargo.toml", CARGO_TOML);
    fixture.write("android/build.gradle.kts", GRADLE_KTS);

    let out = init(fixture, &["--json"]);
    assert_eq!(out.status.code(), Some(0), "stderr:\n{}", stderr_of(&out));

    let document = stdout_of(&out);
    let parsed = assert_surface(&document, "`cutver init --json`", &INIT_KEYS);

    let manifests = parsed["manifests"]
        .as_array()
        .unwrap_or_else(|| panic!("`manifests` is the list discovery found: {document}"));
    let paths: Vec<&str> = manifests
        .iter()
        .map(|manifest| manifest["path"].as_str().expect("a manifest path is a string"))
        .collect();
    assert_eq!(
        paths,
        vec!["package.json", "Cargo.toml", "android/build.gradle.kts"],
        "every manifest is named, in the order the configuration declares them: {document}"
    );
    for manifest in manifests {
        for key in MANIFEST_KEYS {
            assert!(
                manifest.get(key).is_some(),
                "every manifest carries `{key}`, so an adapter parses one shape: {document}"
            );
        }
    }
    assert_eq!(manifests[0]["is_root"], true, "{document}");
    assert_eq!(manifests[0]["kind"], "json", "{document}");
    assert_eq!(manifests[1]["kind"], "cargo-package", "{document}");
    assert_eq!(manifests[2]["kind"], "gradle", "{document}");
    assert_eq!(
        manifests[2]["is_root"], false,
        "a nested manifest is not the root one: {document}"
    );

    let primary = parsed["primary"]
        .as_str()
        .unwrap_or_else(|| panic!("`primary` names the source of truth: {document}"));
    assert_eq!(
        manifests.iter().filter(|manifest| manifest["path"] == primary).count(),
        1,
        "exactly one detected manifest is the primary source of truth: {document}"
    );
    assert_eq!(primary, "package.json", "{document}");
    assert_eq!(
        resolved(
            load_config(fixture)
                .primary_manifest()
                .expect("the generated configuration declares one")
                .path
                .as_path()
        ),
        resolved(fixture.dir.join(primary)),
        "the object names the manifest the configuration makes the source of truth"
    );

    let files = parsed["files_written"]
        .as_array()
        .unwrap_or_else(|| panic!("`files_written` is the list of files the run wrote: {document}"));
    assert!(!files.is_empty(), "a fresh run writes the scaffold: {document}");
    assert!(
        files.iter().any(|file| file == "cutver.toml"),
        "the configuration is one of the files it wrote: {document}"
    );
    for file in files {
        let file = file.as_str().expect("a written file is a path");
        assert!(
            fixture.dir.join(file).is_file(),
            "`{file}` is reported as written, so it must exist on disk: {document}"
        );
    }
}

/// S4, S9: a directory `init` must refuse exits 1 with the `init` class in the envelope, leaves
/// stdout empty, and writes nothing.
#[test]
fn a_refused_init_prints_one_envelope_with_its_class_and_writes_nothing() {
    let guard = FixtureGuard::new("init-json-refusal");
    let fixture = guard.fixture();
    fixture.write("cutver.toml", "# custom existing\n");

    let out = init(fixture, &["--json"]);

    assert_eq!(out.status.code(), Some(1), "stderr:\n{}", stderr_of(&out));
    assert!(
        out.stdout.is_empty(),
        "a failure prints nothing on stdout, or a consumer reads half a document: {:?}",
        stdout_of(&out)
    );

    let stderr = stderr_of(&out);
    assert_eq!(
        stderr.lines().count(),
        1,
        "the failure is one compact object and no prose: {stderr:?}"
    );
    assert!(
        stderr.starts_with(COMPACT_PREFIX),
        "the envelope opens with `schemaVersion`: {stderr:?}"
    );
    let envelope = object(&stderr, "the failure envelope");
    assert_eq!(envelope["error"]["code"], "init", "{stderr}");
    assert_eq!(
        envelope["error"]["exitCode"], 1,
        "the class is in the envelope and the exit code is the process code: {stderr}"
    );

    assert_eq!(
        fixture.read("cutver.toml"),
        "# custom existing\n",
        "the configuration it refused to overwrite is untouched"
    );
    assert!(
        !fixture.dir.join("CHANGELOG.md").exists(),
        "a refusal writes no changelog"
    );
    assert!(!fixture.dir.join(".github").exists(), "a refusal writes no scaffold");
}

/// S4: an `--update` writes no scaffold, so the same keys have to describe what that run did instead
/// of claiming files it never wrote.
#[test]
fn init_update_with_the_flag_reports_only_the_files_it_actually_wrote() {
    let guard = FixtureGuard::new("init-json-update");
    let fixture = guard.fixture();
    fixture.write("Cargo.toml", CARGO_TOML);
    fixture.write(
        "crates/nested/Cargo.toml",
        "[package]\nname = \"nested\"\nversion = \"1.2.3\"\n",
    );
    fixture.write(
        "cutver.toml",
        r#"[version]
strategy = "conventional"

[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"
"#,
    );

    let out = init(fixture, &["--update", "--json"]);
    assert_eq!(out.status.code(), Some(0), "stderr:\n{}", stderr_of(&out));

    let document = stdout_of(&out);
    let parsed = assert_surface(&document, "`cutver init --update --json`", &INIT_KEYS);

    let paths: Vec<&str> = parsed["manifests"]
        .as_array()
        .expect("`manifests` is the list discovery found")
        .iter()
        .map(|manifest| manifest["path"].as_str().expect("a manifest path is a string"))
        .collect();
    assert_eq!(paths, vec!["Cargo.toml", "crates/nested/Cargo.toml"], "{document}");
    assert_eq!(
        parsed["primary"], "Cargo.toml",
        "the primary is the manifest the existing configuration already declares: {document}"
    );
    assert_eq!(
        parsed["files_written"],
        serde_json::json!(["cutver.toml"]),
        "the only file an update writes is the configuration it appended to: {document}"
    );
    assert!(
        !fixture.dir.join("CHANGELOG.md").exists() && !fixture.dir.join(".github").exists(),
        "an update writes no scaffold, and the object must not claim one: {document}"
    );

    let second = init(fixture, &["-u", "--json"]);
    assert_eq!(second.status.code(), Some(0), "stderr:\n{}", stderr_of(&second));
    let document = stdout_of(&second);
    let parsed = assert_surface(&document, "`cutver init -u --json`", &INIT_KEYS);
    assert_eq!(
        parsed["files_written"],
        serde_json::json!([]),
        "an update that found nothing new wrote nothing, and says so: {document}"
    );
}

/// S3, S6: without the flag `attest` prints the one line it always printed, byte for byte.
#[test]
fn without_the_flag_attest_prints_the_one_line_it_always_printed() {
    let guard = FixtureGuard::new("attest-json-human");
    let fixture = guard.fixture();
    fixture.write("cutver.toml", ATTEST_TOML);
    fixture.write("package.json", PACKAGE_JSON);

    let out = attest(fixture, &["ci", "--commit", SHA, "--conclusion", "success"]);

    assert_eq!(out.status.code(), Some(0), "stderr:\n{}", stderr_of(&out));
    assert_eq!(
        stdout_of(&out),
        format!("attested 'ci' as success for commit {SHA} (.cutver/attestations/ci.json)\n"),
        "the human path is byte-for-byte the line it was"
    );
}

/// S4, S6: the same for `init`'s fresh summary, which names the directory it initialized.
#[test]
fn without_the_flag_init_prints_the_summary_it_always_printed() {
    let guard = FixtureGuard::new("init-json-human");
    let fixture = guard.fixture();
    fixture.write("package.json", PACKAGE_JSON);

    let out = init(fixture, &[]);

    assert_eq!(out.status.code(), Some(0), "stderr:\n{}", stderr_of(&out));
    let expected = [
        format!("✔ Initialized cutver configuration in {}", fixture.dir.display()),
        String::new(),
        "Discovered manifests:".to_string(),
        "  ✔ package.json (json: version) [primary source of truth]".to_string(),
        String::new(),
        "Created files:".to_string(),
        "  ✔ cutver.toml".to_string(),
        "  ✔ CHANGELOG.md".to_string(),
        "  ✔ .github/templates/cutver/RELEASE.md".to_string(),
        String::new(),
        "Next steps:".to_string(),
        "  1. Validate your configuration:".to_string(),
        "     cutver doctor".to_string(),
        "  2. Read the release before it happens:".to_string(),
        "     cutver plan".to_string(),
        "  3. Cut it in two phases, so the artifacts can be verified in between:".to_string(),
        "     cutver bump --defer-publish".to_string(),
    ]
    .join("\n")
        + "\n";
    assert_eq!(
        stdout_of(&out),
        expected,
        "the human path is byte-for-byte the summary it was"
    );
}

/// S4, S6: and for the update summary, on both of its branches.
#[test]
fn without_the_flag_init_update_prints_the_summary_it_always_printed() {
    let guard = FixtureGuard::new("init-json-human-update");
    let fixture = guard.fixture();
    fixture.write("Cargo.toml", CARGO_TOML);
    fixture.write(
        "crates/nested/Cargo.toml",
        "[package]\nname = \"nested\"\nversion = \"1.2.3\"\n",
    );
    fixture.write(
        "cutver.toml",
        r#"[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"
"#,
    );

    let out = init(fixture, &["--update"]);
    assert_eq!(out.status.code(), Some(0), "stderr:\n{}", stderr_of(&out));
    let expected = [
        "✔ Updated cutver.toml with 1 new manifest(s):".to_string(),
        String::new(),
        "  ✔ crates/nested/Cargo.toml (cargo-package)".to_string(),
        String::new(),
        "Next steps:".to_string(),
        "  1. Validate your configuration:".to_string(),
        "     cutver doctor".to_string(),
        "  2. Read the release before it happens:".to_string(),
        "     cutver plan".to_string(),
        "  3. Cut it in two phases, so the artifacts can be verified in between:".to_string(),
        "     cutver bump --defer-publish".to_string(),
    ]
    .join("\n")
        + "\n";
    assert_eq!(stdout_of(&out), expected);

    let second = init(fixture, &["--update"]);
    assert_eq!(second.status.code(), Some(0), "stderr:\n{}", stderr_of(&second));
    assert_eq!(
        stdout_of(&second),
        "ℹ cutver.toml is already up to date (no new manifests discovered).\n"
    );
}
