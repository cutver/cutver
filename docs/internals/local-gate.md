# The Local Gate and the Source Line Budget

```bash
scripts/check.sh
```

One command, one verdict: exit `0` means every step the host can run passed, anything else means the
step named in the last line of output failed. It runs from any directory, writes nothing outside a
private temporary directory it removes, and never touches the state of the shell it was called from.

## Why it exists

Three CI runs went red across three phases of one roadmap and **none of them was a logic defect**: an
assertion that only holds on one host, a test suite that only passed without the GitHub Actions
environment, and an `unused import` inside a `#![cfg(unix)]` file that a Windows host never
type-checks. Each cost a full CI cycle to learn something this gate shows in seconds.

So the gate is not a test runner with a nicer name. It is the workflow's order, the workflow's
environment, and an honest account of the steps this host cannot run.

## What it runs

| # | Step | Command it runs |
| :-- | :--- | :--- |
| 1 | toolchain | `rustc -V`, `cargo -V` |
| 2 | build | `cargo build --locked --workspace --all-targets` |
| 3 | test (default) | `cargo test --locked --workspace` |
| 4 | test (plugins) | `cargo test --locked --workspace --features plugins` |
| 5 | clippy (default) | `cargo clippy --locked --workspace --all-targets -- -D warnings` |
| 6 | clippy (plugins) | same, `--features plugins` |
| 7 | clippy (linux target) | same, default features, `--target x86_64-unknown-linux-gnu` |
| 8 | clippy (linux, plugins) | same, both, when a C cross-compiler exists |
| 9 | format | `cargo fmt --all --check` |

Steps run in that order and the gate stops at the first failure, naming it. Steps after a failure are
reported as not reached, never as passed.

**Step 7 is the one that catches what a Windows run cannot see.** `--target x86_64-unknown-linux-gnu`
makes rustc type-check every file the host's `#[cfg(unix)]` quietly excludes, which is exactly where
PR #332's dead import lived. Run it before pushing a change that touches a unix-gated test.

## The Actions environment is part of the test

Every GitHub Actions `run:` step has `GITHUB_ACTIONS`, `GITHUB_OUTPUT` and `GITHUB_STEP_SUMMARY` set.
A suite that only passes without them is lying, and that lie is what turned #332 red: an annotation
duplicated a message one test counts, while a CI-less local run stayed green.

The gate sets all three for its child processes, with the two sinks pointing at files inside its
temporary directory. The assignments belong to the child, so nothing leaks into your shell, and the
directory is removed on exit. Run the suites through the gate before believing a local green.

## Not run here

One step cannot run on every host, and pretending otherwise is the failure mode this gate exists to
prevent. The plugins build for the Linux target needs a C compiler for that target — `zstd-sys`,
reached through Extism, compiles C — so without `x86_64-linux-gnu-gcc` the gate says so:

```text
====> clippy (x86_64-unknown-linux-gnu, plugins feature)
      not run here: zstd-sys (via extism) compiles C for x86_64-unknown-linux-gnu and needs x86_64-linux-gnu-gcc, which is not on PATH here
```

The summary keeps three buckets, so a skipped step is visible instead of green:

```text
=============================== check summary ===============================
passed (8):
  - toolchain (rustc -V, cargo -V)
  - build (default features)
  - test (default features)
  - test (plugins feature)
  - clippy (default features)
  - clippy (plugins feature)
  - clippy (x86_64-unknown-linux-gnu, default features)
  - format (cargo fmt --all --check)
failed (0):
  - none
not run here (1):
  - clippy (x86_64-unknown-linux-gnu, plugins feature): zstd-sys (via extism) compiles C for x86_64-unknown-linux-gnu and needs x86_64-linux-gnu-gcc, which is not on PATH here
-----------------------------------------------------------------------------
gate: PASS (8 passed, 0 failed, 1 not run here)
note: 1 step(s) did not run here. A skipped step is not a pass; the reason is above.
=============================================================================
```

A skipped step is not a pass. On a Linux host with a native `cc` the same step runs; installing the
cross compiler, or adding `rustup target add x86_64-unknown-linux-gnu` for step 7, is what closes the
gap.

## What the gate does not cover

It mirrors the parts of the workflow a local run can mirror, and says nothing about the rest:
`cargo publish --dry-run`, the release and container workflows, the macOS runner's plugin cache
resolution, and the documentation ceiling (which a test in the suite enforces, so `cargo test` catches
it). Those stay CI's job. The gate's job is that CI never tells you something a local run could have.

## The source line budget ratchet

`CONTRACT.md:146` sets a hard ceiling of 400 lines per file under `src/`, and sixteen files were
already over it when the ratchet landed — the worst at 1308. Refactoring them mid-roadmap would have
produced two enormous diffs that build nothing, so the ceiling is enforced going forward instead:

`cargo test --test source_line_budget` fails in four situations, each naming the file, its count and
what to do.

| Situation | What the failure asks for |
| :--- | :--- |
| A file over 400 that is not in the allowlist | Decompose it into `foo.rs` + `foo/` or move code out. Adding it to the allowlist is not the fix. |
| An allowlisted file that grew past its recorded count | Pull it back to its count. The number is a cap, not a target. |
| An allowlisted file that now fits inside 400 | Delete the entry in the same commit that shrank it. |
| An allowlisted file that no longer exists | Delete the entry. |

The allowlist is a table at the top of `tests/source_line_budget.rs`, one line per file with the count
it had the day it was measured. It can only shrink: every entry is a debt the repository already
carried, and the list never grows to cover a file that was written after it.

**The 350-line facade rule stays a human judgement.** The same clause says a file approaching 350
lines must decompose into `foo.rs` + `foo/`, and no test can decide that: whether a file has a seam
depends on what its functions do, and picking a decomposition is a design decision with a review cost.
The ratchet owns arithmetic; review owns the facade.

## Quick reference

```bash
scripts/check.sh                      # the whole gate
cargo test --test source_line_budget  # just the ratchet
cargo clippy --locked --workspace --all-targets --target x86_64-unknown-linux-gnu -- -D warnings
```

The gate prints `rustc -V` first so a toolchain difference is visible while diagnosing. There is no
`rust-toolchain.toml` and no pin: none of the failures this gate was built for was toolchain drift,
and CI installs the channel named `stable` regardless of a pin file, so a pin would add maintenance
without changing what CI does.
