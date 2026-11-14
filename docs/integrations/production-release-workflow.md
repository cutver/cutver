# Production Release Workflow

The bot-driven release workflow: a GitHub App token, an explicit bump input, and a published GitHub
Release.

This is the workflow to copy when releases must be attributed to a bot identity, run on demand, and
be simulable with `dry_run`. The
[GitHub Actions integration guide](github-actions.md) covers the simpler push-triggered flow and the
action outputs.

---

## The Workflow

Create `.github/workflows/release.yml` in your repository:

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

      - name: Execute Cutver Release
        uses: cutver/release@v1
        id: cutver
        env:
          GH_TOKEN: ${{ steps.release-token.outputs.token }}
        with:
          command: release
          bump: ${{ github.event.inputs.bump }}
          first-release: ${{ github.event.inputs.first_release }}
          dry-run: ${{ github.event.inputs.dry_run }}

      - name: Publish GitHub Release
        if: steps.cutver.outputs.released == 'true' && !github.event.inputs.dry_run
        env:
          GH_TOKEN: ${{ steps.release-token.outputs.token }}
        run: |
          gh release create "${{ steps.cutver.outputs.tag }}" \
            --title "${{ steps.cutver.outputs.tag }}" \
            --notes-file "${{ steps.cutver.outputs.notes-path }}"
```

---

## Why Each Piece Is There

| Step | Reason |
| :--- | :--- |
| App token instead of `GITHUB_TOKEN` | Commits pushed with `GITHUB_TOKEN` do not trigger downstream workflows; an app token does. |
| `fetch-depth: 0` | Conventional commit parsing and tag history need the full history. |
| `first_release` input | Tags the current version without bumping, for repositories that already shipped. |
| `dry_run` input | Runs the full pipeline without writing, so the plan can be reviewed in the log. |
| `notes-path` output | The rendered notes file is what `gh release create` consumes. |

---

## Related Documentation

- [GitHub Actions Integration Guide](github-actions.md): `cutver/setup`, `cutver/release`, and every step output.
- [CLI Commands and Exit Codes](../reference/cli.md): what `cutver bump` and `cutver publish` accept locally.
- [Customizing Release Templates](../templates/release-notes.md): the notes file this workflow publishes.
