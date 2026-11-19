# Local gate and source-budget ratchet — foundation pass before phase 4B of #326

Two pieces of infrastructure that the rest of #326 depends on: a local gate that mirrors what CI runs, and a check that
turns `CONTRACT.md:146`'s 400-line ceiling from a wish into a constraint.
Requirement source: this session's own evidence — three red CI runs in three phases, none of them a logic defect — plus
`CONTRACT.md:111` (review advisory) and `CONTRACT.md:146` (module budget).

## Specs

S1. **One command mirrors CI.** `scripts/check.sh` runs, in this order and stopping at the first failure with the failing
step named: the toolchain version, build, test (default features), test (`--features plugins`), clippy default, clippy
plugins, clippy for the `x86_64-unknown-linux-gnu` target (default features), and `cargo fmt --all --check`. Its exit code
is the gate's verdict and nothing else.

S2. **The gate runs the suites in the environment CI has.** Every GitHub Actions `run:` step sets `GITHUB_ACTIONS`,
`GITHUB_OUTPUT` and `GITHUB_STEP_SUMMARY`; a suite that only passes without them is a lie, and that exact lie turned
`#332` red. The gate sets all three (the sinks into a temporary directory) for its own child processes, restores nothing
because it mutates nothing in the caller's shell, and leaves no temporary path behind.

S3. **The gate degrades honestly.** A step the host cannot run says so, visibly, and is never silently reported as
passing. Today that is the plugins build for the linux target: `zstd-sys` needs a C cross-compiler this host lacks. A
skipped step must print the reason, and the gate's summary must distinguish "passed", "failed" and "not run here".

S4. **The 400-line ceiling becomes a ratchet.** `tests/source_line_budget.rs` enforces `CONTRACT.md:146` over
`src/**/*.rs`: a file over 400 that is not in the allowlist fails; an allowlisted file that grows fails; an allowlisted
file that now fits inside 400 fails until its entry is deleted, so the list can only shrink. The allowlist records the
count each entry has today, and the rule for the 350-line facade is left to human judgement, stated as such in the doc
comment, because choosing a decomposition is not a mechanical decision.

S5. **Discoverable.** A page under `docs/` documents the gate and the ratchet, in at most 200 lines, linked from wherever
the other internals pages are listed. The repository has no `CONTRIBUTING.md`, no `Makefile`, no `justfile`, no
`.cargo/config.toml` and no `rust-toolchain.toml` today, so the gate is the only contributor-facing entry point and it has
to be findable without reading 1041 lines of YAML.

## Tasks

- [ ] T1 — S1, S2: `scripts/check.sh` with the CI-shaped environment and the summary.
- [ ] T2 — S3: the skip path with its reason, proven by a run on this host.
- [ ] T3 — S4: `tests/source_line_budget.rs` with the computed allowlist, plus one mutation per rule.
- [ ] T4 — S5: the docs page and its link.

## Log

L1 (user, verbatim): `Mergea 332 y hablemos un poco sobre el estado y robustes actual de cutver y visión como proyecto`

L2 (user, verbatim): `Cosideras que deberíamos frenar el 326 hasta este punto y  corregir lo que me dices?`

L3 (decision, user): `Pase acotado y seguir con 4B` — the gate and the ratchet first, then 4B immediately. Not a stop:
leaving #326 at four of six phases with acceptance criterion 6 half-closed would be a worse resting place than closing
it.

L4 (evidence, this session — why the gate is the urgent half): three red CI runs across three phases and **none was a
logic defect**. (a) `#331` failed all three unix legs on `config::gates::tests::the_gates_dir_must_stay_inside_the_workspace`,
which asserted that `[gates] dir = "C:/absolute"` is refused on every host — on unix that is a relative directory name.
(b) `#332` first run: the whole suite went red in the Actions environment because the annotation duplicated a message
`tests/e2e_gates_channel.rs:404` counts, while a CI-less local run stayed green. (c) `#332` second run: `unused import:
std::process::Command` inside `tests/e2e_bump.rs`, a file that opens with `#![cfg(unix)]` and therefore is never
type-checked on a Windows host. Each cost a full CI cycle to discover what a local gate would have shown in seconds.

L5 (decision, parent): **no `rust-toolchain.toml`.** Pinning would be machinery invented to solve a problem this session
never observed — none of the three failures was toolchain drift. Adding a pin also changes CI behaviour, since
`dtolnay/rust-toolchain@stable` installs stable regardless of the file and cargo would then insist on the pinned channel.
The gate prints `rustc -V` so drift is visible while diagnosing, which is the honest version of the same benefit without
a frozen toolchain to maintain. If drift ever shows up as a real cause, pin then, with the CI change in the same PR.

L6 (decision, parent): **enforce the ceiling, do not remediate it.** Sixteen files under `src/` are over 400 today, the
worst at 1308 lines. Refactoring them mid-roadmap would produce two enormous PRs that build nothing, and would compete
with phases 5 and 6 for review attention. The ratchet changes the incentive from the next PR onward at the cost of an
hour; the remediation is a separate campaign after phase 6, with its own budget.

L7 (decision, parent): keep the doc ceiling where it is, but note the side effect observed in 4A — compression pressure
silently deleted four statements of documented behaviour from `machine-contract.md` while staying under 200 lines. A
ceiling without a "the facts must survive" check trades cognitive load for quiet loss of precision. Not fixed in this
pass: the ratchet is the mechanical half, and a survivor list is a separate idea worth its own issue.
