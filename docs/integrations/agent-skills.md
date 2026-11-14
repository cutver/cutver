# AI Coding Skills

Equip coding assistants with release skills the agent can run on its own.

The skills live in [Cutver Agent Skills](https://github.com/cutver/skills) and install into any
assistant that reads the `skills` convention — Pi, Claude Code, Cursor, Copilot, and Eve.

---

## Install

```bash
npx skills add cutver/skills
# or
pnpm dlx skills add cutver/skills
# or
bunx skills add cutver/skills
```

---

## Included Skills

Eight autonomous skills ship in the bundle:

- `cutver-release`: Execute safe semantic version bumps with mandatory dry-run simulation.
- `cutver-doctor`: Validate manifest synchronization, changelog alignment, and detect drift.
- `cutver-init`: Auto-discover manifests and scaffold canonical `cutver.toml`.
- `cutver-actions`: Scaffold and configure GitHub Actions release workflows.
- `cutver-changelog`: Extract release notes and render MiniJinja templates.
- `cutver-containers`: Run Cutver inside Docker, Podman, or WSL containers.
- `cutver-install`: Environment detection and package manager setup.
- `cutver-open`: Instant navigation to docs, releases, and changelogs.

Each skill runs the commands documented in this repository, so an assistant never has to guess a
flag: the [CLI reference](../reference/cli.md) is the single source of truth behind them.

---

## Related Documentation

- [Getting Started Walkthrough](../guides/getting-started.md): the flow `cutver-release` executes.
- [Customizing Release Templates](../templates/release-notes.md): the templates `cutver-changelog` renders.
- [GitHub Actions Integration](github-actions.md): the workflows `cutver-actions` scaffolds.
