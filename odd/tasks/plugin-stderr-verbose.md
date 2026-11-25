# plugin-stderr-verbose — give a process plugin's stderr a switch, not a workaround

A `runtime = "process"` plugin's stderr is drained into memory and discarded when the call succeeds
(`src/plugin/driver/process.rs:247` `finish_outcome`; `stderr_bytes` is never read on the success
path). The natural way to debug a plugin is to log to stderr, so on a *working* call that advice
produces nothing, which reads as "my logging is broken". Today the drop is documented with two
workarounds (#377). This is the switch that replaces them.

## Specs

- **S1 — One global flag, declared like `--ci`.** `--verbose` is a field on `Cli`
  (`src/cli/args/commands.rs:9`), declared `#[arg(long, global = true)]` with no short form and no
  conflict, so every subcommand accepts it: the flag is read before dispatch picks a command and
  never depends on which one ran. It carries the state down the way `--ci`/`--no-ci` already do —
  `src/cli/runner/dispatch.rs:51` `ci::select(ci::CiMode::from_flags(args.ci, args.no_ci))` writing
  `src/cli/ci.rs:138` `static CHANNEL: OnceLock<CiContext>` — and **not** by adding a parameter to
  `PluginManager::from_config` (`src/plugin/manager/core.rs:45`),
  `ProcessDriver::new` (`src/plugin/manager/factory.rs:109`) or
  `run(args: Cli)` (`src/cli/runner/dispatch.rs:49`). Rationale: those three are public or
  wide-fanout seams, and the plugin layer must not import `cli` to learn a diagnostic preference.
- **S2 — Set, it forwards the child's stderr on a successful call.** With `--verbose`, the bytes the
  child wrote to stderr reach cutver's own stderr, unchanged and in the order the child wrote them,
  on a call that exits 0. Byte-for-byte, no prefix invented, no trim added.
- **S3 — Unset, nothing changes at all.** Without the flag the success path stays silent. This is
  the previous-behaviour pin, not an afterthought: the failure path
  (`src/plugin/driver/process.rs:254` → `:259`, `ExecutionFailed { stderr }`) is out of scope and
  must stay byte-identical with the flag neither set nor unset.
- **S4 — The bytes are already there; forward them, do not re-read.** `stderr` arrives at
  `finish_outcome` from `:208` and is already capped at `MAX_PLUGIN_STDERR_BYTES` (64 KiB, with the
  truncation marker T7 added). Forwarding uses that buffer. No new read, no new cap, no second
  drain.
- **S5 — Scope is `runtime = "process"` only.** A WASM plugin has no child and no stderr; the flag
  changes nothing for it. State that in the docs rather than leaving it to be discovered.
- **S6 — The reference page and its parity test.** `docs/reference/cli.md` gains a row for
  `--verbose` in the form `tests/cli_reference_parity.rs` accepts (that test decides the shape; it
  is the contract, not a suggestion), plus one sentence naming what else becomes verbose today:
  **nothing**. The issue's shape 1 asks for a stated policy, and the honest one is that this flag
  has exactly one effect.
- **S7 — Rails.** `src/cli/args/tests.rs` sits at its recorded **738** cap and may not grow, so the
  flag's parse test goes where there is room (`src/cli/args/commands.rs` is 262 of 400), never as an
  allowlist entry. `src/proc.rs` is at **399/400**: the forwarding must not add a line to it.

## Tasks

| ID | Spec | Route | Commit |
| --- | --- | --- | --- |
| T1 | S2, S3, S4 | **Done** — `tests/plugin_process_stderr_verbose.rs`, four tests that spawn the built binary and read its stderr: the forward, the silence without the flag, the failure path byte-identical with the flag set, and the child's own invocation counter so "forwarded once" is not vacuous. Two reds were observed before the green: first the flag did not exist (`unexpected argument '--verbose'`), then the flag existed and the forward did not (`left: ""` where the child's line belonged) — the second is what proves the test asserts S2 rather than that the flag parses | 1 |
| T2 | S1, S5, S6 | **Done** — `src/plugin/diag.rs` (the `OnceLock`, written once from `dispatch.rs` beside `ci::select`), the global `--verbose` on `Cli`, the forward in `finish_outcome` after the exit-status branch, the reference row, and the sweep of `plugin-process-authoring.md` — which still said there is no way to ask for the bytes — plus the one failure-table row in `plugin-process-runtime.md` | 1 |
| T3 | S7 | Gate: `sh scripts/check.sh` on the branch, both suites, clippy, fmt, ratchet. Record the commit here | 2 |

## Log

L1 (maintainer, verbatim): «ataca los issues en un nuevo worktree, o dime antes que podemos atacar de forma paralela»

L2 (decision, verbatim from the choice): «Flag global --verbose» — shape 1 of the three in #377. The
two rejected: always-forward (turns a release into pages when a plugin logs per commit, and reaches
CI annotations) and an environment opt-in (a configuration surface invisible in `--help`).

L3 (evidence, this session): adding the flag does **not** touch `src/cli/runner.rs` or
`src/cli/runner/tests.rs`. No test in `runner/tests.rs` builds a `Cli` literal — they all
`Cli::try_parse_from` and call `run` — so a new `Cli` field breaks nothing there, which is what
makes this work parallel to #375. Only a change to `run`'s own signature would collide.

L4 (evidence, this session): the precedent the design follows is `--ci`/`--no-ci` —
`src/cli/args/commands.rs:14` and `:17` are `global = true`, and `src/cli/runner/dispatch.rs:51`
turns them into runtime state through `src/cli/ci.rs:141 pub fn select(mode: CiMode)` writing
`static CHANNEL: OnceLock<CiContext>` at `ci.rs:138`. `--config` is threaded as a parameter instead
(`dispatch.rs:67 load_config(args.config)`), which is the cheaper path for a value one command needs
and the wrong one for a value the driver needs.

L5 (T1/T2 implementation, observed): the seam is S1's recommendation with one correction the brief
could not know — `src/cli/diag.rs` is impossible, because a static `finish_outcome` can read must not
live under `crate::cli` if the plugin layer is not to import it. `src/plugin/diag.rs` owns the
`OnceLock`; `src/cli/runner/dispatch.rs` writes it once, before the command match, exactly where
`ci::select` already sits. Four files did not exist under the names the brief guessed (`src/cli/mod.rs`,
`src/plugin/mod.rs`, and both `diag.rs` candidates); `src/cli.rs` and `src/plugin.rs` are the real
module roots, and only `src/plugin.rs` needed the new `pub mod`.

L6 (S6 premise partly false, reported by the writer): `tests/cli_reference_parity.rs` does **not**
decide the shape of a global flag — it reads the flags of the *unbuilt* clap subtree, so a
`### cutver <name>` section never has to carry `--ci`, `--no-ci` or `--verbose`. That is why `--ci`
lives in prose today. The row was still written as a table entry under **Global Options**, and the
guide's own sweep was needed anyway: `plugin-process-authoring.md` asserted "no way to ask for them",
which this change falsifies.

L7 (docs sweep by the parent, measured after the change): `finish_outcome` is at
`src/plugin/driver/process.rs:253`, its failure path at `:260-267`, the forward at `:269`. The brief
said `:247`, which was true *before* the writer's six-line doc comment — inserting a comment above a
function falsifies every citation below it, which is the fourth occurrence of this class in this
feature series. `plugin-process-runtime.md`'s stdout-overrun row gained the one clause it needed.
