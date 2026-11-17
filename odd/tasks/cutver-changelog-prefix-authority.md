# Changelog extraction honours the tag prefix, and stops lying on success

Feature: the changelog read surface stops hardcoding `v`, so a custom `[git] tag_prefix` can find its own
headings, and `--json` no longer reports success with empty commit data when git fails. Branch
`feat/changelog-prefix-authority`, base `origin/main` at `9467646`. Closes audit findings F-13 and the
changelog half of F-12 (`odd/audits/2026-11-13-robustness-audit.md:198-206`).

## Specs

S1. With a `[git] tag_prefix` other than `v`, `cutver changelog show <version>` and
`cutver changelog open <version>` match the heading for that version. Today, with
`tag_prefix = "release-"`, they never match one: `changelog/extract/parser.rs:133` trims a hardcoded `v`/`V`,
and `cli/runner/changelog.rs:56,60` plus `cli/runner/changelog_context.rs:29,38,52,69,86` repeat the same
literal inline.

S2. One authority decides how a tag or heading is stripped of its prefix, and every caller on this path uses
it. The prefix-aware logic already exists in `src/git/types.rs`; no `trim_start_matches(['v', 'V'])` and no
bare `"v"` literal remains in the changelog extraction or changelog CLI path.

S3. `cutver changelog latest --json` and `cutver changelog show <version> --json` fail closed when git
fails. Today `cli/runner/changelog.rs:39` and `:75` turn the error into an empty value, so the command exits
`0` and prints `"commits": []` — a machine reading it cannot tell "no commits" from "git was unavailable".
(`--template` keeps its current degradation: an out-of-scope fixture depends on it, recorded in `L4`.)

S4. The default `v` prefix keeps its current output byte for byte, and a version that is absent from the
changelog keeps its current error and exit code.

## Tasks

| ID | Spec | What | Route | Commit |
| --- | --- | --- | --- | --- |
| T1 | S2 | Route every prefix strip on this path through the one prefix-aware authority. | worker | pending |
| T2 | S1 | Prove a custom prefix matches: `tag_prefix = "release-"` finds the `## release-1.2.0` heading in `show` and `open`. | worker | pending |
| T3 | S3 | Surface the git failure from the two `Result` reads instead of an empty value, with the exit code and stderr that follow from it. | worker | pending |
| T4 | S4 | Pin the default-prefix output unchanged, and the missing-version error unchanged. | worker | pending |
| T5 | S1-S3 | Update the audit status block for F-13 and the changelog half of F-12, and the CLI docs if they describe `--json`. | worker | pending |

## Constraints for the worker

- Prefer tests that run on Windows: unit tests under `src/changelog/extract/` and the existing
  `src/cli/runner/` test module. `tests/e2e_changelog_cli.rs` is `#![cfg(unix)]` and executes **zero** tests
  on this host; add a test there only if a rule cannot be pinned any other way, and say so explicitly.
- Out of scope: the `manifest`/`doctor`/`process`/`wasm` half of F-12, F-14, F-15 and F-16. This document is
  the changelog read surface only. If a fix appears to require one of those, stop and report instead.

## Log

L1. User, verbatim: «Sí, cuando marque verde mergea y sigue con el siguiente slic»

L2. Decision (parent), taken on evidence rather than the roadmap's earlier ranking. The two candidates named
before were dropped after an inventory: **package sections in template mode** contradicts a documented
decision and an existing pinning test (`docs/guides/monorepos.md:43`, and
`tests/e2e_changelog_packages.rs:193 template_mode_keeps_todays_output_when_packages_are_declared` asserts
template mode must *not* grow `### alpha`), and reaching it would require package data that never gets built
before the context is assembled (`src/bump/exec/changelog.rs:174` gate, `src/changelog/sections.rs:21`) — a
product decision for the human, not a slice for me. **`changelog verify --against <tag>`** has no cheap
implementation either: the default build has no HTTP client (`ureq` is behind the `plugins` feature only),
nothing in `src/` reads a forge today, and forge behaviour is pinned only as workflow YAML text.

L3. Chosen instead: F-12 and F-13, because they are already documented defects with exact lines in the
repository's own audit, they are deterministic and locally testable, and one of them is a user-visible
correctness failure — with `tag_prefix = "release-"`, `changelog show`/`open` never match a heading.

L4. Amendments made while implementing, recorded so the document matches what shipped: the audit calls the
JSON field `raw_commits`, but the key the command actually prints is `commits`, so S3 says `commits` now.
S3's fail-closed rule applies to `--json` only: `--template` keeps its silent degradation because
`tests/e2e_changelog_template.rs` feeds it a non-git fixture and would fail otherwise — that file is outside
this slice, so strictness there is a follow-up, not a silent omission. Reaching the prefix-aware extractors
from the CLI required one facade line in `src/changelog.rs`, which is why that file is in the edit surfaces.
