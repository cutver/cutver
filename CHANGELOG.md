# Changelog

All notable changes to this project are documented in this file.
Format based on [Keep a Changelog](https://keepachangelog.com).

## [v0.12.0] - 2026-10-09
### ⚠️ Breaking Changes
- **plugin**: type the invocation operation and couple it to the capability ([763c244](https://github.com/cutver/cutver/commit/763c2444f5beb69ba1a069e48c20bc177f31f155)) by @Row0902
  > ⚠️ **Migration**: the plugin invocation carries typed fields. The JSON wire
- **plugin**: replace capability-as-export ABI with an explicit invocation envelope ([3710632](https://github.com/cutver/cutver/commit/371063249b990c9a9c0faebbe63b4cb33e9a8ed4)) by @Row0902
  > ⚠️ **Migration**: plugins must export `invoke` instead of an export named after

### 🐛 Bug Fixes
- **test**: tolerate CRLF when reading the byte-significant wire fixture ([01e8dba](https://github.com/cutver/cutver/commit/01e8dba5afad8fd57d0ed02f3e546b7a17a1c108)) by @Row0902
- **contract**: surface swallowed degradations and restore domain boundary purity ([fe249d7](https://github.com/cutver/cutver/commit/fe249d70d6d2f240837c9b1d718665cae239460d)) by @Row0902
- **ci**: escape changelog heading match in release notes extraction ([d9c8979](https://github.com/cutver/cutver/commit/d9c8979c984a1f89a0a5c0f390809f736d624898)) by @Row0902

### 📚 Documentation
- **odd**: index the deferred findings and point them at their issues ([963fde9](https://github.com/cutver/cutver/commit/963fde961b8b7077b6c145d40a050033dc762ff2)) by @Row0902
- **odd**: record the cutver-pdk extraction evidence ([6458522](https://github.com/cutver/cutver/commit/6458522fe7954d75b622aa591148528e71925691)) by @Row0902
- **rfc**: document the plugin invocation contract ([328a0b3](https://github.com/cutver/cutver/commit/328a0b3c1ab2a368df7842d2d30aef272744c6ba)) by @Row0902
- **odd**: record contract-compliance-hardening evidence and the RDD false positive ([680636f](https://github.com/cutver/cutver/commit/680636fd8611b31d6dd53e6dd100f90b208a7384)) by @Row0902

### 🔄 Code Refactoring
- **plugin**: move the wire contract into a shared cutver-pdk crate ([c08fb2a](https://github.com/cutver/cutver/commit/c08fb2a4606b931152a0fd9bfe9b7db4708633bd)) by @Row0902

### 🛠️ Maintenance & Dependencies
- **plugin**: pin the plugin wire contract with a golden fixture ([61c2720](https://github.com/cutver/cutver/commit/61c27204f1bff189222874e1b22906f9f9c0c01d)) by @Row0902

### 🚀 Features & Enhancements
- **changelog**: compute first-time contributors and expose release context to plugins ([fab9c1c](https://github.com/cutver/cutver/commit/fab9c1cc4dd1e37a346f105c0a222441c2dd4b3b)) by @Row0902

### 👥 Contributors
- @Row0902


---
**Full Changelog**: https://github.com/cutver/cutver/compare/v0.11.0...v0.12.0 • *CI Build #17*
## [v0.11.0] - 2026-10-08
### 🚀 Features & Enhancements
- **plugin**: introduce versioning.v1 DTOs and pluggable bump strategy engine in [#158](https://github.com/cutver/cutver/pull/158) ([d4d8660](https://github.com/cutver/cutver/commit/d4d86607e873d94dab3af7a3c578a57dd67fb6d9)) by @Row0902
- **plugin**: wire changelog.v1 capability into release notes formatting pipeline in [#157](https://github.com/cutver/cutver/pull/157) ([7a5954e](https://github.com/cutver/cutver/commit/7a5954e04580b84302be759447e61c0db69b98f0)) by @Row0902
- **plugin**: wire manifest.v1 capability into manifest discovery and mutation pipeline in [#156](https://github.com/cutver/cutver/pull/156) ([3627049](https://github.com/cutver/cutver/commit/36270491b78ad643c20040b87cac64606df97a05)) by @Row0902
- **cli**: add Git-style external subcommand dispatch for cutver-* executables in [#151](https://github.com/cutver/cutver/pull/151) ([ae76fb5](https://github.com/cutver/cutver/commit/ae76fb55be130d9690a84a6f118b4e746d6a85a6)) by @Row0902
- **plugin**: implement WasmDriver with Extism runtime behind feature flag in [#150](https://github.com/cutver/cutver/pull/150) ([89fc2b3](https://github.com/cutver/cutver/commit/89fc2b3e2742edda12f897925a689d5c9766eabf)) by @Row0902
- **plugin**: wire lifecycle hooks into two-phase mutation pipeline in [#149](https://github.com/cutver/cutver/pull/149) ([542efed](https://github.com/cutver/cutver/commit/542efed04f9ab047f410b9dee29e9119a2d02f00)) by @Row0902
- **plugin**: implement PluginManager and declarative config orchestration in [#148](https://github.com/cutver/cutver/pull/148) ([cca7e26](https://github.com/cutver/cutver/commit/cca7e267da59ee87cc72bb11416994159800bd6b)) by @Row0902
- **plugin**: implement ProcessDriver with JSON IPC and timeout isolation in [#144](https://github.com/cutver/cutver/pull/144) ([7200c24](https://github.com/cutver/cutver/commit/7200c2417abb7ae37f91317237d44671b55aeac8)) by @Row0902
- **plugin**: introduce microkernel types, DTOs, and Pillar VI contract in [#32](https://github.com/cutver/cutver/pull/32) ([447e322](https://github.com/cutver/cutver/commit/447e322f0818479a7459afeef5b2dfa87241b6e3)) by @Row0902
- **skills**: track cutver project skills and symlinks ([c02f265](https://github.com/cutver/cutver/commit/c02f265f7d149670a296e0341c60edde8ed3156e)) by @Row0902
- **container**: official OCI container packaging and GHCR publishing workflow in [#128](https://github.com/cutver/cutver/pull/128) ([d12e918](https://github.com/cutver/cutver/commit/d12e918e8f2cdc79b40e24bfc4466ebcabc56c14)) by @Row0902

### 🔄 Code Refactoring
- **plugin**: eliminate unwrap in wasm loader by propagating PluginName ([bac7fae](https://github.com/cutver/cutver/commit/bac7faec4fb403d85516d9a8f2c815cb103f0ce1)) by @Row0902

### 🐛 Bug Fixes
- **cli**: resolve batch files and PATHEXT extensions on Windows for external subcommands in [#152](https://github.com/cutver/cutver/pull/152) ([b721007](https://github.com/cutver/cutver/commit/b721007b6f2cbd831e44d23e760e0303d22800e3)) by @Row0902

### 📚 Documentation
- **rfc**: evolve RFC 0001 to Microkernel & Unified Plugin Engine in [#32](https://github.com/cutver/cutver/pull/32) ([d4812e4](https://github.com/cutver/cutver/commit/d4812e4db9353e6559e8f1722e675c20a0cc701c)) by @Row0902
- **rfc**: fix target versions to v0.11.0 / v0.12.0 in RFC 0001 ([5ded749](https://github.com/cutver/cutver/commit/5ded7497900e035b2631e2894fc78c381ddae5a9)) by @Row0902
- **rfc**: add RFC 0001 for Extism WebAssembly plugin architecture in [#32](https://github.com/cutver/cutver/pull/32) ([c174ea5](https://github.com/cutver/cutver/commit/c174ea5db0f2007a8fea9d6ffa1a0451e2b56f26)) by @Row0902
- **readme**: reflect expanded 8-skill catalog from cutver/skills in [#143](https://github.com/cutver/cutver/pull/143) ([f9c6ffb](https://github.com/cutver/cutver/commit/f9c6ffb334d0df1a7d74391f9c493219528d01d6)) by @Row0902
- **odd**: record commit hash and PR evidence for WSLC guide ([5e87910](https://github.com/cutver/cutver/commit/5e87910bfdf8b2a64e7e0eedbe69fcae1478d9b9)) by @Row0902
- **integrations**: add WSL Containers (wslc) execution guide in [#138](https://github.com/cutver/cutver/pull/138) ([2bc66f5](https://github.com/cutver/cutver/commit/2bc66f5f1871448fea22c05947c47ee8ad62ad2f)) by @Row0902
- **contract**: formalize cognitive doc design and transversal skill governance in Pillar IV in [#134](https://github.com/cutver/cutver/pull/134) ([de75eab](https://github.com/cutver/cutver/commit/de75eab97644d869d059f6a209056a4fee6e96ad)) by @Row0902
- restructure documentation into modular subfolders and cognitive line budgets in [#132](https://github.com/cutver/cutver/pull/132) ([097be2e](https://github.com/cutver/cutver/commit/097be2e2e7884e757fcbcbdc33be4808b8b3d6ac)) by @Row0902
- **readme**: apply cognitive doc design, new dogfooding hook, and v0.10.0 feature guides ([270191c](https://github.com/cutver/cutver/commit/270191c18a3c2b0fb2bd8177af5bb933d85b464e)) by @Row0902

### 🛠️ Maintenance & Dependencies
- **container**: enable latest tag when building on main branch in [#136](https://github.com/cutver/cutver/pull/136) ([4252ae6](https://github.com/cutver/cutver/commit/4252ae6ddb0496b08ce4139bb75d9e9ca08d11f1)) by @Row0902

### 👥 Contributors
- @Row0902


---
**Full Changelog**: https://github.com/cutver/cutver/compare/v0.10.0...v0.11.0 • *CI Build #16*
## [v0.10.0] - 2026-10-04
### 🔄 Code Refactoring
- **changelog**: upgrade release template with group_by_type, breaking migration details, and env helpers ([fcf00b5](https://github.com/cutver/cutver/commit/fcf00b58e7681f884e0997db3a2548c703fd916f)) by @Row0902
- **cli**: drop release.toml, workspace-relative diagnostics, dry-run polish, and WSL browser support in [#124](https://github.com/cutver/cutver/pull/124) ([44d0860](https://github.com/cutver/cutver/commit/44d0860f94aa55b5bd7b113561c5ccdf8131db83)) by @Row0902
- **domain**: introduce ManifestPath domain newtype and enforce forward-slash normalization in [#122](https://github.com/cutver/cutver/pull/122) ([27e89cb](https://github.com/cutver/cutver/commit/27e89cb48125c1c5ed2d735612f7693408206144)) by @Row0902
- **changelog**: modularize template evaluation, body rendering, and tests in [#120](https://github.com/cutver/cutver/pull/120) ([7b88787](https://github.com/cutver/cutver/commit/7b887872727a4240ab45361d60be375055b34a98)) by @Row0902
- **bump**: modularize bump domain types, errors, and test suite in [#118](https://github.com/cutver/cutver/pull/118) ([559c073](https://github.com/cutver/cutver/commit/559c07303ddff1bdcbcf54a956bb92eb00c3c8e6)) by @Row0902
- **init**: modularize configuration scaffolding, templates, and runner in [#116](https://github.com/cutver/cutver/pull/116) ([59ddf23](https://github.com/cutver/cutver/commit/59ddf2391c8cc62b10eedaf8c5919d4630682a2d)) by @Row0902
- **cli**: modularize CLI args definitions and normalization in [#114](https://github.com/cutver/cutver/pull/114) ([d12f0be](https://github.com/cutver/cutver/commit/d12f0be3afb710e2c6c149554cd8263e5195ceca)) by @Row0902
- **changelog**: modularize historical extraction engine and tests in [#112](https://github.com/cutver/cutver/pull/112) ([3dc55d6](https://github.com/cutver/cutver/commit/3dc55d6b0eeb4802f7107f5e60c4be397dba56bf)) by @Row0902
- **conventional**: modularize commit parser, types, and bump deduction in [#110](https://github.com/cutver/cutver/pull/110) ([f1fe45b](https://github.com/cutver/cutver/commit/f1fe45b537f195fa86428eb9b1b43eaa48b44b55)) by @Row0902
- **bump**: modularize mutation execution engine and doctor checks in [#108](https://github.com/cutver/cutver/pull/108) ([6386dc1](https://github.com/cutver/cutver/commit/6386dc1fb04a4d0961a74706c5a85e2b9bc061b1)) by @Row0902
- **cli**: modularize runner orchestration into focused handlers in [#106](https://github.com/cutver/cutver/pull/106) ([93d6118](https://github.com/cutver/cutver/commit/93d6118f25db09b2301951d99ee7de285ffc8182)) by @Row0902
- **git**: modularize git engine and introduce CommitSha newtype in [#104](https://github.com/cutver/cutver/pull/104) ([d90c423](https://github.com/cutver/cutver/commit/d90c423781da5a840bfab48c8818060320a6e662)) by @Row0902
- **changelog**: modularize context module and deduplicate chained PR references in [#102](https://github.com/cutver/cutver/pull/102) ([6cf1498](https://github.com/cutver/cutver/commit/6cf1498fb77de6636c6951bb1ce5976344dee34f)) by @Row0902
- **bump**: implement RAII MutationTransaction guard and eliminate procedural rollbacks in [#98](https://github.com/cutver/cutver/pull/98) ([e452998](https://github.com/cutver/cutver/commit/e45299856260335569bd4eb6a9bc8c6a59533552)) by @Row0902
- **changelog**: centralize ReleaseContext resolution and primary manifest version lookup in [#96](https://github.com/cutver/cutver/pull/96) ([154e176](https://github.com/cutver/cutver/commit/154e176b0d0681ba3a21e1cb15b8f5d688e6ba70)) by @Row0902
- **domain**: introduce TagName and TagPrefix newtypes and unify tag normalization in [#94](https://github.com/cutver/cutver/pull/94) ([d3915b8](https://github.com/cutver/cutver/commit/d3915b832d9d89332aee56f6350552b39504ed4d)) by @Row0902
- **bump**: compute changelog update in memory during phase 1 before disk mutation in [#89](https://github.com/cutver/cutver/pull/89) ([7d64178](https://github.com/cutver/cutver/commit/7d64178091ce7056fa0cfa87071cd2c4fe6af5f4)) by @Row0902

### 🚀 Features & Enhancements
- **changelog**: template resolution against root_dir, fail-fast template errors, commit body/breaking details, and MiniJinja helpers in [#126](https://github.com/cutver/cutver/pull/126) ([84b67f1](https://github.com/cutver/cutver/commit/84b67f18a60ae284dee3945dff7fe4fb833c7a93)) by @Row0902
- **cli**: terminal UX & accessibility: workspace-relative paths, OSC 8 hyperlinks, and 'cutver open' command in [#90](https://github.com/cutver/cutver/pull/90) ([b02b081](https://github.com/cutver/cutver/commit/b02b08142fbddcb6094b86cd2eb8a3a5a1ef496e)) by @Row0902

### 📚 Documentation
- **contract**: align quantitative line budgets with rust-craft global skill ([286154a](https://github.com/cutver/cutver/commit/286154a6b0ab024e4b04e0c3b0316912fdeeb0b2)) by @Row0902
- **contract**: formalize concurrency boundaries, RAII rollback, newtypes, and single-source-of-truth invariants ([1646ee8](https://github.com/cutver/cutver/commit/1646ee8d4f336bc45ac4ff8a3d744f82001073e3)) by @Row0902

### ⚡ Performance Improvements
- **engine**: parallelize read-only manifest compute, drift checks, and directory discovery with rayon in [#100](https://github.com/cutver/cutver/pull/100) ([3a20844](https://github.com/cutver/cutver/commit/3a208443e8d92cd8065193e50820a48abfe0f4dc)) by @Row0902
- **git**: adopt Rust 1.99 in-place zero-allocation string conversions and set MSRV in [#92](https://github.com/cutver/cutver/pull/92) ([3fd2000](https://github.com/cutver/cutver/commit/3fd20002d80f267169923e1b315da6d9228dac6a)) by @Row0902

### 🐛 Bug Fixes
- **git**: check remote tag collisions in preflight before mutating workspace in [#88](https://github.com/cutver/cutver/pull/88) ([bbe7e4d](https://github.com/cutver/cutver/commit/bbe7e4d2ddb94244b99aa57a8cc264ca22a9d7e3)) by @Row0902
- **changelog**: preserve original file line endings (LF vs CRLF) in changelog updates in [#87](https://github.com/cutver/cutver/pull/87) ([3cd1bf3](https://github.com/cutver/cutver/commit/3cd1bf37ab4c819ed12abcf7786ae45c8c8a4f6a)) by @Row0902
- **manifest**: eliminate expect() in production code for GradleEditor in [#86](https://github.com/cutver/cutver/pull/86) ([9e35a95](https://github.com/cutver/cutver/commit/9e35a95d085729d9bfbdc8e80c52a8ab29aba36a)) by @Row0902

### 👥 Contributors
- @Row0902


---
**Full Changelog**: https://github.com/cutver/cutver/compare/v0.9.1...v0.10.0 • *CI Build #14*
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
