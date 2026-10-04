# GitHub Actions Integration Guide

Automate releases, preflight verification, and release notes in GitHub Actions.

Official GitHub Actions maintained by the `cutver` project provide zero-boilerplate integration.

---

## Official GitHub Actions

- **[`cutver/setup@v1`](https://github.com/cutver/setup)**: Installs the official, Cosign-verified `cutver` binary matching the runner platform (`ubuntu-latest`, `macos-latest`, `windows-latest`) directly into `$PATH`.
- **[`cutver/release@v1`](https://github.com/cutver/release)**: Executes the release lifecycle, runs preflight verification, handles conventional SemVer bumps, and sets step outputs.

---

## Automated Release Workflow

Create `.github/workflows/release.yml` in your repository:

```yaml
name: Release

on:
  push:
    branches: [main]

permissions:
  contents: write
  id-token: write

jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout repository
        uses: actions/checkout@v7
        with:
          fetch-depth: 0 # Fetch full history for conventional commit parsing

      - name: Install Cutver CLI
        uses: cutver/setup@v1
        with:
          github-token: ${{ secrets.GITHUB_TOKEN }}

      - name: Run Cutver Release
        id: release
        uses: cutver/release@v1
        with:
          command: "release"
          bump: "auto"
          template: ".github/templates/cutver/RELEASE.md"
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}

      - name: Create GitHub Release
        if: steps.release.outputs.released == 'true'
        uses: softprops/action-gh-release@v2
        with:
          tag_name: ${{ steps.release.outputs.tag }}
          name: ${{ steps.release.outputs.tag }}
          body: ${{ steps.release.outputs.notes }}
```

---

## Action Outputs

When `cutver/release` executes, it sets step outputs for downstream jobs:

| Output | Type | Description |
| :--- | :--- | :--- |
| `released` | `string` (`"true"` \| `"false"`) | Whether a new release was cut. |
| `version` | `string` | Target SemVer version string (e.g. `1.2.0`). |
| `tag` | `string` | Tag name created (e.g. `v1.2.0`). |
| `previous_version` | `string` | Prior version before bump. |
| `previous_tag` | `string` | Prior tag before bump. |
| `notes` | `string` | Rendered release notes markdown. |

---

## Pull Request PR Preflight & Drift Check

Catch manifest drift and configuration errors on pull requests before merging into main:

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

      - name: Verify Manifest Sync
        run: cutver doctor --check-changelog
```

---

## Related Documentation

- [Container Integration Guide](containers.md): Running cutver with Docker, Podman, and container actions.
- [Complete cutver.toml Configuration Reference](../reference/configuration.md): Preflight timeouts and git settings.
- [Customizing Release Templates](../templates/release-notes.md): Formatting notes for GitHub Releases.
