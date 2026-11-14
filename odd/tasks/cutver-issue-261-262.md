# cutver issues #261 + #262 — rollback safety and remote-tag fail-closed

Branch: `fix/261-262-rollback-lsremote` (worktree `D:/development/workspaces/cutver-261-262`, base `main@2c1c2b0`).
Two P1 contract violations, one work-unit commit each.

## Specs

### #261 — fix(bump): never back up an unreadable manifest as empty, and report rollback failures

S1. Contract: Pillar I.2 — "if any write, hook, or staging step fails, rollback restores every touched file to its original byte-for-byte content" — and Pillar III.4 (*Zero Swallowed Errors*).

S2. Evidence (verified on `main@2f8805b`):
- `src/bump/exec/manifest.rs:138`: `let original = std::fs::read_to_string(m.path.as_str()).unwrap_or_default();`
- That `""` becomes the recorded backup (`manifest.rs:158`, `:167`).
- `src/bump/exec/transaction.rs:41` `let _ = atomic::write_atomic(path, original);` — the drop guard writes it back, and `transaction.rs:50` discards the `git reset` result the same way.
- `src/bump/exec/doctor.rs:92` repeats the same read pattern.

S3. Failure mode: an existing-but-unreadable target (permissions, invalid UTF-8, transient `EIO`) is backed up as empty; a later failure in stage, commit or a hook turns a real manifest into a zero-byte file. An absent target becomes a created empty file on rollback. The rollback then fails silently, so the operator sees only the original error and has no idea the tree is corrupt.

S4. Remedy:
- The read must fail closed with a typed error (What / Where / Fix, Pillar III.3) **before** the transaction records a backup.
- Distinguish *absent* (legitimate: a new file to create) from *unreadable* (abort, Phase 1, nothing written).
- Record rollback failures in `Summary` and surface them on stderr — Pillar III.4 requires every ignored condition to be logged or tracked with its rationale. No `let _ =` may remain in `transaction.rs`.

S5. Acceptance criteria:
- [ ] A test makes the sink unreadable (mode `000` or non-UTF-8 bytes) and asserts the command aborts before any write, with the original bytes intact.
- [ ] A test forces a post-write failure and asserts the restore failure appears in `Summary` and on stderr.
- [ ] A test asserts an absent target is created and removed cleanly, without being recorded as an empty backup.
- [ ] `git grep -n "let _ =" src/bump/exec/transaction.rs` returns nothing.

### #262 — fix(git): fail closed when the remote tag check cannot reach the remote

S6. Contract: Pillar II.2 — "if a target tag already exists locally or remotely, fail closed (*fail-closed*) before touching files, with an actionable resolution message".

S7. Evidence (verified on `main@2f8805b`), `src/git/tags.rs:30-40`:
```rust
if !has_remote(&repo, remote) {
    return Ok(None);
}
let output = run_git(&repo, &["ls-remote", "--tags", remote, &tag_ref, &peeled_ref])?;
if !output.status.success() {
    return Ok(None);
}
```

S8. Failure mode: an existing remote that cannot be reached — offline CI, expired token, DNS failure, proxy — reads as "no remote tag". `check_tag_collisions` passes, the release writes the manifests, commits and tags locally, `git push` then fails, and the retry aborts with `TagExists`. The operator is left with a local commit and tag that must be cleaned up by hand.

S9. Remedy:
- Split the two cases: *no remote configured* is a legitimate `Ok(None)`; *remote configured but unreachable* is a typed error carrying the remote name, git's stderr and the fix (Pillar III.3).
- Never map a failed subprocess status to a successful domain answer.

S10. Acceptance criteria:
- [ ] A test with an unreachable remote asserts a typed error and asserts that no file was written (Phase 1 of Pillar I.2 stays intact).
- [ ] A test with no remote configured still succeeds as it does today.
- [ ] The error names the remote, quotes git's stderr and suggests the next step.
- [ ] The aborted run leaves the workspace byte-identical.

## Tasks

- T1 · S1-S5 · inline/worker · commit 1 — fail-closed manifest read, absent-vs-unreadable distinction, rollback failures in `Summary` + stderr, no `let _ =` in `transaction.rs`.
- T2 · S6-S10 · inline/worker · commit 2 — typed `ls-remote` failure carrying remote, stderr and fix; keep `Ok(None)` only for "no remote configured".

## Log

L1 (assignment, verbatim):

