#![cfg(feature = "plugins")]
//! Hermetic tests for `cutver plugin add` (decisions D14-D19).
//!
//! Every test drives the real plan/apply functions against a
//! `tempfile::tempdir()` project with an injected fixture transport: no test
//! touches the network, and the cache root is never the ambient one.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use cutver::plugin::error::{InstallError, TransportError};
use cutver::plugin::install::{AddOptions, InstallPlan, apply_add, plan_add};
use cutver::plugin::manage::{execute_remove, plan_remove};
use cutver::plugin::registry::{RegistryIndexUrl, Transport};
use cutver::plugin::types::PluginName;
use cutver::plugin::{PluginCacheRoot, PluginError};

const PAYLOAD: &[u8] = b"hello";
const PAYLOAD_HASH: &str = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
const BASE_CONFIG: &str = "[version]\ncurrent_source = \"Cargo.toml\"\n";

/// Builds a valid registry entry body for `demo` at version `0.1.0`.
fn entry_body(capabilities: &[&str], digest: &str) -> String {
    let caps = capabilities
        .iter()
        .map(|cap| format!("\"{cap}\""))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "name = \"demo\"\ndescription = \"A demo plugin.\"\nauthor = \"cutver\"\ntier = \"community\"\n\
         repository = \"https://github.com/cutver/plugins\"\ncapabilities = [{caps}]\n\n[[release]]\n\
         version = \"0.1.0\"\n\
         url = \"https://github.com/cutver/plugins/releases/download/demo-v0.1.0/demo-0.1.0.wasm\"\n\
         sha256 = \"{digest}\"\n"
    )
}

fn index() -> RegistryIndexUrl {
    RegistryIndexUrl::parse("https://index.test/plugins/").expect("valid index base")
}

fn name() -> PluginName {
    PluginName::new("demo").expect("valid plugin name")
}

/// A transport serving one entry and one artifact, counting every call.
struct Fixture {
    entry: String,
    artifact: Vec<u8>,
    calls: AtomicUsize,
}

impl Fixture {
    fn new(entry: String, artifact: &[u8]) -> Self {
        Self {
            entry,
            artifact: artifact.to_vec(),
            calls: AtomicUsize::new(0),
        }
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl Transport for Fixture {
    fn fetch(&self, url: &str, cap_bytes: u64) -> Result<Vec<u8>, TransportError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let bytes = if url.ends_with(".toml") {
            self.entry.as_bytes().to_vec()
        } else {
            self.artifact.clone()
        };

        if bytes.len() as u64 > cap_bytes {
            return Err(TransportError::TransportTooLarge {
                url: url.to_string(),
                cap_bytes,
            });
        }

        Ok(bytes)
    }
}

/// A transport that can never reach anything.
struct Unreachable;

impl Transport for Unreachable {
    fn fetch(&self, url: &str, _cap_bytes: u64) -> Result<Vec<u8>, TransportError> {
        Err(TransportError::TransportUnreachable {
            url: url.to_string(),
            reason: "offline".to_string(),
        })
    }
}

/// A temporary project with a config file and an isolated cache directory.
struct Project {
    dir: tempfile::TempDir,
    config: PathBuf,
    cache_dir: PathBuf,
}

impl Project {
    fn new(config: &str) -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let config_path = dir.path().join("cutver.toml");
        fs::write(&config_path, config).expect("write config");
        let cache_dir = dir.path().join("cache");
        Self {
            dir,
            config: config_path,
            cache_dir,
        }
    }

    fn cache(&self) -> PluginCacheRoot {
        PluginCacheRoot::from_env_lookup(|key| {
            (key == "CUTVER_PLUGIN_CACHE_DIR").then(|| self.cache_dir.to_string_lossy().to_string())
        })
        .expect("cache root")
    }

    fn read_config(&self) -> String {
        fs::read_to_string(&self.config).expect("read config")
    }

