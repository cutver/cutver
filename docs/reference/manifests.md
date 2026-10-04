# Language Manifests and Surgical Diff Guarantees

Format preservation across multi-language manifest formats.

Most release tools re-serialize entire files using generic object serializers. This strips comments, alters indentation, reflows multiline arrays, and rearranges dictionary keys, creating noisy pull requests.

`cutver` guarantees **surgical diffs**: exactly one line changed per version update, with all comments, whitespace, and formatting preserved.

---

## Supported Manifest Editors

| Kind | Target Format | Editor Implementation | What is Preserved |
| :--- | :--- | :--- | :--- |
| `cargo-package` | `Cargo.toml` | `toml_edit` AST | Comments, inline comments, table ordering, whitespace, and workspace inheritance. |
| `pyproject` | `pyproject.toml` | `toml_edit` AST | Native PEP 621 (`[project] version`) and Poetry (`[tool.poetry] version`). Preserves comments and custom tables. |
| `json` | `*.json`, `*.jsonc` | Byte-span scanner | Replaces **strictly** the string byte range containing the version. Key order, indentation, comments, and newlines remain untouched. |
| `gradle` | `build.gradle`, `*.gradle.kts` | Regex byte replacement | Updates `versionName` string and increments `versionCode` integer without altering Gradle DSL structure. |
| `regex` | Arbitrary text files | Regex capture group | Replaces matched token with `replacement`, substituting `{{version}}`. |

---

## Manifest Examples

### Rust (`cargo-package`)

```toml
[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"
primary = true
```

Updates `[package] version = "..."` or `[workspace.package] version = "..."` preserving comments and structure via `toml_edit`. Produces a single-line diff:
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
- **Elixir**: `mix.lock`

Other files touched by scripts remain unstaged in your working directory.

---

## Related Documentation

- [Complete cutver.toml Configuration Reference](configuration.md): All configuration schema tables.
- [Monorepos and Multi-Package Strategies](../guides/monorepos.md): Multi-manifest workflows and drift detection.
- [CLI Commands and Exit Codes](cli.md): CLI flags and options.
