# Feature: Dogfooding Floating Tags, MiniJinja Templates & Clean Emojis

Goal: Modernize `cutver.toml` with `floating_major_tag = true` and MiniJinja `commit_message`, fix corrupted category heading emojis in `.github/templates/cutver/RELEASE.md` and `src/init/scaffold.rs`, and sanitize historical category emojis in `CHANGELOG.md`, adhering strictly to `CONTRACT.md`.

## Tasks
- [x] Task 1: Update `cutver.toml` to dogfood `floating_major_tag = true` and MiniJinja expression `commit_message = "chore(release): v{{ version }} [skip ci]"`.
- [x] Task 2: Restore crisp, valid Unicode emojis in `.github/templates/cutver/RELEASE.md` (`🚀 Features & Enhancements`, `🐛 Bug Fixes`, `⚡ Performance Improvements`, `🚜 Code Refactoring`, `📝 Documentation`, `🛠️ Maintenance & Dependencies`, `💬 Other Changes`, `👥 Contributors`).
- [x] Task 3: Sync `DEFAULT_RELEASE_TEMPLATE` in `src/init/scaffold.rs` with the restored emojis and ensure unit tests pass.
- [x] Task 4: Sanitize corrupted emoji glyphs in historical sections of `CHANGELOG.md` while strictly preserving tag headings.
- [x] Task 5: Verify with `cargo test`, `cargo clippy`, `cargo fmt`, and `cargo run -- doctor --check-changelog`.
