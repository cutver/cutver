# RFC 0001: Cutver Microkernel & Unified Plugin Engine

> **Status**: Accepted — core and adapters shipped in `v0.11.0` / `v0.12.0`
> **Issue**: [#32](https://github.com/cutver/cutver/issues/32)
> **Implementation Milestones**: `v0.11.0` (Microkernel & Drivers) · `v0.12.0` (Capability Adapters) · `v0.13.0` (Plugin Ecosystem)

---

## 1. Summary

Transform Cutver into an extensible release microkernel by introducing the **Unified Plugin Engine (UPE)**. Cutver decouples manifest parsing, lifecycle hooks, changelog transformations, and versioning strategies from the core binary through a unified dual-runtime architecture: sandboxed WebAssembly (via Extism/Wasmtime) and native system processes (executables, scripts, and `$PATH` subcommands).

---

## 2. Motivation & Architectural Fit

Cutver guarantees surgical, atomic release mutations across diverse package ecosystems. Decoupling language-specific formats and external integrations into a microkernel yields key architectural benefits:
- **Ecosystem Decoupling**: Manifests (`Cargo.toml`, `package.json`, `pyproject.toml`) become built-in adapters adhering to the same contract as third-party formats (Helm `Chart.yaml`, Zig, `.csproj`).
- **Zero Lock-in Dual Runtime**: Teams choose between sandboxed WebAssembly (safe, portable, zero-dependency) or native executables/scripts without rewriting workflows.
- **Fail-Closed Safety**: In accordance with [Cutver Architectural Contract](../../CONTRACT.md) (Pillar I), plugin compute actions run in-memory during preflight (`on_pre_bump`); failure aborts the pipeline before a single byte touches disk. Any post-mutation failure triggers the `MutationTransaction` RAII drop-guard rollback.

---

## 3. Unified IPC Protocol & Dual Drivers

Both WASM modules and native processes communicate through referentially transparent JSON DTOs passed across a shared `PluginDriver` abstraction:

```text
┌────────────────────────────────────────────────────────┐
│                   Cutver Microkernel                   │
│         (Pipeline, Transaction Guard, Rollback)        │
└───────────────────────────┬────────────────────────────┘
                            │
               ┌────────────▼────────────┐
               │    PluginDriver (Trait) │
               └──────┬────────────────┬─┘
                      │                │
          ┌───────────▼────────┐  ┌────▼──────────────┐
          │    WasmDriver      │  │   ProcessDriver   │
          │ (Extism / Sandbox) │  │  (Native Subproc) │
          └────────────────────┘  └───────────────────┘
```

- **`WasmDriver` (Extism)**: Executes guest WASM modules with capability-based isolation (explicit network allowlists, directory preopens, and environment mappings). Gated behind the `plugins` Cargo feature flag.
- **`ProcessDriver`**: Spawns system binaries or scripts, transmitting the invocation over `stdin` and capturing the response DTO from `stdout` with fail-closed timeout guards.

---

## 4. Invocation Contract

Every call, in both runtimes, uses one envelope:

```json
{
  "capability": "changelog.v1",
  "operation": "render",
  "payload": { "...": "capability-specific request DTO" }
}
```

`capability` and `operation` are a fixed pair; the host models them as a single `PluginCall`, so an impossible combination cannot be expressed. The capability and operation domains are closed:

| `capability` | `operation` | Request DTO | Response DTO |
| --- | --- | --- | --- |
| `manifest.v1` | `read` | `ManifestReadRequest` | `ManifestReadResponse` |
| `manifest.v1` | `write` | `ManifestWriteRequest` | `ManifestWriteResponse` |
| `lifecycle.v1` | `on_pre_bump` | `PreBumpPayload` | `PreBumpResponse` |
| `lifecycle.v1` | `on_post_bump` | `PostBumpPayload` | ignored |
| `lifecycle.v1` | `on_post_release` | `PostReleasePayload` | ignored |
| `changelog.v1` | `render` | `ChangelogRenderRequest` | `ChangelogRenderResponse` |
| `versioning.v1` | `compute` | `VersioningComputeRequest` | `VersioningComputeResponse` |

### WebAssembly guests

A WASM plugin exports exactly one function, `invoke`. It takes no parameters, receives the envelope as the Extism input, and returns an `i32` exit code where `0` is success and any non-zero value is an error.

> **Gotcha**: Extism hosts buffers in kernel memory, so a guest cannot return data by pointing at its own linear memory. Write the response through `alloc` + `store_u8` (or `input_load_u8` to copy the input) and then `output_set`. A static data segment in guest memory is invisible to the host and produces correct-length, all-zero output.

Language PDKs hide this. The Extism Rust PDK allocates and writes on your behalf:

```rust
use extism_pdk::*;

#[plugin_fn]
pub fn invoke(input: String) -> FnResult<String> {
    let request: serde_json::Value = serde_json::from_str(&input)?;
    // read request["operation"], build the response DTO
    Ok(serde_json::json!({ "body": "## What's Changed\n" }).to_string())
}
```

### Native process plugins

The envelope is written to the child's `stdin` as a single JSON document; the child writes the response DTO to `stdout`. Execution is bounded by `timeout_seconds` and fails closed when it is exceeded.

---

## 5. Decoupled Capability Domains

Plugins declare one or more of four canonical capabilities. The operation names in §4 are the wire contract.

1. **`manifest.v1`**: Arbitrary manifest reading and updating (`read`, `write`).
2. **`lifecycle.v1`**: Transactional lifecycle hooks (`on_pre_bump`, `on_post_bump`, `on_post_release`).
3. **`changelog.v1`**: Custom release note formatting, LLM-assisted summaries, and issue tracker grouping (`render`).
4. **`versioning.v1`**: Alternative version computation schemes (CalVer, commit hashes, or custom release trains) (`compute`).

---

## 6. Configuration Schema

Plugins are configured declaratively in `cutver.toml`:

```toml
# 1. Sandboxed WebAssembly Plugin
[plugins.slack]
runtime = "wasm"
source = "https://github.com/cutver/plugin-slack/releases/download/v1.0.0/slack.wasm"
hash = "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
capabilities = ["lifecycle.v1"]
events = ["on_post_release"]
permissions.network = ["hooks.slack.com"]
permissions.env = ["SLACK_WEBHOOK_URL"]

# 2. Native Process / Script Plugin
[plugins.helm-adapter]
runtime = "process"
command = "./scripts/helm-version-adapter.sh"
capabilities = ["manifest.v1"]
manifest_match = ["Chart.yaml"]
timeout_seconds = 10
```

---

## 7. Implementation Strategy

- **Cargo Feature Flag**: The Extism WASM engine is gated behind `features = ["plugins"]`, ensuring zero binary size inflation (~4 MB) for minimal installations.
- **AOT Cache**: Precompiles `.wasm` to native machine code in `~/.cache/cutver/plugins/`, maintaining sub-millisecond execution.
- **Companion Diagnostics**: Unhandled plugin exit codes and schema mismatches surface structured, actionable remediation guides.

### Status

Everything in §1–§6 is implemented and covered by tests, including a hermetic WebAssembly fixture that proves the `invoke` export path end to end. The remaining work is ecosystem tooling: a published plugin SDK, a registry, and the first official plugins.