    fn artifact(&self, file: &str) -> PathBuf {
        self.cache_dir.join(file)
    }
}

fn plan(project: &Project, options: &AddOptions, transport: &dyn Transport) -> Result<InstallPlan, PluginError> {
    plan_add(&project.config, &name(), options, &index(), &project.cache(), transport)
}

#[test]
fn add_installs_verified_artifact_and_writes_the_config() {
    let project = Project::new(BASE_CONFIG);
    let fixture = Fixture::new(entry_body(&["changelog.v1"], PAYLOAD_HASH), PAYLOAD);

    let plan = plan(&project, &AddOptions::default(), &fixture).expect("plan succeeds");
    assert_eq!(plan.version.to_string(), "0.1.0");
    assert_eq!(plan.digest, PAYLOAD_HASH);
    assert_eq!(
        plan.artifact_url,
        "https://github.com/cutver/plugins/releases/download/demo-v0.1.0/demo-0.1.0.wasm"
    );

    apply_add(&plan, &fixture).expect("apply succeeds");

    let config = project.read_config();
    assert!(config.contains("[plugins.demo]"), "config: {config}");
    assert!(config.contains("runtime = \"wasm\""), "config: {config}");
    assert!(config.contains("source = \"demo-0.1.0.wasm\""), "config: {config}");
    assert!(
        config.contains(&format!("hash = \"{PAYLOAD_HASH}\"")),
        "config: {config}"
    );
    assert!(config.contains("capabilities = [\"changelog.v1\"]"), "config: {config}");
    assert_eq!(
        fs::read(project.artifact("demo-0.1.0.wasm")).expect("artifact"),
        PAYLOAD
    );
}

#[test]
fn add_creates_an_implicit_parent_without_a_plugins_header() {
    let project = Project::new(BASE_CONFIG);
    let fixture = Fixture::new(entry_body(&["changelog.v1"], PAYLOAD_HASH), PAYLOAD);

    let plan = plan(&project, &AddOptions::default(), &fixture).expect("plan succeeds");
    apply_add(&plan, &fixture).expect("apply succeeds");

    let config = project.read_config();
    assert!(
        !config.lines().any(|line| line.trim() == "[plugins]"),
        "the implicit parent must not render a header: {config}"
    );
    let expected = format!(
        "{BASE_CONFIG}\n[plugins.demo]\nruntime = \"wasm\"\nsource = \"demo-0.1.0.wasm\"\nhash = \"{PAYLOAD_HASH}\"\ncapabilities = [\"changelog.v1\"]\n"
    );
    assert_eq!(config, expected, "the document must be the original plus the new table");
}

#[test]
fn add_then_remove_restores_the_original_config_bytes() {
    let project = Project::new(BASE_CONFIG);
    let fixture = Fixture::new(entry_body(&["changelog.v1"], PAYLOAD_HASH), PAYLOAD);

    let plan = plan(&project, &AddOptions::default(), &fixture).expect("plan succeeds");
    apply_add(&plan, &fixture).expect("apply succeeds");
    assert_ne!(project.read_config(), BASE_CONFIG, "add must change the config");

    let removal = plan_remove(&project.config, &name(), false, &project.cache()).expect("remove plan");
    execute_remove(&removal).expect("remove applies");

    assert_eq!(
        project.read_config().as_bytes(),
        BASE_CONFIG.as_bytes(),
        "an add -> remove round trip must restore the original bytes"
    );
}

#[test]
fn a_hand_written_explicit_plugins_table_survives_add_and_remove() {
    let original = format!("{BASE_CONFIG}\n[plugins]\n");
    let project = Project::new(&original);
    let fixture = Fixture::new(entry_body(&["changelog.v1"], PAYLOAD_HASH), PAYLOAD);

    let plan = plan(&project, &AddOptions::default(), &fixture).expect("plan succeeds");
    apply_add(&plan, &fixture).expect("apply succeeds");
    let after_add = project.read_config();
    assert!(
        after_add.lines().any(|line| line.trim() == "[plugins]"),
        "the user's explicit parent must survive add: {after_add}"
    );

    let removal = plan_remove(&project.config, &name(), false, &project.cache()).expect("remove plan");
    execute_remove(&removal).expect("remove applies");

    assert_eq!(
        project.read_config(),
        original,
        "the user's explicit parent must survive remove byte-for-byte"
    );
}

