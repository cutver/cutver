# Feature: `kind = "template"` — version sinks generated from Jinja templates

Issue #65. A manifest of `kind = "template"` renders a Jinja template with the full
`ReleaseContext` and writes the result over its target path, so `src/version.ts`,
`include/version.h`, `pkg/version/version.go` or a badge block can be generated
deterministically instead of reverse-matched by a brittle regex.

## Specs

- **S1.** `[[manifest]]` accepts `kind = "template"` with an optional `template` key naming the
  template file. When `template` is absent, it defaults to `<path>.jinja` — the target path with
  `.jinja` appended (`src/version.ts` → `src/version.ts.jinja`).
  Quote: `template = "src/version.ts.jinja" # optional, defaults to "<path>.jinja"`.
- **S2.** The template renders with the complete `ReleaseContext` (`version`, `major`, `minor`,
  `patch`, `tag`, `date`, `owner`, `repo`, commit categories, contributors), and the rendered
  bytes are written to `path` through `crate::atomic::write_atomic`.
  Quote: "Renders the template using MiniJinja with the complete `ReleaseContext`".
- **S3.** `cutver doctor` detects drift for a template sink by rendering the template in memory for
  the expected version and comparing it with the file on disk. It never reverse-parses the target.
  Quote: "Rather than attempting to reverse-parse the generated file with regexes, `cutver doctor`
  renders the template in-memory using the expected version and compares the result against the
  file currently on disk."
- **S4.** Configuration fails closed when `template == path`.
  Quote: "Fail closed if `template == path` to prevent destructive template overwrites."
- **S5.** A template sink cannot be the authoritative version source: neither `[version]
  current_source` nor the implicit "first manifest"/`primary = true` deduction may select it.
  Quote: "Prevent `kind = \"template\"` from being configured as `current_source` (template files are
  sinks, not authoritative version sources)."
- **S6.** The template file must exist before `bump` or `doctor` renders it, and a missing template
  is an actionable error naming the path.
  Quote: "Verify that the template file exists before starting bump or doctor."
- **S7.** Rendering for source generation is byte-exact: no `.trim()`, no trailing-newline
  normalisation. A generated `version.h` must be reproducible byte for byte.
- **S8.** No existing kind changes behaviour: `cargo-package`, `json`, `pyproject`, `gradle`,
  `regex` and `plugin` keep their signatures, output and tests.

## Tasks

| ID | Spec | Route | State | Commit |
| :--- | :--- | :--- | :--- | :--- |
| T1 | S1, S4, S5 | Inline: config enum, key allowlist, validation rules, unit tests | done | see Log |
| T2 | S7 | Inline: `render_source` beside `render_template`, without trimming | done | see Log |
| T3 | S1, S2, S6, S7 | Inline: `TemplateEditor` implementing `ManifestEditor` and template rendering | done | see Log |
| T4 | S2 | Inline: assemble the `ReleaseContext` before manifest compute and thread it through | done | see Log |
| T5 | S3 | Inline: doctor renders and compares instead of reading a version | done | see Log |
| T6 | S1–S8 | Inline: reference docs and end-to-end tests | done | see Log |

## Design notes

- `ManifestEditor` keeps `read_version`/`write_version` as they are (six implementors, ~40 call
  sites). The template kind is a **sink**: `read_version` returns an actionable error rather than
  guessing, and the drift check moves to a capability the trait exposes with a default, so no other
  editor changes.
- `assemble_release_context` currently runs inside `prepare_changelog`, after `compute`. T4 lifts it
  before `compute` and runs it regardless of `changelog.path`, because S2 needs it at write time.
- `render_template` calls `.trim()`, which is right for a changelog body and wrong for generated
  source. T2 adds `render_source` sharing `create_environment` and skipping the trim.

## Log

L1. Request: "Mergea todo lo verde y dale al 65" — implement GitHub issue #65.
L3. T1-T5 implemented and verified: config enum and its three fail-closed rules, byte-exact render_source (MiniJinja drops a template trailing newline by default; render_source sets keep_trailing_newline), TemplateEditor as a sink, the release scope lifted before compute and shared with the changelog, and doctor verifying sinks by re-rendering. 368 lib tests and 5 e2e tests green in the default build, 394 with the plugins feature.

L4. T6 documented the kind in the manifest reference together with the residual limit: doctor rebuilds the context from version, tag and remote only, so a template that interpolates date or commit fields can report drift for a file that is current. That limit is stated in the reference rather than hidden behind a green check.

L2. Exploration mapped the blast radius: `ManifestEditor` has 6 implementors and ~40 call sites;
`assemble_release_context` runs after `compute` and only when a changelog path is configured;
`render_template` trims output, so generated source needs its own entry point. Decided to keep the
trait signature stable, treat the kind as a sink, lift context assembly before compute, and add
`render_source`.
