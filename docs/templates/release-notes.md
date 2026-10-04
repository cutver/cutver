# Customizing Release Templates

Craft dynamic, human-centered changelogs and release notes in cutver.

`cutver` features a built-in release notes templating engine that pairs Conventional Commits with rich release context variables and filters. (Template syntax is compatible with MiniJinja/Jinja2 templates, with auto-escaping disabled so Markdown is preserved).

---

## Configuring Release Templates

Templates can be specified in three ways:

1. **Inline in `cutver.toml`**:
   ```toml
   [changelog]
   template = """
   ## Release {{ version }} ({{ date }})
   {{ all_changes }}
   """
   ```

2. **Template File in `cutver.toml`**:
   ```toml
   [changelog]
   template_file = ".github/templates/cutver/RELEASE.md"
   ```

3. **CLI Argument Override**:
   ```bash
   cutver changelog latest --template path/to/template.md
   ```

---

## Release Context Variables

Every template receives the following contextual variables:

| Variable | Type | Description | Example |
| :--- | :--- | :--- | :--- |
| `version` | `string` | Target SemVer version without prefix. | `"1.2.0"` |
| `previous_version` | `string \| null` | Previous version without prefix. | `"1.1.0"` |
| `tag` | `string` | Target release Git tag. | `"v1.2.0"` |
| `previous_tag` | `string \| null` | Previous release Git tag. | `"v1.1.0"` |
| `date` | `string` | ISO 8601 release date. | `"2026-09-30"` |
| `compare_url` | `string \| null` | Diff URL between previous tag and current tag. | `"https://github.com/org/repo/compare/v1.1.0...v1.2.0"` |
| `repository` | `string \| null` | Remote repository web URL. | `"https://github.com/org/repo"` |
| `contributors` | `list<string>` | List of unique authors in this release. | `["alice", "bob"]` |

### Categorized Commit Blocks

Pre-formatted bullet points for each Conventional Commit category:
- `breaking`: Pre-rendered bullet items for breaking changes.
- `features`: Pre-rendered bullet items for `feat` commits.
- `fixes`: Pre-rendered bullet items for `fix` commits.
- `perf`: Pre-rendered bullet items for `perf` commits.
- `refactor`: Pre-rendered bullet items for `refactor` commits.
- `docs`: Pre-rendered bullet items for `docs` commits.
- `maintenance`: Pre-rendered bullet items for `chore`, `ci`, `test`, `build`.
- `other`: Pre-rendered bullet items for unmapped commits.
- `all_changes`: Complete standard Keep-a-Changelog section block.

---

## Commit Record Fields (`commits`)

The `commits` list provides rich commit metadata for custom loops:

| Field | Type | Description |
| :--- | :--- | :--- |
| `commit.type` / `commit.commit_type` | `string` | Conventional commit type (`"feat"`, `"fix"`). |
| `commit.scope` | `string \| null` | Conventional commit scope (e.g. `"cli"`). |
| `commit.description` | `string` | Commit description text. |
| `commit.clean_description` | `string` | Commit description stripped of duplicate PR numbers like `(#123)`. |
| `commit.is_breaking` | `bool` | `true` if commit marks a breaking change. |
| `commit.breaking_description` | `string \| null` | Extracted breaking change explanation. |
| `commit.hash` / `commit.short_hash` | `string \| null` | 40-character and 7-character Git hashes. |
| `commit.author` | `string \| null` | Author name or handle. |
| `commit.pr_number` / `commit.pr_url`| `int \| str` | Linked PR number and web URL. |
| `commit.commit_url` | `string \| null` | Web URL to commit view. |

---

## Built-in Filters & Helpers

- `commits | group_by_type`: Groups commits by conventional type (`feat`, `fix`, etc.).
- `commits | group_by_scope`: Groups commits by conventional scope.
- `env("VAR_NAME", "default")`: Accesses environment variables directly inside templates (e.g. `{{ env("GITHUB_ACTOR", "CI") }}`).

---

## Complete Template Example

```jinja
## Release {{ tag }} ({{ date }})

{% if breaking %}
### ⚠️ Breaking Changes
{{ breaking }}
{% endif %}

{% if features %}
### 🚀 New Features
{{ features }}
{% endif %}

{% if fixes %}
### 🐛 Bug Fixes
{{ fixes }}
{% endif %}

{% if contributors %}
### 👥 Contributors
{% for author in contributors %}
- @{{ author }}
{% endfor %}
{% endif %}

{% if compare_url %}
**Full Changes**: [View Diff]({{ compare_url }})
{% endif %}
```

---

## Related Documentation

- [Complete cutver.toml Configuration Reference](../reference/configuration.md): All changelog configuration keys.
- [GitHub Actions Integration Guide](../integrations/github-actions.md): Passing release notes directly to GitHub Releases.
- [CLI Commands and Exit Codes](../reference/cli.md): Testing templates with `cutver changelog latest`.
