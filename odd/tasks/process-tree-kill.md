# #384 — the deadline must reach the whole tree in one act

Feature branch: `fix/384-atomic-tree-kill`. Scope: `src/proc.rs` (the one supervised spawn
authority) plus the docs that describe it. Issue: #384.

## Specs

- **S1.** "`cargo test --locked --workspace` fails, at roughly one run in four, on one of the
  process-driver timeout assertions" — and "Each of the three passes **in isolation**, every time,
  on the same tree."
- **S2.** The assertion that fires is the tree one:
  "a descendant outlived the invocation and wrote …: the deadline must reach the whole tree".
- **S3.** "Whatever is chosen, the fix needs a test that fails without it, and a measurement before
  and after — a flake 'fixed' by widening a timeout is not fixed, it is hidden."
- **S4.** "Serialising this family … would remove the load variable, at the cost of the suite taking
  longer. That is the trade to decide, not to assume."
- **S5.** The documented contract it breaks: `Reaches | The whole process tree: the plugin and every
  process it started` (`docs/guides/plugin-process-runtime.md:140`).

## Tasks

- **T1** Baseline measurement of the family on `main` at `91c05d7`. Done — see L2.
- **T2** Name the cause from in-binary evidence, not from a guess. Done — see L3.
- **T3** End the tree in one act on Windows (job object at spawn, `TerminateJobObject` on the
  deadline, `taskkill /F /T` kept as the fallback), unix unchanged (`killpg` was already one act).
  Done — `src/proc.rs`, `Grouped`.
- **T4** Say what the code now does in the docs that described a walk. Done — plugin-process-runtime,
  pipeline, references, configuration-tables.
- **T5** After measurement, same method as T1. Done — see L5.
- **T6** Independent verification: `src/proc.rs` is shared with `preflight`, so this change reaches
  behavior the issue did not name (Risk item 3).

## Log

**L1.** "Dale con el 384" — the issue is the brief.

**L2. Baseline, `main` at `91c05d7`, this host.** Ten `cargo test --locked --lib` runs: **4 failed**,
every one at the marker assertion (`tests.rs:347`), never at the wall-clock one. A second batch of
seven runs failed once more (run 7). Instrumented `kill_tree` (temporary, reverted): 20 kills, all
`code=0`, empty stderr, **100–592 ms** each. The failing marker was written **1419 ms** (and, in the
second batch, **1226 ms**) into a **1 s** invocation.

**L3. Cause, named from the evidence.** `ping -n 4` in the fixture takes 3.09 s, so a writer due
"three seconds in" cannot have written at 1.4 s on its own schedule. The recorded `taskkill` output
shows the order it kills in — grandchild `16372`, its parent `1864`, then `20212`, then the root
`11556` — **leaves first**. Between a killed leaf and the death of the shell above it there is a
gap, and a `cmd.exe` whose child has just been killed resumes its own command list in that gap:
`type nul > marker` ran 200–400 ms after the deadline that was supposed to end it. So the tree was
reached, just not in one act — `taskkill /F /T` walks a parentage snapshot, one `TerminateProcess`
at a time, and its own latency is part of the caller's wait. Isolated, the walk wins the race; under
the suite's load it does not, which is the one-in-four.

**L4. Decision.** A container ended in one act: a job object created at spawn and ended with
`TerminateJobObject` on Windows. `killpg` on unix already ends a group with one signal, so unix is
untouched. No `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`: a plugin that exits inside its budget must still
leave its descendants running, which `run_capture` documents as preserved. A host that refuses a job
keeps `taskkill /F /T` as its fallback. Nothing is serialised (S4) and no timeout is widened (S3).

**L5. After, same method.** Twelve `cargo test --locked --lib` runs on the branch: **0 failed**.
Cost of a wrong conclusion at the measured 40%: 0.6^12 ≈ 0.2%.

**L6. Known cost of the guarantee.** Inside cutver's job, a plugin that spawns with
`CREATE_BREAKAWAY_FROM_JOB` is denied by Windows. That is the price of a kill that cannot be escaped,
and it is stated in the runtime guide.
