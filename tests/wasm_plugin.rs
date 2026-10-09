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
        "changelog.v1",
        "render",
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

    let temp_dir = std::env::temp_dir();
    let wasm_file = temp_dir.join(format!("cutver_wasm_plugin_test_{}.wasm", std::process::id()));
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

    let _ = std::fs::remove_file(wasm_file);
}

#[test]
fn test_wasm_missing_invoke_export_is_detected() {
    let name = PluginName::new("no-invoke-wasm").unwrap();
    let capabilities = HashSet::from([Capability::ChangelogV1]);
    let driver = WasmDriver::new(name, MINIMAL_WASM, &Default::default(), Some(5), capabilities, None)
        .expect("minimal module should load");

    let invocation = PluginInvocation::new("changelog.v1", "render", &serde_json::json!({})).unwrap();

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
