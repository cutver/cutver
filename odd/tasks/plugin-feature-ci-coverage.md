# Feature: `plugins` feature coverage in CI (issue #169, slice 1)

Goal: make `plugins` a **first-class, tested build configuration**, so the wasm path stops being code
that CI has never compiled, and so Pillar VI.4's lean build is guarded by a job that actually runs
instead of by an assumption.

Issues: #169 (slice 1). Independent of the command surface in slice 3.

## Why this comes first

- `tests/wasm_plugin.rs` is `#![cfg(feature = "plugins")]` and `.github/workflows/ci.yml` passed no
  `--features`, so `src/plugin/driver/wasm.rs` (328 lines) and the whole Extism path had **never been
  compiled in CI**. The seven `#[cfg(feature = "plugins")]` sites under `src/` were equally uncovered.
- Everything in #169 that ships behind the feature inherits that blind spot.
- The release decision (enable `plugins` in the release workflows) cannot be evaluated before the
  feature is known to compile and pass on every target of the matrix.

## Decision recorded

Keep the flag; CI tests **both** configurations:

| Configuration | Job | Purpose |
| --- | --- | --- |
| feature off (default) | existing `test` job | Pillar VI.4 guard: the lean build still compiles, lints and passes its tests. |
| feature on | new `test-plugins` job | Coverage of the wasm path, on all three OS targets. |

The existing no-feature job *already* is the lean guard; it must not be replaced by the feature job,
only accompanied by it. Both carry a comment saying so, because a future "CI is slow" cleanup would
otherwise delete the guard silently.

## Tasks

- [x] 1. Measure the release binary with and without the feature from the same tree, and record both sizes and wall times as evidence.
- [x] 2. Add a `test-plugins` CI job (3-OS matrix: build, test, clippy with `--features plugins`) and document in the workflow why the no-feature job must stay.
- [x] 3. Validate the workflow change: YAML parses, step parity with the existing job, and the diff is reviewed.
- [x] 4. Close with one work-unit commit on a feature branch, recording its identity here as evidence.

## Evidence

### Measured sizes

Commit `d3a6849`, `rustc 1.99.0`, `x86_64-unknown-linux-gnu`, warm `target/` (no `cargo clean`).

| Configuration | Binary | `gzip -9` asset | Release build wall time |
| --- | --- | --- | --- |
| feature off | 4,279,832 B (4.08 MiB) | 1,772,963 B (1.69 MiB) | 1m18s (dependency crates cached) |
| feature on | 20,181,168 B (19.25 MiB) | 7,753,857 B (7.39 MiB) | 12m17s (cold dependency tree) |

Delta: **+15,901,336 B** on disk (**4.72×**) and **+5,980,894 B** compressed (**4.37×**). The bulk is
27 `wasmtime`/`cranelift` crates in `Cargo.lock`, not the Extism wrapper. The contract's "~4 MB"
figure understated this by a factor of four; these are the numbers the release decision should use.

### The feature configuration is green

- `cargo build --locked --workspace --all-targets --features plugins` → exit 0.
- `cargo test --locked --workspace --features plugins` → exit 0; 21 test binaries, all `ok`, including
  the previously never-executed `tests/wasm_plugin.rs`.
- `cargo clippy --locked --workspace --all-targets --features plugins -- -D warnings` → exit 0, zero
  warnings, none of them in `#[cfg(feature = "plugins")]` code. The new job does not arrive red.

### Workflow change validated

- `yq -e '.'` parses; no tab characters.
- Action pins identical between `test` and `test-plugins`; the run-command diff is exactly the
  `--features plugins` flag; `cargo fmt` is intentionally not duplicated (feature-independent).
- `matrix.target` in the existing job is inert — no step consumes it — so the new job omits it.

### Cache root correction (a correction to #169 as first published)

`plugin_cache_dir()` (`src/plugin/driver/wasm.rs:169`) tries `$HOME`, then `USERPROFILE`, then
silently falls back to `"."`. #169 was published claiming `$HOME` only; the body was corrected. The
real defect is the silent current-directory fallback, which is the Pillar III.4 shape rather than a
Windows gap.

## Open decisions, not this slice

- **Release flip (A).** The 4.37× compressed growth is the evidence that decision needed.
- **HTTP client (B).** Strengthened by this measurement: `ureq v3.4.2` is already in the `plugins`
  feature tree transitively through `extism 1.30`, and extism's HTTP host functions are load-bearing
  (`with_allowed_host` in `build_manifest`), so a direct dependency for the registry client costs no
  new compiled crate.
- **musl.** `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl` and the `rust:alpine`
  Containerfile stage are still unverified with the feature. Verify before either release workflow is
  flipped.
- **`WasmNotSupported` message.** Still tells a user of a downloaded lean binary to set
  `runtime = "process"`, which is not an available remedy for a `.wasm` artifact.

## Commit

- `18e514d` `ci: cover the plugins feature on every target OS` — branch `ci/plugins-feature-coverage`,
  2 files, +125 lines (`.github/workflows/ci.yml`, this document). Local only: nothing pushed.
