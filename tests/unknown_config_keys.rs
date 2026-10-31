//! Behaviour tests for the unknown-key gate over `cutver.toml`.
//!
//! An unknown key in the configuration document must be a hard error that
//! names the offending table and key. The two `deny_unknown_fields` proof
//! tests document why an explicit schema pass is the chosen mechanism.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static TMP_SEQ: AtomicU64 = AtomicU64::new(0);

fn load(toml: &str) -> Result<cutver::config::Config, cutver::config::ConfigError> {
    let n = TMP_SEQ.fetch_add(1, Ordering::SeqCst);
    let dir: PathBuf = std::env::temp_dir().join(format!("cutver-unknown-keys-{}-{}", std::process::id(), n));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("cutver.toml"), toml).unwrap();
    cutver::config::load(dir.join("cutver.toml"))
}

// ---------------------------------------------------------------------------
// RED: the behaviour this feature adds.
// ---------------------------------------------------------------------------

#[test]
fn unknown_top_level_table_is_rejected_and_names_table_and_key() {
    let err = load(
        r#"
[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"

[workspace]
mode = "monorepo"
members = ["a", "b"]
"#,
    )
    .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("workspace"), "error did not name the table: {msg}");
    assert!(msg.contains("mode"), "error did not name the key: {msg}");
}

#[test]
fn unknown_key_in_known_table_is_rejected_and_names_table_and_key() {
    let err = load(
        r#"
[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"

[changelog]
pathh = "CHANGELOG.md"
"#,
    )
    .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("changelog"), "error did not name the table: {msg}");
    assert!(msg.contains("pathh"), "error did not name the key: {msg}");
}

// ---------------------------------------------------------------------------
// GREEN companions: the accepted set must not shrink.
// ---------------------------------------------------------------------------

