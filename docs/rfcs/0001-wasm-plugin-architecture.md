# RFC 0001: Cutver Microkernel & Unified Plugin Engine

> **Status**: accepted (decision axis)
> **Implementation**: complete (implementation axis) — every capability, driver
> and gate named below is in this tree, with `path:line` evidence inline.
> **Issue**: [#32](https://github.com/cutver/cutver/issues/32)

---

## 1. Context

`cutver` releases one repository from one declarative `cutver.toml`. Every
ecosystem format and external integration would otherwise be compiled into the
binary or reached through an untyped shell hook. The tree already models the
extension points as one closed set of four capability domains —
`manifest.v1`, `lifecycle.v1`, `changelog.v1`, `versioning.v1`
(`crates/cutver-pdk/src/types.rs:7-15`). The decision this RFC closes is how
third-party code is admitted through those domains without becoming core code.

## 2. Problem statement

The pipeline must call out to code it did not compile, under one fail-closed
contract, without turning `cutver` into an interpreter host that runs untrusted
code with the user's full authority.

- **Per-ecosystem drift.** Manifest editing is a `match` over kinds
  (`src/manifest/mod.rs:49`); every new format is another core arm.
- **Ordering.** `on_pre_bump` runs at `src/bump/exec/pipeline.rs:66`, before the
  `MutationTransaction` is created at `:81`; `on_post_bump` at `:85` and
  `on_post_release` at `:100` must observe, not mutate, the release.
- **Trust.** Third-party binary code must not inherit ambient filesystem,
  network and environment access by default.

## 3. Prior art

Researched before choosing alternatives. Versions are the ones observed on
2026-10-12; cutver pins the host at `extism = "1.30"` (`Cargo.toml:30`).

| Source | Version | Approach | Relevance |
| :--- | :--- | :--- | :--- |
| [Extism](https://github.com/extism/extism) | v1.41.0 | WASM framework for extensible software with host-controlled HTTP, memory and timers | The chosen host; it packages the sandbox and permission surface cutver needs |
| [Wasmtime](https://github.com/bytecodealliance/wasmtime) | v49.0.2 | Fast, secure runtime, AOT via Cranelift | The runtime Extism pulls in (`Cargo.toml:43-51`); justifies the security claim |
| [WASM Component Model](https://component-model.bytecodealliance.org/) | live spec, accessed 2026-10-12 | Typed, language-neutral interfaces (WIT) | The alternative cutver did not adopt; core modules pass JSON instead |
| [Terraform plugin protocol](https://developer.hashicorp.com/terraform/plugin/terraform-plugin-protocol) | Protocol v6 (gRPC/Protobuf); [go-plugin](https://github.com/hashicorp/go-plugin) v1.8.0 | Out-of-process providers over a versioned RPC | Precedent for cutver's process driver and versioned wire contract |
| [Envoy WASM filters](https://www.envoyproxy.io/docs/envoy/latest/intro/arch_overview/advanced/wasm) | Envoy v1.39.3; [proxy-wasm spec](https://github.com/proxy-wasm/spec) | Sandboxed in-proxy filters over a host ABI | Confirms a narrow host ABI is production-viable |
| [Figma plugins](https://www.figma.com/plugin-docs/how-plugins-run/) | docs, accessed 2026-10-12 | JS in a sandbox main thread plus a browser iframe, message-passing | An embedded-scripting model with a hard API boundary |
| [Shopify Functions](https://shopify.dev/docs/apps/build/functions) | docs, accessed 2026-10-12 | Merchant code compiled to WASM and run in a platform sandbox | Comparable WASM product surface |
| [mlua](https://crates.io/crates/mlua) 0.12.2 / [deno_core](https://crates.io/crates/deno_core) 0.412.0 | crate versions | Embed Lua or V8 in-process | The embedded-scripting alternative and its dependency cost |

## 4. Alternatives

| Option | Delivers / cannot express | Cost | Verdict |
| :--- | :--- | :--- | :--- |
| **Null case** — ship no plugin system; keep `[hooks]` and external subcommands only | Untyped shell hooks. Cannot express a manifest format, a version scheme, or a typed changelog renderer | No new module, no dependency | **rejected** — cannot make per-ecosystem formats adapters; reopen if every integration is expressible as a shell command with no structured result |
| **WASM sandbox via Extism** (chosen) | Untrusted code behind host-controlled network, filesystem and env allowlists (`src/plugin/driver/wasm.rs:116-134`); one JSON envelope | Extism/Wasmtime dependency behind the `plugins` feature; WASI surface to reason about | **accepted** |
| **Native process driver** | Scripts and `$PATH` binaries over the same envelope and closed capabilities (`src/plugin/driver/process.rs:91-110`) | Not sandboxed: runs with the caller's authority; timeout-bounded only | **accepted** — co-shipped second driver, not a rival |
| **Embedded scripting (Lua/Rhai)** | In-process scripting with no subprocess | New interpreter dependency in every build; host-embedding safety burden; no language-agnostic contract | **rejected** — cuts generality and adds a runtime; reopen if a built-in scripting surface is required |
| **Out-of-process subcommands only** | Process isolation without a worker library | No capability/operation contract; no sandbox for untrusted code; no digest gate | **rejected** — cannot host untrusted plugins; reopen if plugins are trusted by construction |
| **WASM Component Model** | Typed WIT interfaces instead of a JSON envelope | Wire-format migration under a pinned Extism 1.30 host; no current capability it unblocks | **dropped** — revisit if a second consumer needs typed interfaces or the host adopts components |

## 5. Recommendation and its cost

Adopt the **dual-runtime microkernel** — a closed capability contract with a
sandboxed WASM driver and a process driver — rather than a single runtime or an
embedded interpreter. Cost: two drivers to maintain, a feature-gated host
dependency, and a process driver that is *not* sandboxed; the sandbox guarantee
is WASM-only. Residual risk: the registry index is an external HTTPS base
(`src/plugin/registry/fetch.rs:18`) whose published content this RFC does not
verify; the process driver runs arbitrary host binaries with the user's
privileges.

## 6. Proposed shape (as proposed; implemented state shown)

The wire contract is unchanged from the proposal: one envelope carrying a
coupled `(capability, operation)` pair and a capability-specific payload
(`crates/cutver-pdk/src/invocation.rs:7-13`), constructed so an impossible
pairing cannot be expressed (`src/plugin/types.rs:91-124`). The domains are
closed (`src/plugin/manager.rs:62,81,108,131,169`):

| `capability` | `operation` |
| :--- | :--- |
| `manifest.v1` | `read`, `write` |
| `lifecycle.v1` | `on_pre_bump`, `on_post_bump`, `on_post_release` |
| `changelog.v1` | `render` |
| `versioning.v1` | `compute` |

Implementations, all present in this tree:

- **One driver trait, two drivers.** `PluginDriver` with `name`/`invoke`
  (`src/plugin/driver.rs:14-19`); `WasmDriver` exports a stable `invoke`
  (`src/plugin/driver/wasm.rs:16`) and `ProcessDriver` speaks JSON over
  stdin/stdout with a fail-closed timeout (`src/plugin/driver/process.rs:91-110`).
- **Feature gate.** `plugins` enables `dep:extism`, `dep:ureq`, `dep:sha2`
  (`Cargo.toml:40`); the WASM driver, install, manage and registry modules are
  gated (`src/plugin.rs:5-11`). The `plugin` subcommand is declared in every
  build but fails closed when the feature is off
  (`src/cli/runner/plugin.rs:41-52`); its flags live in
  `src/cli/args/plugin.rs:10-34`.
- **Two digest gates.** Install time verifies downloaded bytes against the
  registry `sha256` before the rename (`src/plugin/install/apply.rs:57`); load
  time re-verifies the cached file against `hash`
  (`src/plugin/driver/wasm.rs:93-96`), skipped only when `hash` is absent.
- **Registry index.** HTTPS `<name>.toml` entries, fully validated, failing
  closed when the declared name disagrees with the file stem
  (`src/plugin/registry/entry.rs:70,90-97`).
- **Manifest dispatch.** `PluginManifestEditor` routes `read`/`write` through
  the manager (`src/manifest/plugin.rs:25-56`), reachable from the kind dispatch
  at `src/manifest/mod.rs:49`.
- **Lifecycle dispatch order.** `on_pre_bump` may reject and abort before any
  write (`src/bump/exec/pipeline.rs:66`); `on_post_bump` fires after mutation,
  before staging (`:85`); `on_post_release` fires after the tag (`:100`).

## 7. Incremental slices

Slices are ordered and independently verifiable; each names its own check.

1. **Wire contract** — `cutver-pdk` types and the envelope; verified by the PDK
   round-trip tests (`crates/cutver-pdk/src/invocation.rs`).
2. **Driver trait and process driver** — verified by the focused driver tests
   (`src/plugin/driver/process.rs`).
3. **WASM driver and feature gate** — verified by the hermetic WAT fixture that
   proves the `invoke` export and an envelope round-trip end to end
   (`tests/wasm_plugin.rs:1-30`).
4. **Manager and capability resolution** — verified by manager resolution tests
   (`src/plugin/manager.rs:62-167`).
5. **Manifest, lifecycle, changelog and versioning adapters** — verified at the
   pipeline dispatch sites (`src/bump/exec/pipeline.rs:66,85,100`).
6. **Install, registry and digest gates** — verified by the install and registry
   integration tests (`src/plugin/install/apply.rs:57`;
   `src/plugin/registry/entry.rs:70`).

## 8. Non-goals and residual

- **No signing or publisher identity.** The digest gates prove integrity against
  a declared hash; they do not authenticate the publisher or the registry.
- **No sandbox for process plugins.** Containment is a WASM-only property.
- **Not verified here:** that a populated public registry exists at the default
  base, and the external publication state of `cutver-pdk`.
- **Not adopted:** the typed Component Model and in-process embedded scripting
  (verdicts in §4).

## 9. Decision emission

This RFC produces **ADR-0001 (Adopt the dual-runtime plugin microkernel with a
closed capability contract)**. Its record is [ADR-0001: Adopt the dual-runtime plugin microkernel](../adr/0001-dual-runtime-plugin-microkernel.md). The ADR's decision axis is `accepted`; its implementation axis is `complete`.
