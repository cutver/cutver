# Pipeline Execution & Atomic Guarantees

> Outcome first: `cutver` executes release operations through an atomic two-phase commit pipeline, ensuring manifests and Git state are never left in a partially mutated or corrupted state.

This document describes internal process isolation, two-phase transaction execution, rollback boundaries, and lockfile staging guarantees. Architectural decisions live in [Architecture & Design Decisions](design.md).

---

## Configuration Discovery Boundary

When invoked without an explicit `-c / --config` flag, `cutver` locates its configuration using an upward filesystem walk:

1. Starts at the current working directory.
2. Traverses upward through parent directories, searching for `cutver.toml`.
3. Halts traversal immediately upon encountering a Git repository boundary (`.git` directory or submodule file) to prevent leaking into outer environments.
4. Resolves all configured paths relative to the directory containing `cutver.toml`, ensuring consistent behaviour regardless of the invocation subfolder.

---

## Two-Phase Atomic Architecture

The release engine partitions work into four discrete execution phases:

1. **Guard Phase**: Validates repository state before modifying disk. Confirms a clean working tree (`require_clean_tree = true`) and target branch (`require_branch`), then verifies the target Git tag does not already exist locally or remotely.
2. **Preflight Phase**: Executes configured verification commands in strict declaration order. Aborts execution immediately upon any command failure or timeout.
3. **Compute Phase (In-Memory)**: Evaluates all version increments, manifest modifications, and changelog bodies entirely in memory. Zero file mutation occurs during this phase.
4. **Apply Phase (Atomic Swap)**: Writes staged changes to unique temporary files in the same directory (`.{name}.cutver-tmp-{pid}-{seq}`), forces buffers to disk via `fsync` (`sync_all()`), and renames temporary files over targets using atomic filesystem replace operations.

### Replace-Rename Promise

The final swap is a replace-rename whose success depends on the platform, so it is bounded rather than unconditional:

- **POSIX (Linux, macOS)**: `rename` replaces the destination even while other handles hold it, so a single attempt is enough and nothing is retried.
- **Windows**: `MoveFileEx` with replace-existing fails with `ERROR_ACCESS_DENIED` or `ERROR_SHARING_VIOLATION` while another handle holds the destination without delete sharing — an editor, an indexer, an antivirus, or a concurrent writer of the same path. `cutver` retries up to three attempts, 20 ms apart, and only for those two codes.

Any other error, and an exhausted retry, fail closed in the same way: the destination keeps its previous contents, the temporary file is removed by its drop guard, and the error names both paths.

> [!NOTE]
> The bound exists because `cutver` rewrites manifests and changelogs that a human may have open in an editor while the release runs. Declaring the concurrent replace best-effort on Windows instead would leave that real case undocumented and untested, with the primitive the whole release transaction writes through carrying a claim the platform does not keep unconditionally.

---

## Transaction Boundaries & Rollback Guarantees

If a filesystem write, hook execution, or Git staging command fails **prior to the release commit**, `cutver` triggers best-effort RAII rollback:

- Restores modified manifests to their original in-memory snapshots.
- Unstages staged Git index additions (`git reset HEAD`).
- Removes lingering temporary files via drop guards.

> [!WARNING]
> **Boundary Limitation**: Transaction safety ends at `transaction.commit()`. Once the release commit is recorded in Git, subsequent tag creation and remote publication cannot be rolled back atomically. Failures past this point are reported as actionable errors rather than rolled back.

---

## Preflight Process Isolation & Timeouts

Preflight verification steps enforce fail-fast execution and strict process tree termination:

- **Declaration Order**: Commands execute sequentially; any non-zero exit status aborts the release before disk modification.
- **Unix Process Groups (Linux & macOS)**: Commands spawn inside a distinct process group (`setpgid`). Upon reaching the configured timeout, `SIGKILL` is sent to `-pgid`, terminating the command and any spawned descendant processes to eliminate orphaned runners.
- **Windows Process Trees**: Commands are assigned to a job object at spawn, and reaching the timeout ends that job in one act with `TerminateJobObject`, which terminates the complete process hierarchy. A host that refuses the job object falls back to `taskkill /F /T /PID <pid>`.

