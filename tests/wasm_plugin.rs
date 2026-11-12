#![cfg(feature = "plugins")]

//! End-to-end proof that the Extism WASM driver actually executes a real guest
//! module's `invoke` export and round-trips the `PluginInvocation` envelope.
//!
//! The fixtures are inline WAT compiled with the `wat` crate (a dev-only
//! dependency already present in `Cargo.lock`), so this suite is hermetic and
//! requires no prebuilt `.wasm` artifact.

use std::collections::{HashMap, HashSet};

use cutver::plugin::driver::WasmDriver;
use cutver::plugin::dto::{ChangelogRenderRequest, PluginInvocation};
use cutver::plugin::types::PluginCall;
use cutver::plugin::{Capability, PluginConfig, PluginDriver, PluginManager, PluginName, RuntimeKind};

/// Echo fixture: copies the Extism input bytes verbatim into the output.
///
/// Extism 1.30 hosts all buffer state in the kernel's `extism:host/env` memory,
/// so the guest allocates there with `alloc` and writes with `store_u8`; it does
/// not touch its own linear memory. Proves the `invoke` export is resolved and
/// that the envelope crosses the WASM boundary byte-exactly.
const ECHO_WAT: &str = r#"
(module
  (import "extism:host/env" "input_length" (func $input_length (result i64)))
  (import "extism:host/env" "input_load_u8" (func $input_load_u8 (param i64) (result i32)))
  (import "extism:host/env" "alloc" (func $alloc (param i64) (result i64)))
  (import "extism:host/env" "store_u8" (func $store_u8 (param i64 i32)))
  (import "extism:host/env" "output_set" (func $output_set (param i64 i64)))
  (func (export "invoke") (result i32)
    (local $len i64)
    (local $i i64)
    (local $out i64)
    (local.set $len (call $input_length))
    (local.set $out (call $alloc (local.get $len)))
    (local.set $i (i64.const 0))
    (block $done
      (loop $copy
        (br_if $done (i64.ge_u (local.get $i) (local.get $len)))
        (call $store_u8
          (i64.add (local.get $out) (local.get $i))
          (call $input_load_u8 (local.get $i)))
        (local.set $i (i64.add (local.get $i) (i64.const 1)))
        (br $copy)))
    (call $output_set (local.get $out) (local.get $len))
    (i32.const 0)))
"#;

/// Fixed-response fixture: returns the literal `{"body":"rendered by wasm"}`,
/// whose byte length is 27. The literal lives in the guest's own memory and is
/// copied into the kernel output buffer via `alloc`/`store_u8`.
const FIXED_RESPONSE_WAT: &str = r#"
(module
  (import "extism:host/env" "alloc" (func $alloc (param i64) (result i64)))
  (import "extism:host/env" "store_u8" (func $store_u8 (param i64 i32)))
  (import "extism:host/env" "output_set" (func $output_set (param i64 i64)))
  (memory 1)
  ;; literal: {"body":"rendered by wasm"} -> 27 bytes
  (data (i32.const 0) "{\"body\":\"rendered by wasm\"}")
  (func (export "invoke") (result i32)
    (local $out i64)
    (local $i i64)
    (local.set $out (call $alloc (i64.const 27)))
    (local.set $i (i64.const 0))
    (block $done
      (loop $copy
        (br_if $done (i64.ge_u (local.get $i) (i64.const 27)))
        (call $store_u8
          (i64.add (local.get $out) (local.get $i))
          (i32.load8_u (i32.wrap_i64 (local.get $i))))
        (local.set $i (i64.add (local.get $i) (i64.const 1)))
        (br $copy)))
    (call $output_set (local.get $out) (i64.const 27))
    (i32.const 0)))
"#;

/// An 8-byte module header: a valid, loadable module that does NOT export
/// `invoke`. Used to prove the driver detects a missing export.
const MINIMAL_WASM: &[u8] = &[0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];

fn echo_driver() -> WasmDriver {
    let wasm = wat::parse_str(ECHO_WAT).expect("echo WAT should compile");
    let name = PluginName::new("echo-wasm").unwrap();
    let capabilities = HashSet::from([Capability::ChangelogV1]);
    WasmDriver::new(name, &wasm, &Default::default(), Some(5), capabilities, None).expect("driver should build")
}

