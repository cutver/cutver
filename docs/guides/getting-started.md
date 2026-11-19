# Getting Started with cutver

Ship your first multi-manifest release, with the artifacts verified before anything irreversible happens. Every transcript on this page was captured from a real repository.

## 1. Initialize Your Repository

Run `cutver init` in the root of your project:

```bash
cutver init
```

It detects your package manifests (`Cargo.toml`, `package.json`, `pyproject.toml`, Gradle, etc.) and scaffolds `cutver.toml`, a Keep-a-Changelog `CHANGELOG.md` and a release template at `.github/templates/cutver/RELEASE.md`. Commit those files before releasing: the guard refuses a dirty tree by default.

*Gotcha*: packages you add later are appended by `cutver init --update`, which never overwrites your custom rules or hooks.

## 2. Check Workspace Health

Verify that every declared manifest exists, is readable, and shares one version. `--check-changelog` adds a row comparing `CHANGELOG.md` against your Git tags:

```bash
cutver doctor
cutver doctor --check-changelog
```

```text
# trimmed: the plain run, which prints the same three rows
✔ cutver.toml is valid

  Configuration   valid (cutver.toml)
  Manifests       1 tracked (package.json @ 0.1.0)
  Preflight       1 step defined (test: npm test)
  Changelog       consistent with Git tags
```

Version drift exits `2`, so a script can tell "your files disagree" from "your configuration is broken" (exit `1`):

```bash
cutver doctor
```

```text
✖ Version drift detected (1 manifest(s) out of sync):
  • version.json: expected 0.1.0, found 0.1.3

Suggested fix:
  • Align version numbers manually across all files.
  • Or run 'cutver bump patch' to synchronize all manifests in one step.
```

## 3. Read the Plan

`cutver plan` computes the entire release — target version, manifest diffs, changelog body, commit and tag names, publish steps — and performs none of it:

```bash
cutver plan
```

```text
ℹ Release Plan: 0.1.0 -> 0.2.0 (minor (deduced from 1 feature))
├── Manifests
│   └── package.json (0.1.0 -> 0.2.0)
├── Preflight
│   └── test: npm test
├── Changelog
│   └── CHANGELOG.md (prepends 0.2.0)
├── Git
│   ├── commit: chore(release): v0.2.0 [skip ci]
│   └── tag: v0.2.0
└── Publish
    └── push: true

Nothing was written, staged, committed, tagged or pushed.
```

Read-only means read-only: no file is touched, no preflight step runs and no plugin is dispatched. It exits `0` even when the plan carries blockers, because the release decision is yours.

The level is deduced from the Conventional Commits since your last tag; [Conventional Commit Bump Rules](../internals/references.md) lists every signal with an example commit. An explicit level overrides deduction — `cutver plan minor`, or `patch` and `major` — and the tree then says `(bump)` instead of naming the commits.

## 4. Prepare the Release

`cutver bump --defer-publish` runs the pipeline up to and including the release commit and the branch push, then stops:

```bash
cutver bump --defer-publish
```

```text
# trimmed: Manifests to Changelog, plus the deferral
✔ Release Plan: 0.1.0 -> 0.2.0 (minor (deduced from 1 feature))
├── Git
│   ├── commit: chore(release): v0.2.0 [skip ci]
│   └── tag: v0.2.0 [DEFERRED]
└── Publish
    ├── push: true
    └── deferred: cutver publish --tag v0.2.0
```

That `[DEFERRED]` marker is the whole point: no tag is created, no floating major tag moves, no `[publish]` command runs and no `on_post_release` plugin is dispatched. The release commit is on the remote branch — `[publish] push = true`, the scaffolded default — so a build can smoke the artifacts it produced, and a smoke failure publishes nothing.

## 5. Rewrite the Notes and Publish

The notes handed to the post-release plugins are yours to write once you know what the artifacts say. Pass a file, or a stream with `-`, plus the tag and the commit you verified:

```bash
cutver publish --tag v0.2.0 --commit 8ff0bfd4ec2d2685e38e2aba810896e1d66dc499 --notes-file - <<'MD'
## What changes

- `cutver --quiet` keeps the summary tree and drops every other line.
MD
```

```text
# trimmed: git remote path
✔ Published: v0.2.0 at 8ff0bfd4ec2d2685e38e2aba810896e1d66dc499
├── Tag
│   └── v0.2.0
└── Push
    └── git push origin refs/tags/v0.2.0
```

*Gotcha*: `--commit` must be the full SHA `git rev-parse HEAD` prints; a short one is refused before anything is written. With no `on_post_release` plugin installed the notes are read and nothing consumes them yet, so no draft release appears.

## The Short Path

When there are no artifacts to smoke first, one command does the whole release — same pipeline, tag and pushes included:

```bash
cutver bump auto
```

It is the short path, not the recommendation: nothing can be verified between the commit and the tag.

## The Rollback Boundary

The pipeline runs guard, preflight, in-memory compute, atomic apply, hooks, then Git actions, as described in [Pipeline Execution & Atomic Guarantees](../internals/pipeline.md).

> **Rollback Guarantee**: If a step fails **before the release commit**, `cutver` restores the original manifest contents from memory and unstages its own git index changes, leaving the workspace as it was. The commit, the tag and the publish come after that point and are not rolled back.

## Next Steps

- [Monorepos and Multi-Package Strategies](monorepos.md): Declare many package manifests under one shared version.
- [Complete cutver.toml Configuration Reference](../reference/configuration.md): Explore all configuration keys and options.
- [Language Manifests and Surgical Diff Guarantees](../reference/manifests.md): How cutver preserves comments and formatting.
- [CLI Commands and Exit Codes](../reference/cli.md): Comprehensive CLI reference and exit statuses.
- [Customizing Release Templates](../templates/release-notes.md): Craft dynamic changelogs with release context variables.
- [GitHub Actions Integration Guide](../integrations/github-actions.md): Install the CLI on a runner and cut the release with its commands.
