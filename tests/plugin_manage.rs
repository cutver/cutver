#![cfg(feature = "plugins")]
//! Hermetic end-to-end tests for `cutver plugin list` and `cutver plugin remove`.
//!
//! Every test drives the real binary against a `tempfile::tempdir()` project with
//! `CUTVER_PLUGIN_CACHE_DIR` pointing inside that directory, so no test touches
//! the ambient cache, the network or a real plugin (Pillar IV.1).

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

use tempfile::TempDir;

/// `sha256("hello")`; hardcoded so the test crate needs no hashing dependency.
const PAYLOAD: &[u8] = b"hello";
const PAYLOAD_HASH: &str = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";

const BASE_CONFIG: &str = r#"[version]
current_source = "Cargo.toml"

[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"
"#;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_cutver")
}

struct Project {
    dir: TempDir,
    cache: PathBuf,
}

impl Project {
    fn new(config: &str, artifacts: &[(&str, &[u8])]) -> Self {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("cutver.toml"), config).unwrap();
        let cache = dir.path().join("cache");
        fs::create_dir_all(&cache).unwrap();
        for (name, bytes) in artifacts {
            fs::write(cache.join(name), bytes).unwrap();
        }
        Self { dir, cache }
    }

    fn run<I, S>(&self, args: I) -> Output
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        Command::new(bin())
            .current_dir(self.dir.path())
            .env("CUTVER_PLUGIN_CACHE_DIR", &self.cache)
            .env("NO_COLOR", "1")
            .args(args)
            .output()
            .unwrap()
    }

    fn config(&self) -> String {
        fs::read_to_string(self.dir.path().join("cutver.toml")).unwrap()
    }

    fn write_config(&self, text: &str) {
        fs::write(self.dir.path().join("cutver.toml"), text).unwrap();
    }

    fn write(&self, relative: &str, bytes: &[u8]) -> PathBuf {
        let path = self.dir.path().join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, bytes).unwrap();
        path
    }

    fn artifact(&self, name: &str) -> PathBuf {
        self.cache.join(name)
    }
}

fn with_demo_plugin(trailer: &str) -> String {
    format!(
        "{BASE_CONFIG}\n[plugins.demo]\nruntime = \"wasm\"\nsource = \"demo-0.1.0.wasm\"\nhash = \"{PAYLOAD_HASH}\"\n{trailer}"
    )
}

fn config_with_source(source: &str) -> String {
    format!("{BASE_CONFIG}\n[plugins.demo]\nruntime = \"wasm\"\nsource = \"{source}\"\nhash = \"{PAYLOAD_HASH}\"\n")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

#[test]
fn list_reports_installed_and_matched_then_unknown_after_the_artifact_is_removed() {
    let project = Project::new(&with_demo_plugin(""), &[("demo-0.1.0.wasm", PAYLOAD)]);

    let before = project.run(["plugin", "list"]);
    assert_eq!(before.status.code(), Some(0), "stderr: {}", stderr(&before));
    let text = stdout(&before);
    assert!(text.contains("demo"), "output: {text}");
    assert!(text.contains("wasm"), "output: {text}");
    assert!(text.contains("installed:     yes"), "output: {text}");
    assert!(text.contains("digest:        matched"), "output: {text}");

    fs::remove_file(project.artifact("demo-0.1.0.wasm")).unwrap();

    let after = project.run(["plugin", "list"]);
    assert_eq!(after.status.code(), Some(0), "stderr: {}", stderr(&after));
    let text = stdout(&after);
    assert!(text.contains("installed:     no"), "output: {text}");
    assert!(text.contains("digest:        unknown"), "output: {text}");
}

#[test]
fn list_json_reports_effective_state() {
    let project = Project::new(&with_demo_plugin(""), &[("demo-0.1.0.wasm", PAYLOAD)]);

    let output = project.run(["plugin", "list", "--json"]);
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));

    let parsed: serde_json::Value = serde_json::from_str(&stdout(&output)).expect("stdout must be JSON");
    let plugin = &parsed["plugins"][0];
    assert_eq!(plugin["name"], "demo");
    assert_eq!(plugin["runtime"], "wasm");
    assert_eq!(plugin["source"], "demo-0.1.0.wasm");
    assert_eq!(plugin["declared_hash"], PAYLOAD_HASH);
    assert_eq!(plugin["installed"], true);
    assert_eq!(plugin["digest"], "matched");
}

#[test]
fn list_reports_no_plugins_without_an_error() {
    let project = Project::new(BASE_CONFIG, &[]);

    let output = project.run(["plugin", "list"]);
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    assert!(stdout(&output).contains("No plugins are configured."));
}

#[test]
fn remove_of_an_unknown_name_fails_closed_naming_the_plugin_and_config() {
    let project = Project::new(BASE_CONFIG, &[]);

    let output = project.run(["plugin", "remove", "ghost"]);
    assert_eq!(output.status.code(), Some(1));
    let text = stderr(&output);
    assert!(text.contains("ghost"), "stderr: {text}");
    assert!(text.contains("cutver.toml"), "stderr: {text}");
}

