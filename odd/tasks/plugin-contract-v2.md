# Feature: Plugin Invocation Contract v2 (ABI envelope + real WASM fixture)

> **Status**: In Progress
> **Target Version**: v0.13.0
> **RFC**: [0001-wasm-plugin-architecture.md](../../docs/rfcs/0001-wasm-plugin-architecture.md)
> **Constitutional Law**: [CONTRACT.md](../../CONTRACT.md) (Pillars I, III, IV)
> **Blocks**: `cutver/plugins` — first official plugin (`github-releases`, capability `changelog.v1`)

---

## Problem

The WASM plugin path shipped in v0.11.0/v0.12.0 has never executed a real plugin, and two ABI defects
make the first official plugin unimplementable:

1. **Export naming.** The host resolves a plugin export by the raw capability string
   (`wasm.rs`: `plugin.call::<&[u8], Vec<u8>>(capability, payload)`), so it looks for an export named
   `changelog.v1`. The Extism Rust PDK `#[plugin_fn]` macro discards its attribute argument and emits
   `#[no_mangle] extern "C" fn <identifier>()`, so a dot is impossible. The documented ABI cannot be
   satisfied by the reference PDK.

2. **Missing operation discriminator.** `manifest.v1` multiplexes `ManifestReadRequest` and
   `ManifestWriteRequest`, and `lifecycle.v1` multiplexes `PreBumpPayload`, `PostBumpPayload` and
   `PostReleasePayload`, all over the same `invoke` call with no `operation` field. A plugin can only
   guess by sniffing JSON fields. `ProcessDriver` does not even receive the capability string, and the
   already-parsed `events = ["on_post_release"]` config is never transmitted.

3. **No coverage.** No `.wasm` artifact exists anywhere in the repository. The only WASM tests use
   `MINIMAL_WASM` (an 8-byte module header) to exercise SHA-256 verification and capability gating.

## Design

A single explicit envelope for every plugin invocation:

```rust
pub struct PluginInvocation {
    pub capability: String,          // "changelog.v1"
    pub operation: String,           // "render" | "read" | "write" | "on_pre_bump" | ...
    pub payload: serde_json::Value,  // unchanged capability DTO
}
```

- **WASM**: one stable export, `invoke`. The envelope is the Extism input; the response DTO is the
  Extism output.
- **Process**: the envelope is written to stdin, replacing the bare payload.
- Capability DTOs are unchanged on the wire inside `payload`.

---

## Acceptance Criteria
- [x] `PluginInvocation` envelope defined in `src/plugin/dto/invocation.rs`, exported from `src/plugin/dto.rs`, with serde round-trip tests.
- [x] `PluginDriver::invoke` takes the envelope; `WasmDriver` calls the single `invoke` export; `ProcessDriver` writes the envelope to stdin.
- [x] Every `PluginManager::dispatch_*` supplies an explicit `operation` for all four capabilities.
- [x] A real WASM fixture exists and an integration test proves the WASM path end-to-end (request in, rendered out).
- [ ] `ChangelogRenderRequest` carries `repository`, `compare_url`, `is_prerelease` and `contributors` so changelog plugins can render new-contributor and compare-link sections.
- [ ] RFC 0001 documents the invocation envelope, the single `invoke` export, and the operation names.
- [ ] Zero clippy warnings with and without `--features plugins`, canonical formatting, green suite.

---

## Task Breakdown

### Work unit 1 — ABI envelope (core)
- [x] Task 1: Add `PluginInvocation` in `src/plugin/dto/invocation.rs` + serde tests; re-export from `src/plugin/dto.rs`.
- [x] Task 2: Change `PluginDriver::invoke` to accept `&PluginInvocation`; update `WasmDriver` to a single `invoke` export and `ProcessDriver` to write the envelope to stdin.
- [x] Task 3: Add an `operation` argument to `dispatch_raw` and supply explicit operations in every `dispatch_*`.

### Work unit 2 — Real WASM fixture, end-to-end proof and false-coverage closure
- [x] Task 4: Add an inline WAT fixture compiled by the `wat` dev-dependency, exporting `invoke`, and prove the path end-to-end through `WasmDriver` and `PluginManager` in `tests/wasm_plugin.rs`.
- [x] Task 4b: Close the false-coverage gaps found by independent verification: rename the misnamed registration-only WASM test, make `MockEchoDriver` assert `(capability, operation)`, and add discriminating envelope guards to the three process-plugin integration scripts.

### Work unit 3 — Changelog context enrichment
- [ ] Task 5: Extend `ChangelogRenderRequest` with `repository`, `compare_url`, `is_prerelease` and `contributors`, mapped from `ReleaseContext`, with compat for absent fields.
- [ ] Task 6: Document the invocation contract and operation names in RFC 0001.

### Verification
- [ ] Task 7: Full verification suite.

---

## Evidence & Verification
- **Branch**: `feat/plugin-contract-v2`
- **Commit**: work-unit commit on this branch, subject `feat(plugin)!: replace capability-as-export ABI with an explicit invocation envelope` (15 files, +520/-42)
- **Validation**:
  - `cargo fmt -- --check`: ok
  - `cargo clippy --all-targets --all-features -- -D warnings`: ok, 0 warnings
  - `cargo test`: ok (lib 332 passed; new WASM suite compiles to 0 tests without the feature)
  - `cargo test --features plugins`: ok (lib 337 passed; `tests/wasm_plugin.rs` 3 passed, 0 failed)
  - `git diff -- Cargo.lock`: one added line under the `cutver` package dependency list; **no new `[[package]]` block** (confirmed: 328 packages before and after)
  - Independent verification (3 rounds): all four gates reproduced each time; the pre-existing WASM path was found completely unexecuted by the suite, and two residual test defects found after the fixture landed were fixed and proven by reproducing the shell guards against correct, swapped and absent envelopes

---

## Notes
- Breaking change to the plugin ABI is acceptable: no third-party plugin is published.
- The process driver also gains the capability string, which it previously could not observe.

### The `wat` dev-dependency is not a new crate

`wat` is already in `Cargo.lock` as a real dependency of `wasmtime` and `wasm-compose` (both pulled in by
`extism`), so adding it as a dev-dependency introduces **zero** new crates to the resolved graph. Dev-dependencies
never ship in binaries, so `cargo build` without `--features plugins` remains dependency-free (Pillar VI.4).

### Gotcha for future fixture authors: Extism reads output from kernel memory

A WAT guest **cannot** return data by exporting its own linear memory and pointing `output_set` at it. In Extism 1.30
the host reads call output from the Extism kernel's own memory (`extism:host/env`), and importing that memory as a
non-function export is explicitly rejected (`linked modules cannot access non-function exports of extism kernel`).
A static `(data ...)` segment in the guest's memory is invisible to the host and the call returns the correct
**length** of zeroed bytes — a silent, confusing failure.

The guest must allocate and write through the kernel: `alloc` -> `store_u8` (or `input_load_u8` to copy input) ->
`output_set`. The first version of the fixtures got this wrong and failed RED with all-zero output.

Rust `extism-pdk` plugin authors never see this: the PDK handles allocation and output internally.
