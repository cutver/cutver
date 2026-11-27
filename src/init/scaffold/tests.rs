use std::fs;
use std::path::PathBuf;

use super::*;

static TMP_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn tmp_id(prefix: &str) -> String {
    let n = TMP_SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    format!("{}-{}-{}", prefix, std::process::id(), n)
}

struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(tmp_id(name));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn write(&self, rel: &str, content: &str) {
        let p = self.path.join(rel);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(p, content).unwrap();
    }

    fn read(&self, rel: &str) -> String {
        fs::read_to_string(self.path.join(rel)).unwrap()
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn fresh_init_creates_files_and_config() {
    let td = TestDir::new("scaffold-fresh");
    td.write("Cargo.toml", "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n");

    run_init(Some(td.path.clone()), false, false, false, false).unwrap();

    assert!(td.path.join("cutver.toml").is_file());
    assert!(td.path.join("CHANGELOG.md").is_file());
    assert!(td.path.join(".github/templates/cutver/RELEASE.md").is_file());

    let cfg_text = td.read("cutver.toml");
    assert!(cfg_text.contains("commit_message = \"chore(release): v{version} [skip ci]\""));
    assert!(cfg_text.contains("require_branch = \"main\""));
    assert!(cfg_text.contains("push = true"));
    assert!(cfg_text.contains("kind = \"cargo-package\""));
    assert!(cfg_text.contains("check = \"cargo check --workspace\""));
    assert!(cfg_text.contains("post_bump = \"cargo check --workspace\""));
    assert!(cfg_text.contains("mode = \"template\""));
    assert!(cfg_text.contains("template_file = \".github/templates/cutver/RELEASE.md\""));

    let changelog = td.read("CHANGELOG.md");
    assert!(changelog.contains("## [Unreleased]"));

    let template = td.read(".github/templates/cutver/RELEASE.md");
    assert!(template.contains("## {{ tag }} — {{ date }}"));
    assert!(
        template.contains("]({{ compare_url }})") && !template.contains("**: {{ compare_url }}"),
        "the compare link must carry anchor text, not a bare URL"
    );
    assert!(
        !template.contains("GITHUB_RUN_NUMBER"),
        "release notes must not leak CI build identifiers"
    );
}

#[test]
fn fresh_init_no_template_flag() {
    let td = TestDir::new("scaffold-no-template");
    td.write("Cargo.toml", "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n");

    run_init(Some(td.path.clone()), false, false, true, false).unwrap();

    assert!(td.path.join("cutver.toml").is_file());
    assert!(td.path.join("CHANGELOG.md").is_file());
    assert!(!td.path.join(".github/templates/cutver/RELEASE.md").exists());

    let cfg_text = td.read("cutver.toml");
    assert!(cfg_text.contains("mode = \"conventional\""));
    assert!(!cfg_text.contains("template_file"));
}

#[test]
fn fresh_init_fails_if_cutver_toml_exists() {
    let td = TestDir::new("scaffold-exists");
    td.write("cutver.toml", "# existing\n");

    let err = run_init(Some(td.path.clone()), false, false, false, false).unwrap_err();
    assert!(matches!(err, InitError::AlreadyExists { .. }));
}

#[test]
fn fresh_init_overwrites_with_force() {
    let td = TestDir::new("scaffold-force");
    td.write("cutver.toml", "# old\n");
    td.write("package.json", r#"{"name": "test", "version": "1.0.0"}"#);

    run_init(Some(td.path.clone()), false, true, false, false).unwrap();

    let cfg_text = td.read("cutver.toml");
    assert!(cfg_text.contains("kind = \"json\""));
    assert!(cfg_text.contains("field = \"version\""));
    assert!(!cfg_text.contains("# old"));
}

#[test]
fn update_appends_new_manifest_and_preserves_customizations() {
    let td = TestDir::new("scaffold-update");
    let initial_cfg = r#"# Custom user header
[version]
strategy = "conventional"

[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"

[preflight]
custom_lint = "cargo clippy"

[git]
tag_prefix = "rel-"
"#;
    td.write("cutver.toml", initial_cfg);
    td.write("Cargo.toml", "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n");
    td.write(
        "packages/client/package.json",
        r#"{"name": "client", "version": "0.1.0"}"#,
    );

    run_init(Some(td.path.clone()), true, false, false, false).unwrap();

    let updated_cfg = td.read("cutver.toml");
    assert!(updated_cfg.contains("# Custom user header"));
    assert!(updated_cfg.contains("custom_lint = \"cargo clippy\""));
    assert!(updated_cfg.contains("tag_prefix = \"rel-\""));
    assert!(updated_cfg.contains("path = \"packages/client/package.json\""));
}

#[test]
fn update_noop_when_all_manifests_already_present() {
    let td = TestDir::new("scaffold-noop");
    let initial_cfg = r#"[version]
strategy = "conventional"

[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"
"#;
    td.write("cutver.toml", initial_cfg);
    td.write("Cargo.toml", "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n");

    run_init(Some(td.path.clone()), true, false, false, false).unwrap();
    assert_eq!(td.read("cutver.toml"), initial_cfg);
}

/// The template `cutver init` writes and the one this repository releases with must stay the same
/// file. They were byte-identical before this test existed, and nothing stopped them from drifting:
/// an improvement to one would silently skip the other.
#[test]
fn shipped_release_template_matches_the_repository_one() {
    let shipped = crate::init::scaffold::DEFAULT_RELEASE_TEMPLATE;
    let in_repo = include_str!("../../../.github/templates/cutver/RELEASE.md");
    // Git checks the template out with CRLF on Windows, while the constant in the source is LF.
    // Line endings are a checkout setting, not content drift, so normalise before comparing.
    let normalise = |text: &str| text.replace("\r\n", "\n").trim().to_string();
    assert_eq!(
        normalise(shipped),
        normalise(in_repo),
        "DEFAULT_RELEASE_TEMPLATE and .github/templates/cutver/RELEASE.md have drifted apart"
    );
}

/// A scaffolded project without the template uses the built-in renderer, which omits first-time
/// contributors unless asked. With the template, the option would be inert, so it must not appear.
#[test]
fn new_contributors_is_enabled_only_for_the_builtin_renderer() {
    let with_template = TestDir::new("scaffold-tpl");
    with_template.write("Cargo.toml", "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n");
    run_init(Some(with_template.path.clone()), false, false, false, false).unwrap();
    assert!(
        !with_template.read("cutver.toml").contains("new_contributors"),
        "the template renders contributors itself, so the key would be inert"
    );

    let without_template = TestDir::new("scaffold-no-tpl");
    without_template.write("Cargo.toml", "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n");
    run_init(Some(without_template.path.clone()), false, false, true, false).unwrap();
    let config = without_template.read("cutver.toml");
    assert!(
        config.contains("new_contributors = true"),
        "the built-in renderer needs the key to surface first-time contributors: {config}"
    );
    assert!(
        config.contains("kind = \"template\""),
        "the template sink shape must be discoverable from the scaffold: {config}"
    );
}

/// D3 (#410): the sink awareness belongs to the primary-source fallback alone. `declared_manifests`
/// decides which discovered manifests `--update` must not append, so a sink that discovery finds —
/// here `package.json`, generated from a template — is still an already-declared path and must not be
/// appended a second time as a json manifest.
#[test]
fn update_does_not_append_a_template_sink_the_document_already_declares() {
    let td = TestDir::new("scaffold-sink-noop");
    let initial_cfg = r#"[[manifest]]
path = "package.json"
kind = "template"
template = "package.jinja"

[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"
"#;
    td.write("cutver.toml", initial_cfg);
    td.write("Cargo.toml", "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n");
    td.write("package.json", r#"{"name": "demo", "version": "0.1.0"}"#);
    td.write("package.jinja", "{}\n");

    run_init(Some(td.path.clone()), true, false, false, false).unwrap();
    assert_eq!(
        td.read("cutver.toml"),
        initial_cfg,
        "the sink is declared already, so appending it again would declare the path twice"
    );
}

/// Both answers reduced to one spelling. `load` resolves `current_source` against the root
/// (`src/config/discovery.rs:27`), so the loader names an absolute path while the document names the
/// spelling it declares; comparing those two strings would pass or fail for the wrong reason.
fn resolved_manifest(root: &std::path::Path, path: &str) -> PathBuf {
    fs::canonicalize(root.join(path)).unwrap_or_else(|e| panic!("the fixture declares `{path}`, which must exist: {e}"))
}

/// The document's answer for `fixture`, after asserting it is the manifest the loader uses.
fn both_sides_agree(fixture: &str, cutver_toml: &str) -> String {
    let td = TestDir::new(fixture);
    td.write("Cargo.toml", "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n");
    td.write("package.json", r#"{"name": "demo", "version": "0.1.0"}"#);
    td.write("src/version.ts", "export const version = '0.1.0';\n");
    td.write("cutver.toml", cutver_toml);

    let config = crate::config::load(td.path.join("cutver.toml"))
        .expect("only a configuration that loads is part of the agreeing set");
    let document: toml_edit::DocumentMut = cutver_toml.parse().expect("the fixture is valid TOML");
    let from_document = config_gen::primary_source(&document).expect("the fixture declares a manifest");

    assert_eq!(
        resolved_manifest(&td.path, &from_document),
        resolved_manifest(&td.path, &config.version.current_source),
        "`init --update` reports {from_document:?} for the {fixture} fixture, but the loader uses {:?}",
        config.version.current_source
    );
    from_document
}

/// S1/S2 (#410): the `primary` `init --update` reports and the source the loader actually uses are two
/// copies of one ordering rule — `[version] current_source` (or its deprecated alias `source`), then
/// the manifest flagged `primary = true`, then the first manifest that is not a template sink — and
/// they must name the same manifest for every configuration that loads.
///
/// This test catches that rule moving on one side only, in either direction: if `primary_source`
/// (`config_gen.rs`) or `deduce_current_source` (`src/config/validation.rs`) changes its order, which
/// of the two `[[manifest]]` spellings it reads, or whether it skips `kind = "template"` sinks,
/// exactly the fixture covering that step disagrees and the assertion names both answers.
///
/// A configuration the loader refuses is outside the agreeing set by construction, so it stays the
/// document's business: more than one `primary = true`, a `[version] current_source` that contradicts
/// the flagged manifest, a list where every entry is a sink, and an empty list. `load` errors on each
/// of them, so there is no loader answer to agree with.
#[test]
fn init_update_and_the_loader_name_the_same_primary_source() {
    // `[[manifest]]` array of tables: an explicit `current_source` beats the first declared manifest.
    let declared = both_sides_agree(
        "scaffold-agree-aot",
        r#"[version]
current_source = "package.json"

[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"
"#,
    );
    assert_eq!(declared, "package.json");

    // The same list in the other accepted spelling, an array of inline tables, with no `[version]`
    // table at all: both sides fall back to the first declared manifest.
    let declared = both_sides_agree(
        "scaffold-agree-inline",
        r#"manifest = [
  { path = "Cargo.toml", kind = "cargo-package" },
  { path = "package.json", kind = "json", field = "version" },
]
"#,
    );
    assert_eq!(declared, "Cargo.toml");

    // The deprecated `[version] source` alias selects the same manifest `current_source` would.
    let declared = both_sides_agree(
        "scaffold-agree-alias",
        r#"[version]
source = "package.json"

[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"
"#,
    );
    assert_eq!(declared, "package.json");

    // `primary = true` on a manifest that is not declared first wins over declaration order.
    let declared = both_sides_agree(
        "scaffold-agree-primary",
        r#"[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"
primary = true
"#,
    );
    assert_eq!(declared, "Cargo.toml");

    // The divergence #410 reports: a template sink is declared first and nothing names a source. The
    // loader skips the sink, so the document must skip it too instead of reporting a primary the
    // loader would never use.
    let declared = both_sides_agree(
        "scaffold-agree-sink-first",
        r#"[[manifest]]
path = "src/version.ts"
kind = "template"

[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"
"#,
    );
    assert_eq!(declared, "Cargo.toml");
}

/// The freshness rule applied to a command's own output: the first-run page sends the reader to
/// `plan` and then to a two-phase release, so `cutver init` may not still recommend the single-command
/// dry run. Without this, the page and the CLI drift apart silently and the more authoritative of the
/// two — the binary the reader just ran — teaches the older flow.
#[test]
fn the_next_steps_recommend_the_flow_the_first_run_page_teaches() {
    let printed = runner::NEXT_STEPS.join("\n");
    assert!(
        printed.contains("cutver doctor"),
        "the first step validates the configuration: {printed}"
    );
    assert!(
        printed.contains("cutver plan"),
        "reading the plan comes before anything mutates, which is what the first-run page teaches: {printed}"
    );
    assert!(
        printed.contains("cutver bump --defer-publish"),
        "the last step is the two-phase release, so the artifacts can be verified between the commit \
         and the tag: {printed}"
    );
    assert!(
        !printed.contains("--dry-run"),
        "the single-command dry run is no longer the recommended first step: {printed}"
    );
}
