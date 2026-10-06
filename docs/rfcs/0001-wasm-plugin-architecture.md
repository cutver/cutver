# RFC 0001: Extensible WebAssembly (WASM) Plugin Architecture

> **Status**: Proposed / Under Review  
> **Issue**: [#32](https://github.com/cutver/cutver/issues/32)  
> **Target Version**: `v0.11.0` (Core Engine) / `v0.12.0` (Ecosystem Hooks)

---

## 1. Summary

Introduce a sandboxed WebAssembly (WASM) plugin architecture for Cutver using the Extism runtime engine (powered by Wasmtime). This enables teams to extend release lifecycles, manifest parsing, changelog formatting, and notification dispatches without forking the Rust core or compromising mutation safety.

---

## 2. Motivation & Architectural Fit

Cutver guarantees surgical, atomic release mutations across diverse package ecosystems. However, production workflows demand custom extensions:
- **Ecosystem Manifests**: Parsing proprietary or niche configuration formats.
- **Pre-Release Compliance**: Validating security scans, git branches, or Jira/Linear issue references before disk mutations.
- **Post-Release Notifications**: Dispatching authenticated webhooks (Slack, Discord, internal web services).

Adhering to [Cutver Architectural Contract](../../CONTRACT.md):
- **Functional Core Purity**: Plugin computations (parsing, validation) run in-memory within an isolated sandbox.
- **Fail-Closed Safety**: A plugin failing during preflight aborts the lifecycle before any file is touched.
- **Zero Ambient Access**: No unconstrained filesystem or network access; capabilities must be explicitly declared in `cutver.toml`.

---

## 3. Technology Selection: Extism over Raw Wasmtime

Extism is chosen as Cutver's WebAssembly host engine:
1. **Developer Experience Across Languages**: Extism provides official PDKs (Plugin Development Kits) for Rust, Go/TinyGo, TypeScript/JavaScript, Zig, and C. Contributors write standard code without authoring low-level WIT contracts.
2. **Powered by Wasmtime**: Under the hood, Extism embeds Wasmtime and the Cranelift JIT compiler, retaining memory safety, isolation, and near-native speed.
3. **Declarative Capabilities**: Extism manages filesystem mappings, HTTP domain allowlists, and environment variables directly through configuration manifests.
4. **Distribution Flexibility**: Plugins can be loaded from local files or remote URLs with built-in SHA-256 integrity validation.

---

## 4. Configuration Schema

Plugins are declared under the `[plugins]` table in `cutver.toml`:

```toml
[plugins.slack]
source = "https://github.com/cutver/plugin-slack/releases/download/v1.0.0/slack.wasm"
hash = "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
timeout_ms = 5000
permissions.network = ["hooks.slack.com"]
permissions.env = ["SLACK_WEBHOOK_URL"]

[plugins.custom-manifest]
path = "./plugins/custom_manifest.wasm"
permissions.filesystem = ["."]
```

---

## 5. Lifecycle Hooks Contract

Plugins expose exported functions conforming to Cutver's two-phase lifecycle:

```text
[cutver bump / release]
       │
       ▼
 1. on_pre_bump ─────────────► (Pure compute/validation; failure aborts before disk I/O)
       │
       ▼
 [Atomic Manifest & Changelog Writes]
       │
       ▼
 2. on_post_bump ────────────► (Post-mutation actions; e.g. lockfile regen)
       │
       ▼
 [Git Commit & Git Tag]
       │
       ▼
 3. on_post_release ─────────► (External side-effects; e.g. webhooks, telemetry)
```

Communication across the host-guest boundary uses structured JSON DTOs passed via Extism memory buffers, maintaining referential transparency and strict domain type-checking.

---

## 6. Implementation Strategy

To protect binary footprint and startup speed:
- **Cargo Feature Flag**: The Extism runtime is gated behind the `plugins` feature (`cargo build --features plugins`). Lean CLI builds remain unaffected (~4 MB).
- **AOT Cache**: Precompiles `.wasm` to native machine code in `~/.cache/cutver/plugins/`, reducing subsequent execution latency to sub-millisecond ranges.
- **Fail-Closed Diagnostics**: Any plugin error surfaces actionable diagnostics adhering to Companion Tone standards.
