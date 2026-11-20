# GitHub Actions Integration Guide

Run the release from a GitHub runner with the CLI: `cutver/setup@v1` installs the binary, and the
workflow steps call `cutver` itself.

> **Deprecated:** `cutver/release@v1` is deprecated. It stays published and keeps working for existing
> workflows, but it is no longer the recommended path: it runs a `cutver` already on `PATH`, it has no
> `publish` command and no `--defer-publish` input, so it cannot split the bump from the closing phase.
> Follow [Migrating from the `cutver/release` Action](migrating-from-the-release-action.md).

---

## Installing the CLI

- **[`cutver/setup`](https://github.com/cutver/setup)**: downloads the `cutver` release archive matching the runner platform (`ubuntu-latest`, `macos-latest`, `windows-latest`), verifies it against the release's `SHA256SUMS`, and adds it to `$PATH`.
  - Pin it by commit, not by the floating major tag: `uses: cutver/setup@<40-character-sha>`. A tag can move; a commit cannot, and this step installs the binary the workflow then runs. This repository's own workflow does that (`release.yml`, `Smoke test fresh Cutver installation`).
  - `version` selects the release (default `latest`; pin a version when the runner must be reproducible).
  - `plugins: true` installs the plugin-enabled build; the default is the lean core, which hides the `plugin` subcommand and has no plugin runtime.
  - `cache: false` bypasses the runner tool cache and downloads again.
  - `github-token` is used only to resolve `latest`; it defaults to `${{ github.token }}`.
  - `verify-checksum` defaults to `true`. With it on, the archive's digest is compared against the line `SHA256SUMS` carries for that file, and a mismatch fails the step naming the expected and the actual digest. A release with no `SHA256SUMS`, or one that does not list the file, also fails unless you set `verify-checksum: false` — which is only for releases published before the checksum existed, and it extracts the archive unverified.
  - The output `installed-version` reports the exact version that was installed.
  - What verification does **not** cover: the signature. `SHA256SUMS` is signed with Cosign at release time, and the action reads the file but does not check that signature, so an actor able to replace both release assets defeats it. Verify the signature separately when the runner's trust boundary requires it.
  - A tool-cache hit is trusted: nothing is downloaded and nothing is verified. A tree cached by a run with `verify-checksum: false` is reused by a later run that has verification enabled, so use `cache: false` when that matters.
  - When the release must be cut by the code being released, build the CLI in the workflow and put it first on `PATH` instead of installing a published binary; `.github/workflows/release.yml` in this repository does exactly that.

---

## The release workflow

`.github/workflows/release.yml`, push-triggered, one job:

```yaml
name: Release

on:
  push:
    branches: [main]

permissions:
  contents: write

jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7
        with:
          fetch-depth: 0 # Conventional Commits and tag history need the full history
          token: ${{ secrets.RELEASE_TOKEN }} # a GITHUB_TOKEN push does not trigger downstream workflows

      - uses: cutver/setup@v1
        with:
          plugins: true

      - name: Restore the declared plugin artifacts
        run: cutver plugin restore

      - name: Doctor
        run: cutver doctor --check-changelog

      - name: Release (bump only)
        id: cutver
        run: cutver bump auto --defer-publish
        env:
          GITHUB_TOKEN: ${{ secrets.RELEASE_TOKEN }}

      - name: Render the release notes
        if: steps.cutver.outputs.released == 'true'
        run: cutver changelog latest --template .github/templates/cutver/RELEASE.md > RELEASE_NOTES.md

      - name: Publish the release
        if: steps.cutver.outputs.released == 'true'
        run: cat RELEASE_NOTES.md | cutver publish --tag "${{ steps.cutver.outputs.tag }}" --commit "${{ steps.cutver.outputs.commit }}" --notes-file -
        env:
          GITHUB_TOKEN: ${{ secrets.RELEASE_TOKEN }}
          GITHUB_REPOSITORY: ${{ github.repository }}
```

Why the two commands instead of one: `--defer-publish` makes the bump create the release commit and
push the branch, and `cutver publish` does everything after it — the tag, the floating major tag,
`[publish].commands` and the `on_post_release` plugins. Anything between the two runs against the exact
commit the step output names, which is what lets a build and a smoke test sit in front of the tag. The
step outputs are also the only thing a later step reads: `cutver bump` writes `released`, `deferred`,
`version`, `tag` and `commit` to `$GITHUB_OUTPUT` itself, so the job hands them on without a report
file and a second parser in between.

`plugin restore` is needed when `cutver.toml` declares plugins: it hydrates the cache from the `source`
and `hash` the configuration owns, so the workflow never pins a plugin version of its own. Drop the
step when there are no `[plugins.<name>]` entries.

---

## Doctor on pull requests

```yaml
name: CI / Cutver Doctor

on:
  pull_request:

jobs:
  doctor:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7
        with:
          fetch-depth: 0

      - uses: cutver/setup@v1

      - name: Verify manifest sync
        run: cutver doctor --check-changelog
```

`cutver doctor` exits `2` on version drift or on a changelog entry that does not match a release tag,
so the check is red before the drift reaches `main`.

---

## Related Documentation

- [Migrating from the `cutver/release` Action](migrating-from-the-release-action.md): what the deprecated action does, and the command that replaces each of its inputs.
- [Production Release Workflow](production-release-workflow.md): the bot-token workflow for `workflow_dispatch` releases.
- [Container Integration Guide](containers.md): Running cutver with Docker, Podman, and container actions.
- [Complete cutver.toml Configuration Reference](../reference/configuration.md): Preflight timeouts and git settings.
- [Customizing Release Templates](../templates/release-notes.md): Formatting notes for GitHub Releases.
