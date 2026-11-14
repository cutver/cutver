# cutver issues #271 + #270 — repository hygiene (docs + tests)

Branch `chore/271-270-docs-tests` from `main @ 2c1c2b0`. Zero production surface; may merge first.
Work only in this worktree (`D:/development/workspaces/cutver-271-270`). Measured at start: `README.md` = 427 lines.

## Specs

### #271 — docs: decompose README under the 200-line ceiling

S1. **Priority**: P2 · 15 of 16 — the entry point exceeds the attention budget the contract sets by more than 2×.
S2. **Contract**: Pillar IV.5 (*Strict Cognitive Line Budgets*), `CONTRACT.md:114`: "Target: 100–150 lines per document; strict hard ceiling ≤ 200 lines. Documents approaching limits must decompose into focused submodules under `docs/`."
S3. **Failure mode**: the first document a reader opens is the one that violates the budget the project declares as law. The rule is unenforced, so it will drift again.
S4. **Remedy**: decompose README into `docs/` submodules and leave a short entry point. Keep link text semantic — no raw paths or file extensions in visible link text (Pillar IV.5, *Semantic Link Hygiene*).
S5. Acceptance: `wc -l README.md` reports ≤ 200.
S6. Acceptance: Every moved section lives under `docs/` and is linked from README by a descriptive label.
S7. Acceptance: `git grep -nE '\]\([^)]*\.md\)' README.md` shows no visible link text that is a file path.
S8. Acceptance: A CI step asserts the 200-line ceiling for `README.md` and `docs/**/*.md`, so the rule fails closed from now on.

### #270 — test: isolate the process CWD and parameterize the render clock

S9. **Priority**: P2 · 14 of 16 — nondeterminism that turns into flaky tests and, once, into inconsistent output.
S10. **Contract**: Pillar IV.1 (*Absolute Test Isolation*: `tempfile::tempdir()`, parameterized dates or mock clocks, "tests must never fail due to time zone differences or clock drift") and Pillar I.1 (*Zero global state*, referentially transparent rendering).
S11. **Failure mode**: an assertion fails when the two `now()` calls straddle local midnight. A test sees another test's working directory. In one render, the `## [x] - date` heading and `{{date}}` can disagree, so identical inputs produce different bytes — which breaks the determinism Pillar I.1 requires.
S12. **Remedy**: `tempfile::tempdir()` everywhere, no `set_current_dir` in tests, the date injected as a value into the render core and never re-derived inside it.
S13. Acceptance: `git grep -n "set_current_dir\|env::temp_dir" tests/` returns nothing.
S14. Acceptance: A test renders the same changelog twice with a midnight-straddling injected clock and asserts byte-identical output and a single consistent date.
S15. Acceptance: The date assertions compare against a value supplied by the test, not by a second `now()` call.

### Track boundaries (from the assignment)

S16. "no src/ changes except the render clock #270 requires — if you need more, stop and ask the human."
S17. "Do not touch src/bump/**, src/git/**, src/config/**, src/cli/** (other sessions own them)."
S18. "Do not reformat files you are not changing."
S19. "Do not touch src/bump/**, src/git/**, src/config/**, src/cli/**" — the issue evidence names `src/bump/exec/pipeline.rs:244`; if the injection cannot be correct without it, stop and ask instead of editing it.
S20. Gates: `cargo build`, `cargo test --workspace`, clippy `--features plugins -- -D warnings`, `fmt --check`.
S21. "verify every command, flag and path you document still exists: README claims are product surface and a wrong one is a bug."
S22. "never put `Closes #NNN` in a PR body unless every acceptance criterion of that issue is inside that same PR." Push, PR and merge are the human's decision.
S23. Hard semantics: the injected clock feeds every date in one render (heading and `{{date}}`), so identical inputs plus identical injected clock produce identical bytes.

## Tasks