#[test]
fn tampered_artifact_fails_the_digest_gate_with_no_trace() {
    let project = Project::new(BASE_CONFIG);
    let before = project.read_config();
    let fixture = Fixture::new(entry_body(&["changelog.v1"], PAYLOAD_HASH), b"tampered");

    let plan = plan(&project, &AddOptions::default(), &fixture).expect("plan succeeds");
    let error = apply_add(&plan, &fixture).expect_err("tampered artifact must fail");

    assert!(matches!(error, PluginError::IntegrityMismatch { .. }), "error: {error}");
    assert_eq!(project.read_config(), before);
    assert!(!project.artifact("demo-0.1.0.wasm").exists());
    assert_eq!(
        fs::read_dir(&project.cache_dir).expect("cache dir").count(),
        0,
        "the temp file must be cleaned up on digest failure"
    );
}

#[test]
fn unknown_capability_fails_closed_and_writes_nothing() {
    let project = Project::new(BASE_CONFIG);
    let fixture = Fixture::new(entry_body(&["time-travel.v9"], PAYLOAD_HASH), PAYLOAD);

    let error = plan(&project, &AddOptions::default(), &fixture).expect_err("unknown capability must fail");

    assert!(
        matches!(error, PluginError::RegistryUnknownCapability { .. }),
        "error: {error}"
    );
    assert_eq!(project.read_config(), BASE_CONFIG);
    assert!(!project.artifact("demo-0.1.0.wasm").exists());
}

#[test]
fn existing_entry_fails_before_the_transport_is_ever_called() {
    let existing = format!(
        "{BASE_CONFIG}\n[plugins.demo]\nruntime = \"wasm\"\nsource = \"demo-0.1.0.wasm\"\nhash = \"{PAYLOAD_HASH}\"\n"
    );
    let project = Project::new(&existing);
    let fixture = Fixture::new(entry_body(&["changelog.v1"], PAYLOAD_HASH), PAYLOAD);

    let error = plan(&project, &AddOptions::default(), &fixture).expect_err("existing entry must fail");

    assert!(
        matches!(
            error,
            PluginError::Install(InstallError::PluginAlreadyConfigured { .. })
        ),
        "error: {error}"
    );
    assert_eq!(
        fixture.calls(),
        0,
        "D14: no network call before the existing-entry check"
    );
    assert_eq!(project.read_config(), existing);
}

#[test]
fn force_replaces_the_table_and_preserves_unrelated_content() {
    let existing = format!(
        "# top comment\n{BASE_CONFIG}\n[plugins.other]\nruntime = \"process\"\ncommand = \"x\"\n\n\
         [plugins.demo]\nruntime = \"wasm\"\nsource = \"old.wasm\"\nhash = \"deadbeef\"\n"
    );
    let project = Project::new(&existing);
    let fixture = Fixture::new(entry_body(&["changelog.v1"], PAYLOAD_HASH), PAYLOAD);
    let options = AddOptions {
        force: true,
        ..AddOptions::default()
    };

    let plan = plan(&project, &options, &fixture).expect("plan succeeds");
    apply_add(&plan, &fixture).expect("apply succeeds");

    let config = project.read_config();
    assert!(config.contains("# top comment"), "config: {config}");
    assert!(config.contains("[plugins.other]"), "config: {config}");
    assert_eq!(config.matches("[plugins.demo]").count(), 1, "config: {config}");
    assert!(config.contains("source = \"demo-0.1.0.wasm\""), "config: {config}");
    assert!(!config.contains("old.wasm"), "config: {config}");
}

