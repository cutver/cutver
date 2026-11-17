# Release notes travel as a stream

Feature: `cutver publish` accepts the release notes on stdin so no transient file has to live in the
worktree, and an empty stream fails closed instead of reaching a plugin as empty notes.
Branch: `feat/release-notes-stream`. Base: `origin/main` at `e6af4c3`.

## Specs

S1. `cutver publish --notes-file -` reads the release notes from standard input. `-` is the stdin
sentinel, not a file named `-`; no file named `-` is opened, created or required.

S2. A streamed run declares no worktree input. `run_publish` passes an empty declared-input slice to
`git::status::require_clean_tree_except`, so every untracked entry still aborts — including one named
`RELEASE_NOTES.md`, and including the case where the caller also left a real notes file in the tree.
The exemption added for #302 continues to apply to the path form only.

S3. An empty stream fails closed. When stdin reaches EOF with zero bytes, `cutver publish` exits
non-zero and writes exactly `release notes on stdin were empty` to stderr; it does not create a tag,
push anything, run `[publish] commands`, or call a post-release plugin. The same must hold when stdin
carries only whitespace: `"   \n\n"` is empty for this rule.

S4. The path form is unchanged. `cutver publish --notes-file RELEASE_NOTES.md` behaves exactly as it
did before this feature for a relative path, `./RELEASE_NOTES.md`, an absolute path and a symlink to
an untracked file — the exemption still applies — and a tracked modified file still aborts.

## Tasks

| ID | Spec | What | Route | Commit |
| --- | --- | --- | --- | --- |
| T1 | S1 | Accept `-` as the stdin sentinel on `--notes-file`, read the stream, and hand the text to the post-release payload without touching the filesystem. | inline | `7535096` |
| T2 | S2 | Keep the clean-tree exemption off the streamed path: empty declared inputs, message unchanged when a stray untracked file is present. | inline | `7535096` |
| T3 | S3 | Reject an empty or whitespace-only stream before any write, with the exact stderr text. | inline | `7535096` |
| T4 | S4 | Pin the path form: the four `#302` spelling tests and the tracked-file refusal stay green in the same run. | inline | `7535096` |
| T5 | S1-S3 | Update `docs/reference/cli.md`, `docs/internals/pipeline.md` and the `publish` help text. | inline | `7535096` |

Verification for the slice (all green, one run): `cargo fmt --all --check`; `cargo clippy --all-targets --features plugins -- -D warnings`;
`cargo test --workspace`; `cargo test --workspace --features plugins`. RED was observed first: the four new tests failed
with `failed to read '-': The system cannot find the file specified. (os error 2)`, then `release_deferred_publish`
went to 18/18.

Declared limit: the payload content itself is not asserted end to end for the closing phase — the fixture has no
plugin that echoes `notes`. The bytes are proven by symmetry (a non-empty stream publishes, an empty one is refused)
and the payload wiring is pinned for the inline phase by `tests/e2e_lifecycle.rs:1014-1039`.

## Log

L1. User, verbatim: «Mientras termina el ci, como podemos haer que cutver evolucione para las release notes y el mejor release engeniering»

L2. User, verbatim: «arranca lo lo que dices»

L3. Decision (parent): the first slice is the one named first in the proposed order — `#306`, notes as a
stream — together with the fail-closed rule for missing notes, because both are the same seam: the
notes handoff contract. Audit items F-12/F-13, template sections, `changelog verify`, channels and
`release plan` stay out of this document and get their own later.

L4. Evidence: `src/cli/args/commands.rs:66-69` declares `--notes-file`; `src/bump/exec/publish.rs:112-120`
collects it into `declared_inputs` for the clean-tree guard; `src/bump/exec/publish.rs:192-213` reads it
and yields `Option<String>` notes, `None` when absent; `src/git/status.rs:27-70` documents the #302
exemption that this feature narrows for the streamed case.

L5. Done in `7535096`. The public API did not change: `PublishRequest.notes_file` keeps its shape and `-`
is interpreted at read time, so no consumer of the crate breaks. The `.github/workflows/release.yml` step
that still renders into `RELEASE_NOTES.md` was deliberately left alone: switching CI to the stream is the
next slice, not this one, because the released binary that cuts a release would not know the sentinel yet.