| ID | Task | Specs | Route | Commit |
| --- | --- | --- | --- | --- |
| T1 | Feature document | S1-S23 | inline | chore(docs): track #271 and #270 |
| T2 | Decompose README into `docs/` submodules, entry point ≤ 200 lines, semantic link text | S5-S7, S21 | writer | docs(readme): decompose README into focused docs submodules (#271) |
| T3 | CI step asserting the 200-line ceiling for README and `docs/**/*.md` | S8 | writer | ci(docs): fail when README or docs exceed the 200-line ceiling (#271) |
| T4 | Tests: `tempfile::tempdir()`, no `set_current_dir` | S13 | writer | test: isolate the process CWD in tests (#270) |
| T5 | Inject the release date into the render core; no re-derived `now()` | S12, S14, S15, S16-S19, S23 | writer | fix(changelog): inject the render clock instead of re-deriving it (#270) |
| T6 | Gates + verification of documented surface | S20, S21 | inline/verify | — |
| T7 | Delivery report: criteria, commits, gate exit codes, omissions | S22 | inline | — |

## Log

L1 (2026-03-25, assignment, verbatim):

> Session message from 01a1d7fb-9488-773b-8c07-358c43ebb75b (correlation 2d7735c1acd74566ac5c44d94f45be6c): ASSIGNMENT — Track 3 (Par 3) of the cutver remediation queue. Repository hygiene only: zero production surface, so your track may merge first and must not wait for anyone.
>
> Your worktree is ALREADY CREATED: D:/development/workspaces/cutver-271-270, branch chore/271-270-docs-tests, based on main @ 2c1c2b0. Work THERE, never in D:/development/workspaces/cutver — another session owns that tree and concurrent edits corrupt both (shared index and HEAD).
>
> First two things: (1) session_worktree_register("D:/development/workspaces/cutver-271-270"); (2) publish state via orchestrator_session_id (objective + work.topic) — this session never published any.
>
> Issues, one work-unit commit each:
> 1. #271 docs: decompose README under the 200-line ceiling. README.md is 427 lines today. Budget: README.md + docs/**.
> 2. #270 test: isolate the process CWD and parameterize the render clock. Budget: tests/** plus the render clock.
>
> Read each issue body before writing: priority, CONTRACT.md pillar, path:line evidence, failure mode, remedy, acceptance criteria.
>
> Hard boundaries: no src/ changes except the render clock #270 requires — if you need more, stop and ask the human. Do not touch src/bump/**, src/git/**, src/config/**, src/cli/** (other sessions own them). Do not reformat files you are not changing.
>
> Order: #271 first (cheaper) unless #270 has to change a shared tests/ helper that your own new tests depend on.
>
> Gates: cargo build, cargo test --workspace, clippy --features plugins -- -D warnings, fmt --check. For the docs half, verify every command, flag and path you document still exists: README claims are product surface and a wrong one is a bug.
>
> Tracking: create odd/tasks/cutver-issue-271-270.md (specs quoting the issue bodies, task lines, Log starting with this instruction) and commit it with the work.
>
> Delivery: commit on your branch; push, PR and merge are the human's decision. Native review cannot reach terminal approval right now, so any merge needs explicit human authorization; never put `Closes #NNN` in a PR body unless every acceptance criterion of that issue is inside that same PR.
>
> Report back: which acceptance criteria you covered, the commits, each gate's exit code, and what you deliberately left out.

L2: `#271` body read verbatim (issue acceptance criteria quoted in S1-S8 above). `README.md` measured at 427 lines on `2c1c2b0`; the issue body says 426 on `main@2f8805b` — the difference is drift since that snapshot.
L3: `#270` body read verbatim (S9-S15 above). Note the conflict between S17/S19 and the issue's own evidence naming `src/bump/exec/pipeline.rs:244`; resolution direction: prefer `src/changelog/**` + `tests/**` only, and report `src/bump/**` as deliberately left out if the injection is correct without it.
