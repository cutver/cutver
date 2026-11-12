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

    run_init(Some(td.path.clone()), false, false, false).unwrap();

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
    assert!(template.contains("## [{{ tag }}] - {{ date }}"));
    assert!(
        template.contains("[Compare the full changelog]({{ compare_url }})"),
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

    run_init(Some(td.path.clone()), false, false, true).unwrap();

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

    let err = run_init(Some(td.path.clone()), false, false, false).unwrap_err();
    assert!(matches!(err, InitError::AlreadyExists { .. }));
}

#[test]
fn fresh_init_overwrites_with_force() {
    let td = TestDir::new("scaffold-force");
    td.write("cutver.toml", "# old\n");
    td.write("package.json", r#"{"name": "test", "version": "1.0.0"}"#);

    run_init(Some(td.path.clone()), false, true, false).unwrap();

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

    run_init(Some(td.path.clone()), true, false, false).unwrap();

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

    run_init(Some(td.path.clone()), true, false, false).unwrap();
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
    run_init(Some(with_template.path.clone()), false, false, false).unwrap();
    assert!(
        !with_template.read("cutver.toml").contains("new_contributors"),
        "the template renders contributors itself, so the key would be inert"
    );

    let without_template = TestDir::new("scaffold-no-tpl");
    without_template.write("Cargo.toml", "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n");
    run_init(Some(without_template.path.clone()), false, false, true).unwrap();
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
