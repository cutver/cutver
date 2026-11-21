# cutver — one supervised spawn authority, and the process plugin driver as its fourth caller

Feature branch `fix/295-316-preflight-spawn`, worktree `D:/development/workspaces/cutver-295-316`.
Carries #316 on top of the #295 root contract that landed first (`fix(preflight): run a step in the
repository root`).
Spec source: GitHub issue #316, read in full. Every `S#` below quotes it verbatim; nothing here adds
a requirement the issue does not state.

## Specs

**S1** — the defect. "A `runtime = "process"` plugin that outlives `timeout_seconds` loses its
**direct child** and nothing else. A process the plugin spawned itself keeps running, keeps holding
the inherited stdout/stderr pipes, and is never accounted for."

**S2** — the cause. "`src/plugin/driver/process.rs:74-90` — `spawn_child` starts the child with no
process group, so the runtime has no handle on a tree it could signal." · "`src/plugin/driver/process.rs:279-284`
— the timeout path calls `kill_child`, which is `Child::kill()` and `wait()` on that one handle." ·
"`src/plugin/driver/process/tests.rs:283-330` — the committed test asserts that the direct child is
killed **while the descendant survives**, i.e. today the leak is pinned as expected behavior."

**S3** — AC 1. "A test spawns a process-plugin that starts a descendant writing a marker file, times
the invocation out, and asserts the marker is never written on both Windows and unix."

**S4** — AC 2. "The existing timeout tests still pass and the invocation budget is unchanged."

**S5** — AC 3. "No second copy of the kill machinery exists: the driver reuses the supervisor."

**S6** — AC 4. "The reason a descendant cannot outlive the plugin is stated in the code where it is
enforced, not in a comment explaining why it is tolerated."

**S7** — remedy 1. "Move nothing twice: the supervisor stays the single copy; the driver calls it,
with `dir`, environment grants and the invocation exchange adapted to what the driver needs."

**S8** — remedy 2. "Keep `timeout_seconds` semantics unchanged: one budget for the exchange, and a
typed error naming plugin, capability and limit."

## Established facts (do not re-derive)

`run_supervised` **cannot be called as-is** from the driver, and this is not a matter of taste:

- `src/preflight.rs` hardcodes the platform shell (`shell()` → `sh -c` / `cmd /C`) and takes a single
  `&str`. The driver spawns the program directly with an argv, documented as "No shell… a process
  plugin is spawned directly". Routing through `run_supervised` would drop `args` or reinterpret the
  command through a shell — a silent change to the plugin contract.
- `run_supervised` never sets `Stdio`: stdin/stdout/stderr are inherited, and `Ended` carries no
  bytes. The driver needs all three (`driver/process.rs:172-173`, `:203-215`).
- `Ended`/`SupervisionError` cannot carry captured output without rewriting the arms at
  `preflight.rs:78`, `hooks.rs:92-93`, `command.rs:68-69`.
- `src/preflight.rs` is at the 400-line ceiling edge and its `tests/source_line_budget.rs` allowlist
  entry does not exist; the ratchet says that table "can shrink but never grow".

So the shared thing is the machinery **below** `run_supervised` — own process group, deadline poll,
tree kill — and the shape is: neutral module, two callers.

## Tasks

- **T1** — extract `Ended`, `SupervisionError`, `POLL`, `spawn`→`spawn_group(&mut Command)`,
  `kill_tree`, `shell()` and the `setpgid`/`killpg` extern block into a new `src/proc.rs`; add
  `run_capture` that bounds the whole exchange (stdin write, wait, drain) by one budget. Route:
  inline or worker. Specs: S5, S7.
- **T2** — rewire `src/preflight.rs` to the new module: keep `run_supervised` as the thin shell
  wrapper, keep the `dir: &Path` contract that #295 established. Specs: S4, S7.
- **T3** — rewire `src/plugin/driver/process.rs::invoke` through the new module, preserving
  `.env_clear()` + grants, delete `wait_with_timeout`, `await_writer`, `take_output_streams`,
  `poll_or_kill`, `try_check_child`, `kill_child`, and delete the detached-thread rationale comment
  at `:163-167` — replace it with the enforcement statement S6 asks for. Specs: S1, S2, S6, S8.
- **T4** — invert `src/plugin/driver/process/tests.rs:283-330` from "the descendant survives" to "no
  descendant survives", and add the marker test S3 demands, non-unix-gated. Specs: S2, S3, S6.
- **T5** — update `docs/guides/plugin-process-runtime.md`, whose §`timeout_seconds` currently
  documents "Reaches | The direct child only", plus its `:56-57` note on the child's working
  directory if `dir` changes it. Spec: S4 (the budget is unchanged; the reach is what changes).

## Log

**L1** (maintainer, in order): «podemos trabajar con worktree de forma paralela?» · «DALE» —
authorising the two-worktree plan, track A (`#295` + `#316`) with test-first, and leaving the docs PR
ready. The #295 half landed first and is green end to end (`fmt --check`, `clippy -D warnings`,
`cargo test --workspace`).

**L2** (evidence, this session): `#295` shipped `run_supervised(command, dir: &Path, timeout)` — the
`Option` is gone, so a supervised command cannot be spawned without naming its directory.

**L3** (open question for T3, decide explicitly and write the decision down): on a **successful**
child exit with a descendant still holding the pipes, the joins at `driver/process.rs:172-173` are
unbounded today, so the caller waits past `timeout_seconds`. Bounding them fixes a second hole but
changes observable behaviour beyond what #316 names. Recommended: bound the joins by the remaining
budget and do **not** kill a tree whose direct child exited zero — no new kill semantics, no hang.
Record the choice in the commit message; do not leave it implicit.
