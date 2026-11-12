# ADR-0003: Render context-derived changelog sections via opt-in configuration

**Status**: accepted
**Date**: 2026-11-12
**Owner**: Row0902
**Review date**: 2027-05-12 (proposal)
**Implementation**: complete
**Supersedes**: none

## Context

`ReleaseContext` computes repository-level metadata that cannot be derived from any individual commit alone, including `first_time_contributors` (`src/git/log.rs:311-318`). While custom MiniJinja templates and plugins consume this field, the built-in conventional renderer (`src/changelog/render/body.rs`) historically ignored it across both flat and package-grouped layouts.

During workspace package development (issue #201, `odd/tasks/workspace-package-changelog.md:L17-L18`), emitting a `### New Contributors` section by default was identified as a breaking changelog output change for existing repositories. The open question is how context-derived sections are admitted to the standard conventional changelog while preserving byte-for-byte output stability.

## Decision

We will render context-derived sections through an opt-in configuration setting:

1. **Opt-in Configuration**: Add `new_contributors = false` (default) under `[changelog]` in `cutver.toml`. Existing projects retain exact byte-for-byte output stability unless explicitly opted in.
2. **Heading Level & Placement**: When enabled and `first_time_contributors` is non-empty, emit `### New Contributors` at Level 3 (`###`). In flat mode, it appears after conventional commit categories; in package-grouped mode, it appears at the root level after all package sections and unowned changes.
3. **Shared Formatter**: Flat and grouped renderers share a single formatting helper (`render_new_contributors`), guaranteeing that grouping changes only the grouping.
4. **Item Formatting & Omission**: Contributors render as `- @{author}` (or `- {name}`). If `first_time_contributors` is empty, or if Git history was unreadable (triggering the fail-safe warning), the section is omitted entirely.

## Alternatives

| Option | Trade-off | Verdict |
| :--- | :--- | :--- |
| Enabled by default (`new_contributors = true`) | Emits new sections on next release for all existing repos with new authors | rejected — breaks output stability for existing projects |
| Opt-in via `[changelog] new_contributors = true` (chosen) | Preserves default output; users enable recognition deliberately | accepted |
| Template-only (status quo) | Forces users into custom MiniJinja templates for a standard feature | rejected — reopen if conventional changelogs are strictly commit-only |
| Per-package contributor breakdown | Attributes first-time authors to packages | rejected — contributor identity is repository-wide |

## Consequences

- Easier: Projects can celebrate first-time contributors automatically without writing custom MiniJinja templates.
- Safety: Existing repositories experience zero changelog drift or unintended layout changes.
- Invariant preserved: Flat and package-grouped changelogs share identical contributor formatting.
- Guard: Unreadable history fails safe by omitting the section rather than misidentifying established authors as newcomers.

## Implementation

- Configuration schema: `src/config/types.rs` (`Changelog::new_contributors`), allowlisted in `src/config/validation.rs`.
- Shared renderer: `src/changelog/render/body.rs` (`render_new_contributors`).
- Integration: Passed through `render_conventional` and `render_body_with_packages` (`src/changelog/sections.rs`).
- Specification: Documented in [Configuration Reference](../reference/configuration.md).

## Traceability

- Issue #203: `feat(changelog): render context-derived sections such as first-time contributors`
- Historical analysis: `odd/tasks/workspace-package-changelog.md` (Log L17–L18)
- Sibling decision: [ADR-0002: Shared version with path-filtered changelogs](0002-shared-version-with-path-filtered-changelogs.md)
