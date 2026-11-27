# `init --json`'s `primary`: one rule, proven

GitHub issue **#410** (`type:refactor`). One equivalence test, one divergence it exposes, one doc sentence.

## Specs

**S1.** A test that, for a configuration that **does** load, `primary_source(&document)` and `deduce_current_source(&config)` name the same manifest — covering the array-of-tables and the array-of-inline-tables spellings, and the deprecated `source` alias.

**S2.** That test must be able to fail: name the change it catches (the ordering rule moving on one side only).

**S3.** No behaviour change to `init`'s human output, and `init --json` keeps the same keys (`CONTRACT.md:232`: additive only).

### Decision

**D1 — the equivalence test (the issue's first bullet), not "delete the duplication".** `cutver::config::load` resolves paths against the root (`src/config/discovery.rs:27`), so the loaded `current_source` comes back absolute while `--update` reports the spelling the document declares — `"Cargo.toml"`, pinned by `tests/attest_init_json.rs:499-503`. Answering from the loader would change the *value* of a documented key, which S3 forbids; relativizing it back would add a translation layer whose output differs for spellings like `./package.json`. The test pins the copy instead.

**D2 — the divergence the test exposes, fixed rather than dodged.** For a configuration that loads, the two rules differ in exactly one case: the first declared manifest is a template sink and nothing names a source. `deduce_current_source` picks the first **non-sink** manifest (`src/config/validation.rs:33-39`); `primary_source` picks the first declared (`src/init/scaffold/config_gen.rs:225-228`), so `init --update --json` reports a `primary` the loader disagrees with — the lie the issue describes, on a configuration that loads. The other loader refusals (`>1 primary`, conflicting `current_source`/`primary`, all-sinks, empty list) make the configuration unloadable, so they are outside the test's reach by construction and stay the document's answer.

**D3 — `declared_manifests` is not touched.** It is shared with `existing_paths` (`src/init/scaffold/config_gen.rs:145`), which decides which discovered manifests `--update` appends. The sink awareness goes in its own reader so the append path keeps its exact behaviour (Risk: item 3 avoided by construction, not by hoping).

## Tasks

- **T1** — the document copy skips template sinks when it falls back to the first manifest, and refuses none of the cases the loader refuses only because the config does not load. Covers D2, S1. — done: `declared_entries` (the unchanged enumeration) plus a single pass over the entries matched **by position**, so two entries declaring one path cannot hide each other; `declared_manifests` untouched (D3).
- **T2** — the equivalence test: for each fixture that loads, both sides name the same manifest; the sink-first fixture fails before T1 and passes after. The doc comment names the change it catches. Covers S1, S2. — done: `init_update_and_the_loader_name_the_same_primary_source`, five fixtures, both answers canonicalized before comparing; plus `update_does_not_append_a_template_sink_the_document_already_declares` as the D3 guard.
- **T3** — `docs/internals/machine-contract-release-tail.md`'s `primary` bullet currently describes `--update` as "an entry flagged `primary = true`, otherwise the first declared", omitting the `[version] current_source` step that the code applies first. State the whole order once. — done.
- **T4** — gate, commit, PR. — done: commit `35f99e0`, PR #413, squash-merged to `main` as **`617303b`**.

## Log

**L1** (user, verbatim): «seguimos con el 410»

**L2** (issue #410, verbatim acceptance criteria): «A test that, for a configuration that **does** load, `primary_source(&document)` and `deduce_current_source(&config)` name the same manifest — covering the array-of-tables and the array-of-inline-tables spellings, and the deprecated `source` alias.» · «That test must be able to fail: name the change it catches (the ordering rule moving on one side only).» · «Or delete the duplication: load the config where it can be loaded, and keep a document-level answer only for the case that motivated the copy — a configuration that does not load — with that fallback itself tested.» · «No behaviour change to `init`'s human output, and `init --json` keeps the same keys (`CONTRACT.md:232`: additive only).»

**L3** (evidence for D1/D2, from the code as it stands at `c21138f`): `primary_source` is called only from `update_existing_config` (`config_gen.rs:152`, `:178`); `init` never loads a `Config`; `warnings::deprecations` is deduped process-wide (`src/warnings.rs:15-18`) and `--update` already runs the same scan (`config_gen.rs:141`), so no warning line changes; `deprecations_in_document` and `report` share `in_document` (`src/config/deprecations.rs:78-97`). A third copy of the rule exists and is **out of scope**: `Config::primary_manifest` (`src/config/types.rs:145-152`) matches `current_source` by string and falls back to `manifest.first()` without the sink rule.

**L4** (RED, observed by the worker before T1, on the sink-first fixture): `init --update` reports `"src/version.ts"` while the loader uses `"…/scaffold-agree-sink-first-6840-4/Cargo.toml"` — the other four fixtures already agreed, so the divergence is exactly the one D2 names. GREEN after T1: `init::scaffold::tests` 11 passed, `config::validation::tests` 13 passed, `attest_init_json` 9 passed.

**L5** (differential, the D3 evidence). The worker built the base commit `c21138f` through `git archive` in a scratch directory and compared `init --update` and `init --update --json` on four fixtures: human stdout and exit code identical, written `cutver.toml` identical, and the only difference is the intended one — sink-first with nothing named, `"primary":"src/version.ts"` → `"primary":"Cargo.toml"`. A throwaway experiment that made `declared_manifests` skip sinks made the D3 guard fail with a duplicated `package.json` entry, so the guard discriminates.

**L6** (delivery, 2026-11-26). `sh scripts/check.sh` → `gate: PASS (9 passed, 0 failed, 1 not run here)`, the not-run step being the cross-target `clippy` this Windows host cannot run. PR **#413** (`type:refactor`) green on every leg, squash-merged as **`617303b`**; branch deleted. The deferred index's #410 row moved to "Closed and re-verified" with this sha.
