# v1.0 block 2 — the deprecation channel (#272)

**Closed.** Merged as `256a9e8` (PR #348, 2026-11-20); #272 closed by the merge.

Block 2 of the v1.0 program (#346): a surface is deprecated **before** it is removed, and the warning reaches the human
who has to act on it and the machine that has to report it. Branch: `feat/346-block-2-deprecation-channel`.

## Specs

**S1 — a deprecated config spelling is accepted, never rejected.** Three legacy spellings exist today as silent
`#[serde(alias)]` renames (`src/config/types.rs:174,192,202`):

| Deprecated | Canonical |
| :--- | :--- |
| `[version].source` | `current_source` |
| `[[manifest]].kind = "toml"` | `"cargo-package"` |
| `[[manifest]].kind = "pyproject-toml"` | `"pyproject"` |

A document using any of them loads and the command succeeds with exit code `0`. It is never a `ConfigError`.

**S2 — every deprecated use warns on stderr, always, with `--json` and without it, in exactly this text:**

```text
warning: [version].source is deprecated, use current_source; it will be removed in 1.0.0
```

Shape: `warning: <where> is deprecated, use <canonical>; it will be removed in <version>`. `<where>` names the table and
the key (or the key and its value) as the consumer wrote it, `<version>` is the removal version.

**S3 — stdout is never the warning channel.** The warning does not appear on stdout, in any command, `--json` or not:
stdout carries the payload and nothing else.

**S4 — under CI, the same text is also one GitHub annotation on stderr**, `::warning::<text>`, using the channel phase
4A already built (`src/cli/ci.rs:79,88`, detection `GITHUB_ACTIONS=true` plus a named `GITHUB_OUTPUT` or
`GITHUB_STEP_SUMMARY`). Both lines appear — the human line and the annotation — and the exit code is untouched.

**S5 — the text comes from one table in code**: surface, deprecated spelling, canonical replacement, removal version.
Never from a call site. Every entry warns and every entry is detected by the scanner, so no entry is decorative.

**S6 — `cutver doctor --json` carries the deprecation structurally** in a new additive `warnings` array naming both
spellings; `schemaVersion` stays `1` and the key set only grows. `cutver doctor`'s human path prints the same
`warning: …` line as every other command.

**S7 — `cutver init --update` prints the same migration warning naming both spellings**, because `CONTRACT.md:125`
names `doctor` and `init --update` as the two migration-warning surfaces.

**S8 — deprecated and unknown stay distinguishable.** A never-valid key still fails closed with the existing
`ConfigError::UnknownKey` and its exact message (`src/config/types.rs:72`), exit code unchanged, and emits no warning.

**S9 — nothing degenerate.** A clean document warns nothing, `doctor --json` reports `"warnings": []`, and the six
surfaces pinned by #347 keep their required key sets (`tests/schema_compat.rs`).

## Tasks

| ID | Task | Spec | Route | Commit |
| :--- | :--- | :--- | :--- | :--- |
| T1 | Deprecation table, message renderer and document scanner | S5 | inline | 0583d33 |
| T2 | Human + CI warning channel, wired where config is loaded, once per process | S2, S3, S4 | inline | 0583d33 |
| T3 | `doctor --json` `warnings` array (the human line already came from the loader) | S6 | inline | 7ceb361 |
| T4 | `init --update` migration warning, before its early return | S7 | inline | 7ceb361 |
| T5 | Pins: deprecated loads, inline table and array forms, dedupe, unknown still fails, CI annotation, registry completeness | S1, S8, S9 | inline | 0583d33, 7ceb361 |
| T6 | Docs: configuration reference aliases, machine contract, `CONTRACT.md` Pillar IV.2/VII.2, doctor surface | S2, S6 | inline | _this commit_ |
| T7 | Independent verification (`gentle-ai-verify`: no blockers, S1-S9 reproduced), the #346 comments, and the merge | all | `gentle-ai-verify` | 256a9e8 |

## Log

**L1 (maintainer, verbatim):** «continúa con el bloque siguiente del 346»

**L2 — the two decisions of block 2, answered before code.** Channel shape: *stderr always, plus `::warning::` when CI
is detected* (chosen over stderr-only, which buries the warning in a CI log, and over an extra `$GITHUB_STEP_SUMMARY`
row, which adds a render surface for no new information). Structural carrier: the maintainer delegated the call —
«Para esta última decisión recomienda que es mejor» — and the recommendation taken is **`doctor --json` only**, because
`CONTRACT.md:125` already names `doctor` as a migration-warning surface and `DoctorReport.checks`/`warnings` is an
additive slot, whereas an envelope-wide `warnings` array would touch all six surfaces pinned by #347 and ask every
consumer to tolerate an array it does not need today. Growth later stays legal by construction (`CONTRACT.md:228-234`).

**L3 — evidence that made the decision concrete.** No `::warning::` emitter exists: `src/cli/ci.rs:156` only writes
`::error::`. `warnings` is structural on no surface: `SummaryJson` has no such field
(`src/cli/runner/bump.rs:55-64`), `ReleasePlan.warnings` is `#[serde(skip)]` (`src/bump/plan.rs:131-132`), and
`DoctorReport` is `{ok, checks, version_drift, changelog_drift}` (`src/cli/doctor.rs:21-32`). The three aliases are
accepted silently today and are documented in no reference page.

**L4 — removal version spelled `1.0.0`, not `1.0`.** The approved preview read `will be removed in 1.0`; the table
carries `1.0.0` so the promise names the release the project actually cuts, and so a later release cannot be mistaken
for the one that must remove the alias. The alias becomes a real obligation of the 1.0.0 release, which is the point of
block 2.

**L5 — the line ratchet decided two refactors, not the feature.** All three files the change had to touch are
allowlisted at their exact cap (`src/cli/doctor.rs` 438, `src/config/discovery.rs` 467, `src/config/types.rs` 693),
and `tests/source_line_budget.rs` fails on any growth. So the change paid for its lines instead of raising a cap:
`impl Default for Config` (19 lines) became `#[derive(Default)]`, because every entry already called the same
constructor; and a unit-test `Config` literal in `src/cli/doctor.rs` became `..Default::default()` for the fields it
did not exercise. Final counts: 434 / 467 / 676, no allowlist entry touched.

**L6 — the loader reports after validation, and that ordering is the S8 guarantee.** A never-valid key still fails
closed before anything is scanned, so a rejected document warns about nothing. The flip side, recorded because it is
observable: a document that carries a deprecated spelling *and* then fails for another reason (a `current_source` that
names no manifest) prints the warning first and the unchanged error second. That is S2's "always", not a regression.

**L7 — one advisory from verification, accepted.** The annotation omits the redundant `warning: ` label
(`::warning::<sentence>`), the way `::error::` already omits `Error: `; the documented phrase says "the same sentence",
which is literally what happens.

**L8 — the gate's single failure is a pre-existing Windows flake, not this branch.**
`atomic::tests::concurrent_writes_to_same_file_do_not_collide` failed once with `PermissionDenied` on rename. The
verifier ran it 18 times per tree — branch 0 failures, pristine `84c9c33` 0 failures — and `git diff main --
src/atomic.rs` is empty. It belongs to #344's class and joins the reachability condition in #346: 1.0 is not reachable
while a suite can fail a run in N.

**L9 — the gate caught one lint, and it was folded back into its own slice.** `clippy --all-targets -D warnings` refused the
`match` that only destructured `Ok` in `src/warnings.rs`; the emit now uses `if let Ok(...)`, which is the same behaviour
(the sentence still prints when the lock is poisoned, because the print statements sit outside the block). The three
work-unit commits were replayed so the slice that introduced the file is the slice that compiles clean, and
`scripts/check.sh` then passed 9/9 — the only step not run is the plugins cross-target clippy, which needs
`x86_64-linux-gnu-gcc` that this host does not have.

**L10 — merged.** PR #348 squash-merged as `256a9e8` on 2026-11-20, CI green on all seven required legs, macOS included;
#272 closed by the merge. The only red the PR ever showed was `Exactly one type label`, which is the repository's own
gate asking for a `type:*` label — the fix was a label, not code, and it belongs in the standing checklist for the next
PR. The branch is deleted; `main` is the record.