#[test]
fn remove_deletes_the_table_and_the_unreferenced_artifact() {
    let project = Project::new(&with_demo_plugin(""), &[("demo-0.1.0.wasm", PAYLOAD)]);

    let output = project.run(["plugin", "remove", "demo"]);
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    assert!(stdout(&output).contains("Deleted cached artifact"));

    let config = project.config();
    assert!(!config.contains("[plugins.demo]"), "config: {config}");
    assert!(!project.artifact("demo-0.1.0.wasm").exists());
}

#[test]
fn remove_keep_artifact_preserves_the_file() {
    let project = Project::new(&with_demo_plugin(""), &[("demo-0.1.0.wasm", PAYLOAD)]);

    let output = project.run(["plugin", "remove", "demo", "--keep-artifact"]);
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    assert!(stdout(&output).contains("--keep-artifact"));

    assert!(!project.config().contains("[plugins.demo]"));
    assert!(project.artifact("demo-0.1.0.wasm").exists());
}

#[test]
fn remove_keeps_an_artifact_that_another_entry_still_references() {
    let config = format!(
        "{BASE_CONFIG}\n[plugins.demo]\nruntime = \"wasm\"\nsource = \"shared.wasm\"\nhash = \"{PAYLOAD_HASH}\"\n\n[plugins.copy]\nruntime = \"wasm\"\nsource = \"shared.wasm\"\nhash = \"{PAYLOAD_HASH}\"\n"
    );
    let project = Project::new(&config, &[("shared.wasm", PAYLOAD)]);

    let output = project.run(["plugin", "remove", "demo"]);
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("still referenced by copy"), "stdout: {text}");

    assert!(project.artifact("shared.wasm").exists());
    let config = project.config();
    assert!(!config.contains("[plugins.demo]"));
    assert!(config.contains("[plugins.copy]"));
}

#[test]
fn remove_keeps_comments_and_unrelated_tables_byte_for_byte() {
    let config = with_demo_plugin("\n# trailing comment\n");
    let project = Project::new(&config, &[("demo-0.1.0.wasm", PAYLOAD)]);

    let output = project.run(["plugin", "remove", "demo"]);
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));

    let updated = project.config();
    assert!(updated.contains("# trailing comment"), "config: {updated}");
    assert!(updated.contains("current_source = \"Cargo.toml\""), "config: {updated}");
}

#[test]
fn remove_never_deletes_an_absolute_source_outside_the_cache_root() {
    let project = Project::new(BASE_CONFIG, &[]);
    let outside = project.write("outside.wasm", PAYLOAD);
    let source = outside.to_string_lossy().replace('\\', "/");
    project.write_config(&config_with_source(&source));

    let output = project.run(["plugin", "remove", "demo"]);
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    assert!(outside.exists(), "remove must not delete a file outside the cache root");
    assert!(
        stdout(&output).contains("outside the plugin cache root"),
        "stdout: {}",
        stdout(&output)
    );
}

#[test]
fn remove_keeps_a_local_source_file_that_list_reports_installed() {
    let project = Project::new(BASE_CONFIG, &[]);
    let local = project.write("localdir/rel.wasm", PAYLOAD);
    project.write_config(&config_with_source("localdir/rel.wasm"));

    let listed = project.run(["plugin", "list"]);
    assert!(
        stdout(&listed).contains("installed:     yes"),
        "stdout: {}",
        stdout(&listed)
    );

    let output = project.run(["plugin", "remove", "demo"]);
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    assert!(local.exists(), "remove must keep the user's local source file");
    assert!(
        stdout(&output).contains("local source file"),
        "stdout: {}",
        stdout(&output)
    );
}

#[test]
fn remove_deletes_an_artifact_in_a_cache_subdirectory() {
    let project = Project::new(&config_with_source("nested/deep/artifact.wasm"), &[]);
    let artifact = project.artifact("nested/deep/artifact.wasm");
    fs::create_dir_all(artifact.parent().unwrap()).unwrap();
    fs::write(&artifact, PAYLOAD).unwrap();

    let output = project.run(["plugin", "remove", "demo"]);
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    assert!(!artifact.exists(), "an in-root subdirectory artifact must be deleted");
}

#[test]
fn remove_surfaces_a_typed_error_when_the_artifact_cannot_be_deleted() {
    let project = Project::new(&config_with_source("artifact-dir"), &[]);
    fs::create_dir_all(project.artifact("artifact-dir")).unwrap();

    let output = project.run(["plugin", "remove", "demo"]);
    assert_eq!(output.status.code(), Some(1), "stdout: {}", stdout(&output));
    let error = stderr(&output);
    assert!(error.contains("failed to delete plugin artifact"), "stderr: {error}");
    assert!(error.contains("artifact-dir"), "stderr: {error}");
    assert!(
        !project.config().contains("[plugins.demo]"),
        "config was not written: {}",
        project.config()
    );
    assert!(project.artifact("artifact-dir").is_dir());
}
