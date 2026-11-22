# Machine Contract: Command Surfaces

> Outcome first: three surfaces need more than a table row, so the CI output channel `bump` and `plan`
> fill on a runner, `doctor --json` and `plan --json` are specified here as data. The protocol they all
> obey — `schemaVersion` first, the exit codes, the error envelope — is the
> [machine contract](machine-contract.md), and its `## Surfaces` table is the index to this page.

## CI output channel

On GitHub Actions — `GITHUB_ACTIONS` reading `true`, trimmed and case-insensitive, **and**
`GITHUB_OUTPUT` or `GITHUB_STEP_SUMMARY` naming a file — `bump` and `plan` fill the runner's sinks as
well. The contract above does not change: **no CI output is written to stdout**, so every `--json`
surface is byte-for-byte what it was with CI off. `--ci` and `--no-ci` are global, so every
subcommand accepts them: `--ci` turns the channel on without `GITHUB_ACTIONS`, the annotation
included when no sink is named, and `--no-ci` turns it off entirely.

- `$GITHUB_OUTPUT` receives the payload's field names, one `key=value` per line, appended and never
  truncated. `bump`: `released`, `deferred`, `version`, `tag`, `commit`; `plan`: `next_version`, `tag`,
  `commit`. A value the run lacks is omitted, so a `--dry-run` bump writes no `commit`.
- `$GITHUB_STEP_SUMMARY` receives one bounded markdown block per run: what was released, or the plan
  in brief. Every value is condensed to one line and fenced inside a code span long enough for the
  backticks it carries, so a tag or a prefix cannot inject markdown, and a changelog entry is never
  copied there.
- A failure writes one `::error::` annotation to **stderr** — never stdout — carrying the human
  message with `%`, CR and LF percent-escaped, so it stays one line. It precedes the machine envelope
  when both are emitted; a successful run annotates nothing.
