# Changelog

All notable changes to this project are documented in this file.
Format based on [Keep a Changelog](https://keepachangelog.com).

## [v0.9.1] - 2026-09-28
### 🐛 Bug Fixes
- **cli**: enrich raw git commits in changelog commands and build release binary from source (#84) in [#84](https://github.com/cutver/cutver/pull/84) ([36e7e16](https://github.com/cutver/cutver/commit/36e7e16e47a877b894b8221592daea3306b39040)) by @Row0902

### 📚 Documentation
- **odd**: record completion evidence for cli raw commits fix ([74a83d3](https://github.com/cutver/cutver/commit/74a83d3ab11e6ab8b041a3cbdc2625df9d6a39d3)) by @Row0902
- **changelog**: hydrate v0.9.0 release notes and guard release workflow tag detection [skip ci] ([5ddab07](https://github.com/cutver/cutver/commit/5ddab073ac460e6d075005b7cd8b10884ee81266)) by @Row0902

### 👥 Contributors
- @Row0902

---
**Full Changelog**: https://github.com/cutver/cutver/compare/v0.9.0...v0.9.1
## [v0.9.0] - 2026-09-28
### 🚀 Features & Enhancements
- **config**: add [project] table and expose project_name in release context (#82) in [#83](https://github.com/cutver/cutver/pull/83) ([2375d5c](https://github.com/cutver/cutver/commit/2375d5cda502aadb9363db3006b1f8d6475801a0)) by @Row0902
- **changelog**: declarative commit line formatting (c.line) and rich category pre-rendering (#80) in [#81](https://github.com/cutver/cutver/pull/81) ([40b1aab](https://github.com/cutver/cutver/commit/40b1aab5f7ca639e5c6ae25149ae1e008acba524)) by @Row0902
- **cli**: structured JSON export for release context (--json flag) (#66) in [#78](https://github.com/cutver/cutver/pull/78) ([0c8511d](https://github.com/cutver/cutver/commit/0c8511d89d24aa9be3e7b28e635aaf7c5bd6c279)) by @Row0902
- **changelog**: ANSI-styled terminal rendering for changelog commands in interactive TTY (#72) in [#77](https://github.com/cutver/cutver/pull/77) ([8fdd65a](https://github.com/cutver/cutver/commit/8fdd65a708f7fbc83bd1800c13d4361aa6524986)) by @Row0902
- **doctor**: structured status grid dashboard for 'cutver doctor' (#71) in [#76](https://github.com/cutver/cutver/pull/76) ([946bd38](https://github.com/cutver/cutver/commit/946bd38aad466aad1c38270e6b0dd1868c341210)) by @Row0902
- **cli**: tree-structured release plan layout and simulation banner for 'cutver bump' (#69) in [#75](https://github.com/cutver/cutver/pull/75) ([e7a89e9](https://github.com/cutver/cutver/commit/e7a89e983447116bb10fb4d30f0fbe5d526019d0)) by @Row0902
- **bump**: explain SemVer deduction rationale in 'cutver bump auto' (#70) in [#74](https://github.com/cutver/cutver/pull/74) ([b8494c5](https://github.com/cutver/cutver/commit/b8494c5f9f05ee86a9abf4ed8b1694fa494b70a6)) by @Row0902
- **cli**: terminal color hierarchy and TTY/NO_COLOR auto-detection (#68) in [#73](https://github.com/cutver/cutver/pull/73) ([8bbbc22](https://github.com/cutver/cutver/commit/8bbbc22ab3ad1dfb7443f77443af1e5a5452dc44)) by @Row0902
- **templating**: universal MiniJinja interpolation in git messages and publish hooks (#43) in [#67](https://github.com/cutver/cutver/pull/67) ([4e8c1a3](https://github.com/cutver/cutver/commit/4e8c1a3e1b1d8bc229406e9aff471793b5f6c264)) by @Row0902
- **template**: include both PR link and commit hash in release template in [#61](https://github.com/cutver/cutver/pull/61) ([d7bf008](https://github.com/cutver/cutver/commit/d7bf008574077c43c8bfd2f91400e56e3ae096a9)) by @Row0902

### 📚 Documentation
- **readme**: add visual CLI showcase screenshots and fix emojis ([c587af8](https://github.com/cutver/cutver/commit/c587af8ec78bfa012060891319f72bfd16fb6fd8)) by @Row0902
- **readme**: clarify homebrew 6.0+ tap trust syntax in [#64](https://github.com/cutver/cutver/pull/64) ([02fc599](https://github.com/cutver/cutver/commit/02fc599418cbddaa1da8d766e3de9714391ac3a0)) by @Row0902
- **readme**: add homebrew installation guide for macos and linux in [#63](https://github.com/cutver/cutver/pull/63) ([b93afdd](https://github.com/cutver/cutver/commit/b93afddf859de5344d387cab98339f56d5d0a159)) by @Row0902
- **readme**: add scoop installation guide for windows in [#62](https://github.com/cutver/cutver/pull/62) ([9a330e6](https://github.com/cutver/cutver/commit/9a330e6890600a8c362d27b389d8d492b584efe5)) by @Row0902
- **changelog**: deduplicate v0.8.0 heading line [skip ci] ([f846947](https://github.com/cutver/cutver/commit/f84694788f5d31373dd04c9df309a6371a3b66a9)) by @Row0902

### 🛠️ Maintenance & Dependencies
- **config**: dogfood floating major tag and MiniJinja release template in [#79](https://github.com/cutver/cutver/pull/79) ([a8eb347](https://github.com/cutver/cutver/commit/a8eb34762462429d75c625120c220431829b9f80)) by @Row0902
- **git**: ignore local linkedin post draft ([2149124](https://github.com/cutver/cutver/commit/21491247d097ce7e8efd0cd16321622b10ffdc95)) by @Row0902
- **git**: ignore local discord post drafts ([3c941aa](https://github.com/cutver/cutver/commit/3c941aa687a7e10df73f59b72b7c0d38d2ea1f44)) by @Row0902

### 👥 Contributors
- @github-actions[bot]
- @Row0902

---
**Full Changelog**: https://github.com/cutver/cutver/compare/v0.8.0...v0.9.0

## [v0.8.0] - 2026-09-26

### 🚀 Features & Enhancements
- **changelog**: support clean_description and adopt flagship release template (#57)
- **init**: scaffold rich MiniJinja release template by default and support full templates (#46) (#56)
- **git**: native floating major tags support (v1, v2) in bump, doctor, and publish (#50) (#55)
- **bump**: support first release flag --first-release and -fr (#49) (#54)

### 🐛 Bug Fixes
- **changelog**: filter non-version headings in list_versions and clean duplicate headers (#58)

### ⚡ Performance Improvements
- **cargo**: configure production release profile with fat LTO, strip, and panic abort (#60)

### 🛠️ Maintenance & Dependencies
- remove floating tag prune workaround and enable check-changelog in release workflow (#59)

### 👥 Contributors
- @Row0902

---
**Full Changelog**: https://github.com/cutver/cutver/compare/v0.7.0...v0.8.0
## [v0.7.0] - 2026-09-26

**✨ What's Changed in v0.7.0**

### 🚀 Features & Enhancements
- **init**: align starter cutver.toml template and wire cargo token in ci release (#53)

### 👥 Contributors
- @Row0902

---
**Full Diff**: https://github.com/cutver/cutver/compare/v0.6.0...v0.7.0
## [v0.6.0] - 2026-09-26

**✨ What's Changed in v0.6.0**

### 🚀 Features & Enhancements
- **changelog**: filter out self-referential release commits and ignored scopes (#51) (#52)

### 👥 Contributors
- @Row0902

---
**Full Diff**: https://github.com/cutver/cutver/compare/v0.5.3...v0.6.0
## [v0.5.3] - 2026-09-26

**✨ What's Changed in v0.5.3**

### 📚 Documentation
- **readme**: add ai coding agent skills section and install guide (#48)

### 👥 Contributors
- @Row0902

---
**Full Diff**: https://github.com/cutver/cutver/compare/v0.5.2...v0.5.3
## [v0.5.2] - 2026-09-26

**✨ What's Changed in v0.5.2**

### 📚 Documentation
- update org urls, showcase github actions, and overhaul readme (#47)

### 🛠️ Maintenance & Dependencies
- **config**: add [skip ci] to release commit message

### 👥 Contributors
- @Row0902

---
**Full Diff**: https://github.com/cutver/cutver/compare/v0.5.1...v0.5.2
## [v0.5.1] - 2026-09-25

### 🐛 Bug Fixes
- **changelog**: only break release extraction on versioned headings (#45)

### 🛠️ Maintenance & Dependencies
- automate release pipeline using cutver actions and conditional build matrix

### 📦 Other Changes
- **template**: add sparkle emoji to release notes header
- **template**: restore contributors section in release notes template
- **template**: remove redundant contributors section from release notes template

### 👥 Contributors
- @Row0902

---
**Full Diff**: https://github.com/cutver/cutver/compare/v0.5.0...v0.5.1
## [v0.5.0] - 2026-09-21

### 🚀 Features & Enhancements
- **git**: respect .mailmap and resolve GitHub handles from noreply emails
- **changelog**: dynamic release notes templating with MiniJinja (#42) (#44)

### 🛠️ Maintenance & Dependencies
- **config**: adopt MiniJinja release notes template in cutver.toml and release.yml
- **release**: prepend 'What's Changed' header and append full changelog link in release.yml (#41)

### 📦 Other Changes
- **template**: trim whitespace on release notes template blocks

### 👥 Contributors
- @Row0902

---
**Full Diff**: https://github.com/Row0902/cutver/compare/v0.4.0...v0.5.0
## [v0.4.0] - 2026-09-21

### Features
- **cli**: add cutver init for manifest discovery and cutver.toml generation (#38) (#39)
- **manifest**: native pyproject.toml and modern lockfiles (bun.lock, uv.lock) (#35)
- **doctor**: validate changelog consistency across git tags (#34)
- **cli**: add 'cutver changelog show <version>' primitive (#33)
- **cli**: add 'changelog latest' primitive to extract recent release notes (#26)

### Bug Fixes
- **git**: annotate init_test_repo with #[cfg(test)] to exclude from release builds (#37)

### Refactoring
- **cli**: polish diagnostics, error guidance, and dry-run banner with companion tone (#40)
- **arch**: modularize config, changelog, main, and e2e test suites (#36)

### Documentation
- **odd**: mark all refactoring tasks completed
- **odd**: complete tasks for issue #30
- **odd**: mark PR task as completed in cutver-ci-automate-release-notes.md

### Maintenance
- **release**: extract notes via cutver changelog latest in release.yml (#27)
## [v0.3.1] - 2026-09-20

### Bug Fixes
- **publish**: enforce publish.default_timeout and migrate e2e test fixtures (#24)

### Documentation
- modernize README with freeze SVG demo and author docs/references.md

### Maintenance
- **config**: configure publish.default_timeout in cutver.toml
- **config**: automate cargo publish in cutver.toml publish table
## [v0.3.0] - 2026-09-20

### Features
- **phase1**: adopt cutver.toml, conventional auto bump, and lifecycle hooks (#23)

### Bug Fixes
- unstage git index and restore changelog on stage/commit rollback (#11)
- resolve review findings for Windows types, post-commit tag rollback, and checksum globbing
- cross-platform process tree termination for preflight timeouts (#14)
- halt config discovery at repository boundary and canonicalize symlinks (#13)
- rollback manifests on post-write git errors and check tag in dry-run (#11, #12)
- atomic.rs tempfile collision resilience, fsync, and error cleanup (#10)

### Refactoring
- deduplicate init_git test helper into git::init_test_repo (#16)
- initialize TempFileGuard as active and document cleanup invariants (#15)

### Documentation
- record completed RDD review outcome in task log
- record follow-up tasks evidence for issues #9-#14

### Maintenance
- **config**: enable automated git push in cutver.toml publish table
- match source file name in config defaults test for cross-platform canonical paths
- compare manifest file name in drift test to support macOS and Windows canonical paths
- generate SHA256SUMS and sign release assets using Cosign (#9)

### Other Changes
- cargo fmt atomic.rs
- integrate issue #13 (config discovery boundary and symlinks)
- integrate issues #11 and #12 (bump rollback and dry-run tag check)
## [v0.2.0] - 2026-09-19

- **Format-preserving JSON editor**: Custom byte-span scanner replaces only the version value, preserving key order, indentation, comments, and trailing newlines (#4).
- **Atomic manifest mutations**: Two-phase release pipeline computes all edits in memory before writing with temp-and-rename atomic writes and automatic rollback on failure (#1).
- **Early tag collision abort**: Aborts pre-mutation with actionable guidance when the target release tag already exists (#3).
- **Config-relative paths**: `release.toml` paths and Git root resolve relative to the configuration directory, enabling invocation from any subdirectory (#5).
- **Preflight timeouts**: Optional per-step and global `default_timeout` with Unix process-group kill (`SIGKILL`) prevents hanging verification commands (#2).
- **Semantic changelog idempotency**: Section detection by version key decouples idempotency from the date heading, preventing duplicate entries across re-runs (#6).
- **Type-driven ManifestKind**: Strongly typed enum with serde validation replaces string-based kinds and eliminates silent defaults (#7).
- **Cross-platform CI & binaries**: GitHub Actions test matrix across Ubuntu, macOS, and Windows with automated binary releases for 5 architectures on `v*` tags.
- **Detailed CLI documentation**: Comprehensive help text and argument descriptions for `cutver --help`, `bump`, and `doctor`.

## [v0.1.0] - 2026-09-17

- Initial release: `cutver bump patch|minor|major` and `cutver doctor`,
  driven by `release.toml` with JSON, Cargo.toml, Gradle, and regex manifest editors;
  fail-fast preflight; annotated tags; published to crates.io.
