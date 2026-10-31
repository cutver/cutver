# Monorepo and Multi-Package Strategies

Coordinate multi-package and polyglot repositories without version drift or messy diffs.

Monorepos combine multiple ecosystems: Rust crates, Node/TypeScript apps, Tauri desktop packages, mobile Gradle builds, and shared documentation. `cutver` manages all targets concurrently from a single `cutver.toml`.

---

## One Shared Version, Many Manifests

`cutver` has no independent per-package release mode. There is no `[workspace]` table and no `--package` flag; declaring a `[workspace]` table now fails the load, because unknown configuration keys are a hard error ([configuration reference](../reference/configuration.md)). Independent package releases are deferred rather than planned: [ADR-0002](../adr/0002-shared-version-with-path-filtered-changelogs.md) records the decision, its reasons and the condition that would reopen it.

What a monorepo release is: one shared version across every declared `[[manifest]]`. One bump writes that version into all of them and cuts one shared tag. `primary = true` (or `[version] current_source`) selects the manifest that supplies the current version; when neither is set, the first declared manifest is the source.

---

## Per-Package Changelog Sections

A manifest can carry an optional package identity, which splits the release section into one block per package. The key and its validation rules live in the [configuration reference](../reference/configuration.md).

The version heading stays the shared one; each declared package becomes a `###` heading with the usual category headings nested beneath it:

```markdown
## [v0.14.0] - 2026-10-31

### cutver
#### Features
- **plugin**: content-address the artifact cache

### cutver-pdk
#### Features
- **types**: add `manifest_match` to `PluginManifest`

### Other changes
- tune the CI matrix
```

The rendered shape has four rules:

- **Declaration order.** Packages appear in the order their `[[manifest]]` entries appear in `cutver.toml` — not alphabetically and not by commit count.
- **No empty sections.** A package with no commit in the release range renders no heading at all, rather than an empty block.
- **Unowned commits stay visible.** A commit that touches no declared manifest path renders under `### Other changes`, without category subheadings. It is never dropped.
- **Absence is opt-in.** When no manifest declares a package identity, the changelog renders exactly as it did before: flat category headings with no package grouping. `format = "plugin"` and custom-template modes also keep their existing flat output, because those renderers receive the flattened commit list.

---

## Polyglot Monorepo Example

Declare every package's version file under `[[manifest]]`. The example below marks the Cargo manifest with `primary = true` to make it the source of truth.

```toml
# cutver.toml

# Primary manifest: source of truth for the shared version
[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"
primary = true

# Secondary manifests: synchronized simultaneously
[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[[manifest]]
path = "apps/web/package.json"
kind = "json"
field = "version"

[[manifest]]
path = "src-tauri/tauri.conf.json"
kind = "json"
field = "version"

[[manifest]]
path = "android/app/build.gradle.kts"
kind = "gradle"
version_name_field = "versionName"
version_code_field = "versionCode"

[preflight]
default_timeout = 600
cargo_test = "cargo test --workspace"
web_test   = "bun run test --filter=web"
build      = "cargo build --workspace --release"

[hooks]
post_bump = "cargo check --workspace && bun install"

[git]
tag_prefix = "v"
commit_message = "chore(release): v{version}"
require_clean_tree = true
require_branch = "main"

[publish]
push = true
```

---

## Drift Detection across Monorepo Packages

In polyglot setups, manual releases frequently leave behind forgotten files. `cutver doctor` evaluates all declared manifests across your repository in parallel:

```bash
cutver doctor
```

```text
Evaluating manifests:
  ✓ Cargo.toml (0.10.0) [primary]
  ✓ package.json (0.10.0)
  ✓ apps/web/package.json (0.10.0)
  ✓ src-tauri/tauri.conf.json (0.10.0)
  ✓ android/app/build.gradle.kts (versionName: 0.10.0, versionCode: 10)

Result: All 5 manifests are synchronized.
```

If any file falls out of sync, `cutver doctor` reports the mismatch and exits with code `2`.

---

## Lockfile Coordination in Monorepos

When package manifests change, their lockfiles require corresponding updates. Running packager tools can introduce untracked file noise.

`cutver` solves this with declarative `[hooks] post_bump` and an automatic lockfile staging filter:

```toml
[hooks]
post_bump = "cargo check --workspace && pnpm install --lockfile-only"
```

### Safety Guarantee

`cutver` strictly filters files modified during `post_bump`. Only recognized lockfile paths are staged:
- `Cargo.lock`
- `package-lock.json`, `pnpm-lock.yaml`, `yarn.lock`, `bun.lock`, `bun.lockb`
- `uv.lock`, `poetry.lock`, `Pipfile.lock`, `pdm.lock`
- `gradle.lockfile`
- `composer.lock`

Any other file modified by hook scripts remains unstaged in your working tree, guaranteeing that release commits remain surgical and clean.

---

## Adding New Packages

When onboarding a new subpackage to a monorepo, let `cutver init` discover and register it:

```bash
cutver init --update
```

`cutver` scans your project tree, discovers new package manifests, and appends them to `cutver.toml` while preserving all existing preflight checks, hooks, and git settings.

---

## Related Documentation

- [Getting Started with cutver](getting-started.md): Standard release workflow and dry-run walkthrough.
- [Language Manifests and Surgical Diff Guarantees](../reference/manifests.md): Detailed behavior for Cargo, JSON, PyProject, Gradle, and Regex.
- [Complete cutver.toml Configuration Reference](../reference/configuration.md): Exhaustive table and key definitions.