#[test]
fn lifecycle_requires_an_event_and_writes_events_when_given() {
    let project = Project::new(BASE_CONFIG);
    let fixture = Fixture::new(entry_body(&["lifecycle.v1"], PAYLOAD_HASH), PAYLOAD);

    let error = plan(&project, &AddOptions::default(), &fixture).expect_err("missing --event must fail");
    assert!(
        matches!(error, PluginError::Install(InstallError::PluginFlagRequired { .. })),
        "error: {error}"
    );
    let message = error.to_string();
    assert!(message.contains("events"), "message: {message}");
    assert!(message.contains("lifecycle.v1"), "message: {message}");

    let options = AddOptions {
        events: vec!["pre_bump".to_string()],
        ..AddOptions::default()
    };
    let plan = plan(&project, &options, &fixture).expect("plan succeeds");
    apply_add(&plan, &fixture).expect("apply succeeds");
    assert!(project.read_config().contains("events = [\"pre_bump\"]"));
}

#[test]
fn manifest_requires_a_match_pattern() {
    let project = Project::new(BASE_CONFIG);
    let fixture = Fixture::new(entry_body(&["manifest.v1"], PAYLOAD_HASH), PAYLOAD);

    let error = plan(&project, &AddOptions::default(), &fixture).expect_err("missing --manifest-match must fail");
    assert!(
        matches!(error, PluginError::Install(InstallError::PluginFlagRequired { .. })),
        "error: {error}"
    );
    assert!(error.to_string().contains("manifest_match"), "message: {error}");
}

#[test]
fn plan_only_leaves_no_trace_and_exposes_provenance() {
    let project = Project::new(BASE_CONFIG);
    let before = project.read_config();
    let fixture = Fixture::new(entry_body(&["changelog.v1"], PAYLOAD_HASH), PAYLOAD);

    let plan = plan(&project, &AddOptions::default(), &fixture).expect("plan succeeds");

    assert_eq!(plan.version.to_string(), "0.1.0");
    assert!(plan.artifact_url.ends_with("demo-0.1.0.wasm"));
    assert_eq!(plan.digest, PAYLOAD_HASH);
    assert!(plan.diff.contains("+ [plugins.demo]"), "diff: {}", plan.diff);
    assert!(
        plan.diff.contains("+ source = \"demo-0.1.0.wasm\""),
        "diff: {}",
        plan.diff
    );
    assert_eq!(project.read_config(), before, "--dry-run must not touch the config");
    assert!(!project.cache_dir.exists(), "--dry-run must not create the cache");
    assert_eq!(fixture.calls(), 1, "--dry-run must not download the artifact");
}

#[test]
fn a_failed_config_write_leaves_no_installed_artifact() {
    let project = Project::new(BASE_CONFIG);
    let fixture = Fixture::new(entry_body(&["changelog.v1"], PAYLOAD_HASH), PAYLOAD);
    let plan = plan(&project, &AddOptions::default(), &fixture).expect("plan succeeds");

    fs::remove_file(&project.config).expect("remove config");
    fs::create_dir(&project.config).expect("config path becomes a directory");

    let error = apply_add(&plan, &fixture).expect_err("config write must fail");
    assert!(matches!(error, PluginError::PluginConfigWrite { .. }), "error: {error}");
    assert!(
        !project.artifact("demo-0.1.0.wasm").exists(),
        "artifact must be rolled back"
    );
}