---

## Lifecycle Hooks & Lockfile Allowlist

The `[hooks] post_bump` command executes immediately after manifests and changelog are written to disk, but **prior** to Git staging and commit.

### Automatic Lockfile Staging

To eliminate accidental staging of unrelated workspace changes, `cutver` stages only recognized lockfile artifacts regenerated by `post_bump`:

- **Cargo**: `Cargo.lock`
- **JavaScript / TypeScript**: `package-lock.json`, `pnpm-lock.yaml`, `yarn.lock`, `bun.lock`, `bun.lockb`
- **Gradle**: `gradle.lockfile`
- **Python**: `poetry.lock`, `Pipfile.lock`, `uv.lock`, `pdm.lock`
- **PHP**: `composer.lock`

Arbitrary untracked files and unstaged edits outside declared manifests and the lockfile allowlist are left untouched in the working directory.

---

## Two-Phase Release: Prepare, Verify, Publish

`cutver bump` runs the whole release in one process, tag included. A release whose artifacts must be built and smoked first is cut in two instead, because a smoke failure has to publish nothing (issue #259):

> [!IMPORTANT]
> Nothing irreversible — no tag, no tag push, no floating tag move, no `[publish]` command, no `on_post_release` dispatch — happens before the artifacts built from the release commit pass their smoke.

**Prepare** — `cutver bump <level> --defer-publish`:

1. Guard, preflight, in-memory compute, atomic apply, hooks, staging and the commit, all as above.
2. `git push origin refs/heads/<branch>`, only when `[publish] push = true`. It is the only push the prepare phase makes, and it is what makes the release commit reachable by the jobs that build and smoke it.
3. Stop. The summary tree marks the tag `[DEFERRED]` and names the `cutver publish --tag <TAG>` command that finishes the release.

**Publish** — `cutver publish --tag <TAG> [--commit <SHA>] [--notes-file <PATH|->] [--dry-run]`, after the caller verified the artifacts:

1. Validate, before anything is written or pushed: the working tree is clean (`require_clean_tree`, with the file named by `--notes-file` exempt because it is the command's input rather than a change in the tree — `--notes-file -` reads the notes from standard input instead, which exempts nothing because nothing has to exist in the tree, and an empty stream is refused here), the tag is a valid ref, it equals the version the manifests declare at `HEAD`, `--commit` (when given) is `HEAD`, and no release tag exists locally or remotely.
2. Create the annotated tag on `HEAD`.
3. `git push origin refs/tags/<release tag>`, only when `[publish] push = true`.
4. Move the floating major tag onto the release commit when `[git] floating_major_tag = true`, and force-push it when `[publish] push` is enabled too.
5. Run `[publish].commands` in declaration order.
6. Dispatch the `on_post_release` plugins, which is what creates the draft release.

Steps 2 to 6 are the code the single-process pipeline runs inline, in that order: it creates the tag first and only then pushes the branch, so a tag that cannot be created never leaves the release commit on the remote. One implementation, two entry points. Without `--defer-publish` the tail still runs in the bump process, so the split is a choice at cut time rather than a new default.

**The residual, stated plainly.** The crate is published by `[publish].commands` (step 5), while the release assets are uploaded and the draft is promoted by CI afterwards. An asset-upload failure therefore leaves a published crate version with a draft nobody can see. Nothing was lost and nothing needs re-cutting: re-run the asset upload and the promotion, which clobbers the assets and treats an already-published release as a no-op. `cutver publish` itself now fails closed on the tag it already created, and a crate version can be yanked but not withdrawn.

`cutver publish --resume --commit <sha>` re-enters the closing phase of a release whose steps were left undone, and it repeats only what is harmless to repeat. `[publish].commands` is not in that set: a resume refuses to run an entry without a `publish-<n>` success attestation covering the release commit, and it names the entry by its 1-based position in `[publish].commands`. Recording that attestation is what authorizes the repeat, so it must not be recorded for a crate that already went out — this residual is finished by the upload and the promotion, never by attesting a resume over it. `tests/release_deferred_publish.rs` pins both the refusal and the attestation that lifts it.

---

## Publish Ordering

`publish.push = true` sends three pushes in dependency order, one ref each. The branch push belongs to the run that creates the release commit; the two tag pushes are the closing phase of a two-phase run, and the tail of the same run otherwise. The annotated tag is created before the first of them, so a tag that cannot be created pushes nothing.

1. `git push origin refs/heads/<branch>`
2. `git push origin refs/tags/<release tag>`
3. `git push origin +refs/tags/<floating major tag>`, only when `[git] floating_major_tag = true`

No push carries `--tags`, and the floating-major-tag refspec is the only forced ref
(`branch_push_args`, `tag_push_args` and `tag_force_push_args` in `src/git/ops.rs`). A batched
`push --tags` put the branch and every local tag in one receive-pack transaction, so one rejected
tag rolled the branch update back with it: issue #263 left `main` unmoved, the release tag
uncreated and the floating `v0` tag orphaned on a commit that never reached `main`.

A rejected push returns `git::Error::PushRef`, naming the refspec and the step that stopped, so the
remote state after an abort is predictable:

| Rejected step | Remote and local state |
| :--- | :--- |
| Branch | nothing pushed; the floating tag did not move; no publish command ran |
| Release tag | the branch moved; the floating tag did not move; no publish command ran |
| Floating major tag | branch and release tag published; no publish command ran |

> [!NOTE]
> A detached HEAD cannot name a branch. The push then aborts with
> `git::Error::PushBranchUnresolved` before anything is sent; set `[git] require_branch` to name the
> released branch explicitly.

---

## Floating Major Tags Mechanics

When `floating_major_tag = true` is enabled in `[git]`:

1. **Tag Filtering**: Floating tags matching `^[vV]?[0-9]+$` are ignored during `git::latest_tag` queries. Commit distance and SemVer bumping always calculate against full release version tags rather than floating major tags.
2. **Drift Immunity**: `cutver doctor --check-changelog` filters out floating major tags to avoid false changelog drift warnings.
3. **Ordered Tag Update**: Force-updates the local floating tag `vX` to the release commit after the branch and release tag pushes succeeded, then pushes it with a force refspec (`+refs/tags/vX:refs/tags/vX`) when `publish.push = true`. That refspec is the only forced ref in the pipeline.

---

## Source Module Layout

```text
src/
├── main.rs          # Entry point: parse arguments, run pipeline, exit with status code
├── lib.rs           # Module declarations and public library interface
├── atomic.rs        # Same-directory atomic file replacement and fsync routines
├── bump/            # Pipeline orchestration and execution transactions
│   └── exec/        # Submodules: pipeline, publish, transaction, manifest, changelog, command, guard, hooks, doctor
├── cli/             # Clap derive arguments, runners, and styling
├── config/          # cutver.toml discovery, validation, path resolution, preflights
├── conventional/    # Conventional Commits parser and bump deduction
├── changelog/       # Context assembly, section extraction, MiniJinja rendering
├── git/             # Clean-tree verification, branch checks, commits, tags, push
├── init/            # Manifest discovery and cutver.toml scaffolding
├── manifest/        # ManifestEditor trait and format-specific AST/byte implementations
├── plugin/          # Microkernel cache, DTOs, drivers (WASM/process), registry
├── preflight.rs     # Fail-fast command execution and timeout guards
└── semver_bump.rs   # SemVer bump arithmetic over the semver crate
```