#[test]
fn test_wasm_invoke_export_echoes_envelope_byte_for_byte() {
    let driver = echo_driver();

    let invocation = PluginInvocation::new(
        PluginCall::ChangelogRender,
        &serde_json::json!({
            "root_dir": "/workspace",
            "version": "1.4.0",
            "tag_name": "v1.4.0",
            "release_date": "2025-05-18",
            "commits": [
                {"sha": "abc123", "message": "feat: add thing", "is_breaking": false}
            ],
        }),
    )
    .unwrap();

    // The guest echoes its input. A match against the serialized envelope can
    // only hold if the bytes physically crossed the WASM boundary and returned.
    let echoed = driver.invoke(&invocation).expect("invoke export should execute");
    let expected = serde_json::to_vec(&invocation).unwrap();

    assert_eq!(
        echoed, expected,
        "guest output must equal the serialized PluginInvocation envelope"
    );
    assert!(
        String::from_utf8_lossy(&echoed).contains("\"capability\":\"changelog.v1\""),
        "echoed envelope must carry the capability"
    );
}

#[test]
fn test_plugin_manager_wasm_round_trip() {
    let wasm = wat::parse_str(FIXED_RESPONSE_WAT).expect("fixed-response WAT should compile");

    let dir = tempfile::tempdir().unwrap();
    let wasm_file = dir.path().join("plugin.wasm");
    std::fs::write(&wasm_file, &wasm).unwrap();

    let name = PluginName::new("wasm-render-plugin").unwrap();
    let config = PluginConfig {
        runtime: RuntimeKind::Wasm,
        source: Some(wasm_file.to_str().unwrap().to_string()),
        hash: None,
        command: None,
        capabilities: vec![Capability::ChangelogV1],
        events: vec![],
        manifest_match: vec![],
        permissions: Default::default(),
        timeout_seconds: Some(5),
        config: None,
    };

    let mut map = HashMap::new();
    map.insert(name.clone(), config);

    let manager = PluginManager::from_config(&map).expect("manager should instantiate wasm plugin");

    let req = ChangelogRenderRequest {
        root_dir: "/workspace".to_string(),
        version: "1.0.0".to_string(),
        tag_name: "v1.0.0".to_string(),
        previous_tag: None,
        release_date: "2025-05-18".to_string(),
        commits: vec![],
        repository: None,
        compare_url: None,
        is_prerelease: false,
        contributors: vec![],
    };

    let response = manager
        .dispatch_changelog(&name, &req)
        .expect("dispatch_changelog should execute guest and deserialize response");
    assert_eq!(response.body, "rendered by wasm");
}

#[test]
fn test_wasm_missing_invoke_export_is_detected() {
    let name = PluginName::new("no-invoke-wasm").unwrap();
    let capabilities = HashSet::from([Capability::ChangelogV1]);
    let driver = WasmDriver::new(name, MINIMAL_WASM, &Default::default(), Some(5), capabilities, None)
        .expect("minimal module should load");

    let invocation = PluginInvocation::new(PluginCall::ChangelogRender, &serde_json::json!({})).unwrap();

    let err = driver
        .invoke(&invocation)
        .expect_err("a module without an `invoke` export must not succeed");

    // `ExecutionFailed` is the catch-all for every non-timeout Extism error, so
    // matching the variant alone would also accept a load/instantiation failure
    // for the wrong reason. The message must name the missing export.
    match err {
        cutver::plugin::PluginError::ExecutionFailed { stderr, .. } => assert!(
            stderr.contains("Function not found") && stderr.contains("invoke"),
            "failure must be attributable to the missing `invoke` export, got: {stderr}"
        ),
        other => panic!("expected ExecutionFailed for the missing export, got {other:?}"),
    }
}