- A deprecated spelling in `cutver.toml` writes one `::warning::` annotation with the same sentence
  the human line carries, and the run keeps its exit code: a deprecation is not a failure. The
  spellings and the release that removes each one are in the
  [configuration reference](../reference/configuration.md#deprecated-keys).
- A sink that cannot be written degrades to a `warning:` line and the exit code still reports the
  release. Outside CI nothing is opened, annotated or created.

## `doctor --json`

`doctor` writes the diagnosis to stdout instead of the dashboard, and the human path is untouched: no
dashboard, no rendered drift block, no suggested-fix text.

```json
{
  "schemaVersion": 1,
  "ok": false,
  "checks": [{ "name": "versions", "status": "drift" }, { "name": "changelog", "status": "skipped" }],
  "version_drift": [{ "path": "b.json", "expected": "1.1.0", "actual": "1.0.0" }],
  "changelog_drift": null,
  "warnings": ["[version].source is deprecated, use current_source; it will be removed in 1.0.0"],
  "plugins": [
    {
      "name": "probe",
      "capabilities": ["changelog.v1"],
      "grants": [
        { "declared": "assets", "resolved": "/work/repo/assets", "canonical": "/work/repo/assets", "location": "inside" }
      ]
    }
  ],
  "plugin_load_error": null
}
```

- `warnings` is the deprecation channel as data: one sentence per deprecated spelling the document
  uses, naming the spelling, its replacement and the release that removes it. It is `[]` for a
  canonical document and never absent, and it never changes `ok` or the exit code — a deprecated
  spelling still works, which is exactly what makes it a warning.
- `ok` is `false` when any check that ran found drift. `checks` holds one entry per declared check,
  always in that order: `versions`, then `changelog`, and `status` is `ok`, `drift`, or `skipped` — a
  check that did not run says so instead of looking like an empty result. Without `--check-changelog`
  the changelog check is `skipped` and `changelog_drift` is `null`.
- `version_drift` is always an array, empty when every manifest agrees; `path` is relative to the
  workspace root, as in the human block. `changelog_drift` carries `missing_in_changelog` (a Git tag
  with no entry) and `orphan_sections` (an entry with no tag) when the changelog check ran.
- `plugins` holds one entry per declared `[plugins.<name>]`, in name order: `name`, `capabilities`, and
  one `grants` entry per `permissions.filesystem` entry. Each grant carries the three values resolution
  keeps — `declared` (the spelling the document wrote), `resolved` (the guest path the plugin mounts)
  and `canonical` (the host directory the sandbox really opens, `null` when the directory does not exist
  yet) — plus `location`, which is `inside`, `outside` or `unresolved`. Reporting them is why resolution
  stopped overwriting the declared spelling with the resolved one (#268).
- `plugin_load_error` is `null` when the plugin manager could be built, and otherwise the typed error
  that stopped it: `permissions.env` is resolved while the drivers are constructed, so an unset declared
  grant fails it. It is a field of its own rather than a `warnings` entry (that array is the deprecation
  channel), and it never moves `ok` or the exit code — a host that cannot load its plugins still gets a
  diagnosis instead of a failed command.

A check that cannot run — an unreadable manifest, an unusable Git — fails the command with exit `1`
and the error on stderr, which is why no check reports a `failed` status.

## `cutver plan --json`

`plan` computes the release the workspace is about to cut and mutates nothing — no file, no git
mutation, no `[preflight]`, no `[hooks]`, no `[publish] commands`, no plugin dispatch — so the object
is evidence, not a report of something that happened.

```json
{
  "schemaVersion": 1, "read_only": true, "level": "minor",
  "current_version": "0.19.0", "next_version": "0.20.0", "tag": "v0.20.0",
  "floating_tag": "v0", "commit": "abc1234", "branch": "main",
  "commit_message": "chore(release): v0.20.0 [skip ci]",
  "notes": "## [0.20.0] - 2026-11-18\n\n### ✨ Features\n- …",
  "changelog": { "path": "CHANGELOG.md", "prepends": "0.20.0" },
  "manifests": [{ "path": "Cargo.toml", "from": "0.19.0", "to": "0.20.0" }],
  "preflight": [{ "name": "test", "command": "cargo test", "skipped": false }],
  "phases": [{ "name": "validate", "mutates": false, "steps": ["…"] }],
  "artifacts": [{ "glob": "dist/*.tar.gz", "at_least": 8, "description": "lean binaries", "matched": 8 }],
  "gates": [{ "name": "ci", "conclusion": "success", "commit": "abc1234", "verified_at": "2026-11-18", "run": null }],
  "blockers": []
}
```

`read_only` is always `true` — a property of the command, not of the invocation. `commit` is `HEAD`,
the commit the release would be cut from; `branch` is the checked-out branch, `null` on a detached `HEAD`.

- `notes` is the **rendered** entry the release would prepend, produced without writing the changelog.
  A plan that cannot render it fails with exit `1` and a `plan` envelope; it never emits `"notes": null`.
- `phases` is the release in order — `validate` (`mutates: false`), `mutate` and `publish` (`mutates:
  true`) — one step per thing the run would do, derived from the same computation `bump` runs, so it
  cannot describe a pipeline that does not exist.
- `artifacts` carries one entry per declared `[[artifact]]`: `glob`, `at_least`, `description` (only
  when set) and `matched`, the number of files the glob matches right now. It is a count, never a
  verdict: a plan never blocks on a missing artifact, because the assets are built after it plans.
- `gates` carries one entry per gate the release requires — `[gates] require` in union with
  `--require-gate` — in declaration order: `name`, `conclusion` (`"success"`, `"failure"`) and the
  `commit`, `verified_at` and `run` an attestation recorded. All four are `null` together when no
  attestation covers the commit this plan reports, because one recorded for another commit is not
  this release's, and a file that does not parse is not an attestation. A gate is data here, like a
  blocker: `publish` is where a verdict refuses a release.
- `blockers` lists the guards that would abort a real `bump` — a dirty tree under
  `[git] require_clean_tree`, a `[git] require_branch` that does not match, a release tag that already
  exists locally or on the remote, an invalid tag name — one self-contained line each, naming what to
  fix. **A blocked plan still exits `0`**: the release decision belongs to the consumer, and exit `1`
  keeps its single meaning, "no plan could be computed".

## `changelog latest --json`

The release context as data: what a template interpolates, plus what a consumer needs to decide. The
aliases (`owner`, `repo`, `project`) are template surface, not redundancy to tidy away — dropping one
breaks a template that reads it.

```json
{
  "schemaVersion": 1,
  "version": "0.21.0",
  "previous_version": "0.20.0",
  "tag": "v0.21.0",
  "previous_tag": "v0.20.0",
  "date": "2026-11-20",
  "compare_url": "https://github.com/cutver/cutver/compare/v0.20.0...v0.21.0",
  "repository": "https://github.com/cutver/cutver",
  "features": "- feat(cli): …",
  "fixes": "", "breaking": "", "perf": "", "refactor": "", "docs": "", "maintenance": "", "other": "",
  "all_changes": "### Features
…",
  "commits": [{ "type": "feat", "scope": "cli", "description": "…", "hash": "a309d26", "pr_number": 348 }],
  "sections": [{ "heading": "Features", "entries": [], "is_breaking": false }],
  "contributors": ["…"], "first_time_contributors": [],
  "major": 0, "minor": 21, "patch": 0, "is_prerelease": false, "prerelease": null, "build": null,
  "year": 2026, "month": 11, "day": 20,
  "repo_owner": "cutver", "owner": "cutver", "repo_name": "cutver", "repo": "cutver",
  "forge": "github", "project_name": "cutver", "project": "cutver"
}
```

Every one of the 37 keys:

| Key | What it carries |
| :--- | :--- |
| `schemaVersion` | The contract version, first, always `1`. |
| `version`, `previous_version` | The release from the changelog heading, and the one before it (`null` for the first). |
| `tag`, `previous_tag` | The two tags, prefix included. |
| `date` | The heading date, or today when the heading carries none. |
| `compare_url`, `repository` | The forge URL and `compare/<previous_tag>...<tag>`; `null` without both. |
| `features`, `fixes`, `breaking`, `perf`, `refactor`, `docs`, `maintenance`, `other` | One pre-rendered markdown block per category, empty when the release has none. |
| `all_changes` | Every category in one Keep-a-Changelog block. |
| `commits` | One record per commit: `type`, `commit_type`, `scope`, `description`, `clean_description`, `is_breaking`, `body`, `breaking_description`, `breaking_change_description`, `hash`, `short_hash`, `author`, `author_email`, `pr_number`, `pr_url`, `issue_numbers`, `commit_url`, `line`, `bullet`. |
| `sections` | The grouped form: `heading`, `entries`, and `is_breaking`, true for the breaking section and no other, so a template never infers it from a heading. |
| `contributors`, `first_time_contributors` | Authors between the tags, and those in no earlier range. |
| `major`, `minor`, `patch`, `is_prerelease`, `prerelease`, `build` | The version parsed, so no template splits or detects a string. |
| `year`, `month`, `day` | `date` split, for the same reason. |
| `repo_owner`, `owner`, `repo_name`, `repo`, `project_name`, `project`, `forge` | The forge identity: owner, name, its aliases, and the forge itself. Without an origin remote all seven are `null` together, so a template renders an empty string instead of failing. |

`changelog show --version <v> --json` prints the same object for an older release.

## Related Documentation

- [The Machine Contract](machine-contract.md): `schemaVersion` first, the exit-code table and the machine error envelope every surface here obeys.
- [CLI Commands and Exit Codes](../reference/cli.md): the commands these three surfaces belong to.
- [Pipeline Execution & Atomic Guarantees](pipeline.md): the two-phase run that `plan` describes and `bump` performs.
