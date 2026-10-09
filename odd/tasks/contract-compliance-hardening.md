# Feature: Contract Compliance Hardening (pre-plugin)

> **Status**: In Progress
> **Target Version**: v0.13.0
> **Law**: [CONTRACT.md](../../CONTRACT.md) (Pillars III, IV, V)
> **Blocks**: `cutver/plugins` — the published plugin ABI must be clean before it ships

---

## Why this exists

An audit of the work landed in `3710632` and `fab9c1c` found real `CONTRACT.md` violations.
Two of them sit in code we just wrote, and one of them (`operation` as a bare string) is part of the
plugin ABI we are about to publish to the world. Fixing it after release would mean a second
breaking change to the same contract, so it is fixed first.

One finding from RDD lineage `review-721004fde10e60d7` was mis-triaged as a minor nit and is in fact a
contract violation: **R3-SHALLOW-HISTORY** is Pillar III.4 verbatim.

## Findings and dispositions

| # | Pillar | Finding | Evidence | Disposition |
| --- | --- | --- | --- | --- |
| 1 | III.1, V.2, V.5 | `operation` is a bare `String` across the plugin domain boundary; a typo compiles and fails at runtime. `Capability`/`operation` can also be mis-paired. | `PluginInvocation.operation: String`, `dispatch_raw(..., operation: &str, ...)` | Fix: typed `PluginOperation` + `PluginCall` coupling |
| 2 | III.4 | Silent error swallow with no diagnostic or rationale. | `src/git/log.rs:116` `Err(_) => Vec::new()`; `:95` legacy suppression | Fix: make the degradation observable |
| 3 | V.1 | `match` inside `match` reaches three indentation levels; the Contract mandates guard clauses. | `src/git/log.rs:112-118` | Fix: `let-else` guard |
| 4 | IV.1 | Tests create ad-hoc temp paths instead of `tempfile::tempdir()`. | `tests/wasm_plugin.rs`, `src/git/log.rs` tests, `src/plugin/manager.rs:~455`, `src/plugin/driver/wasm.rs:~320` | Fix in touched files |
| 5 | V.6 | `first_time_contributors` is in the `git` facade but `list_authors_before` is not. | `src/git.rs:38-41` (RDD R3-REEXPORT-OMISSION) | Fix: add the re-export |
| 6 | — | RDD R3-001: the mock asserts set membership, so a mis-wired operation still passes. | `src/plugin/manager.rs` `MockEchoDriver::invoke` | Fix: consume expected pairs in order |
| 7 | V.4 | `src/plugin/manager.rs` production portion is 381 lines: inside the 400 ceiling, above the 300 target. **Not** a hard-ceiling breach — an earlier reading of "1038" wrongly counted the test module. | `src/plugin/manager.rs` (`mod tests` at line 382) | Follow-up, not this unit |

## Not in scope

- `3710632` was 563 changed lines against the 300-400 review budget. Already committed; recorded, not
  rewritten.
- Decomposing `src/plugin/manager.rs` itself (finding 7).
- **Pre-existing layering inversion: `use crate::cli::BumpLevel;` in `src/bump/exec/pipeline.rs:12` and
  `src/bump/tests.rs:2`.** Present in `HEAD` before this work unit. The domain depends on a
  clap-derived `ValueEnum` defined in `src/cli/args/commands.rs:125`, while the domain already owns
  `semver_bump::Bump` with a `From<Bump> for BumpLevel` conversion in the CLI. This is the same class
  of defect as the one fixed here, but resolving it means moving or wrapping a clap-derived enum
  across the boundary — a separate refactor with its own risk. Recorded as a follow-up.

## Correction to this work unit's acceptance criteria

The original acceptance item read `grep -rn "eprintln\|println\|cli::" src/bump/` must return
nothing. That criterion was **wrong**: it demanded fixing a pre-existing violation that is out of
scope. The correct check is the one the defect was about — terminal I/O and `cli::style` in the
domain:

```
grep -rn "eprintln\|println" src/bump/   -> nothing
grep -rn "cli::style" src/bump/          -> nothing
```

The worker stopped and asked instead of forcing the wider scope, which is the correct behavior.

---

## Acceptance Criteria
- [ ] `PluginOperation` is a typed enum with exhaustive matching and no wildcard dispatch.
- [ ] Capability and operation are coupled so a mismatched pair is unrepresentable at the dispatch layer.
- [ ] The wire envelope still serializes `capability` as `"changelog.v1"` and `operation` as `"render"`; existing plugin payloads and the three process-plugin integration scripts keep working unchanged.
- [ ] No path swallows an error without the degradation being observable.
- [ ] New and touched tests use `tempfile::tempdir()`.
- [ ] `list_authors_before` is re-exported from the `git` facade.
- [ ] `MockEchoDriver` consumes expected pairs in order.
- [ ] Zero clippy warnings with and without `--features plugins`, canonical formatting, green suite.

---

## Task Breakdown
- [x] Task 1: Add `PluginOperation` and `PluginCall`; make `PluginInvocation` typed; thread `PluginCall` through `dispatch_raw` and every driver.
- [x] Task 2: Make the first-time-contributor degradation observable instead of silent (Pillar III.4).
- [x] Task 3: Replace the nested `match` with a guard clause (Pillar V.1) and re-export `list_authors_before`.
- [x] Task 4: Move touched tests to `tempfile::tempdir()` (Pillar IV.1).
- [x] Task 5: Make `MockEchoDriver` consume expected pairs in order (RDD R3-001).
- [x] Task 6: Restore domain boundary purity — the first fix for Task 2 introduced terminal I/O and a
  `bump -> cli` dependency in `src/bump/`, violating Pillar I.1. The domain now returns a data-only
  `ChangelogPlan { update, warnings }`, the pipeline carries warnings into `Summary.warnings`, and a
  single boundary helper renders them. This also made the degradation unit-testable.
- [ ] Task 7: Full verification suite.

---

## Evidence & Verification
- **Branch**: `feat/contract-compliance-hardening`
- **Validation**: pending

---

## Notes

`PluginError::UnsupportedCapability` keeps its `capability: String` field, populated from
`Capability::as_str()`. The distinction is deliberate: the invocation is a domain input and must be
typed, while an error payload is diagnostic output whose shape is already covered by Pillar III.3
(rich, actionable messages). Existing error messages must not change.
