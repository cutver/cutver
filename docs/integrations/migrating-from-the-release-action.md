# Migrating from the `cutver/release` Action

`cutver/release@v1` is deprecated: still published, still usable for existing workflows, no longer the
recommended way to release. This page states what the action does today, why it cannot cut the release
this tool requires, and the commands that replace each of its inputs and outputs.

`cutver/setup@v1` is unaffected. It stays the way to install the CLI on a runner
([GitHub Actions integration guide](github-actions.md)).

---

## What the action actually does

Read from the action's own `action.yml` at the root of the `cutver/release` repository:

- **It runs a `cutver` already on `PATH`.** `src/runner/cutver-runner.ts` calls `exec.exec('cutver', …)`,
  so the binary the action drives is whatever your workflow installed — with `cutver/setup@v1`'s
  `version` default of `latest`, the last published release rather than the code in your checkout.
- **`command` selects one of four pipelines**: `release` (the default), `bump`, `changelog`, `doctor`.
  `release` runs doctor, then bump, then `changelog latest`, and returns. `bump` stops after the bump;
  `changelog` and `doctor` run only their own command.
- **It never creates the GitHub Release itself.** No command in the set creates a release record, and
  `action.yml` declares no such output. Whatever the bump triggers through `[publish].commands` and
  `on_post_release` is your `cutver.toml` doing it, not the action. The release body in the old examples
  came from a separate `softprops/action-gh-release` or `gh release create` step, which is why every
  consumer already had to add a follow-up step of their own.
- **There is no `publish` command and no `--defer-publish` input**, and no `--report` input either.
  `version` and `tag` are recovered by comparing `git rev-parse HEAD` before and after the bump and
  reading `git describe --tags --match=v*.*.*`; nothing machine-readable is emitted.

---

## Why that cannot cut the release this tool requires

`cutver` splits a release in two phases: `bump --defer-publish` prepares the release commit and pushes
the branch, and `publish` creates the tag, moves the floating major tag, runs `[publish].commands` and
dispatches `on_post_release` ([Release Pipeline Order and Atomic Guarantees](../internals/pipeline.md)).
The split exists so the artifacts built from the release commit are verified before anything is tagged
or published.

The action cannot express that boundary: it can run the bump, but only as one pass that closes the
release itself — the tag, the pushes and the publish commands all fire inside it, before your build job
has seen the commit — and it has no way to call `cutver publish` afterwards. It also pins no CLI version
of its own, so the binary that cuts the release is whatever `cutver/setup` last resolved. This
repository's own release workflow builds the CLI from the commit being released, puts it first on
`PATH`, and only then runs `cutver bump` (`.github/workflows/release.yml:103-166`).

---

## Inputs, before and after

| Action input | What the action did with it | CLI equivalent |
| :--- | :--- | :--- |
| `command: release` (default) | doctor, then bump, then `changelog latest` | three steps: `cutver doctor --check-changelog`, `cutver bump <level> …`, `cutver changelog latest …` |
| `command: bump` | `cutver bump` only | `cutver bump <level> [--dry-run] [--first-release] [--skip-preflight <step>]` |
| `command: changelog` | `changelog latest`, written to `notes-file` | `cutver changelog latest --template <path> > <file>` |
| `command: doctor` | `cutver doctor` | `cutver doctor [--check-changelog]` |
| `bump` (default `auto`) | level passed as `cutver bump <level>` | the `bump` positional argument |
| `first-release` | `--first-release` on the bump | `cutver bump <level> --first-release` |
| `config` | `-c <path>`, passed to the command and again to `changelog` | `-c <path>` on each `cutver` invocation |
| `dry-run` | `--dry-run` on the bump | `cutver bump <level> --dry-run` |
| `skip-preflight` | comma-separated steps, one `--skip-preflight` each | `--skip-preflight <step>`, repeated |
| `check-changelog` (default `true`) | `--check-changelog` on doctor | `cutver doctor --check-changelog` |
| `notes-file` (default `RELEASE_NOTES.md`) | file the notes were written to | the shell redirection target |
| `template` | `--template <path>` on `changelog latest` | `cutver changelog latest --template <path>` |

The outputs become one file: `cutver bump --report <path>` writes `released`, `deferred`, `version`,
`tag`, `commit`, `floating_tag`, `publish_push_command` and `publish_commands`
([CLI reference](../reference/cli.md#cutver-bump)).

| Action output | Replaced by |
| :--- | :--- |
| `released` | the `released` field of the report; `false` on a `--dry-run` and on a [no-op](../reference/cli.md#no-op) |
| `version` | the `version` field, stated by the bump instead of recovered with `git describe` |
| `tag` | the `tag` field, same |
| `notes-path` | the file you redirected `changelog latest` into |
| `release-notes` | that file's contents, streamed into `cutver publish --notes-file -` |

---

## The recipe

```bash
cutver doctor --check-changelog
cutver plugin restore                     # only when cutver.toml declares plugins
cutver bump auto --defer-publish --report .cutver-release.json
jq -r .commit .cutver-release.json        # the commit the artifacts must be built from
cutver changelog latest --template .github/templates/cutver/RELEASE.md > RELEASE_NOTES.md
# build and smoke the artifacts from that commit
cat RELEASE_NOTES.md | cutver publish --tag "$TAG" --commit "$COMMIT" --notes-file -
```

`--notes-file -` reads the notes from standard input, and an empty stream is refused before the tag is
created, so a failed render cannot publish a release with no body. The full workflows are in the
[GitHub Actions integration guide](github-actions.md) and the
[Production Release Workflow](production-release-workflow.md).

A step-by-step migration is fine: workflows that used only `command: doctor` or `command: bump` can be
moved by replacing the `uses: cutver/release@v1` block with the matching command and nothing else.

---

## Related Documentation

- [GitHub Actions Integration Guide](github-actions.md): `cutver/setup@v1` and the CLI release workflow.
- [Production Release Workflow](production-release-workflow.md): the bot-token workflow with a deferred bump.
- [CLI Commands and Exit Codes](../reference/cli.md): every flag, the release report, and exit codes.
- [Customizing Release Templates](../templates/release-notes.md): the notes file the publish step consumes.
