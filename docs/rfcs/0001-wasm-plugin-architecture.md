# RFC 0001: Cutver Microkernel & Unified Plugin Engine

> **Status**: Proposed / Under Review  
> **Issue**: [#32](https://github.com/cutver/cutver/issues/32)  
> **Target Version**: `v0.11.0` (Core Microkernel & Drivers) / `v0.12.0` (Ecosystem Adapters)

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
- **`ProcessDriver`**: Spawns system binaries or scripts, transmitting JSON payloads over `stdin` and capturing structured responses from `stdout` with fail-closed timeout guards.

---

## 4. Decoupled Capability Domains

Plugins declare and implement one or more of four canonical capabilities:

1. **`manifest.v1`**: Arbitrary manifest reading and updating (`read_version`, `write_version`).
2. **`lifecycle.v1`**: Transactional lifecycle hooks (`on_pre_bump`, `on_post_bump`, `on_post_release`).
3. **`changelog.v1`**: Custom release note formatting, LLM-assisted summaries, and issue tracker grouping.
4. **`versioning.v1`**: Alternative version computation schemes (CalVer, commit hashes, or custom release trains).

---

## 5. Configuration Schema

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

## 6. Implementation Strategy

- **Cargo Feature Flag**: The Extism WASM engine is gated behind `features = ["plugins"]`, ensuring zero binary size inflation (~4 MB) for minimal installations.
- **AOT Cache**: Precompiles `.wasm` to native machine code in `~/.cache/cutver/plugins/`, maintaining sub-millisecond execution.
- **Companion Diagnostics**: Unhandled plugin exit codes or schema mismatch errors surface structured, actionable remediation guides.
