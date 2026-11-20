# Machine Contract

> Outcome first: every JSON object `cutver` prints opens with `schemaVersion`, so a consumer can tell
> whether it reads the schema it was written for instead of silently misreading a field. Human output
> is unaffected: this page is the contract for `--json`, `--report` and their exit codes.

The rule is one key, in one place (`src/cli/json.rs`):

```json
{"schemaVersion": 1, "…": "the surface's own keys, in their documented order"}
```

`schemaVersion` is always the **first** key, its value is an integer, and the payload's keys follow in
the order the surface documents. Version `1` only grows additively: a new key is not a new version,
because a consumer that ignores unknown keys keeps reading the same values. It changes only when an
existing key changes meaning or disappears.

## Surfaces

| Command | Shape | Versioned since |
| :--- | :--- | :--- |
| `cutver bump --json` | One compact object on the last stdout line. | `1` |
| `cutver bump --report <PATH>` | The same object, in that file, atomically, plus one trailing newline. | `1` |
| `cutver plan --json` | The release plan: tag, commit, notes, phases and blockers, all as data. | `1` |
| `cutver changelog latest --json` | The release context, pretty-printed. | `1` |
| `cutver changelog show <V> --json` | The release context, pretty-printed. | `1` |
| `cutver doctor --json` | The diagnosis: drift and check statuses as data. | `1` |
| `cutver plugin list --json` | `{"schemaVersion", "plugins": [...]}`. | `1` |
| `cutver plugin info <NAME> --json` | The registry entry, pretty-printed. | `1` |
| `cutver plugin search --json` | `{"schemaVersion", "plugins": [...]}`. | `1` |

`changelog` carries two keys that look alike and are not: `schemaVersion` is the contract version and
`version` the release version the entry describes.

The surfaces that do not emit JSON are deliberate: `init`, `attest`, `open --print-url` (a URL),
`publish` (the plan tree), `plan` without `--json` (the same tree the dry run prints) and `plugin
add|update|restore|remove` print human output only. Passing `--json` to them is a usage error, not a
silent fallback.

Three of these need more than a row, so the CI output channel `bump` and `plan` also fill on a
runner, `doctor --json` and `plan --json` are specified on the
[command surfaces page](machine-contract-surfaces.md); this table stays the index to them.

## Exit Codes

The exit code is the outcome and never the format — `--json` does not move it:

| Code | Meaning | Includes |
| :---: | :--- | :--- |
| `0` | Success | A finished release, a clean `doctor` run, notes emitted, entries listed. |
| `1` | General error | Configuration, git, preflight, plugin, registry or serialization failure. |
| `2` | Drift detected | `cutver doctor` found version drift, or `CHANGELOG.md` disagrees with the Git tags. |

A machine reads the outcome from the exit code first and the object second. `doctor --json` makes
the split visible: the diagnosis is on stdout as data, the code is `2` when it found drift, and
nothing is rendered for a human to scrape.

## Errors

When an invocation asked for machine output, its failure is machine output too: one compact object on
**stderr**, nothing added to stdout. `--json` selects it, and so does `--report <PATH>` for `bump`: the
flag for a machine that must read the object alone, so its failure is readable alone too.

```json
{"schemaVersion":1,"error":{"code":"config","exitCode":1,"message":"no cutver.toml found …"}}
```

- `code` names the failure class, never the message: a consumer branches on it instead of matching
  prose. A new `code` is additive and does not bump `schemaVersion`.
- `exitCode` always equals the process exit code, so the code and the object cannot disagree. It is
  `1`: `2` is drift, and drift is an outcome `doctor` reports as data rather than a failure.
- `message` is the text the human path prints, without the coloured `Error:` marker, newlines
  included, so `Where:` and `Fix:` lines stay inside the string. The two sites that print their own
  marker (`Error loading config:`, `Error checking changelog:`) keep it, so both read alike.

A message is escaped, so the envelope is one JSON line however long the human text is.

| `code` | Failure |
| :--- | :--- |
| `attest` | Recording a gate attestation: the gate name, the commit or the write. |
| `bump` | The release run: preflight, manifests, git, the plan itself. |
| `changelog` | Reading the changelog, resolving a release, rendering notes. |
| `changelog_check` | `doctor --check-changelog` could not compare entries with tags. |
| `config` | Discovering, loading or parsing `cutver.toml`. |
| `doctor` | A `doctor` check could not run, or its JSON could not be rendered. |
| `external` | A `cutver-<name>` subcommand could not be resolved or executed. |
| `init` | Scaffolding the configuration. |
| `open` | Resolving the URL or launching the browser. |
| `plan` | `cutver plan` could not compute the release: a plugin-driven surface, or notes it could not render. |
| `plugin` | Any `plugin` subcommand: cache, registry, digest, the plugin feature. |
| `publish` | The closing phase prepared by `bump --defer-publish`. |
| `report` | Serializing or writing the `--json`/`--report` result after the release finished. |

`init`, `attest` and `external` have no machine surface today, so their failures still render for a
human; the codes are reserved so a `--json` on them later does not have to invent one.

### Limits

- A degradation warning (`warning: …`) stays human prose: it is not a failure, so the command
  continues and exits `0`.
- A `cutver-<name>` external subcommand's own stderr and exit code pass through unwrapped: cutver does
  not own that output, and the exit code is the child's. Clap usage errors — a missing argument, an
  unknown flag — exit before this code runs and are never enveloped.
- Without machine mode nothing changes: the human path keeps `Error: …` on stderr, the exit codes,
  and the same rendered tree, dashboard and notes.

## Migration: `plugin search --json` (breaking in `1`)

It was a bare array, which has no room for a version: read `.plugins[]` instead of `.[]`.
