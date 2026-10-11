# Monorepo and Multi-Package Strategies

Coordinate multi-package and polyglot repositories without version drift or messy diffs.

Monorepos combine multiple ecosystems: Rust crates, Node/TypeScript apps, Tauri desktop packages, mobile Gradle builds, and shared documentation. `cutver` manages all targets concurrently from a single `cutver.toml`.

---

## One Shared Version, Many Manifests

`cutver` has no independent per-package release mode. There is no `[workspace]` table, no `--package` flag, and no path-filtered commit logic; the binary reads none of them. A `[workspace]` table in `cutver.toml` is silently ignored. Independent package releases are Phase 2 work, recorded in [the design document](../internals/design.md).

What a monorepo release is: one shared version across every declared `[[manifest]]`. One bump writes that version into all of them and cuts one shared tag. `primary = true` (or `[version] current_source`) selects the manifest that supplies the current version; when neither is set, the first declared manifest is the source.

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
