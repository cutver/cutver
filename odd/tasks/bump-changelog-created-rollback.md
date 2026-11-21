# #289 — a changelog the run created is removed by rollback

The issue as filed says a run can create `CHANGELOG.md` and leave it behind after a failed bump, because
`src/bump/exec/changelog.rs` recorded a backup only when `original` was `Some`. **Half of that premise is wrong and the
half that is wrong changes the fix.**

Verified on `main@3f6b40a`:

- `default_changelog_path()` returns `None` (`src/config/types.rs:279`), so a config without `[changelog] path`
  declares no changelog and `prepare_changelog` returns `ChangelogPlan::none()` without reading anything.
- `cutver init` scaffolds `CHANGELOG.md` with a starter (`src/init/scaffold/runner.rs:109,151`).
- With `[changelog] path` declared and the file absent, `prepare_changelog` called `read(cl_path)?` — a plain
  `fs::read_to_string` mapped to `Error::Read` — so the bump **aborted before writing**. A run therefore never reached
  the state "it created the changelog", which is why no rollback could undo it: the hole was unreachable, and the
  issue's AC1 ("a test that starts with no changelog, fails the bump after the changelog is written") cannot be RED on
  `main` for the reason it names.

So the defect is not "rollback forgets a created file"; it is that the changelog sink and the template sink disagreed
about what an absent target means. `#261` already settled the answer for templates: absent is a normal state
(`read_existing`: `NotFound → None`, anything else → `Error::ReadUnreadable`), the run generates the file, and rollback
removes it because the transaction's backup is an `Option` whose `None` means "there was nothing here"
(`src/bump/exec/transaction.rs:33,130-141`).

## Specs

**S1 — an absent declared changelog is a normal state.** `prepare_changelog` reads through `read_existing`, and renders
the entry against empty content when the file is absent. A read failure other than `NotFound` still fails closed with
`Error::ReadUnreadable` naming the path — an unreadable changelog is never backed up as empty.

**S1b — `plan` reads it the same way.** `preview_notes` (`src/bump/exec/changelog.rs:263-271`) reads through `read_existing` too and renders against empty content, so a declared-but-absent changelog does not make `plan` fail where `bump` would create: the module comment that calls `render_entry` "the single render authority ... neither can drift from the other" stays true, and `plan` keeps reporting the notes the run would write. A plan still creates nothing.
**S2 — the run records the absence it created.** `apply_changelog` calls
`transaction.record_backup(update.path.clone(), update.original.clone())` unconditionally, so `None` reaches the
transaction and rollback removes the file. `git grep -n "if let Some(orig) = update.original" src/bump/exec/` prints
nothing.

**S3 — one reader, not two.** `read_existing` has exactly one definition, in `src/bump/exec/command.rs`, shared by
`manifest.rs` and `changelog.rs`. Its doc states what `None` means and why an unreadable file must not become empty.

**S4 — a changelog the run created is gone after an abort, and the failure is the one the operator caused.** The new
test starts with `[changelog] path = "CHANGELOG.md"` and **no file**, makes a later step fail (`[hooks] post_bump =
"exit 1"`), and asserts two things in this order: the error is the hook failure (not `Error::Read`, i.e. the run truly
created the changelog), and the file does not exist afterwards. It must be RED on `main` at the first assertion for a
reason the message states.

**S5 — an existing changelog still restores byte-identical.** `tests/e2e_lifecycle.rs:355`
(`post_bump_failure_triggers_rollback_and_aborts`) stays green untouched, and so does
`tests/template_manifest.rs:313` for the template sink.

**S6 — a rehearsal writes nothing and records nothing.** Dry run keeps `original: None, updated: None`, so
`apply_changelog` neither records nor writes; no test in this change may make a dry run touch the tree.

**S7 — budgets.** `src/bump/exec/changelog.rs` must stay at or below 400 lines (385 after the product change, 15 lines
of headroom, and it is not allowlisted). The new test lives in a file that runs on **all three hosts**: do not put it in
`tests/e2e_lifecycle.rs`, which is `#![cfg(unix)]` and therefore runs zero tests on Windows.

## Tasks

- **T1** — the tests: S4's created-then-removed case in a host-agnostic integration file, plus the RED proof against the
  base tree. Evidence: RED output naming the read failure, then GREEN.
- **T2** — S1, S2, S3, S7: the product change (already applied in the working tree; the worker verifies it against the
  specs rather than rewriting it) and the shared-reader check.
- **T3** — S5, S6: the two regression suites and the dry-run claim, run focused.

## Log

- **L1** (maintainer, verbatim): «continuemos con lo que tenemos pendiente mientras arreglo una clase de mi hermana,
  estudio para una exposición y un examen».
- **L2** (maintainer, on the two inconsistent sinks): «Sí: test + gate + PR» — decision recorded after the minimal diff
  (+29/−25) was shown: the changelog sink adopts the template sink's contract.