/// A pre-existing artifact that cannot be read must fail closed, never be deleted.
///
/// Regression for the data-loss path: when `read_original` degraded a read failure
/// to `None`, the rollback guard would later `remove_file` a file it never backed
/// up, so a `--force` reinstall could destroy a previously installed artifact.
#[cfg(unix)]
#[test]
fn unreadable_preexisting_artifact_fails_closed_instead_of_being_deleted() {
    use std::os::unix::fs::PermissionsExt;

    let project = Project::new(BASE_CONFIG);
    let fixture = Fixture::new(entry_body(&["changelog.v1"], PAYLOAD_HASH), PAYLOAD);
    let plan = plan(&project, &AddOptions::default(), &fixture).expect("plan succeeds");

    let artifact = project.artifact("demo-0.1.0.wasm");
    fs::create_dir_all(&project.cache_dir).expect("cache dir");
    fs::write(&artifact, b"previously installed").expect("write pre-existing artifact");
    fs::set_permissions(&artifact, fs::Permissions::from_mode(0o000)).expect("make it unreadable");

    let error = apply_add(&plan, &fixture).expect_err("an unreadable original must fail closed");

    fs::set_permissions(&artifact, fs::Permissions::from_mode(0o644)).expect("restore permissions");

    assert!(
        matches!(error, PluginError::Install(InstallError::ArtifactRead { .. })),
        "error: {error}"
    );
    assert!(artifact.exists(), "the pre-existing artifact must survive");
    assert_eq!(fs::read(&artifact).expect("read"), b"previously installed");
    assert_eq!(project.read_config(), BASE_CONFIG, "the config must be untouched");
}

#[test]
fn unreachable_registry_fails_naming_the_index_url() {
    let project = Project::new(BASE_CONFIG);

    let error = plan(&project, &AddOptions::default(), &Unreachable).expect_err("unreachable must fail");
    let message = error.to_string();

    assert!(
        matches!(error, PluginError::RegistryUnreachable { .. }),
        "error: {error}"
    );
    assert!(
        message.contains("https://index.test/plugins/demo.toml"),
        "message must name the index URL: {message}"
    );
    assert_eq!(project.read_config(), BASE_CONFIG);
    assert!(!project.artifact("demo-0.1.0.wasm").exists());
}

#[test]
fn registry_base_is_https_only_and_normalized_to_one_trailing_slash() {
    let insecure = RegistryIndexUrl::parse("http://insecure.test/plugins/").expect_err("http must fail");
    assert!(
        matches!(insecure, PluginError::Install(InstallError::IndexUrl { .. })),
        "error: {insecure}"
    );

    let missing_slash = RegistryIndexUrl::parse("https://index.test/plugins").expect("https is accepted");
    assert_eq!(missing_slash.as_str(), "https://index.test/plugins/");
    let doubled = RegistryIndexUrl::parse("https://index.test/plugins//").expect("https is accepted");
    assert_eq!(doubled.as_str(), "https://index.test/plugins/");
}

#[test]
fn version_without_a_matching_release_fails_closed() {
    let project = Project::new(BASE_CONFIG);
    let fixture = Fixture::new(entry_body(&["changelog.v1"], PAYLOAD_HASH), PAYLOAD);
    let options = AddOptions {
        version: Some("9.9.9".to_string()),
        ..AddOptions::default()
    };

    let error = plan(&project, &options, &fixture).expect_err("no matching release must fail");
    assert!(
        matches!(error, PluginError::RegistryNoMatchingRelease { .. }),
        "error: {error}"
    );
}

#[test]
fn cli_rejects_an_http_registry_before_any_work() {
    let project = Project::new(BASE_CONFIG);

    let output = Command::new(env!("CARGO_BIN_EXE_cutver"))
        .current_dir(project.dir.path())
        .env("CUTVER_PLUGIN_CACHE_DIR", &project.cache_dir)
        .env("NO_COLOR", "1")
        .args([
            "plugin",
            "add",
            "demo",
            "--registry",
            "http://insecure.test/plugins/",
            "--dry-run",
        ])
        .output()
        .expect("run cutver");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("https"), "stderr: {stderr}");
    assert_eq!(project.read_config(), BASE_CONFIG);
    assert!(!project.cache_dir.exists());
}
