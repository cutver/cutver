# ADR-0001: Adopt the dual-runtime plugin microkernel with a closed capability contract

**Status**: accepted
**Date**: 2026-10-05
**Owner**: Row0902
**Review date**: 2027-04-05 (proposal)
**Implementation**: complete
**Supersedes**: none

## Context

`cutver` releases one repository from one declarative `cutver.toml`. Every
ecosystem format and external integration would otherwise be compiled into the
binary or reached through an untyped shell hook. Manifest editing is a `match`
over kinds (`src/manifest/mod.rs:49`), so each new format is another core arm.
Third-party binary code must not inherit ambient filesystem, network and
environment access by default. Lifecycle hooks must fire at defined points in
the pipeline: `on_pre_bump` before any mutation
(`src/bump/exec/pipeline.rs:66`). The open question is how third-party code is
admitted without becoming core code.

## Decision

We will adopt the dual-runtime plugin microkernel: one closed capability
contract with four domains — `manifest.v1`, `lifecycle.v1`, `changelog.v1`,
`versioning.v1` (`crates/cutver-pdk/src/types.rs:8-15`) — served by a sandboxed
WASM driver and a native process driver behind one shared trait
(`src/plugin/driver.rs:14-19`).

## Alternatives

| Option | Trade-off | Verdict |
| :--- | :--- | :--- |
| Null case: ship no plugin system, keep `[hooks]` and external subcommands | Shell hooks cannot express a manifest format, a version scheme, or a typed changelog result | rejected — reopen if every integration is expressible as a shell command with no structured result |
| WASM sandbox via Extism (chosen) | Untrusted code behind host-controlled allowlists (`src/plugin/driver/wasm.rs:115-134`); adds an Extism/Wasmtime dependency and a WASI surface behind the `plugins` feature | accepted |
| Native process driver | Scripts and `$PATH` binaries over the same envelope (`src/plugin/driver/process.rs:91-110`); not sandboxed, runs with the caller's authority, timeout-bounded only | accepted — co-shipped second driver, explicitly not a rival |
| Embedded scripting (Lua/Rhai) | In-process scripting with no subprocess; adds an interpreter dependency to every build, a host-embedding safety burden, and no language-agnostic contract | rejected — reopen if a built-in scripting surface is required |
| Out-of-process subcommands only | Process isolation without a worker library; no capability/operation contract, no sandbox for untrusted code, no digest gate | rejected — reopen if plugins are trusted by construction |
| WASM Component Model | Typed WIT interfaces instead of a JSON envelope; a wire-format migration under a pinned Extism 1.30 host with no current capability it unblocks | dropped — revisit if a consumer needs typed interfaces or the host adopts components |

## Consequences

- Easier: new ecosystems become plugins rather than core arms, and untrusted
  code is confined by host-controlled allowlists
  (`src/plugin/driver/wasm.rs:115-134`).
- Downside: the WASM runtime, and the install/manage/registry modules, sit
  behind the `plugins` Cargo feature (`Cargo.toml:40`, `src/plugin.rs:5-11`),
  so the default lean build ships no plugin surface at all
  (`docs/reference/plugins.md:40`).
- Downside: the install-time digest gate depends on the registry index
  (`src/plugin/registry/fetch.rs:18`) and on TLS for that index.
- Downside: the loader's digest re-check is conditional on the entry declaring
  `hash`; `verify_sha256` returns `Ok(())` for `None`
  (`src/plugin/driver/wasm.rs:93-96`).
- Downside: the process driver spawns a host process
  (`src/plugin/driver/process.rs:91-110`) with no permissions configuration —
  sandbox permissions are a WASM-only concept (`src/plugin/types.rs:158`), a
  real asymmetry with the WASM route.

## Implementation

Implementation axis: `complete`.

- Four capabilities: `crates/cutver-pdk/src/types.rs:8-15`.
- One driver trait, WASM and process drivers: `src/plugin/driver.rs:1-20`,
  `src/plugin/driver/wasm.rs:16`, `src/plugin/driver/process.rs:91`.
- Registry index: `src/plugin/registry/fetch.rs:18`; entry validation,
  `src/plugin/registry/entry.rs:70`.
- Two digest gates: install `src/plugin/install/apply.rs:57`; load
  `src/plugin/driver/wasm.rs:178-179`.
- `plugins` Cargo feature: `Cargo.toml:40`; gated modules,
  `src/plugin.rs:5-11`.

## Reopen condition

Each rejected or dropped alternative carries its reopen condition in the table
above. This decision itself is revisited if one driver contract cannot be
maintained behind a single trait, or if process plugins must be sandboxed.

## Traceability

- [RFC 0001: Cutver Microkernel & Unified Plugin Engine](../rfcs/0001-wasm-plugin-architecture.md) — the proposal this record accepts.
- [Design Document, Plugin Engine](../internals/design.md#plugin-engine) — the same engine at design level.
- Decision recorded in commit `d4812e4` (2026-10-05); no separate RFC-acceptance commit is recoverable from git history. No ADR-specific issue or pull request is recorded, so none is cited.
