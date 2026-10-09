# Spike: how much of the `plugins` feature binary is reducible?

Goal: replace hope with numbers. The release decision (#169, decision A) is single artifact with the
feature on (~4.4× every download) versus two artefacts, so it matters how much of the 19.25 MiB is
actually irreducible.

Baseline, measured on `d3a6849` (warm `target/`, `rustc 1.99.0`, `x86_64-unknown-linux-gnu`):

| Configuration | Binary | `gzip -9` |
| --- | --- | --- |
| feature off | 4,279,832 B (4.08 MiB) | 1,772,963 B (1.69 MB) |
| feature on | 20,181,168 B (19.25 MiB) | 7,753,857 B (7.39 MB) |

## Recon

- `extism 1.30` is declared as `extism = { version = "1.30", optional = true }`, so its defaults are on:
  `default = ["http", "register-http", "register-filesystem", "wasmtime-default-features"]`, and
  `wasmtime-default-features = ["wasmtime/default"]`. **Extism opts the `wasmtime` crate into its own
  default features and cutver never overrides that**, so the host runtime is built with everything
  wasmtime ships by default, not with what a plugin host needs.
- rlib attribution (proxy for static footprint, not the linked size):

  | Family | rlib bytes |
  | --- | --- |
  | cranelift | 94 MB |
  | wasmtime | 79 MB |
  | extism | 14 MB |
  | rustls | 9 MB |
  | ring | 6 MB |
  | ureq | 4 MB |

  Largest single artefacts: `cranelift_codegen` 43 MB, `wasmtime` 35 MB, `wasmparser` 25 MB,
  `cranelift_assembler_x64` 25 MB, `wasmtime_environ` 20 MB, `wast` 18 MB (text format),
  `zerocopy` 16 MB, `wasi_common` 15 MB, `pulley_interpreter` 15 MB (wasmtime's interpreter),
  `wit_parser` 14 MB (component model).
- What cutver itself asks of extism: `with_wasi(true)`, `with_allowed_host` (`permissions.network`),
  `with_allowed_path`, `with_config_key`, `with_timeout`. Nothing requires the text format, the
  component model or the interpreter.

## Variants

| # | Variant | Risk |
| --- | --- | --- |
| A | extism `default-features = false` + `["http", "register-http", "register-filesystem"]` | May fail to build or to execute wasm: extism may depend on wasmtime's defaults (cranelift, wasi). |
| B | A plus `CARGO_PROFILE_RELEASE_OPT_LEVEL=z` | The candidate configuration; no new semantic risk. |
| C | `opt-level=z` alone, extism restored | Isolates the profile lever from the feature lever. |
| D | A minus `http` and `register-http` | Informational: prices `permissions.network`. A capability removal, not a size tweak. |

Method: one release build per variant, then a validity check with
`cargo test --locked --release --workspace --features plugins` so the test binaries reuse the release
dependency graph instead of recompiling the whole tree in debug.

## Tasks

- [x] 1. Recon: extism feature inventory and rlib attribution. Evidence above.
- [ ] 2. Measure the variants, with the wasm test suite as validity evidence.
- [ ] 5. Measure the lean build with `opt-level = "z"` as well: the profile key is global, so the gain is not feature-specific.
- [ ] 3. Record the outcome and the resulting recommendation for the release decision.

## Evidence

### Partial (12:14) — attributed to variant C

The first verification run failed before reporting, leaving `target/release/cutver` at **12,456,960 B**
(**5,214,805 B** `gzip -9`) against a 20,181,168 / 7,753,857 baseline: **-38.3% on disk, -32.7%
compressed**. The resumed run attributed it and preserved it as `$HOME/.cache/cutver-spike/cutver-C`.

**It is variant C: `CARGO_PROFILE_RELEASE_OPT_LEVEL=z` alone, with the extism declaration untouched.**
The whole gain comes from the profile knob, not from dropping `wasmtime/default` — which matters,
because the profile lever carries no risk of breaking wasm execution while variant A does.

Two consequences to settle before this is a recommendation:

- `opt-level` is a property of the whole `release` profile, so it also shrinks the **lean** binary
  (4,279,832 B today). That variant was not in the original list and must be measured.
- `"z"` trades runtime speed for size. For a CLI that mostly spawns `git` and parses text the cost is
  probably small, but it is a trade-off to state, not to assume.

The repository state after the failure was intact: `git diff Cargo.toml` empty, no commits, only this
untracked document. Disk and inodes were ruled out as the cause (884 GB and 2% inodes free).

### Verification runs and the infrastructure failure

Two verification runs died before reporting, and neither was a defect in the spike: a single bash call that
keeps running stalls the whole background task, and the harness kills it around 30 minutes. The command
responsible was `cargo test --release --workspace --features plugins` — `lto = "fat"` applies to every
test target, so ~20 integration binaries each pay a full fat-LTO link and it never finishes.

The protocol for the third run, recorded because it is reusable: bound every long command with
`timeout 1500`, scope validity checks to `--test wasm_plugin`, append measurements to an append-only
file outside the repository after each step (`$HOME/.cache/cutver-spike/progress.txt`), copy artifacts
to `$HOME/.cache/cutver-spike/` because `/tmp` is a tmpfs that does not survive the task, and state
explicitly that a failing variant is a result to report rather than a reason to abort.

### Variant measurements

Validated by running the suite that proves a wasm module still executes
(`cargo test --locked --release --features plugins --test wasm_plugin`), and — for the size profile — by
executing the built binary against this repository.

| Variant | Binary | `gzip -9` | vs feature-on baseline (disk / gzip) | Validity |
| --- | --- | --- | --- | --- |
| feature on (baseline) | 20,181,168 B (19.25 MiB) | 7,753,857 B (7.39 MB) | 100% / 100% | suite green (earlier run) |
| **C: `opt-level = "z"`** | **12,456,960 B (11.88 MiB)** | **5,214,805 B (4.97 MB)** | **61.7% / 67.3%** | **wasm suite green (3 passed); binary smoke-tested** |
| lean (feature off) | 4,279,832 B (4.08 MiB) | 1,772,963 B (1.69 MB) | 21.2% / 22.9% | suite green (earlier run) |
| **lean + `opt-level = "z"`** | **3,178,688 B (3.03 MiB)** | **1,313,343 B (1.25 MB)** | **15.8% / 16.9%** | build green |
| A: extism without `wasmtime/default` | — | — | — | **FAILS TO COMPILE** |
| B: A + `opt-level = "z"` | — | — | — | not run: conditional on A |

### Verdict on the main hypothesis: A is not available

Variant A does not build. `cargo build --offline --release --features plugins` with
`default-features = false` on extism fails with **42 errors inside `extism-1.30.0` itself** (exit 101):

```
error[E0277]: `?` couldn't convert the error: `wasmtime::Error: std::error::Error` is not satisfied
  --> extism-1.30.0/src/sdk.rs:52:28
  = note: required for `extism_convert::Error` to implement `From<wasmtime::Error>`
error: could not compile `extism` (lib) due to 42 previous errors
```

Extism's own source requires the feature set that `wasmtime/default` provides, so the option-cut cannot
come from cutver's manifest. **The `wasmtime` default-feature leak is real but not a lever**: it would
need a patch to extism or a fork, which is a different proposition from a profile line and out of scope
for this spike. B dies with A, since it was A plus the size profile.

The suspected mass therefore stays in the graph, and the earlier attribution of it (`pulley_interpreter`,
`wast`, `wit_parser`, `wasi_common`) is a description of what a plugin host carries, not of something
removable by declaration.

### Smoke test of the size-profile binary

`opt-level = "z"` changes only the shipped artifact, and the shipped artifact is not executed by CI,
so the preserved binary was run against this repository: `--version`, `--help` and `doctor` all exit 0,
and `changelog latest` renders the expected entry. `bump --dry-run` was inconclusive for an unrelated
reason — it aborted with `git guard failed: working tree has uncommitted changes`, correctly, because
the retry's manifest edit was already applied. To be re-run on a clean tree before any profile change
is proposed.

The gain is not feature-specific: `opt-level` is a property of the whole `release` profile, so the lean
artifact drops from 4.08 to 3.03 MiB as well. The trade-off is runtime speed for size, which matters far
less for a CLI that mostly spawns `git` than the ratio suggests, but it is a trade-off to state.
