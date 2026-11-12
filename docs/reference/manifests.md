# Language Manifests and Surgical Diff Guarantees

Format preservation across multi-language manifest formats.

Most release tools re-serialize entire files using generic object serializers. This strips comments, alters indentation, reflows multiline arrays, and rearranges dictionary keys, creating noisy pull requests.

`cutver` guarantees **surgical diffs**: for the AST and byte-span editors the diff is the version and nothing else, with comments, whitespace and formatting preserved. The Gradle editor also increments `versionCode`, and the regex editor replaces every match, so those two can touch more than one line.

---

## Supported Manifest Editors

| Kind | Target Format | Editor Implementation | What is Preserved |
| :--- | :--- | :--- | :--- |
| `cargo-package` | `Cargo.toml` | `toml_edit` AST | Comments, inline comments, table ordering, and whitespace around `[package] version`. |
| `pyproject` | `pyproject.toml` | `toml_edit` AST | Native PEP 621 (`[project] version`) and Poetry (`[tool.poetry] version`). Preserves comments and custom tables. |
| `json` | `*.json` | Byte-span scanner | Replaces **strictly** the string byte range containing the version. Key order, indentation, and newlines remain untouched; input must be strict JSON, so a comment-bearing JSONC file fails to parse. |
| `gradle` | `build.gradle`, `*.gradle.kts` | Regex byte replacement | Updates `versionName` string and increments `versionCode` integer without altering Gradle DSL structure. |
| `regex` | Arbitrary text files | Regex capture group | Replaces matched token with `replacement`, substituting `{{version}}`. |
| `plugin` | Any file an installed `manifest.v1` plugin handles | Wasm plugin (`manifest.v1`) | Format preservation is the plugin's responsibility. Requires a plugin-enabled build. |
| `template` | Any file generated from a template | MiniJinja over the release context | The whole target file is replaced. A sink generates output; it never supplies a version. |

---

## Manifest Examples

### Rust (`cargo-package`)

```toml
[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"
primary = true
```

> **Gotcha**: only `[package] version` is read and written (`src/manifest/cargo_toml.rs:11,27`). A Cargo workspace that inherits its version — `[workspace.package] version` or `version.workspace = true` — fails with `FieldNotFound("package.version")`. Workspace awareness is planned, not implemented (see [the roadmap](../internals/design.md)).

Updates `[package] version = "..."`, preserving comments and structure via `toml_edit`. Produces a single-line diff:
```diff
-version = "0.9.0"
+version = "0.10.0"
```

### Python (`pyproject`)

```toml
[[manifest]]
path = "pyproject.toml"
kind = "pyproject"
```

Auto-detects PEP 621 `[project] version` (used by `uv`, Hatch, PDM, Flit, Maturin, setuptools) and falls back to `[tool.poetry] version`. Preserves all comments and formatting.

### JavaScript / Web (`json`)

```toml
[[manifest]]
path = "package.json"
kind = "json"
field = "version"
```

Uses a byte-span scanner: parses JSON in memory, finds the exact byte offset interval `[start, end]` of the version value in the raw file buffer, and splices only that byte span.

### Android (`gradle`)

```toml
[[manifest]]
path = "android/app/build.gradle.kts"
kind = "gradle"
version_name_field = "versionName"
version_code_field = "versionCode"
```

Replaces `versionName` with target SemVer string and increments integer `versionCode` by 1.

### Universal Fallback (`regex`)

```toml
[[manifest]]
path = "version.txt"
kind = "regex"
pattern = 'release/v\d+\.\d+\.\d+'
replacement = 'release/{{version}}'
```

### Plugin (`plugin`)

```toml
[[manifest]]
path = "custom.version"
kind = "plugin"
plugin = "my-manifest"   # optional; omit to match by manifest_match globs
```

Delegates reads and writes to an installed `manifest.v1` plugin. The `plugin` key is optional: without it, `cutver` matches the declared `path` against each plugin's `manifest_match` globs. Format preservation is the plugin's responsibility, not `cutver`'s. A lean build has no plugin runtime and fails with `PluginManagerRequired` (see [the plugin reference](plugins.md)).

### Generated source (`template`)

```toml
[[manifest]]
path = "src/version.ts"
kind = "template"
template = "src/version.ts.jinja"   # optional; defaults to <path>.jinja
```

Renders the template with the full release context and writes the result over `path`, so a
`version.h`, `version.ts` or `version.go` is generated instead of reverse-matched by a regex that
breaks when the file is reformatted. The template sees the same context a release-notes template
gets: `version`, `tag`, `date`, `major`, `minor`, `patch`, `repository` and the commit categories.
Output is byte-exact, including the template's trailing newline.

> **A template manifest is a sink, not a source.** It replaces the whole target file, so a template
> that also held hand-written content would lose it. It can never be `[version] current_source`, and
> `cutver` fails closed if `template` names the target itself. Point `current_source` at a manifest
> that holds the version.

`cutver doctor` verifies a sink by rendering the template for the expected version and comparing
it with the file on disk. The comparison uses what survives after a release: version, tag and
remote. The commit set is empty and the release date is not recoverable, so a template that
interpolates `date` or commit-derived fields can report drift for a file that is in fact current.

---

## Automatic Lockfile Synchronization

When manifests are updated, package managers require updating lockfiles. `cutver` pairs manifest editing with automatic lockfile staging via `post_bump`:

```toml
[hooks]
post_bump = "cargo check --workspace && pnpm install --lockfile-only"
```

### Staged Lockfile Allowlist

`cutver` verifies git status following `post_bump` and stages only recognized lockfiles:
- **Rust**: `Cargo.lock`
- **JavaScript/TypeScript**: `package-lock.json`, `pnpm-lock.yaml`, `yarn.lock`, `bun.lock`, `bun.lockb`
- **Python**: `uv.lock`, `poetry.lock`, `Pipfile.lock`, `pdm.lock`
- **Android/Java**: `gradle.lockfile`
- **PHP**: `composer.lock`

Other files touched by scripts remain unstaged in your working directory.

---

## Related Documentation

- [Complete cutver.toml Configuration Reference](configuration.md): All configuration schema tables.
- [Monorepos and Multi-Package Strategies](../guides/monorepos.md): Multi-manifest workflows and drift detection.
- [CLI Commands and Exit Codes](cli.md): CLI flags and options.
