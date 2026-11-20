# Production Release Workflow

The bot-driven release workflow: a GitHub App token, an explicit bump input, and a release that
`cutver publish` tags only after the bump's commit has been verified.

This is the workflow to copy when releases must be attributed to a bot identity, run on demand, and be
simulable with `dry_run`. The [GitHub Actions integration guide](github-actions.md) covers the
push-triggered variant and the `cutver/setup@v1` inputs.

> **Deprecated:** the `cutver/release@v1` action this guide used to be built around is deprecated. It
> stays published and keeps working for existing workflows, but it is no longer the recommended path:
> with no `publish` command and no `--defer-publish` input it cannot separate the bump from the
> closing phase. [Migrating from the `cutver/release` Action](migrating-from-the-release-action.md)
> maps each of its inputs and outputs to the command that replaces it.

---

## The Workflow

`.github/workflows/release.yml` in your repository:

```yaml
name: Release

on:
  workflow_dispatch:
    inputs:
      first_release:
        description: "First release (tag current version without bumping)"
        required: false
        default: false
        type: boolean
      bump:
        description: "Release bump level"
        required: true
        default: "auto"
        type: choice
        options: [auto, patch, minor, major]
      dry_run:
        description: "Simulate release"
        required: false
        default: false
        type: boolean

permissions:
  contents: write
  pull-requests: write

jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - name: Generate Token for cutver-release[bot]
        id: release-token
        uses: actions/create-github-app-token@v3
        with:
          client-id: ${{ secrets.CUTVER_CLIENT_ID }}
          private-key: ${{ secrets.CUTVER_APP_PRIVATE_KEY }}

      - name: Checkout repository
        uses: actions/checkout@v7
        with:
          fetch-depth: 0
          token: ${{ steps.release-token.outputs.token }}

      - name: Setup Git credentials
        run: |
          git config --global user.name "cutver-release[bot]"
          git config --global user.email "${{ steps.release-token.outputs.app-slug }}[bot]@users.noreply.github.com"

      - name: Install Cutver CLI
        uses: cutver/setup@v1
        with:
          plugins: true

      - name: Restore the declared plugin artifacts
        run: cutver plugin restore

      - name: Doctor
        env:
          GITHUB_TOKEN: ${{ steps.release-token.outputs.token }}
        run: cutver doctor --check-changelog

      - name: Release (bump only)
        id: cutver
        run: |
          args=(--defer-publish)
          if [ "${{ inputs.first_release }}" = "true" ]; then args+=(--first-release); fi
          if [ "${{ inputs.dry_run }}" = "true" ]; then args+=(--dry-run); fi
          cutver bump "${{ inputs.bump || 'auto' }}" "${args[@]}"
        env:
          GITHUB_TOKEN: ${{ steps.release-token.outputs.token }}
          GITHUB_REPOSITORY: ${{ github.repository }}

      - name: Render the release notes
        if: steps.cutver.outputs.released == 'true'
        run: cutver changelog latest --template .github/templates/cutver/RELEASE.md > RELEASE_NOTES.md

      - name: Publish the release
        if: steps.cutver.outputs.released == 'true'
        run: cat RELEASE_NOTES.md | cutver publish --tag "${{ steps.cutver.outputs.tag }}" --commit "${{ steps.cutver.outputs.commit }}" --notes-file -
        env:
          GITHUB_TOKEN: ${{ steps.release-token.outputs.token }}
          GITHUB_REPOSITORY: ${{ github.repository }}
```

The example keeps the bump and the publish in one job so the commands sit next to each other. The
steps between them are yours: build the artifacts from `steps.cutver.outputs.commit`, run the smoke
tests over them, and only then let the publish step create the tag. This repository moves those steps
into separate jobs, so the assets a user downloads are the ones signed and uploaded before the tag
exists ([`.github/workflows/release.yml`](../../.github/workflows/release.yml)).

---

## Why Each Piece Is There

| Step | Reason |
| :--- | :--- |
| App token instead of `GITHUB_TOKEN` | Commits pushed with `GITHUB_TOKEN` do not trigger downstream workflows; an app token does. |
| `fetch-depth: 0` | Conventional commit parsing and tag history need the full history. |
| `plugins: true` on `cutver/setup` | `cutver plugin restore` needs the plugin-enabled build; the lean binary hides the subcommand. |
| `first_release` input | `--first-release` tags the current version without bumping, for repositories that already shipped. |
| `dry_run` input | `--dry-run` runs the full pipeline without writing, and the step outputs it writes say `released=false`, so every later step is skipped. |
| `--defer-publish` | The bump creates the release commit and pushes the branch, then stops: no tag, no publish commands until you ask for them. |
| Step outputs instead of a report file | `cutver bump` writes `released`, `deferred`, `version`, `tag` and `commit` to `$GITHUB_OUTPUT` itself, so later steps and jobs read one machine-readable result and no step parses a file. `--report <path>` remains the equivalent for a local or scripted run. |
| `notes-file -` on publish | The notes arrive on standard input, so no path is named and an empty stream is refused before the tag exists. |

Nothing in the workflow creates the GitHub Release record by hand: `cutver publish` runs
`[publish].commands` and dispatches the `on_post_release` plugins, and the `github-publish` plugin
declared in `cutver.toml` owns the release body
([the reference plugin](../guides/plugin-github-publish.md)).

---

## Related Documentation

- [GitHub Actions Integration Guide](github-actions.md): `cutver/setup@v1`, the CLI release steps, and the deprecated `cutver/release` action.
- [Migrating from the `cutver/release` Action](migrating-from-the-release-action.md): why the action cannot defer a publish, and the command that replaces each input.
- [CLI Commands and Exit Codes](../reference/cli.md): what `cutver bump`, `cutver publish` and `--report` accept.
- [Customizing Release Templates](../templates/release-notes.md): the notes file this workflow publishes.
