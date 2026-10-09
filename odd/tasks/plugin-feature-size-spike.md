# Spike: how much of the `plugins` feature binary is reducible?

Goal: replace hope with numbers. The release decision (#169, decision A) is a single artifact with the
feature on (a multiple of every download) versus two artefacts, so it matters how much of the 19.25 MiB
is actually irreducible.

Baseline, measured on `d3a6849` (warm `target/`, `rustc 1.99.0`, `x86_64-unknown-linux-gnu`):

| Configuration | Binary | `gzip -9` |
| --- | --- | --- |
| feature off (lean) | 4,279,832 B (4.08 MiB) | 1,772,963 B (1.69 MB) |
| feature on | 20,181,168 B (19.25 MiB) | 7,753,857 B (7.39 MB) |

## Outcome

**One lever exists and it is free of feature semantics: `opt-level = "z"`.** It cuts the feature-on
binary by 33% compressed and the lean binary by 26%, and because `opt-level` is a property of the whole
`release` profile, **both artefacts benefit independently of the plugin decision**.

| Variant | Binary | `gzip -9` | vs feature-on baseline (disk / gzip) | Validity |
| --- | --- | --- | --- | --- |
| feature on (baseline) | 20,181,168 B (19.25 MiB) | 7,753,857 B (7.39 MB) | 100% / 100% | suite green |
| lean (feature off) | 4,279,832 B (4.08 MiB) | 1,772,963 B (1.69 MB) | 21.2% / 22.9% | suite green |
| **C: `opt-level = "z"`** | **12,456,960 B (11.88 MiB)** | **5,214,805 B (4.97 MB)** | **61.7% / 67.3%** | wasm suite green; binary smoke-tested |
| **lean + `opt-level = "z"`** | **3,178,688 B (3.03 MiB)** | **1,313,343 B (1.25 MB)** | **15.8% / 16.9%** | build green |
| A: extism without `wasmtime/default` | — | — | — | **fails to compile** |
| B: A + `opt-level = "z"` | — | — | — | not run: conditional on A |

### A is not available: the runtime cannot be reduced from this manifest

Variant A does not build. With `default-features = false` on extism,
`cargo build --offline --release --features plugins` fails with exit 101 and **42 errors inside
`extism-1.30.0` itself**:

```
error[E0277]: `?` couldn't convert the error: `wasmtime::Error: std::error::Error` is not satisfied
  --> extism-1.30.0/src/sdk.rs:52:28
  = note: required for `extism_convert::Error` to implement `From<wasmtime::Error>`
error: could not compile `extism` (lib) due to 42 previous errors
```

Extism's own source requires the feature set that `wasmtime/default` provides, so the option-cut cannot
come from cutver's manifest. **The `wasmtime` default-feature leak is real but not a lever**: closing it
would need a patch to extism or a fork, which is a different proposition from a profile line and out of
scope here. B dies with A. The mass identified below therefore stays in the graph, and the attribution
is a description of what a wasm plugin host carries, not of something removable by declaration.

## Recon

- `extism 1.30` is declared as `extism = { version = "1.30", optional = true }`, so its defaults are on:
  `default = ["http", "register-http", "register-filesystem", "wasmtime-default-features"]`, where
  `wasmtime-default-features = ["wasmtime/default"]`. Extism opts the `wasmtime` crate into its own
  default features and cutver never overrides that — and, per the outcome above, cannot.
- rlib attribution (a proxy for static footprint, not the linked size):

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
  `wit_parser` 14 MB (component model). `cranelift` is the JIT and cannot go: no JIT, no plugin.
- What cutver itself asks of extism: `with_wasi(true)`, `with_allowed_host` (`permissions.network`),
  `with_allowed_path`, `with_config_key`, `with_timeout`. Nothing requires the text format, the
  component model or the interpreter — but A shows that requesting less is not something this manifest
  can express.

## Method

One release build per variant. Validity is established by the suite that proves a wasm module still
executes (`cargo test --release --features plugins --test wasm_plugin`), and for the size profile also by
executing the built binary against this repository. Variant D (dropping `http`/`register-http`) was
dropped from scope as a capability removal we are not going to make.

## Tasks

- [x] 1. Recon: extism feature inventory and rlib attribution.
- [x] 2. Measure the variants, with the wasm test suite as validity evidence.
- [x] 3. Attribute the measured binary to a variant, so the number is evidence rather than an orphan.
- [x] 4. Measure the lean build with `opt-level = "z"`, since the profile key is global.
- [x] 5. Record the outcome and the recommendation.

## Evidence

### Smoke test of the size-profile binary

`opt-level = "z"` changes only the shipped artifact, and CI never executes the shipped artifact because
it tests in debug. So the preserved binary was run against this repository on `main` with a clean tree:

| Command | Result | Wall time |
| --- | --- | --- |
| `--version` / `--help` | exit 0 | 0.01 s |
| `doctor` | exit 0 (`cutver.toml` valid, 1 manifest, 1 preflight step) | 0.00-0.01 s |
| `changelog latest` | renders the expected entry | 0.01 s |
| `bump --dry-run --skip-preflight tests` | exit 0, prints the whole release plan | 0.91-1.30 s (3 runs) |

Two observations from that table. The heaviest command measured is ~1 s, which bounds the exposure of
any optimization-level regression to hundreds of milliseconds on a command a human runs a few times per
release. And `bump` refuses to run outside `main` and with a dirty tree, both correctly — the first two
attempts were blocked by those guards, not by the binary.

### Verification runs and the infrastructure failure

Two verification runs died before reporting, and neither was a property of the spike: a single bash call
that keeps running stalls the whole background task, and the harness kills it around 30 minutes. The
command responsible was `cargo test --release --workspace --features plugins` — `lto = "fat"` applies to
every test target, so ~20 integration binaries each pay a full fat-LTO link and it never finishes.

The protocol that worked, recorded because it is reusable: bound every long command with
`timeout 1500` (exit 124 becomes a reportable result), scope validity checks to `--test wasm_plugin` and
never `--workspace`, append measurements to a file outside the repository after each step
(`$HOME/.cache/cutver-spike/progress.txt`), copy artefacts to `$HOME/.cache/cutver-spike/` because `/tmp`
is a tmpfs that does not survive the task, and state explicitly that a failing variant is a result to
report rather than a reason to abort.

Two further traps cost a run each. A feature change requires Cargo to rewrite `Cargo.lock`, so `--locked`
aborts the build before the real error appears — A looked like a lock problem when it was 42 compile
errors. And the verification environment rejects `git restore` as a destructive command, so manifest
experiments are applied and reverted by the operator instead of by the agent.

## Recommendation

Adopt `opt-level = "z"` in `[profile.release]` as its own reviewable unit. One line, no dependency change,
no workflow change, and both artefacts shrink.

Two things belong with that change rather than after it:

1. **State the trade-off with a number.** The delta against `opt-level = 3` is unmeasured, because the
   previous-profile binary no longer exists and re-deriving it costs a full rebuild. The ~1 s heavy
   command bounds it, but a before/after timing of `bump --dry-run` is cheap and should be in the change.
2. **Give the shipped artefact a behavioural gate.** CI tests in debug, so the size profile — and any
   future profile or dependency change — reaches users without ever being executed. A `--version` plus
   `doctor` smoke step in the release workflow would close that, and it is a gap this spike found rather
   than created.

What the spike does **not** decide is #169's decision A. It improves both sides of it: lean would ship at
1.25 MB compressed and feature-on at 4.97 MB, still roughly 4× apart. One artifact with plugins for
everyone, or a lean default plus a plugin-enabled variant, remains a product choice.
