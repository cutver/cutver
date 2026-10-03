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
