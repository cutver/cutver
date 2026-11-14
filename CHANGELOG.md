# Changelog

All notable changes to this project are documented in this file.
Format based on [Keep a Changelog](https://keepachangelog.com).

## v0.18.3 — 2026-11-14

### 🐛 Fixes
- **release**: verify before publishing, with the release split into two phases in [#282](https://github.com/cutver/cutver/pull/282) ([157ea25](https://github.com/cutver/cutver/commit/157ea257993e91bd8fe8c1f443d0993d761bb861)) by @Row0902
- **bump**: validate the release tag before the commit and name the landed state after it in [#280](https://github.com/cutver/cutver/pull/280) ([7019e02](https://github.com/cutver/cutver/commit/7019e0244d6c029c8056bb0749935dbee1fdf16d)) by @Row0902

### 📚 Documentation
- mark F-03 fixed in the audit status block in [#283](https://github.com/cutver/cutver/pull/283) ([5989f66](https://github.com/cutver/cutver/commit/5989f66312d0cc0d6a6cdef3d4baa5c7611f6bda)) by @Row0902
- mark F-09 fixed in the audit status block in [#281](https://github.com/cutver/cutver/pull/281) ([c3198b7](https://github.com/cutver/cutver/commit/c3198b733243ed67397d6bf4596cc69cb894bb8c)) by @Row0902
- record the 2026-11-13 robustness audit and what has since changed in [#279](https://github.com/cutver/cutver/pull/279) ([5136e1d](https://github.com/cutver/cutver/commit/5136e1dc351f04ba9c13c800d9cd0f65458c8cad)) by @Row0902

### 🛠️ Maintenance
- **release**: rehearse a red smoke to prove nothing is published in [#284](https://github.com/cutver/cutver/pull/284) ([616bb6b](https://github.com/cutver/cutver/commit/616bb6b29b09aa1ce711197b663313f6b745283d)) by @Row0902

### 👥 Contributors
@Row0902

[Compare v0.18.2...v0.18.3](https://github.com/cutver/cutver/compare/v0.18.2...v0.18.3)
## v0.18.2 — 2026-11-13

### 🐛 Fixes
- **git**: push one named ref per publish so a rejected tag cannot roll back the branch in [#278](https://github.com/cutver/cutver/pull/278) ([eb11d06](https://github.com/cutver/cutver/commit/eb11d0602a978e2ac2a341cf1b586eed227bd0e5)) by @Row0902
- **release**: require the release app token instead of falling back silently in [#277](https://github.com/cutver/cutver/pull/277) ([2f3c79d](https://github.com/cutver/cutver/commit/2f3c79d9fd67bd9d16d11cbcc4970d15bd21162a)) by @Row0902
- **release**: push as the release app so the main ruleset lets the bump through in [#275](https://github.com/cutver/cutver/pull/275) ([c796701](https://github.com/cutver/cutver/commit/c7967016dcc2f32beb35e2a62b443a38a49d439c)) by @Row0902

### 🛠️ Maintenance
- scope the release push guard to its job and correct the push ordering in [#276](https://github.com/cutver/cutver/pull/276) ([1831dfc](https://github.com/cutver/cutver/commit/1831dfc9454a5f845620ffd747c63e1f03bf58e2)) by @Row0902
- make the quality gates block a merge with one umbrella check in [#273](https://github.com/cutver/cutver/pull/273) ([6cada5a](https://github.com/cutver/cutver/commit/6cada5aed5a05128f045a439eeed22b6a1ce61d9)) by @Row0902

### 👥 Contributors
@Row0902

[Compare v0.18.1...v0.18.2](https://github.com/cutver/cutver/compare/v0.18.1...v0.18.2)
## v0.18.1 — 2026-11-13

### 🐛 Fixes
- **release**: cut the release with the CLI built from the released commit in [#256](https://github.com/cutver/cutver/pull/256) ([ccd2fe0](https://github.com/cutver/cutver/commit/ccd2fe0a041d36f5c8279a02b8a88ebd7fae02fe)) by @Row0902
- **release**: restore the github-publish pin now that 0.18.0 ships the fix in [#255](https://github.com/cutver/cutver/pull/255) ([4fd0f05](https://github.com/cutver/cutver/commit/4fd0f0582a607bb531482b6e434c8a6504800b56)) by @Row0902

### 👥 Contributors
@Row0902

[Compare v0.18.0...v0.18.1](https://github.com/cutver/cutver/compare/v0.18.0...v0.18.1)
## v0.18.0 — 2026-11-13

### ✨ Features
- **release**: bot-free contributors, section grouping and a plugin-owned title in [#247](https://github.com/cutver/cutver/pull/247) ([8533a99](https://github.com/cutver/cutver/commit/8533a9976a552571b307391806894aea4f266616)) by @Row0902

### 🐛 Fixes
- **release**: pin the plugin the published binary can actually run in [#253](https://github.com/cutver/cutver/pull/253) ([368fea4](https://github.com/cutver/cutver/commit/368fea4cf4a09e0e3b4730a7683338ce7f46ebcc)) by @Row0902
- **plugin**: mount declared filesystem grants where the guest looks for them in [#252](https://github.com/cutver/cutver/pull/252) ([8243740](https://github.com/cutver/cutver/commit/82437407186b72b2c569239956cf5b42acc24f49)) by @Row0902
- **ci**: stop the type-label gate from failing on opened in [#250](https://github.com/cutver/cutver/pull/250) ([ff04085](https://github.com/cutver/cutver/commit/ff040856157e3c9017eed93ea94b2c638a538f04)) by @Row0902
- **release**: stop the promotion from replacing the body the plugin published in [#249](https://github.com/cutver/cutver/pull/249) ([83a5789](https://github.com/cutver/cutver/commit/83a5789341c2729f353ed15a380ccbb4b2153a34)) by @Row0902
- **changelog**: drop the bot from the v0.17.0 contributor line in [#248](https://github.com/cutver/cutver/pull/248) ([80992f3](https://github.com/cutver/cutver/commit/80992f33a2204fef76173ff9a984c43b6aa16aa3)) by @Row0902
- **changelog**: restore the entries the 0.16.0 renderer filtered from v0.17.0 in [#245](https://github.com/cutver/cutver/pull/245) ([90b9ca4](https://github.com/cutver/cutver/commit/90b9ca49be611d670983e395bbae090ca54c613d)) by @Row0902

### 🛠️ Maintenance
- **release**: pin github-publish 0.2.1 now that the releasing binary sends notes in [#246](https://github.com/cutver/cutver/pull/246) ([f2f7d97](https://github.com/cutver/cutver/commit/f2f7d9733ef7bad54875137d842d56951d2a921f)) by @Row0902

### 👥 Contributors
@Row0902

[Compare v0.17.0...v0.18.0](https://github.com/cutver/cutver/compare/v0.17.0...v0.18.0)
## v0.17.0 — 2026-11-12

### ✨ Features
- **render**: add group_by_section so a heading is emitted once in [#242](https://github.com/cutver/cutver/pull/242) ([ac421fd](https://github.com/cutver/cutver/commit/ac421fdea7fad1d804ea5e50b6534d0221fc2848)) by @Row0902
- **release**: redesign the release entry for the way people read it now in [#239](https://github.com/cutver/cutver/pull/239) ([0e17737](https://github.com/cutver/cutver/commit/0e17737cccc4c6f802d53f7403671d75842f09c8)) by @Row0902
- **plugin**: hand the rendered release entry to on_post_release plugins in [#235](https://github.com/cutver/cutver/pull/235) ([6cb2b3e](https://github.com/cutver/cutver/commit/6cb2b3e7f5296a44b867e4545666197a2faa2edd)) by @Row0902

### 🐛 Fixes
- **release**: pin the plugin to the version the releasing binary speaks in [#241](https://github.com/cutver/cutver/pull/241) ([c9c4d17](https://github.com/cutver/cutver/commit/c9c4d17ff94f1fc16ce3e7b3012e6f69b428a726)) by @Row0902
- **release**: name the compare link and stop emitting build noise in [#238](https://github.com/cutver/cutver/pull/238) ([35a16f8](https://github.com/cutver/cutver/commit/35a16f8de5a1b6ab31d0d81fd2222ee1ef6e1fed)) by @Row0902
- **changelog**: keep release-scoped commits that carry content in [#237](https://github.com/cutver/cutver/pull/237) ([935fb51](https://github.com/cutver/cutver/commit/935fb516baa74b1a00c6cc53aa48662832f6ea93)) by @Row0902
- **changelog**: restore the v0.16.0 entry the release filter erased in [#234](https://github.com/cutver/cutver/pull/234) ([1e2f0cb](https://github.com/cutver/cutver/commit/1e2f0cb17b8da3826db785b0f163f705fd9d1bc4)) by @Row0902
- **release**: attribute the plugin's release to cutver-release[bot] in [#232](https://github.com/cutver/cutver/pull/232) ([6704936](https://github.com/cutver/cutver/commit/670493641c5d71c4aa8cda89d8a663213b9447d9)) by @Row0902

### 🛠️ Maintenance
- **release**: publish the crate again, and record why 0.16.0 never did in [#244](https://github.com/cutver/cutver/pull/244) ([b6ad841](https://github.com/cutver/cutver/commit/b6ad841727154d461670ed443481bc7e1898500b)) by @Row0902
- **release**: add a tag-scoped workflow to publish the crate in [#243](https://github.com/cutver/cutver/pull/243) ([42784bd](https://github.com/cutver/cutver/commit/42784bdfa41c91b691766f7695a5688beb20957f)) by @Row0902
- **release**: gate the bump on formatting and name the release in [#240](https://github.com/cutver/cutver/pull/240) ([e733514](https://github.com/cutver/cutver/commit/e733514dcfbb7bc1fab42f30cbf8ba0523999957)) by @Row0902
- **release**: pin the github-publish plugin to v0.2.1 in [#236](https://github.com/cutver/cutver/pull/236) ([67f0913](https://github.com/cutver/cutver/commit/67f0913aec9a3edd66d908ee7b252d7e8942a153)) by @Row0902

### 👥 Contributors
@Row0902

[Compare v0.16.0...v0.17.0](https://github.com/cutver/cutver/compare/v0.16.0...v0.17.0)
## [v0.16.0] - 2026-11-12
### 🚀 Features & Enhancements
- **plugin**: let the github-publish plugin own the release and publish its draft in [#230](https://github.com/cutver/cutver/pull/230) ([10c851e](https://github.com/cutver/cutver/commit/10c851e))
- **plugin**: load the github-publish plugin in the release workflow and add a dry-run rehearsal in [#229](https://github.com/cutver/cutver/pull/229) ([28c0d3d](https://github.com/cutver/cutver/commit/28c0d3d))

### 🛠️ Maintenance & Dependencies
- **release**: pause crates.io publication until the GitHub path is proven in [#231](https://github.com/cutver/cutver/pull/231) ([54d34c0](https://github.com/cutver/cutver/commit/54d34c0))

### 👥 Contributors
- @Row0902


---
**Full Changelog**: https://github.com/cutver/cutver/compare/v0.15.0...v0.16.0
## [v0.15.0] - 2026-11-12
### 🚀 Features & Enhancements
- **init**: teach the scaffold the template sink and first-time contributors in [#228](https://github.com/cutver/cutver/pull/228) ([7785834](https://github.com/cutver/cutver/commit/7785834c9d417b01f764b62eb97aa6842db4ade8)) by @Row0902

### 📚 Documentation
- **cli**: list every plugin subcommand the binary ships in [#227](https://github.com/cutver/cutver/pull/227) ([dcb4794](https://github.com/cutver/cutver/commit/dcb47944d89d792f2d79bbc4fe14c418c3e7699a)) by @Row0902
- **config**: state how cutver-pdk is versioned and published in [#226](https://github.com/cutver/cutver/pull/226) ([1d1e5b2](https://github.com/cutver/cutver/commit/1d1e5b2db6c9477ff0471e539aec675fa48cbe9f)) by @Row0902
- **guides**: add a public plugin authoring guide in [#225](https://github.com/cutver/cutver/pull/225) ([e67b636](https://github.com/cutver/cutver/commit/e67b6362d13f2485196987394bf08a0f30a32c1e)) by @Row0902

### 👥 Contributors
- @Row0902


---
**Full Changelog**: https://github.com/cutver/cutver/compare/v0.14.0...v0.15.0 • *CI Build #21*
## [v0.14.0] - 2026-11-12
### 🚀 Features & Enhancements
- **manifest**: kind = "template" for AST-safe source generation in [#224](https://github.com/cutver/cutver/pull/224) ([517f556](https://github.com/cutver/cutver/commit/517f5567faf6b07d3842f310657e5b453241da91)) by @Row0902
- **changelog**: render context-derived sections such as first-time contributors in [#215](https://github.com/cutver/cutver/pull/215) ([8ef1a88](https://github.com/cutver/cutver/commit/8ef1a88fd26b601166f264fc050028f80749dac5)) by @Row0902
- **plugin**: add restore, update, search, info and preserve subtables in [#209](https://github.com/cutver/cutver/pull/209) ([864c918](https://github.com/cutver/cutver/commit/864c918f42b826b23aa8175f0fa1b77a7dc5ad85)) by @Row0902
- **changelog**: render one section per declared package ([feaf057](https://github.com/cutver/cutver/commit/feaf057657c12669a9ed02b2bc2b19c9e3e13a58)) by @Row0902
- **git**: select commits per declared package path ([92e702f](https://github.com/cutver/cutver/commit/92e702f18cd1640098fbdf2831b6a0e4cd59aac6)) by @Row0902
- **config**: add an optional package identity to manifest entries ([97b8102](https://github.com/cutver/cutver/commit/97b810249d476aa5a4c4aed36b0b81e48555ac2b)) by @Row0902
- **skills**: add the docs-voice claim-sweep skill ([e5d6179](https://github.com/cutver/cutver/commit/e5d6179adc9795624efc6ea5bd0513aa0e6275a9)) by @Row0902

### 🔄 Code Refactoring
- make the manifest, changelog and git module roots pure facades in [#223](https://github.com/cutver/cutver/pull/223) ([ad3d4d9](https://github.com/cutver/cutver/commit/ad3d4d9069507677b79455143d82214de3d20025)) by @Row0902
- **manifest**: replace the legacy src/manifest/mod.rs with a sibling module in [#221](https://github.com/cutver/cutver/pull/221) ([59278f0](https://github.com/cutver/cutver/commit/59278f0b7ffe8dc57b162c78240b84a3e094d499)) by @Row0902
- **git**: decompose log.rs into a facade over focused submodules in [#219](https://github.com/cutver/cutver/pull/219) ([6187651](https://github.com/cutver/cutver/commit/6187651d8fc52904b69a9a532c5f2c1b2de943d1)) by @Row0902
- **changelog**: remove duplicated render, filter and heading paths in [#218](https://github.com/cutver/cutver/pull/218) ([de1739e](https://github.com/cutver/cutver/commit/de1739e553ec5cb832ea705f15b0d1c3c4c8b311)) by @Row0902
- **plugin**: bring manager.rs production code back under the Pillar V.4 target in [#210](https://github.com/cutver/cutver/pull/210) ([032577a](https://github.com/cutver/cutver/commit/032577a8fb54783a1a58320e73b33d2c53ceb841)) by @Row0902

### 🐛 Bug Fixes
- **bump**: remove cli::BumpLevel dependency from domain layer in [#217](https://github.com/cutver/cutver/pull/217) ([91d22da](https://github.com/cutver/cutver/commit/91d22dad6b1abda28d8e9993abaf780ea82db1d8)) by @Row0902
- **changelog**: insert a blank line between header and new entry in [#216](https://github.com/cutver/cutver/pull/216) ([807bb4e](https://github.com/cutver/cutver/commit/807bb4eaf3b29240b72ec2c9d91a614ef3033bf4)) by @Row0902
- **cli**: align --help strings for --first-release and plugin add --dry-run with behavior in [#214](https://github.com/cutver/cutver/pull/214) ([93a8d08](https://github.com/cutver/cutver/commit/93a8d081439bb0abca0981f8b38d67b6c6ebd07c)) by @Row0902
- **ci**: validate PR type labels against defined set in [#213](https://github.com/cutver/cutver/pull/213) ([d775ff5](https://github.com/cutver/cutver/commit/d775ff53a2ad670fe433cfaff35491e31a9c9637)) by @Row0902
- **hooks**: execute pre_bump hook before manifest mutations in [#211](https://github.com/cutver/cutver/pull/211) ([589f888](https://github.com/cutver/cutver/commit/589f888a0161d6ad70366087067bf7443e05c1e4)) by @Row0902
- **pipeline**: execute publish push before running on_post_release plugins in [#207](https://github.com/cutver/cutver/pull/207) ([5e38935](https://github.com/cutver/cutver/commit/5e38935431260ccfa7a29b38e8599ae30fa8c732)) by @Row0902
- **git**: gate the symlink test on unix and keep the guard diagnostic canonical ([5b38a33](https://github.com/cutver/cutver/commit/5b38a33a4b1befada1167b0f7e861229fbe65566)) by @Row0902
- **git**: normalize declared paths against the repository on every platform ([ad2ba2a](https://github.com/cutver/cutver/commit/ad2ba2a128b7eca1c41b5ce27c11d1b6862a02b5)) by @Row0902
- **git**: anchor package pathspecs and never drop range commits ([c86a3c2](https://github.com/cutver/cutver/commit/c86a3c2d4f098a0c64a6870bc46fa018955e10d3)) by @Row0902
- **git**: place a mixed commit only in its package, never twice ([86c2a66](https://github.com/cutver/cutver/commit/86c2a666c604162e0672543660fef4e9526d20e3)) by @Row0902
- **git**: keep merge commits in per-package selection ([a526bd2](https://github.com/cutver/cutver/commit/a526bd261718f9d666fafc5c841afb95eaa5f72b)) by @Row0902
- **config**: name the offending table when an unknown nested table is empty ([377f0fa](https://github.com/cutver/cutver/commit/377f0fae00e4dbe424454a9d20c388cbcfa9b94b)) by @Row0902
- **config**: reject unknown cutver.toml keys with a hard error ([b2a37a2](https://github.com/cutver/cutver/commit/b2a37a238f92c588107f6172c40be1ef54ef729c)) by @Row0902
- **odd**: scope the feature to unknown keys only ([30526c3](https://github.com/cutver/cutver/commit/30526c3be2f397fae3f929782356c0fb989bec06)) by @Row0902
- **odd**: open the unknown-config-keys feature ([86ea7e0](https://github.com/cutver/cutver/commit/86ea7e0e3d718ecfc5d2e79d916790c3e5014429)) by @Row0902
- **container**: keep `latest` on the lean image ([2018391](https://github.com/cutver/cutver/commit/2018391b1fd1902efe401089a4a8aef4bf6675e1)) by @Row0902

### 📚 Documentation
- **internals**: bring references.md and design.md within Pillar IV.5 line budget in [#212](https://github.com/cutver/cutver/pull/212) ([4d9f526](https://github.com/cutver/cutver/commit/4d9f5263a8257abbb5a118e142b13307bff94436)) by @Row0902
- update plugin showcase gallery asset with github-publish ([a4195c0](https://github.com/cutver/cutver/commit/a4195c0ea63b2c00c7aa9e3bbd7e586f576c8dbd)) by @Row0902
- **readme**: standardize on modern client-id for create-github-app-token action ([33a6781](https://github.com/cutver/cutver/commit/33a678101a35efaf4b630f2d24ccf281d9294923)) by @Row0902
- **readme**: align release workflow snippet with CUTVER_APP_ID secret ([98b6b0c](https://github.com/cutver/cutver/commit/98b6b0c6d70f4b71cc7060e7f5b2e83dc84a7580)) by @Row0902
- **readme**: add official brand identity, accessibility metadata, accordions, and FAQ ([48b333d](https://github.com/cutver/cutver/commit/48b333dd71ceb10781064ca3190d6eb6197a2fd7)) by @Row0902
- **readme**: replace ASCII architecture diagram with adaptive SVG diagrams ([c51bee6](https://github.com/cutver/cutver/commit/c51bee6815f57f51b8ef692876beb1a7e963f163)) by @Row0902
- **readme**: simplify gallery lead text and focus alt-text strictly on cutver output ([6d79c91](https://github.com/cutver/cutver/commit/6d79c914d99faaf9480baae0ebdffc3c4dc999e5)) by @Row0902
- **readme**: add visual gallery with modern terminal showcases and refine language ([4fc9ae2](https://github.com/cutver/cutver/commit/4fc9ae2e5afc0bf3fde38613f47825a3fc0a9078)) by @Row0902
- **readme**: redesign with cognitive architecture, official insignia, and interactive sections ([46854b9](https://github.com/cutver/cutver/commit/46854b955ff8516b607ae78c1a2760739da8eb9c)) by @Row0902
- **odd**: drop T6 and reclassify its finding as a review false positive ([f8bea88](https://github.com/cutver/cutver/commit/f8bea889e05604e71d5a4db8e673306dea7f9234)) by @Row0902
- **adr**: mark ADR-0002 implemented and correct the roadmap row ([6bf7076](https://github.com/cutver/cutver/commit/6bf70769482bf39a12c1ffe4a449f5d206dd307f)) by @Row0902
- **odd**: track workspace package identity with a changelog rendering baseline ([e514511](https://github.com/cutver/cutver/commit/e5145110b3e423b8822d6ecd0c3e7a873e1b4774)) by @Row0902
- point the roadmap and the monorepo guide at the decision log ([0a5e90b](https://github.com/cutver/cutver/commit/0a5e90be8dc4e0538c6513a6e48cec7e854398e0)) by @Row0902
- **adr**: open the decision log with ADR-0001 and ADR-0002 ([6fb0e8d](https://github.com/cutver/cutver/commit/6fb0e8d0c656a7f6504aeee8168c6d5b9cecdf97)) by @Row0902
- **rfc**: align both RFCs with the rfc-authoring skill ([a0aab17](https://github.com/cutver/cutver/commit/a0aab1733126adb6a6e98d51ef07af8d6d2a94b0)) by @Row0902
- **guides**: point the monorepo guide at the decision record ([d0f8070](https://github.com/cutver/cutver/commit/d0f8070b372c2ee2a3af6ffbbbf59c4aa7a86bc9)) by @Row0902
- **rfc**: record the workspace releases decision ([f219183](https://github.com/cutver/cutver/commit/f219183d4deb10655a8275abd68329bdef7031be)) by @Row0902
- **rfc**: propose per-package release units for monorepos ([8e4da10](https://github.com/cutver/cutver/commit/8e4da1078f5c07d1d34974f576311d6ef7962fff)) by @Row0902
- **odd**: open the workspace support RFC feature ([95be9b2](https://github.com/cutver/cutver/commit/95be9b21861fd0389f6df1167af90abd4953f883)) by @Row0902
- **config**: scope the accepted-key claim and bring the reference under the ceiling ([65ad33a](https://github.com/cutver/cutver/commit/65ad33a6c279a2df6bc103e5da8e1444a5e13108)) by @Row0902
- **internals**: repair the design document citations and split the doctor row ([fe8b920](https://github.com/cutver/cutver/commit/fe8b9206d10919ac862e9ba7483f8176f056551d)) by @Row0902
- **odd**: record the design evolution evidence and the roadmap classifications ([99b1827](https://github.com/cutver/cutver/commit/99b182729e7fd1640d103d7861c6eec89dcdb9aa)) by @Row0902
- **internals**: evolve the design document with explicit capability status ([184e5d3](https://github.com/cutver/cutver/commit/184e5d383836b87548a22332c9da59fd61e7b1f7)) by @Row0902
- **odd**: open the design document evolution feature ([7fff633](https://github.com/cutver/cutver/commit/7fff6332f9f804f09bbe9c130428dcab58c12d59)) by @Row0902
- **odd**: record the round 5 findings, the verification and the residual ([5f7ca5e](https://github.com/cutver/cutver/commit/5f7ca5e7d241594fa5e0d2e16c6f63cffcbcf6df)) by @Row0902
- correct the pyproject table key and scope the legacy reference title ([6f0581b](https://github.com/cutver/cutver/commit/6f0581bc374146baf0ea58b9cc5600021a0320e4)) by @Row0902
- scope the versioning strategy claim to its one implemented value ([ec08155](https://github.com/cutver/cutver/commit/ec08155ac7a05cd65e1a46f5ebd47fa640f08e93)) by @Row0902
- **odd**: record the round 4 findings ([9e77b4e](https://github.com/cutver/cutver/commit/9e77b4ec85bdfb7ba58f907c2ecf8205b8839bd0)) by @Row0902
- correct the documented defaults and fill the reference gaps ([7c6d215](https://github.com/cutver/cutver/commit/7c6d2158876098a0eb8a778f0c4cae78b161de9c)) by @Row0902
- **odd**: record the round 3 findings and the method change ([84c311d](https://github.com/cutver/cutver/commit/84c311d161e567551549d2cc20aa5480d360e7d8)) by @Row0902
- document the real workspace behaviour and the plugin manifest kind ([dd39702](https://github.com/cutver/cutver/commit/dd3970270be9599615370068c29ba83c859eef59)) by @Row0902
- **odd**: record the round 2 findings and their evidence ([e826a1b](https://github.com/cutver/cutver/commit/e826a1b1d63a3d26747723f84b5bedea84ad88ea)) by @Row0902
- scope the last two repeats of already-fixed claims ([a82bd0e](https://github.com/cutver/cutver/commit/a82bd0ed0c97354baf10b4d5cf2725269f9a0bb5)) by @Row0902
- correct the integration guides and the remaining overstatements ([1b9a6e3](https://github.com/cutver/cutver/commit/1b9a6e317c5e5d1d357ce915d8dc1b32b81b41b7)) by @Row0902
- **odd**: record the claim fixes and the cli help follow-up ([5287074](https://github.com/cutver/cutver/commit/5287074ff4484cc0a23cbde910a513219477d0c0)) by @Row0902
- scope the dry-run, staging and format-preservation claims to the code ([89c6546](https://github.com/cutver/cutver/commit/89c65461afbf7bc09e3fe197668ddcb752819829)) by @Row0902
- **bump**: drop the mix.lock staging claim the code does not honour ([9c5b8c0](https://github.com/cutver/cutver/commit/9c5b8c05d9ecf98d74ff61b70bad5ec34324da6c)) by @Row0902
- **odd**: open the documentation claim-accuracy closure feature ([f9dd3d2](https://github.com/cutver/cutver/commit/f9dd3d2f4897caaec124794c7d7f13a1d87bd78e)) by @Row0902
- **internals**: two core principles claimed more than the code does ([8f35fc0](https://github.com/cutver/cutver/commit/8f35fc09929ee5e07f2b7058f1afe3035f821344)) by @Row0902
- **readme**: the Gradle and regex editors can touch more than the version ([0aa792b](https://github.com/cutver/cutver/commit/0aa792b43289bb6d8c12dccd5db76a0bb65abf1b)) by @Row0902
- scope the documented guarantees to what the code does ([8e3be1f](https://github.com/cutver/cutver/commit/8e3be1f2341ebf0b3617325c6de14983c2d1f9e1)) by @Row0902
- fix the overstatements the verification found ([52eda4d](https://github.com/cutver/cutver/commit/52eda4d1c2d03d5d071c381a8d8a82872087c290)) by @Row0902
- document the plugin dispatch switch and the optional hash ([c0d65c8](https://github.com/cutver/cutver/commit/c0d65c82a6ec1a69e6a326501be86db0ca21d6af)) by @Row0902
- **readme**: lead with the microkernel and first-class plugins ([1e9fd75](https://github.com/cutver/cutver/commit/1e9fd758f09fc6d7136e0dcd88e6f4d30ffa4b14)) by @Row0902

### 📦 Assets
- **avatar**: optimize avatar with full-bleed background and generous safe-area padding ([422a204](https://github.com/cutver/cutver/commit/422a204b760b6f5ff3ccc2b2a9cc693e9c34b7d9)) by @Row0902
- add official GitHub Open Graph social preview card (1280x640) ([ce6770d](https://github.com/cutver/cutver/commit/ce6770d5a97cde440ccd1daad1254a9db3eca329)) by @Row0902
- add high-resolution PNG renders for avatar, brand marks, and architecture ([9b08836](https://github.com/cutver/cutver/commit/9b08836d966c5cef120522f45945bb32b6a4686c)) by @Row0902

### 🛠️ Maintenance & Dependencies
- require exactly one type label on every pull request ([d31c6c8](https://github.com/cutver/cutver/commit/d31c6c8d1a13c8001dff39f15230e2936bfe0ac2)) by @Row0902

### 👥 Contributors
- @Row0902


---
**Full Changelog**: https://github.com/cutver/cutver/compare/v0.13.0...v0.14.0 • *CI Build #20*
## [v0.13.0] - 2026-10-11
### 📚 Documentation
- **odd**: record slice 5 as delivered and close the task list ([e9afa4a](https://github.com/cutver/cutver/commit/e9afa4ad48c6d69f1b16d9f4f034229a17d35c4f)) by @Row0902
- **plugin**: add the plugin reference and wire it into the docs ([4daa4b6](https://github.com/cutver/cutver/commit/4daa4b6f4408cedb7a4a948bb47fd947f8e546b4)) by @Row0902
- **odd**: record slice 3b, D20-D22, and what verification caught ([4148a82](https://github.com/cutver/cutver/commit/4148a82ba04fd2d9bc8e7a65075cba645153c6d2)) by @Row0902
- **odd**: record slice 3a, D13, and what verification rejected ([0bd3c49](https://github.com/cutver/cutver/commit/0bd3c49b79f4bd6a7e03cd7025167e9094943288)) by @Row0902
- **odd**: record slice 2 and the design decisions its verification produced ([94bdaa8](https://github.com/cutver/cutver/commit/94bdaa82821c70a6d210c24d071468228e9beec1)) by @Row0902
- **odd**: record the plugin install CLI plan and slice 1 evidence ([aa3f85e](https://github.com/cutver/cutver/commit/aa3f85ebde97928c1b996bed39d64bbb5ebe6afe)) by @Row0902
- **odd**: record the runner proof and close the #174 tasks ([c9599e4](https://github.com/cutver/cutver/commit/c9599e4abb7f533079bcf1e375bfaf6af9fc17eb)) by @Row0902
- **odd**: require a noise-aware protocol before timing the size profile ([21cdfd2](https://github.com/cutver/cutver/commit/21cdfd250242acaf207428a45cc8afb12f9bd651)) by @Row0902
- **odd**: close the size spike with its outcome and recommendation ([c07d49b](https://github.com/cutver/cutver/commit/c07d49bb8eb49a9a34bdfcb1e636dea784635c6f)) by @Row0902
- **odd**: record the plugin-feature size spike ([c6a04eb](https://github.com/cutver/cutver/commit/c6a04eb09f747a0f6ca31b2153064e39d3845e68)) by @Row0902
- **odd**: record the plugins-feature CI coverage evidence ([dc86be7](https://github.com/cutver/cutver/commit/dc86be7d4b71ea7bbbb1ebf62987a680562e601e)) by @Row0902

### 🐛 Bug Fixes
- **plugin**: keep the [plugins] parent implicit on add and remove ([32e881e](https://github.com/cutver/cutver/commit/32e881e37a2148a908d21f9fd7fccebef900f51f)) by @Row0902
- **ci**: pass -f Containerfile to the alpine build in the verification workflow ([3c20072](https://github.com/cutver/cutver/commit/3c20072b41e09a3f7df8d4ca29cb632dac0e89ef)) by @Row0902
- **plugin**: tell the user of a lean binary how to get plugin support ([7f0f6d6](https://github.com/cutver/cutver/commit/7f0f6d67ccebceaee6cbd68916a934ed59b771e9)) by @Row0902
- **container**: touch every workspace member's sources before the real build ([d3a6849](https://github.com/cutver/cutver/commit/d3a6849520c365f85f0cab5df15e2ca8d638b57f)) by @Row0902
- **container**: copy the workspace member manifest into the image ([bb33b4e](https://github.com/cutver/cutver/commit/bb33b4e163760191576b88c3d596f82e2d6411af)) by @Row0902

### 🚀 Features & Enhancements
- **plugin**: add `cutver plugin add` with a verified digest gate ([988d7fa](https://github.com/cutver/cutver/commit/988d7faea08cd393edffb4cb6c4e454e2404791b)) by @Row0902
- **plugin**: add plugin list and a surgical plugin remove ([1203692](https://github.com/cutver/cutver/commit/12036922384f7ffe299ea40766c787627a41c959)) by @Row0902
- **plugin**: add a typed registry client behind the plugins feature ([0f96632](https://github.com/cutver/cutver/commit/0f96632be92e8f110a0fb617093d714aa601c4b3)) by @Row0902
- **plugin**: fail closed on an unresolvable plugin cache root ([d42b625](https://github.com/cutver/cutver/commit/d42b625f28f4f180709088c161d08592d06a9eb2)) by @Row0902

### 🛠️ Maintenance & Dependencies
- **ci**: record the lean-image guard and remove the throwaway workflow ([f2ad074](https://github.com/cutver/cutver/commit/f2ad074405e9dcab65b5c636dcd788d7db77d030)) by @Row0902
- **ci**: record the musl/Alpine run and guard the Containerfile knob's default ([0af7dc5](https://github.com/cutver/cutver/commit/0af7dc510179dbc3bbe2522c0c0305dd25dde685)) by @Row0902
- **ci**: verify the plugins feature on the musl targets and the Alpine image ([096fa8a](https://github.com/cutver/cutver/commit/096fa8ad8ffabb2258876aacfad9574d5ba37a95)) by @Row0902

### 🛠️ Maintenance & Dependencies
- cover the plugins feature on every target OS ([18e514d](https://github.com/cutver/cutver/commit/18e514d7e173a8692c740c6748e20e12f09d8889)) by @Row0902

### 👥 Contributors
- @Row0902


---
**Full Changelog**: https://github.com/cutver/cutver/compare/v0.12.0...v0.13.0 • *CI Build #19*
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
