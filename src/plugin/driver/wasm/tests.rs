//! The `WasmDriver` unit tests: digest verification, the capability gate, construction, and artifact
//! loading.
//!
//! These live here rather than inside `wasm.rs` because that file sits under the 400-line ratchet in
//! `tests/source_line_budget.rs`, whose table "can shrink but never grow". They read private items of
//! the parent, which `use super::*` reaches without re-exporting anything.

use super::*;
use crate::plugin::types::PluginCall;

const MINIMAL_WASM: &[u8] = &[0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];

#[test]
fn test_integrity_verification_success_plain_and_prefix() {
    let name = PluginName::new("test-plugin").unwrap();
    let mut hasher = Sha256::new();
    hasher.update(MINIMAL_WASM);
    let hex = format!("{:x}", hasher.finalize());

    assert!(verify_sha256(&name, MINIMAL_WASM, Some(&hex)).is_ok());
    let prefixed = format!("sha256:{hex}");
    assert!(verify_sha256(&name, MINIMAL_WASM, Some(&prefixed)).is_ok());
}

#[test]
fn test_integrity_mismatch_returns_error() {
    let name = PluginName::new("test-plugin").unwrap();
    let wrong_hash = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let err = verify_sha256(&name, MINIMAL_WASM, Some(wrong_hash)).unwrap_err();

    match err {
        PluginError::IntegrityMismatch {
            name: err_name,
            expected,
            actual,
        } => {
            assert_eq!(err_name, name);
            assert_eq!(expected, wrong_hash);
            assert!(actual.starts_with("sha256:"));
        }
        other => panic!("expected IntegrityMismatch, got {other:?}"),
    }
}

#[test]
fn test_capability_check_fails_when_undeclared() {
    let name = PluginName::new("test-wasm").unwrap();
    let perms = PermissionsConfig::default();
    let caps = HashSet::from([Capability::ManifestV1]);

    let driver = WasmDriver::new(name.clone(), MINIMAL_WASM, &perms, None, caps, None).unwrap();

    let invocation = PluginInvocation::new(PluginCall::PreBump, &serde_json::json!({})).unwrap();
    let err = driver.invoke(&invocation).unwrap_err();
    match err {
        PluginError::UnsupportedCapability {
            name: err_name,
            capability,
        } => {
            assert_eq!(err_name, name);
            assert_eq!(capability, "lifecycle.v1");
        }
        other => panic!("expected UnsupportedCapability, got {other:?}"),
    }
}

#[test]
fn test_wasm_driver_creation_with_valid_minimal_wasm() {
    let name = PluginName::new("valid-driver").unwrap();
    let perms = PermissionsConfig {
        network: vec!["api.example.com".to_string()],
        env: vec!["PATH".to_string()],
        filesystem: vec![".".to_string()],
        ..Default::default()
    };
    let caps = HashSet::from([Capability::ManifestV1]);

    let driver = WasmDriver::new(name.clone(), MINIMAL_WASM, &perms, Some(5), caps, None);
    assert!(driver.is_ok());
    let driver = driver.unwrap();
    assert_eq!(driver.name(), &name);
    assert!(driver.capabilities().contains(&Capability::ManifestV1));
}

#[test]
fn test_load_wasm_bytes_local_file() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("cutver_test_minimal.wasm");
    fs::write(&file_path, MINIMAL_WASM).unwrap();

    let name = PluginName::new("test-load").unwrap();
    let loaded = load_wasm_bytes(&name, file_path.to_str().unwrap(), None).unwrap();
    assert_eq!(loaded, MINIMAL_WASM);
}