/// Reports the guest name of preopen `fd 3` through `fd_prestat_get` /
/// `fd_prestat_dir_name`. That name is the only thing a guest can match a host
/// path against, so it is what the harness must get right.
const PREOPEN_NAME_WAT: &str = r#"
(module
  (import "extism:host/env" "alloc" (func $alloc (param i64) (result i64)))
  (import "extism:host/env" "store_u8" (func $store_u8 (param i64 i32)))
  (import "extism:host/env" "output_set" (func $output_set (param i64 i64)))
  (import "wasi_snapshot_preview1" "fd_prestat_get"
    (func $fd_prestat_get (param i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "fd_prestat_dir_name"
    (func $fd_prestat_dir_name (param i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (func (export "invoke") (result i32)
    (local $err i32)
    (local $len i32)
    (local $out i64)
    (local $i i32)
    (local.set $err (call $fd_prestat_get (i32.const 3) (i32.const 0)))
    (if (i32.ne (local.get $err) (i32.const 0)) (then (return (i32.const 1))))
    (local.set $len (i32.load (i32.const 4)))
    (local.set $err (call $fd_prestat_dir_name (i32.const 3) (i32.const 64) (local.get $len)))
    (if (i32.ne (local.get $err) (i32.const 0)) (then (return (i32.const 2))))
    (local.set $out (call $alloc (i64.extend_i32_u (local.get $len))))
    (local.set $i (i32.const 0))
    (block $done
      (loop $copy
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (call $store_u8
          (i64.add (local.get $out) (i64.extend_i32_u (local.get $i)))
          (i32.load8_u (i32.add (i32.const 64) (local.get $i))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $copy)))
    (call $output_set (local.get $out) (i64.extend_i32_u (local.get $len)))
    (i32.const 0)))
"#;

/// Opens `cutver.toml` relative to preopen `fd 3` and returns its bytes. The
/// path is relative on purpose: WASI resolves guest paths against preopens, so a
/// read can only succeed when the harness mounted the directory where the guest
/// looks for it.
const READ_CONFIG_WAT: &str = r#"
(module
  (import "extism:host/env" "alloc" (func $alloc (param i64) (result i64)))
  (import "extism:host/env" "store_u8" (func $store_u8 (param i64 i32)))
  (import "extism:host/env" "output_set" (func $output_set (param i64 i64)))
  (import "wasi_snapshot_preview1" "path_open"
    (func $path_open (param i32 i32 i32 i32 i32 i64 i64 i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "fd_read"
    (func $fd_read (param i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 1024) "cutver.toml")
  (func (export "invoke") (result i32)
    (local $err i32)
    (local $fd i32)
    (local $nread i32)
    (local $out i64)
    (local $i i32)
    (local.set $err
      (call $path_open (i32.const 3) (i32.const 0) (i32.const 1024) (i32.const 11)
        (i32.const 0) (i64.const 2) (i64.const 0) (i32.const 0) (i32.const 8)))
    (if (i32.ne (local.get $err) (i32.const 0)) (then (return (i32.const 10))))
    (local.set $fd (i32.load (i32.const 8)))
    (i32.store (i32.const 16) (i32.const 4096))
    (i32.store (i32.const 20) (i32.const 4096))
    (local.set $err (call $fd_read (local.get $fd) (i32.const 16) (i32.const 1) (i32.const 24)))
    (if (i32.ne (local.get $err) (i32.const 0)) (then (return (i32.const 11))))
    (local.set $nread (i32.load (i32.const 24)))
    (local.set $out (call $alloc (i64.extend_i32_u (local.get $nread))))
    (local.set $i (i32.const 0))
    (block $done
      (loop $copy
        (br_if $done (i32.ge_u (local.get $i) (local.get $nread)))
        (call $store_u8
          (i64.add (local.get $out) (i64.extend_i32_u (local.get $i)))
          (i32.load8_u (i32.add (i32.const 4096) (local.get $i))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $copy)))
    (call $output_set (local.get $out) (i64.extend_i32_u (local.get $nread)))
    (i32.const 0)))
"#;

/// A temp workspace whose `cutver.toml` grants the plugin `filesystem = ["."]`,
/// exactly like this repository's own configuration. `config::load` is the real
/// entry point, so the grant under test is the one the CLI would produce.
struct Probe {
    dir: tempfile::TempDir,
    plugin: PluginConfig,
    root: String,
    root_path: std::path::PathBuf,
    config_bytes: Vec<u8>,
}

impl Probe {
    fn new(wasm: &[u8]) -> Self {
        let dir = tempfile::tempdir().expect("temp workspace");
        let root = dir.path();
        let wasm_path = root.join("plugin.wasm").to_string_lossy().replace('\\', "/");

        std::fs::write(root.join("plugin.wasm"), wasm).expect("write wasm artifact");
        std::fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"probe\"\nversion = \"0.1.0\"\n",
        )
        .expect("write manifest");
        std::fs::write(root.join("CHANGELOG.md"), "# Changelog\n\n## [Unreleased]\n").expect("write changelog");

        let config_text = format!(
            "[project]\nname = \"probe\"\n\n\
             [[manifest]]\npath = \"Cargo.toml\"\nkind = \"cargo-package\"\n\n\
             [changelog]\npath = \"CHANGELOG.md\"\n\n\
             [plugins.sandbox-probe]\nruntime = \"wasm\"\nsource = \"{wasm_path}\"\n\
             capabilities = [\"changelog.v1\"]\nevents = []\n\n\
             [plugins.sandbox-probe.permissions]\nfilesystem = [\".\"]\n"
        );
        std::fs::write(root.join("cutver.toml"), &config_text).expect("write config");

        let config = cutver::config::load(root.join("cutver.toml")).expect("config must load");
        let plugin = config
            .plugins
            .get(&PluginName::new("sandbox-probe").expect("plugin name"))
            .expect("plugin must be declared")
            .clone();

        Self {
            root: config.root_dir.to_string_lossy().replace('\\', "/"),
            root_path: config.root_dir.clone(),
            plugin,
            config_bytes: std::fs::read(root.join("cutver.toml")).expect("read config"),
            dir,
        }
    }

    fn driver(&self, wasm: &[u8]) -> WasmDriver {
        WasmDriver::new(
            PluginName::new("sandbox-probe").expect("plugin name"),
            wasm,
            &self.plugin.permissions,
            Some(30),
            HashSet::from([Capability::ChangelogV1]),
            None,
        )
        .expect("driver should build")
    }

    fn invocation(&self) -> PluginInvocation {
        PluginInvocation::new(
            PluginCall::ChangelogRender,
            &serde_json::json!({ "root_dir": self.root, "version": "0.1.0" }),
        )
        .expect("invocation should build")
    }
}

#[test]
fn test_config_resolves_a_declared_filesystem_grant_to_the_workspace_root() {
    let wasm = wat::parse_str(ECHO_WAT).expect("echo WAT should compile");
    let probe = Probe::new(&wasm);

    assert_eq!(
        probe.plugin.permissions.filesystem.len(),
        1,
        "the declared grant must resolve to exactly one host directory"
    );
    assert_eq!(
        std::path::Path::new(&probe.plugin.permissions.filesystem[0]),
        probe.root_path.as_path(),
        "`filesystem = [\".\"]` must resolve to the absolute workspace root"
    );
    assert!(
        !probe.root.contains('\\'),
        "the harness must never hand a backslash to the guest: {}",
        probe.root
    );
    drop(probe.dir);
}

#[test]
fn test_guest_sees_the_declared_directory_mounted_where_the_payload_points() {
    let wasm = wat::parse_str(PREOPEN_NAME_WAT).expect("preopen-name WAT should compile");
    let probe = Probe::new(&wasm);
    let observed = probe
        .driver(&wasm)
        .invoke(&probe.invocation())
        .expect("guest must report its preopen name");

    assert_eq!(
        String::from_utf8_lossy(&observed),
        probe.root,
        "the guest mount name must equal the `root_dir` the payload advertises"
    );
    drop(probe.dir);
}

#[test]
fn test_guest_reads_a_file_from_a_declared_filesystem_grant() {
    let wasm = wat::parse_str(READ_CONFIG_WAT).expect("read-config WAT should compile");
    let probe = Probe::new(&wasm);
    let observed = probe
        .driver(&wasm)
        .invoke(&probe.invocation())
        .expect("guest must read a declared workspace file");

    assert_eq!(
        observed, probe.config_bytes,
        "the guest must read `cutver.toml` through the granted directory, byte for byte"
    );
    drop(probe.dir);
}

/// Pins the ARGUMENT ORDER of the Extism mount, which the other tests cannot see: when a
/// grant's declared spelling and its canonical target are the same string, swapping host and
/// guest name changes nothing. A symlinked grant makes them differ, so the guest must still
/// report the declared name — the one the payload's `root_dir` prefixes.
#[cfg(unix)]
#[test]
fn test_grant_reached_through_a_symlink_keeps_its_declared_guest_name() {
    let wasm = wat::parse_str(PREOPEN_NAME_WAT).expect("preopen-name WAT should compile");
    let mut probe = Probe::new(&wasm);
    let declared = format!("{}/linked", probe.root);
    std::os::unix::fs::symlink(probe.root_path.as_path(), probe.root_path.join("linked"))
        .expect("symlink the workspace");
    probe.plugin.permissions.filesystem = vec![declared.clone()];

    let observed = probe
        .driver(&wasm)
        .invoke(&probe.invocation())
        .expect("guest must report its preopen name");

    assert_eq!(
        String::from_utf8_lossy(&observed),
        declared,
        "the guest name must stay the declared path even when the host opens a different directory"
    );
    drop(probe.dir);
}
