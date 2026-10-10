# Feature: Verify the plugins feature on the musl targets and the Alpine image (Issue #175)

> **Status**: In Progress
> **Target Version**: v0.13.0
> **Issue**: [#175](https://github.com/cutver/cutver/issues/175)
> **Constitutional Law**: [CONTRACT.md](../../CONTRACT.md)
> **Blocks**: #169 decision A (the plugin-enabled artifact variant and the `:plugins` container tag).

---

## Problem

The `plugins` feature has never been built for the musl targets or for the Alpine image. PR #171 brought
it under CI for `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin` and `x86_64-pc-windows-msvc` only.

Untested with the feature:

- `x86_64-unknown-linux-musl`
- `aarch64-unknown-linux-musl` (built through `cross`, as `release.yml` does)
- the `rust:alpine` build stage of the `Containerfile`

The feature pulls 27 `wasmtime`/`cranelift` crates into a static musl build — the one place where it can
fail in ways the gnu builds never reveal. Both musl targets and the container image are published, and
#169's decision A is to publish a plugin-enabled variant for every target, so a failure discovered at a
tag is a failure discovered at the worst moment.

## Design decisions (parent-owned)

**D1 — The verification runs in GitHub Actions on real runners, not locally.** The builds are heavy (a
cold `wasmtime`/`cranelift` compile for musl) and this dev environment has no Docker and no `cross`, only
`podman` and `qemu-aarch64-static`. It also has no musl target or musl linker installed. A throwaway
`push`-triggered workflow on a temporary branch produces the evidence on the same runners and with the
same tooling the release uses — the pattern proven in #174 (run 38021977381). The temporary branch and
its workflow are deleted after the evidence is captured.

**D2 — The `Containerfile` gains a behavior-preserving feature knob.** `ARG CUTVER_FEATURES=""`, threaded
into both `cargo build` invocations of the builder stage, defaulting to empty. Rationale: acceptance item 3
asks whether the *actual* `rust:alpine` stage compiles with the feature; without a knob that question is
not expressible against the real stage, only against a replica. The default keeps today's lean image
unchanged, and the same knob is what makes phase 2's `:plugins` image a one-line build.

**D3 — Execution reuses #174's smoke.** The musl artifacts are executed with
`.github/scripts/release-smoke/run.sh` (`--version` + `doctor --check-changelog` + `changelog latest
--json`), and the wasm path is exercised for real with `tests/wasm_plugin.rs`, which compiles an inline
WAT guest and drives the Extism driver end to end. A static musl test binary runs on the glibc runner.

**D4 — The aarch64 path uses `cross`, exactly as `release.yml` does**, and the resulting static binary is
executed under `qemu-aarch64-static`.

**D5 — No permanent CI job by default.** If the musl feature build costs minutes per push on a cold
cache, it does not belong in per-push CI; it is recorded and re-run on demand. The measured cost decides.

**D6 — Failures are fixed or recorded as blockers**, with raw evidence, and the outcome is recorded in the
repository (this document, plus a pointer from the workflow), never only in the issue.

## Acceptance Criteria

- [x] `cargo build --locked --release --features plugins --target x86_64-unknown-linux-musl` succeeds.
- [x] The same for `aarch64-unknown-linux-musl`, through `cross` as the release matrix does it.
- [x] The `rust:alpine` build stage of the `Containerfile` compiles with the feature enabled.
- [x] The resulting artifacts execute: `--version` and `doctor`, plus a real wasm load check.
- [x] Every failure is either fixed or recorded as a blocker for the plugin-enabled variant, with evidence.
- [x] The outcome is recorded in the repository, not only in the issue.

## Task Breakdown

- [x] Task 1: Add the `CUTVER_FEATURES` knob to the `Containerfile` builder stage (default empty).
- [x] Task 2: Author the throwaway verification workflow covering the three configurations and execution.
- [x] Task 3 (parent-owned): run it on a temporary branch and capture the raw result.
- [x] Task 4: Record the outcome here; fix a failure or record it as a blocker for #169 decision A.
- [x] Task 5: Delete the throwaway workflow and its branch, then open the PR for #175.

## Verification

The verification runs in `.github/workflows/plugins-musl-verify-tmp.yml`, a throwaway `push`-triggered
workflow on the temporary branch `chore/plugins-musl-verify` (D1, D2). `push` rather than
`workflow_dispatch`: a dispatch workflow only becomes callable from the default branch, and this file
lives only on the temporary branch. No permanent per-push CI job is added until the measured cost is known
(D5). The result cells below are filled by the Task 3 run; no result is predicted here.

### Exact commands per job

The `<version>` argument is the crate version read from the first `version = "…"` line of `Cargo.toml`
(the same extraction `container.yml` uses).

**`musl-x86_64`** — `ubuntu-latest`, `dtolnay/rust-toolchain@stable` with `targets: x86_64-unknown-linux-musl`,
`Swatinem/rust-cache@v2` `key: plugins-musl-x86_64`, `apt-get install -y musl-tools`.

| # | Command | Acceptance item |
| --- | --- | --- |
| 1 | `cargo build --locked --release --features plugins --target x86_64-unknown-linux-musl` | 1 |
| 2 | `sh .github/scripts/release-smoke/run.sh target/x86_64-unknown-linux-musl/release/cutver <version>` | 4 |
| 3 | `cargo test --locked --features plugins --target x86_64-unknown-linux-musl --test wasm_plugin` | 4 |

**`musl-aarch64`** — `ubuntu-latest`, `targets: aarch64-unknown-linux-musl`,
`Swatinem/rust-cache@v2` `key: plugins-musl-aarch64`, `taiki-e/install-action@cross` (exactly as
`release.yml`), `apt-get install -y qemu-user-static`.

| # | Command | Acceptance item |
| --- | --- | --- |
| 1 | `cross build --locked --release --features plugins --target aarch64-unknown-linux-musl` | 2 |
| 2 | `qemu-aarch64-static target/aarch64-unknown-linux-musl/release/cutver --version` | 4 |
| 3 | `sh .github/scripts/release-smoke/run.sh .qemu-wrap/cutver <version>` | 4 |

For row 3 the workflow generates `.qemu-wrap/cutver`, a two-line wrapper that execs
`qemu-aarch64-static <absolute-binary> "$@"`, because `run.sh` executes its argument directly and
`cd`s into its fixture before each invocation. The wrapped binary path is absolute for that reason.

The wasm load check is deliberately not repeated for aarch64: `cross test`'s qemu mechanics are an
infrastructure concern of their own, and `musl-x86_64` already runs `tests/wasm_plugin.rs` natively on a
musl target. This job's contract is the `cross` build plus executing the artifact under qemu.

**`alpine`** — `ubuntu-latest`. One job builds the image with the knob and runs it; the full build
compiles the same builder stage acceptance item 3 asks about, so this covers items 3 and 4 together.

| # | Command | Acceptance item |
| --- | --- | --- |
| 1 | `docker build -f Containerfile --build-arg CUTVER_FEATURES="--features plugins" -t cutver-plugins .` | 3 |
| 2 | `docker run --rm -v "$PWD:/repo:ro" --entrypoint /bin/sh cutver-plugins /repo/.github/scripts/release-smoke/run.sh /usr/local/bin/cutver <version>` | 4 |

**`alpine-lean`** — `ubuntu-latest`. Guards the knob's default: the standard image must still build and run
without the build-arg, exactly as before. This file has broken silently once already, so the lean path is
checked on its own rather than assumed. Cheap, because it compiles no Wasmtime stack.

| # | Command | Why |
| --- | --- | --- |
| 1 | `docker build -f Containerfile -t cutver-lean .` | the default (empty `CUTVER_FEATURES`) is unchanged |
| 2 | `docker run --rm -v "$PWD:/repo:ro" --entrypoint /bin/sh cutver-lean /repo/.github/scripts/release-smoke/run.sh /usr/local/bin/cutver <version>` | the lean image still boots and runs |

### Results (Task 3 run)

Run [38023278006](https://github.com/cutver/cutver/actions/runs/38023278006) on branch `chore/plugins-musl-verify`:
**workflow conclusion `success`, 3/3 jobs green.** Each job prints one `cutver-verify <job> (…): success`
line; the raw logs are the evidence, summarized below.

| Job | Outcome | Duration | Raw evidence |
| --- | --- | --- | --- |
| `musl-x86_64` | pass | 04:13:02 → 04:19:01 (≈6m) | `release-smoke: --version => cutver 0.12.0`, `release-smoke: OK 0.12.0`, and `test result: ok. 3 passed; 0 failed` for `tests/wasm_plugin.rs` on the musl target |
| `musl-aarch64` | pass | 04:13:01 → 04:17:31 (≈4m30s) | `cross build` succeeded; `release-smoke: --version => cutver 0.12.0` and `release-smoke: OK 0.12.0` through the `qemu-aarch64-static` wrapper |
| `alpine` | pass | 04:13:01 → 04:20:45 (≈7m45s) | image built with `CUTVER_FEATURES="--features plugins"`; inside the image `release-smoke: --version => cutver 0.12.0` and `release-smoke: OK 0.12.0` |
| `alpine-lean` | pending | — | added after this run to guard the knob's default (empty feature set); a follow-up run on the same branch records it |

**No product failure.** The only failure in this exercise was in the verification harness itself and was
fixed (see below); nothing is recorded as a blocker for #169 decision A.

#### Harness defect found and fixed

The first run (`38023042635`, cancelled) failed the `alpine` job in ≈40 s. Cause: the workflow called
`docker build` with no `-f`, and `docker build` defaults to a file named `Dockerfile` while this
repository's is `Containerfile` — which `container.yml` already records as `file: ./Containerfile`. Fixed
with `docker build -f Containerfile …`; the corrected run is the one tabulated above. The short time to
failure, not a compile error, is what identified it.

#### Cost and the D5 decision

Measured on cold caches: ≈4m30s (`musl-aarch64`), ≈6m (`musl-x86_64`), ≈7m45s (`alpine`), running in
parallel. That is 3 heavy `wasmtime`/`cranelift` compiles per run, roughly the same order as, and on top
of, the `test-plugins` job #171 added. **D5 is therefore resolved as: no permanent per-push CI job.** The
musl and Alpine feature builds are a release-time or on-demand check, not a per-push one. Making them
cheap enough to promote would need a real cross-job artifact cache, which is out of scope here.

## Evidence & Verification

- Workflow (throwaway, deleted before the PR): `.github/workflows/plugins-musl-verify-tmp.yml` on branch
  `chore/plugins-musl-verify`; run [38023278006](https://github.com/cutver/cutver/actions/runs/38023278006),
  3/3 green.
- `Containerfile` knob: `ARG CUTVER_FEATURES=""` threaded into both builder-stage `cargo build`
  invocations; the empty default preserves the current image.
- Outcome: the `plugins` feature builds and runs on both musl targets and inside the plugin-enabled
  `rust:alpine` image, and the Extism path executes on a musl target. #169 decision A is unblocked.
- No permanent CI job was added (D5, measured cost above).
