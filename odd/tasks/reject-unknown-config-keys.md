# Feature: Reject Unknown `cutver.toml` Keys

Status: in progress
Branch: `fix/reject-unknown-config-keys`
Issue: #193
Predecessor: `docs/design-evolution` merged as `8b291a0`

## Goal

A `cutver.toml` key the loader does not know is accepted and ignored. That silence is the root cause
shared by two findings from the documentation sweep: the `[workspace] mode/members` table that was
documented as a supported feature for three verification rounds and never existed, and
`[hooks] pre_bump`, which is parsed and never executed (`src/config/types.rs:101`, only
`hooks.post_bump` is read at `src/bump/exec/pipeline.rs:279`).

**User decision (2026-10-11): a hard error.** An unknown key must fail the parse and name the
offending table and key. This is also the prerequisite for the workspace RFC: while unknown keys are
ignored, a half-built `[workspace]` is indistinguishable from a working one.

## Constraint discovered before writing any code

The mechanism the issue proposed — `#[serde(deny_unknown_fields)]` on `Config` — **cannot be used as
written**, for two independent reasons that must be proven with a test before choosing a mechanism:

1. **`[preflight]` is `#[serde(skip)]` and carries arbitrary user-named keys.** `Config.preflight`
   and `Config.preflight_default_timeout` are `#[serde(skip)]` (`src/config/types.rs`) because the
   table is parsed separately with `toml_edit` (`src/config/preflight.rs:3`), where every user key is
   a step name. A `deny_unknown_fields` pass over `Config` sees `preflight` as an unknown key and
   rejects **every real configuration**.
2. **`Manifest` uses `#[serde(flatten)] kind: ManifestKind`, and serde does not support
   `deny_unknown_fields` together with a flattened field.** The manifest keys (`field`, `pattern`,
   `replacement`, `table`, `plugin`, ...) come from the flattened enum, so rejecting unknown keys
   there needs the schema, not the derive attribute.

So the behaviour stays *hard error*; the mechanism is an explicit schema validation pass, which also
produces a better message (it can name the table and the key) than a serde error would.

## Tasks

- [ ] 1. Test-first: prove the current acceptance with a failing test that declares a `[workspace]` table and a typo'd key and expects a hard error naming it.
- [ ] 2. Prove the serde constraint in the same test file — a `[preflight]` table with arbitrary step names must keep parsing, and `[[manifest]]` kind keys must keep parsing — so the chosen mechanism cannot regress them.
- [ ] 3. Design and implement the validation pass over the parsed document: a known-schema check that reports the first offending table and key. Keep `[preflight]`'s arbitrary user keys and every `ManifestKind` key in the accepted set.
- [ ] 4. Sweep the blast radius: the repo's own `cutver.toml`, `cutver init` scaffold output (`src/init/scaffold/config_gen.rs`), `.github/templates/**`, test fixtures under `tests/` and `src/**/tests`, and the configuration examples in `docs/`. Any unknown key found there is a discovery, not a test to delete.
- [ ] 5. Update `docs/reference/configuration.md` so the reference states the loader now rejects what it does not know. Do not touch `[hooks] pre_bump`: it is a *known* field that is inert, which is #194, not this feature.
- [ ] 6. Verify with the full suite and clippy; open the pull request.

## Non-goals

- Implementing `[workspace]`. That is the workspace RFC, which this unblocks.
- Changing any other parse behaviour.

## Residual

- The user's chosen behaviour is a hard error. If the blast-radius sweep finds unknown keys in the
  wild that must keep working, that is a migration question and must come back to the user rather
  than soften the rule silently.
