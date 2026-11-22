//! The `(host directory, guest name)` pair `filesystem_mounts` produces.
//!
//! These live here rather than inside `wasm.rs` because that file sits under the 400-line ratchet in
//! `tests/source_line_budget.rs`, whose table "can shrink but never grow". The symlink case that
//! caught the regression lives in `tests/wasm_plugin.rs` and needs a real filesystem; these two need
//! nothing but a `PermissionsConfig`, which is why they run on every host.

use super::*;

/// #268 S2: "with the mount policy from #251, the guest mount name is the **declared** path while the
/// sandbox opens the **canonical** one."
///
/// `filesystem` is the authority for which grants exist and what the guest sees; `filesystem_grants`
/// is only the recorded canonical host per entry, so a stale record must never stand in for the entry
/// the caller actually supplied. That is the shape of a driver built from a hand-made
/// `PermissionsConfig` — a driver test, or a plugin manager handed a raw `PluginConfig` — and it is
/// what made the symlink test in `tests/wasm_plugin.rs` fail on ubuntu and macOS while every Windows
/// leg stayed green: that test overrides `permissions.filesystem` after discovery had already filled
/// the records, so the old branch mounted the record instead of the entry.
#[test]
fn a_stale_grant_record_does_not_replace_the_entry_the_caller_supplied() {
    let permissions = PermissionsConfig {
        filesystem: vec!["/guest/asked".into()],
        filesystem_grants: vec![crate::plugin::types::FilesystemGrant {
            declared: ".".into(),
            resolved: "/guest/other".into(),
            canonical: Some("/host/other".into()),
        }],
        ..PermissionsConfig::default()
    };

    assert_eq!(
        filesystem_mounts(&permissions),
        vec![("/guest/asked".to_string(), "/guest/asked".to_string())],
        "the caller's entry decides the guest name; a record it does not match is not its grant"
    );
}

/// The invariant #251 chose and #268 documents, pinned without a symlink so it runs on every host: the
/// host opens the recorded canonical directory, and the guest keeps the name the lifecycle payload
/// advertises as `root_dir`.
#[test]
fn the_recorded_canonical_host_mounts_under_the_guest_name() {
    let permissions = PermissionsConfig {
        filesystem: vec!["/guest/dir".into()],
        filesystem_grants: vec![crate::plugin::types::FilesystemGrant {
            declared: "dir".into(),
            resolved: "/guest/dir".into(),
            canonical: Some("/host/real-dir".into()),
        }],
        ..PermissionsConfig::default()
    };

    assert_eq!(
        filesystem_mounts(&permissions),
        vec![("/host/real-dir".to_string(), "/guest/dir".to_string())],
        "the sandbox opens the recorded canonical host; the guest keeps its own name"
    );
}
