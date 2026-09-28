# Feature: Declarative Commit Line Formatting (c.line) & Rich Categories (Issue #80)

Goal: Abstract commit line formatting from MiniJinja templates into the functional core (`CommitContext.line`), make pre-rendered category strings (`features`, `fixes`, etc.) rich with PR links, hashes, and authors, and streamline templates to clean declarative views, adhering strictly to `CONTRACT.md` Pillars I, II, III, IV, and V.

## Tasks
- [x] Task 1: Add `line: String` and `bullet: String` to `CommitContext` in `src/changelog/context.rs`, serialize it, implement pure `format_commit_line` adhering to Pillar V (depth <= 2, <= 30 lines), and populate `c.line` and `c.bullet` in `enrich_commit_context` and `CommitContext::from(&ConventionalCommit)`.
- [x] Task 2: Update `build_context` in `src/changelog/context.rs` so pre-rendered categories (`features`, `fixes`, etc.) use the rich canonical line format with PR links, commit hashes, and authors.
- [x] Task 3: Simplify `.github/templates/cutver/RELEASE.md` and `DEFAULT_RELEASE_TEMPLATE` in `src/init/scaffold.rs` to declarative `- {{ c.line }}` lines.
- [x] Task 4: Add unit tests in `src/changelog/context.rs` for `format_commit_line` and bullet serialization/deserialization.
- [x] Task 5: Verify with `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt -- --check`, and `cargo run -- doctor --check-changelog`.

## Completion Evidence
- `cargo test`: 253 lib tests passed, all E2E test suites passed (e2e_bump, e2e_changelog_cli, e2e_changelog_template, e2e_conventional, e2e_doctor, e2e_init, e2e_lifecycle, e2e_style, test_context).
- `cargo clippy --all-targets -- -D warnings`: Clean, 0 warnings.
- `cargo fmt -- --check`: Formatting compliant.
- `cargo run -- doctor --check-changelog`: Successful validation, clean report.