> Session message from 01a1d7fb-9488-773b-8c07-358c43ebb75b (correlation 92fec45b13564a8b877ff4a9cd43a76e): ASSIGNMENT — Track 2 (Par 2) of the cutver remediation queue. Only you touch these files.
>
> Your worktree is ALREADY CREATED: D:/development/workspaces/cutver-261-262, branch fix/261-262-rollback-lsremote, based on main @ 2c1c2b0. Work THERE, never in D:/development/workspaces/cutver — another session owns that tree and concurrent edits corrupt both (shared index and HEAD).
>
> First two things: (1) session_worktree_register("D:/development/workspaces/cutver-261-262"); (2) publish state via orchestrator_session_id (objective + work.topic) — this session never published any.
>
> Issues, in this order, one work-unit commit each:
> 1. #261 fix(bump): never back up an unreadable manifest as empty, and report rollback failures. Evidence: src/bump/exec/manifest.rs:138 read_to_string(..).unwrap_or_default(); src/bump/exec/transaction.rs:41 and :50 are `let _ =`; same read pattern src/bump/exec/doctor.rs:92. Risk item 1: a zero-byte backup destroys the manifest on rollback and the failed restore is silent.
> 2. #262 fix(git): fail closed when the remote tag check cannot reach the remote. Evidence: src/git/tags.rs:36-40 returns Ok(None) on a failed ls-remote, which reads as "no remote tag" and leads to commit+tag then a push failure.
>
> Read each issue body before writing: it carries priority, the violated CONTRACT.md pillar, path:line evidence, failure mode, contract-constrained remedy and acceptance criteria. The issue is the spec.
>
> Hard boundaries: do NOT touch src/git/types.rs (belongs to #267); do not touch src/config/**, src/cli/doctor.rs, src/bump/exec/hooks.rs; do not sweep other unwrap_or_default/let _ = sites (#266 is a later separate issue).
>
> Method: read-only exploration first; then test-first — one RED test per requested rule plus one test proving the previous behavior of anything existing you touch. Fixtures that run shell plugins must grant PATH (lesson from #264). Gates: cargo build, cargo test --workspace, clippy --features plugins -- -D warnings, fmt --check.
>
> Tracking: large tracked task, so create odd/tasks/cutver-issue-261-262.md (specs quoting the issue bodies, task lines, Log starting with this instruction) and commit it with the work.
>
> Delivery: commit on your branch; push, PR and merge are the human's decision. Two caveats: native review cannot reach terminal approval right now (correction-plan binding defect), so any merge needs explicit human authorization; and never put `Closes #NNN` in a PR body unless every acceptance criterion of that issue is inside that same PR.
>
> Report back: which acceptance criteria you covered, the commits, each gate's exit code, and what you deliberately left out.

L2 (T1 evidence, #261):

- RED: `cargo test --lib bump::exec::transaction` -> 11 compile errors (no `Error::ReadUnreadable`, no `MutationTransaction::abort`, `record_backup` still took `String`). Then `cargo test --test template_manifest` failed `an_unreadable_sink_aborts_before_any_write` with `Git(Guard { detail: "working tree has uncommitted changes:\n?? src/version.ts" })`: the fixture wrote the unreadable sink after the last commit, so `require_clean_tree` fired first. Fixed the fixture (commit the unreadable bytes) — the assertion itself was never weakened.
- GREEN: `cargo test --lib bump::exec::transaction` 3 passed; `cargo test --test template_manifest` 8 passed; `cargo test --workspace` 418 lib + every integration suite passed, 0 failed.
- Deviation (S5, second criterion): the restore failure cannot appear *in `Summary`*. `pipeline::run` returns `Result<Summary, Error>` and `Summary` is only constructed on the success path, while a rollback failure only exists on the error path. Adding a `Summary` field would be dead state, so the failure travels in `Error::RollbackFailed` instead, which the CLI prints to stderr through `print_error` (`cli/runner/dispatch.rs`). The intent of the criterion — the operator learns the tree may no longer match its committed content, on stderr — is met; the literal word `Summary` is not.
- Platform note: `tests/e2e_bump.rs` is `#![cfg(unix)]`, so the new end-to-end restore-failure test compiles to zero tests on Windows and could only be reviewed, not executed, here. The same mechanism is pinned cross-platform by the `transaction` unit tests.
- Deliberately left out: `exec/changelog.rs` still records a backup only when the changelog existed, so a newly created changelog is left behind by rollback (out of scope for S4, same family, reported rather than swept); `has_remote` swallowing its subprocess result belongs to T2; the `unwrap_or_default` sweep outside these files is #266.

L3 (T1/T2 boundaries, #262 evidence):

- Slice boundary: commit 1 holds T1 together with this document (531 authored changed lines, past the ~400-line review slice). T2 is its own commit so each can be reviewed alone; neither commit is pushed.
- RED: `cargo test --test remote_tag_check` -> `error[E0599]: no variant named 'RemoteTagCheck' found for enum 'cutver::bump::Error'`.
- GREEN: `cargo test --test remote_tag_check` 2 passed; `cargo test --lib git::tests::remote_tag` 2 passed; `plugin::error::tests::plugin_error_stays_within_the_bump_result_budget` still passes with the wider error enum.
- S10 mapping: AC1+AC4 -> `an_unreachable_remote_aborts_before_any_write` (typed error, workspace snapshot byte-identical, no commit, no tag); AC2 -> `a_repository_without_a_remote_still_bumps` plus `remote_tag_lookup_answers_absent_without_a_remote` (both portable) and the pre-existing unix-gated `test_remote_tag_exists_handles_missing_remote_and_tags`; AC3 -> the same test asserts the remote name, git's quoted stderr and the `git ls-remote <remote>` next step.
- `RemoteUnreachable` lives in `src/git/tags.rs` and `bump::Error::RemoteTagCheck` carries it, so `src/git/types.rs` (issue #267) is untouched.
- Deliberately left out: `git::tags::has_remote` still answers `false` for a `git remote` that failed to run, so a broken git binary still reads as "no remote configured". That is a different failure from an unreachable remote (the local command never reaches the network), it is not reachable from the criteria in S10, and the same-class sweep is #266; recorded here instead of widened silently.
- Platform: `tests/e2e_bump.rs` is `#![cfg(unix)]`. Its new test was validated with `cargo check --tests --target x86_64-unknown-linux-gnu` (exit 0) and cannot be executed on this Windows host; the `transaction` unit tests pin the same mechanism cross-platform.