#[test]
fn preflight_arbitrary_step_names_and_timeout_keep_parsing() {
    let cfg = load(
        r#"
[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"

[preflight]
default_timeout = 5
check = "cargo check"
tests = { command = "cargo test", timeout = 60 }
"#,
    )
    .unwrap();
    assert_eq!(cfg.preflight_default_timeout, Some(5));
    assert_eq!(cfg.preflight.len(), 2);
    assert_eq!(cfg.preflight[0].1.command, "cargo check");
    assert_eq!(cfg.preflight[1].1.command, "cargo test");
}

#[test]
fn every_manifest_kind_key_keeps_parsing() {
    let cfg = load(
        r#"
[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"

[[manifest]]
path = "Cargo.toml.toml"
kind = "toml"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[[manifest]]
path = "app/build.gradle.kts"
kind = "gradle"
version_name_field = "versionName"
version_code_field = "versionCode"

[[manifest]]
path = "version.txt"
kind = "regex"
pattern = 'v\d+'
replacement = 'v{{version}}'

[[manifest]]
path = "pyproject.toml"
kind = "pyproject"
table = "tool.poetry"

[[manifest]]
path = "pyproject-alt.toml"
kind = "pyproject-toml"

[[manifest]]
path = "plugin-manifest.txt"
kind = "plugin"
plugin = "custom-adapter"

[[manifest]]
path = "plugin-auto.txt"
kind = "plugin"
"#,
    )
    .unwrap();
    assert_eq!(cfg.manifest.len(), 9);
}

#[test]
fn package_key_parses_for_every_manifest_kind_and_inline_array() {
    let cfg = load(
        r#"
[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"
package = "pkg-cargo"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"
package = "pkg-json"

[[manifest]]
path = "app/build.gradle.kts"
kind = "gradle"
version_name_field = "versionName"
version_code_field = "versionCode"
package = "pkg-gradle"

[[manifest]]
path = "version.txt"
kind = "regex"
pattern = 'v\d+'
replacement = 'v{{version}}'
package = "pkg-regex"

[[manifest]]
path = "pyproject.toml"
kind = "pyproject"
table = "tool.poetry"
package = "pkg-pyproject"

[[manifest]]
path = "plugin-manifest.txt"
kind = "plugin"
plugin = "custom-adapter"
package = "pkg-plugin"
"#,
    )
    .unwrap();
    let debug = format!("{:?}", cfg.manifest);
    for package in [
        "pkg-cargo",
        "pkg-json",
        "pkg-gradle",
        "pkg-regex",
        "pkg-pyproject",
        "pkg-plugin",
    ] {
        assert!(debug.contains(package), "manifest debug did not retain package {package}: {debug}");
    }

    let cfg = load(
        r#"
manifest = [
  { path = "Cargo.toml", kind = "cargo-package", package = "inline-cargo" },
  { path = "package.json", kind = "json", field = "version", package = "inline-json" },
]
"#,
    )
    .unwrap();
    let debug = format!("{:?}", cfg.manifest);
    assert!(debug.contains("inline-cargo"), "inline manifest did not retain package: {debug}");
    assert!(debug.contains("inline-json"), "inline manifest did not retain package: {debug}");
}

#[test]
fn duplicate_package_names_are_rejected_naming_the_value() {
    let err = load(
        r#"
[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"
package = "shared"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"
package = "shared"
"#,
    )
    .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("duplicate package name"), "error did not report a duplicate package: {msg}");
    assert!(msg.contains("shared"), "error did not name the packaged value: {msg}");
}

#[test]
fn invalid_package_names_are_rejected() {
    for package in ["", "   ", "two words", "a/b", "a\\b"] {
        let toml = format!(
            "[[manifest]]\npath = \"Cargo.toml\"\nkind = \"cargo-package\"\npackage = '{package}'\n"
        );
        assert!(load(&toml).is_err(), "invalid package value {package:?} was accepted");
    }
}

#[test]
fn plugin_tables_and_permissions_keep_parsing() {
    load(
        r#"
[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"

[plugins.helm-adapter]
runtime = "process"
command = "helm-plugin"
capabilities = ["manifest.v1", "lifecycle.v1"]
timeout_seconds = 15

[plugins.helm-adapter.permissions]
network = ["example.com"]
"#,
    )
    .unwrap();
}

#[test]
fn inline_plugins_and_inline_manifest_array_keep_parsing() {
    load(
        r#"
manifest = [{ path = "Cargo.toml", kind = "cargo-package" }]
plugins = { helm = { runtime = "process", permissions = { env = ["HOME"] } } }
"#,
    )
    .unwrap();
}

#[test]
fn unknown_plugin_and_permissions_keys_are_rejected() {
    let err = load(
        r#"
[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"

[plugins.helm]
runtime = "process"
runntime = "wasm"
"#,
    )
    .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("plugins.helm"), "{msg}");
    assert!(msg.contains("runntime"), "{msg}");

    let err = load(
        r#"
[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"

[plugins.helm]
runtime = "process"

[plugins.helm.permissions]
networkk = ["example.com"]
"#,
    )
    .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("plugins.helm.permissions"), "{msg}");
    assert!(msg.contains("networkk"), "{msg}");
}

#[test]
fn unknown_top_level_scalar_key_is_rejected() {
    let err = load(
        r#"
[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"

version_strategy = "manual"
"#,
    )
    .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("version_strategy"), "{msg}");
}

// ---------------------------------------------------------------------------
// Empty nested unknown tables must name the table, not an empty key.
// ---------------------------------------------------------------------------

#[test]
fn empty_nested_unknown_table_names_the_table_not_an_empty_key() {
    let err = load(
        r#"
[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"

[foo]
[foo.bar]
"#,
    )
    .unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("unknown table [foo.bar]"),
        "error did not name the offending table: {msg}"
    );
    assert!(!msg.contains("''"), "error contains an empty key: {msg}");
}

#[test]
fn empty_inline_nested_unknown_table_names_the_table_not_an_empty_key() {
    let err = load(
        r#"
foo = { bar = {} }

[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"
"#,
    )
    .unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("unknown table [foo.bar]"),
        "error did not name the offending table: {msg}"
    );
    assert!(!msg.contains("''"), "error contains an empty key: {msg}");
}

#[test]
fn neighbouring_unknown_cases_still_name_a_real_table_or_key() {
    // Empty top-level unknown table keeps naming the key.
    let err = load(
        r#"
[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"

[foo]
"#,
    )
    .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("unknown top-level key 'foo'"), "{msg}");

    // Nested unknown table with keys names a real key, never an empty one.
    let err = load(
        r#"
[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"

[foo]
[foo.bar]
baz = 1
"#,
    )
    .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("baz"), "{msg}");
    assert!(!msg.contains("''"), "{msg}");

    // An unknown key inside a known table names the enclosing table and the key.
    let err = load(
        r#"
[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"

[changelog]
x = 1
"#,
    )
    .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("changelog") && msg.contains("'x'"), "{msg}");

    // An unknown key inside [[manifest]] names the key.
    let err = load(
        r#"
[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"
typpo = 1
"#,
    )
    .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("manifest") && msg.contains("typpo"), "{msg}");
}

// ---------------------------------------------------------------------------
// Why `#[serde(deny_unknown_fields)]` cannot be used as written.
// ---------------------------------------------------------------------------

/// Proof 1: `Config` skips `preflight` on deserialize, so a `deny_unknown_fields`
/// pass over `Config` rejects the `preflight` key itself and every real config.
#[test]
fn deny_unknown_fields_on_config_mirror_rejects_skipped_preflight_key() {
    #[allow(dead_code)]
    #[derive(Debug, serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct ConfigMirror {
        #[serde(default)]
        manifest: Vec<serde_json::Value>,
        #[serde(skip)]
        preflight: Vec<String>,
        #[serde(default)]
        changelog: serde_json::Value,
    }

    let err = toml::from_str::<ConfigMirror>(
        "[[manifest]]\npath = \"a\"\nkind = \"cargo-package\"\n[preflight]\ntests = \"x\"\n",
    )
    .unwrap_err();
    assert!(
        err.to_string().contains("preflight"),
        "expected the skipped preflight key to be reported unknown: {err}"
    );
}

/// Proof 2: `Manifest` flattens `ManifestKind`. With `deny_unknown_fields`
/// serde accepts the derive but rejects the flattened variant keys (`field`,
/// ...) as unknown at deserialize time, so it cannot be used there.
#[test]
fn deny_unknown_fields_with_flatten_rejects_flattened_manifest_keys() {
    #[allow(dead_code)]
    #[derive(Debug, serde::Deserialize)]
    #[serde(tag = "kind", rename_all = "kebab-case")]
    enum KindMirror {
        Json { field: String },
    }

    #[allow(dead_code)]
    #[derive(Debug, serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct ManifestMirror {
        path: String,
        #[serde(flatten)]
        kind: KindMirror,
    }

    let err = toml::from_str::<ManifestMirror>("path = \"a\"\nkind = \"json\"\nfield = \"version\"\n").unwrap_err();
    assert!(
        err.to_string().contains("unknown field") && err.to_string().contains("field"),
        "expected the flattened variant key to be reported unknown: {err}"
    );
}
