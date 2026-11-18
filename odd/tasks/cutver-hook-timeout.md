# Hooks cannot hang cutver forever

Feature: audit F-16. `[hooks] pre_bump` and `post_bump` are spawned with `.status()`, so a hook that never
exits blocks cutver with no timeout and leaves its process tree alive. Branch `fix/hook-timeout`, base
`origin/main` at `b117673`.

## Specs

S1. A hook that does not exit within its timeout is killed and the release fails with a typed error naming
the hook, its command and the timeout. Today `src/bump/exec/pipeline.rs` spawns both hooks with `.status()`
and waits forever.

S2. The kill reaches the whole process tree, not only the direct child, using the same machinery the
preflight and `[publish].commands` paths already carry. A hook's descendant must not survive the timeout.

S3. A hook that exits non-zero keeps its current error type and exit code; only the never-exiting case gains
a new failure. A hook that finishes inside the timeout behaves exactly as it does today.

S4. The timeout value comes from configuration and is consistent with the other timeouts the tool already
declares (`[preflight] default_timeout`, `[publish] default_timeout`): the same default, the same unit, and a
hook-specific key only if the existing tables genuinely have no honest home for it. State the choice and why.

S5. A `post_bump` timeout tells the truth about what already happened. `[hooks] post_bump` runs **before**
the release commit (`src/bump/exec/pipeline.rs`), so no commit exists when it is killed: the error says the
hook was killed after the manifests and the changelog were written and before the release commit, and that
the rollback restores every file the run wrote. The post-commit timeout case belongs to
`[publish].commands`, which already names its commit and tag through `PublishAfterCommit` and is pinned by a
test rather than changed here. A release must never look successful when the last step was killed.

## Tasks

| ID | Spec | What | Route | Commit |
| --- | --- | --- | --- | --- |
| T1 | S1, S2 | Run both hooks through the bounded spawn the other paths use, so a hang is killed with its tree. | worker | pending |
| T2 | S4 | Resolve the timeout from configuration, matching the existing tables. | worker | pending |
| T3 | S3 | Keep the non-zero-exit behaviour and the happy path identical. | worker | pending |
| T4 | S5 | Name the commit and the completed steps when `post_bump` is killed. | worker | pending |
| T5 | S1-S5 | Pin all of it with tests that run on every host, and update the docs for the timeout. | worker | pending |

## Constraints for the worker

- Prefer Windows-runnable tests. `#![cfg(unix)]` files execute zero tests on this host; if the tree-kill can
  only be observed on unix, say so explicitly and keep the observable behaviour pinned cross-platform where
  possible.
- Do not invent a second kill mechanism: `src/preflight.rs` and `src/bump/exec/command.rs` already carry it,
  and audit F-16 exists because that machinery is triplicated. Reusing one of them is the fix; adding a
  third copy is the defect.
- Out of scope: F-12's remaining halves, F-14, F-15.

## Log

L1. User, verbatim: «Sí, cuando marque verde mergea y sigue con el siguiente slic»

L2. Decision (parent): F-16 next, ahead of the remaining F-12 halves, because it is the only open finding
where the user loses control of the process: a hanging `pre_bump` blocks cutver with no timeout and no kill
tree, and `post_bump` leaves a live child behind a half-applied transaction. The others convert read errors
into empty values; this one stops the tool from finishing at all.

L3. Slice chain so far, for continuity: `#309` `4e07746` (promo film docs), `#310` `fe88de1` (notes as a
stream, closes #306), `#311` `9467646` (grouped template sections), `#312` `3e61c42` (changelog prefix
authority, F-13 read surface), `#313` `b117673` (doctor prefix, F-13 closed, plus the local documentation
line-ceiling gate).

L4. Spec correction, recorded because S5 was written wrong. S5 originally required a timed-out `post_bump`
to name the commit it created. `[hooks] post_bump` has always run *before* `git::commit_ext`, so no commit
can exist at that moment: the criterion was unsatisfiable, not unimplemented. Resolution: the error states
the truth (killed after the manifests and the changelog were written, before the release commit, rollback
restores what the run wrote), and the genuinely post-commit timeout — `[publish].commands` — keeps the
`PublishAfterCommit { commit, tag }` wrapper it already had, now pinned by a test instead of assumed.
Moving `post_bump` after the commit was rejected: the hook's writes and the lockfiles it stages would stop
being part of the release commit.

L5. `[hooks] default_timeout` is additive and optional, defaulting to unbounded, matching
`[preflight] default_timeout` and `[publish] default_timeout` in unit and default. It does not reuse either
of them: bounding preflight to let a slow hook finish would silently re-bound unrelated release steps.
`cutver init` does not scaffold it yet — the reference doc is the only advertisement, recorded as a
follow-up.
