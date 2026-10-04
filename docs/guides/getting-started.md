# Getting Started with cutver

Ship your first multi-manifest release in three commands.

`cutver` synchronizes version numbers across any set of manifests, runs preflights, updates changelogs, and cuts git tags using a single declarative configuration file.

---

## 1. Initialize Your Repository

Run `cutver init` in the root of your project:

```bash
cutver init
```

What `cutver init` performs automatically:
- Scans and detects package manifests (`Cargo.toml`, `package.json`, `pyproject.toml`, Gradle, etc.).
- Scaffolds a tailored `cutver.toml` configuration at the repository root.
- Generates a starter `CHANGELOG.md` following the Keep-a-Changelog standard.
- Creates a default release template at `.github/templates/cutver/RELEASE.md`.

*Gotcha*: If you add new packages or manifests later, run `cutver init --update` to append newly detected packages without overwriting your custom rules or hooks.

---

## 2. Check Workspace Health

Verify that all declared manifests exist, are readable, and share the same version:

```bash
cutver doctor
```

```text
✓ cutver.toml is valid
✓ Primary manifest: Cargo.toml (0.1.0)
✓ Synchronized: package.json (0.1.0)
✓ Preflight steps configured: 2
```

To verify that your Git tags match entries in `CHANGELOG.md`, pass `--check-changelog`:

```bash
cutver doctor --check-changelog
```

If manifest versions drift, `cutver doctor` reports the mismatch and exits with code `2`.

---

## 3. Simulate Your Release (Dry Run)

Always simulate version calculations and git actions before modifying files or creating tags:

```bash
cutver bump auto --dry-run
```

```text
Evaluating conventional commits since v0.1.0...
Found 3 commits (1 feat, 2 fixes) -> Deduced bump: minor (0.2.0)

Preflight checks:
✓ tests (cargo test)
✓ lint (cargo clippy -- -D warnings)

Planned updates:
  Cargo.toml: 0.1.0 -> 0.2.0
  package.json: 0.1.0 -> 0.2.0
  CHANGELOG.md: prepending ## [v0.2.0] - 2026-09-30

Git release plan:
  commit: chore(release): v0.2.0
  tag: v0.2.0
[dry-run] No changes written to disk.
```

### SemVer Auto-Deduction Rules

| Commit Signal | Deduced Bump | Example Commit |
| :--- | :--- | :--- |
| Breaking change (`!` or `BREAKING CHANGE:`) | **Major** (`X.0.0`) | `feat(api)!: redesign auth flow` |
| Feature commit (`feat`) | **Minor** (`0.X.0`) | `feat(cli): add quiet mode` |
| Fix or maintenance (`fix`, `perf`, `refactor`, `chore`) | **Patch** (`0.0.X`) | `fix(git): unstage index on error` |

To override automatic deduction, pass an explicit level:
```bash
cutver bump patch
cutver bump minor
cutver bump major
```

---

## 4. Cut Your Release

Execute the automated release pipeline:

```bash
cutver bump auto
```

### The Atomic Two-Phase Pipeline

```text
1. Guard Phase   ──▶ Verify clean git working tree and branch restrictions
2. Preflight     ──▶ Run test and lint checks (fail-fast with process tree kill)
3. Compute       ──▶ In-memory manifest diffs & changelog generation
4. Apply         ──▶ Atomic file write via sync_all and temporary swap
5. Hooks         ──▶ Run post_bump hooks and stage modified lockfiles
6. Git Actions   ──▶ Create release commit and SemVer tag (optional upstream push)
```

> **Rollback Guarantee**: If any step in the pipeline fails, `cutver` restores original manifest contents from memory, unstages git index changes, and leaves your repository clean.

---

## Next Steps

- [Monorepos and Multi-Package Strategies](monorepos.md): Coordinate unified workspaces or independent packages.
- [Complete cutver.toml Configuration Reference](../reference/configuration.md): Explore all configuration keys and options.
- [Language Manifests and Surgical Diff Guarantees](../reference/manifests.md): How cutver preserves comments and formatting.
- [CLI Commands and Exit Codes](../reference/cli.md): Comprehensive CLI reference and exit statuses.
- [Customizing Release Templates](../templates/release-notes.md): Craft dynamic changelogs with release context variables.
- [GitHub Actions Integration Guide](../integrations/github-actions.md): Automate CI/CD releases with official actions.
