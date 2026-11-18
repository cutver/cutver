# GitHub Actions Integration Guide

Run the release from a GitHub runner with the CLI: `cutver/setup@v1` installs the binary, and the
workflow steps call `cutver` itself.

> **Deprecated:** `cutver/release@v1` is deprecated. It stays published and keeps working for existing
> workflows, but it is no longer the recommended path: it runs a `cutver` already on `PATH`, it has no
> `publish` command and no `--defer-publish` input, so it cannot split the bump from the closing phase.
> Follow [Migrating from the `cutver/release` Action](migrating-from-the-release-action.md).

---

## Installing the CLI

- **[`cutver/setup@v1`](https://github.com/cutver/setup)**: downloads the `cutver` release archive matching the runner platform (`ubuntu-latest`, `macos-latest`, `windows-latest`) and adds it to `$PATH`.
  - `version` selects the release (default `latest`; pin a version when the runner must be reproducible).
  - `plugins: true` installs the plugin-enabled build; the default is the lean core, which hides the `plugin` subcommand and has no plugin runtime.
  - `cache: false` bypasses the runner tool cache and downloads again.
  - `github-token` is used only to resolve `latest`; it defaults to `${{ github.token }}`.
  - The download is not verified: the action checks no checksum and no signature. Verify `SHA256SUMS` and the cosign signature yourself if the runner's trust boundary requires it.
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
        run: |
          cutver bump auto --defer-publish --report .cutver-release.json
          for key in released deferred version tag commit; do
            echo "$key=$(jq -r --arg k "$key" '.[$k] // ""' .cutver-release.json)" >> "$GITHUB_OUTPUT"
          done
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
commit the report names, which is what lets a build and a smoke test sit in front of the tag. The
[release report](../reference/cli.md#cutver-bump) is the only thing a later step needs to read.

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
