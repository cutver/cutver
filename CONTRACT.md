# Cutver Architectural & Quality Contract

> **Status**: Active / Canonical Law  
> **Scope**: Applies to all contributions, refactors, features, and automated agents in the `cutver` ecosystem.

---

## Vision & Objective

`cutver` is a mission-critical release orchestration engine. It touches canonical Git histories, mutates package manifests across diverse ecosystems, generates changelogs, tags releases, and triggers production deployment hooks.

Because developers and automated CI/CD pipelines entrust `cutver` with their canonical versioning and release workflows, software quality in this project is not an afterthought—it is a non-negotiable prerequisite.

---

## ⚖️ Enforcement

A rule nothing executes is a wish with a citation. This project learned that four times in one session: `permissions.env` (#300), `[gates] require`, `--ci` without a named sink, and the 400-line ceiling itself were each declared and each unchecked. That is a pattern, not four accidents, so every limit below is exactly one of four things:

| Level | Means | What the clause must name |
| :--- | :--- | :--- |
| **Enforced** | A check fails the build or the required `CI gate` | The test or gate step |
| **Ratchet** | Enforced, and its recorded allowance may only tighten | The check and its allowlist |
| **Judgement** | No machine can decide it | Who decides, and why a machine cannot |
| **Target** | An aspiration that is deliberately not a gate | The evidence a change attaches when it misses |

The limits this contract carries, and which of the four each one is:

| Limit | Level | Where it is checked |
| :--- | :--- | :--- |
| 200 lines per document | **Enforced** | `every_document_stays_under_the_line_ceiling` in `tests/ci_workflow_contract.rs`, mirrored by a `ci.yml` step |
| 400 lines per production source file | **Ratchet** | `tests/source_line_budget.rs`, with its sixteen allowlisted files |
| 350-line facade trigger | **Judgement** | Review; `tests/source_line_budget.rs` states why it is not mechanical |
| 300–400 lines per pull request | **Target** | The pull request states its slice boundary, or splits |
| The lean build's dependency shape | **Enforced** | `tests/lean_dependency_graph.rs`, plus a `cargo tree` step in `scripts/check.sh` and in `ci.yml` |

**Every declaration ships with its check.** A clause that a machine could verify may not rely on review, and a new numeric limit added anywhere in this document joins the table above with its level and its checker. When a check is expensive, it lands in the same pull request that declares the rule. When a check is genuinely impossible, the clause says *judgement* and says why — "we will remember to do this" is not an enforcement level.

---

## 🏛️ Pillar I: Architectural Purity & Mutation Safety

### 1. Functional Core, Imperative Shell
- **Domain logic is pure and in-memory**: SemVer calculation, Conventional Commit parsing, template rendering, and manifest change computation must never perform disk I/O, Git process spawning, or network calls directly.
- **I/O lives strictly at the boundaries**: All filesystem access, Git executions, and terminal outputs are handled by boundary adapters (`crate::manifest`, `crate::git`, `crate::cli`).
- **Zero global state**: Functions must be referentially transparent and deterministic.

### 2. Two-Phase Mutation & Atomic Rollback
Every command that mutates files or Git state must adhere to a strict two-phase execution lifecycle:
1. **Phase 1: Compute (In-Memory Validation)**:
   - Compute 100% of all changes in memory.
   - Run preflight validations and tag collision checks.
   - If any calculation, syntax check, or validation fails, abort immediately without touching a single byte on disk.
2. **Phase 2: Apply & Rollback (Atomic I/O)**:
   - Apply mutations using atomic file writes (`crate::atomic::write_atomic`).
   - **RAII Transaction Guarantee**: Rollback must be guaranteed structurally by the type system or RAII guards (e.g., transaction drop guards), never by scattering manual procedural `rollback()` invocations across error return paths.
   - If any write, hook, or staging step fails, rollback restores every touched file to its original byte-for-byte content.
   - Never leave a workspace or Git repository in a half-mutated, corrupted state.

### 3. Concurrency Boundaries & Mutation Serialization
- **Strict Serialization of Mutations**: Mutating Git history (`stage`, `commit`, `tag`, `push`) or the workspace filesystem must be **strictly sequential**. Git enforces exclusive process locks (`.git/index.lock`); spawning concurrent tasks or threads during the mutation phase is strictly forbidden.
- **Bounded Parallelism for Pure Read/Compute**: Concurrency (e.g., via `rayon` or parallel iterators) is strictly limited to:
  1. Read-only filesystem discovery (traversing monorepo directories with hundreds of packages).
  2. Pure in-memory computation (AST parsing across multiple manifests in `compute()` or drift analysis in `doctor()`).

---

## 🛡️ Pillar II: Git & Filesystem Surgical Invariants

### 1. Surgical Diffs (Zero Collateral Damage)
- When updating manifests (`Cargo.toml`, `package.json`, `pyproject.toml`, `Chart.yaml`, etc.), the resulting `git diff` must **only** reflect the targeted version mutation.
- Preserving formatting, indentation (tabs vs. spaces), quote style, key order, and comments is mandatory. Re-serializing entire files through naive formatters is strictly forbidden.

### 2. Non-Destructive Git Operations
- `cutver` **never** performs destructive Git operations: no force pushes (`--force`), no history rewriting, and no blind overwrites of existing tags.
- If a target tag already exists locally or remotely, fail closed (*fail-closed*) before touching files, with an actionable resolution message.

### 3. Cross-Platform Parity & Path Normalization
- All internal paths, repository relative paths, tracked manifest paths, and Git tags must use forward slashes (`/`).
- Never leak Windows backslashes (`\`) into Git trees, changelogs, or configuration keys.
- Preserve original file line endings (LF vs CRLF).

---

## 💎 Pillar III: Type Rigor & Error Observability

### 1. Parse, Don't Validate & Mandatory Domain Newtypes
- Do not pass unstructured primitive types (`String`, `&str`, `PathBuf`) across domain boundaries.
- **Mandatory Domain Newtypes**:
  - `TagName`: Must be validated upon creation against Git ref-format invariants (`git-check-ref-format`); unvalidated strings must never represent a Git tag.
  - `TagPrefix`: Strongly typed tag prefix (e.g. `"v"` or custom namespace).
  - `CommitSha`: Strongly typed Git commit hash with hexadecimal and length validation.
  - `ManifestPath`: Workspace-relative or canonicalized repository path with strictly normalized forward slashes (`/`).
  - `PluginName`: Validated plugin identifier ensuring ASCII alphanumeric, hyphens, and underscores, strictly non-empty.
  - `Capability`: Canonical enum representing decoupled microkernel capabilities (`ManifestV1`, `LifecycleV1`, `ChangelogV1`, `VersioningV1`).
- Parse inputs at boundaries into rich domain types (`Version`, `BumpLevel`, `ManifestPath`, `TagName`, `ConventionalCommit`).
- Make invalid states unrepresentable through the type system.

### 2. Zero `unwrap()` or `expect()` in Production Code
- `unwrap()`, `expect()`, and `panic!()` are strictly forbidden inside `src/` (production code).
- The only permissible location for `unwrap()` is within unit/integration tests (`tests/` or `#[cfg(test)]`).
- All errors must be modeled with typed domain enums using `thiserror`.

### 3. Rich, Actionable Error Messages
Every error surfaced to the user must answer three questions:
1. **What failed?** (Clear failure statement).
2. **Where did it fail?** (File path, commit hash, or tag name).
3. **How to fix it?** (Actionable recommendation or next step).

### 4. Zero Swallowed Errors
- Swallowing errors with `if let Err(_) = ...` or discarding process outputs without checking status codes is forbidden.
- Every ignored condition, skipped hook, or non-critical failure must be explicitly logged or tracked in the execution `Summary` with its causal rationale.

---

## 🧪 Pillar IV: Determinism & Quality Gates

### 1. Absolute Test Isolation
- Integration tests must execute inside isolated, ephemeral temporary directories (`tempfile::tempdir()`).
- Tests must never touch the user's ambient Git configuration, global files, or host repositories.
- Time-dependent tests (changelog dates, timestamps) must accept parameterized dates or mock clocks; tests must never fail due to time zone differences or clock drift.

### 2. Configuration Backward Compatibility
- Configuration syntax in `cutver.toml` must maintain backwards compatibility.
- Deprecated syntax must remain functional with clear migration warnings emitted in `cutver doctor` and `cutver init --update`, both of which read one table of deprecated spellings (`src/config/deprecations.rs`, enforced by `tests/config_deprecations.rs`).
- Breaking schema changes require a major version bump and automated migration tooling.
- Machine surfaces — JSON output, exit codes, the plan and attestation files — carry a separate policy: **Pillar VII.2**.

### 3. Quality Gates (Fail-Closed)
- `cargo clippy --all-targets -- -D warnings`: Zero warnings tolerated.
- `cargo fmt -- --check`: Canonical formatting strictly enforced.
- Minimal dependency footprint: External crates require architectural justification. If a feature can be implemented cleanly with standard library tools or existing approved dependencies (`serde`, `minijinja`, `semver`), no new dependency may be introduced.

### 4. Cognitive Load & Work-Unit Commits
- Changes should be broken down into atomic, reviewable work-unit commits (`feat:`, `fix:`, `refactor:`, `test:`, `docs:`).
- Code, tests, and documentation must accompany the behavior within the same commit.
- Pull requests stay under 300–400 lines of effective code to protect human review focus. That is a **target**, not a gate: past it the pull request states its slice boundary and why the change is one unit, and chained pull requests are the sanctioned answer.

### 5. Cognitive Documentation & Human-Centered Design
- **Strict Cognitive Line Budgets**: All repository documentation (`README.md`, `CONTRIBUTING.md`, `docs/**/*.md`) adheres to human attention horizons (Nielsen Norman 3–5 minute reading threshold). Target 100–150 lines per document, hard ceiling $\le$ 200 lines — **enforced** by `every_document_stays_under_the_line_ceiling` and by a step in `ci.yml`, which walk the same roots. **The trigger is 150**: crossing it obliges a decision — split the page, or record in the pull request why it is one thing — because the cheapest way to satisfy a ceiling is deletion and the cheapest words to delete are the qualifiers, which are the semantics. Documents approaching either line decompose into focused submodules under `docs/`.
- **The survivor rule**: compaction moves a fact to another page; it never drops one. A page compacted to fit the ceiling is read back against its predecessor, and a statement of behaviour that cannot move is a reason to split the page rather than a sentence to delete. Phase 4A compacted `machine-contract.md` to 199 lines and silently lost four statements of behaviour; this clause exists because of that diff.
- **Documentation freshness**: shipping docs in the same commit is not enough on its own — it asks for *presence* while the failure is *freshness*. Three parts: **(1) same commit** — a new or changed surface ships its reference row and at least one worked example in the commit that changes it; **(2) freshness** — shipping a surface obliges revisiting the pages that describe the flow *around* it, because a new flag is not only a row in the reference, it changes what the first-run page recommends; **(3) examples that were run** — an example is a transcript or it is a claim. Enforcement stops where a test would start lying: flag surface parity between the clap tree and `docs/reference/**` is mechanical (`tests/cli_reference_parity.rs`), and a `cutver …` line in a `bash`-tagged fence must parse against the CLI (`tests/documentation_examples.rs`, parse only, nothing executed). Narrative pages — `design.md`, `pipeline.md`, the guides — stay a **judgement** obligation, because no test can verify that a sentence is still true, and pretending otherwise produces prose that passes and lies. A `last-verified: vX.Y.Z` stamp is vanity: easy to add, easier to fake.
- **Direct Companion Tone**: Documentation speaks directly from engineer to engineer. Lead with executable snippets, explain concisely afterwards, and highlight gotchas via visible callouts. Marketing hype (*"blazing fast"*, *"revolutionary"*) and corporate bureaucratic prose are strictly forbidden.
- **Semantic Link Hygiene**: Hyperlink anchor text must communicate conceptual destinations and value. Exposing raw file paths (`docs/...`), file extensions (`.md`), or generic phrases (*"click here"*) in visible link text is strictly forbidden.
- **Domain-First Abstraction**: Frame features around user outcomes (e.g. *"Release Templates"*, *"Manifest Synchronization"*), treating underlying implementation libraries (e.g. MiniJinja, toml_edit) as secondary technical notes.

### 6. Transversal Skill & Workflow Governance
- **Proactive Identification**: When project-specific adaptations, recurring workflow patterns, or transversal capabilities emerge with reusability potential across repositories, agents must proactively surface them as candidate skills.
- **Explicit Rationale ("The Why")**: Every skill proposal must articulate the concrete problem it solves, why it is transversal, and the value it brings.
- **Human Authorization Gate**: Agents must never author, generate, or register new skills unprompted. Implementation requires explicit human authorization.
- **Precondition of Stability**: Skill creation can only be proposed or initiated after the active feature work is 100% functional, verified, and green.

### 7. Checks Name Their Object
- **One object per check.** A rule about the content of a commit must not read the working tree; a rule about the environment a plugin runs in must not fire while the configuration is merely loaded; a rule about a repository-relative path must not resolve against the process working directory. A check that reaches past its object produces confident answers to questions nobody asked. **Judgement**: no test can prove which object a check reads, so review owns it — and a check whose object is wrong is a defect even when it is green.
- **No inherited ambient state.** Every subprocess names its working directory explicitly and never inherits the caller's. Every path declared repository-relative resolves against `root_dir` before a consumer reads it, and validation judges the declared spelling, never the resolved one. **Judgement with behavioural coverage**: the `tests/e2e_*` suites drive each boundary from an unrelated directory, which catches a regression of this shape without proving the property.
- **A declared input is not drift.** A guard that protects the tree exempts exactly the paths the invocation named, compared canonically, and stays strict for everything else: exempting an entry must never degrade into not checking. **Enforced** by `tests/clean_tree_exemptions.rs`, which asserts that `--notes-file` and `[gates] dir` are exempt and nothing else is, for the collapsed tree descriptions git's porcelain really prints.
- **Enforcement is derived, not reviewed.** A set that a rule speaks about is derived from the source and fails on the outlier, never listed by hand and trusted. **Enforced where a set is derivable**: `tests/container_build_inputs.rs` derives the image's embedded-file set from the sources, `tests/source_line_budget.rs` derives the source set by walking `src/`, and `tests/ci_workflow_contract.rs` derives the job graph from `ci.yml`. A rule whose set cannot be derived falls back to the levels in *Enforcement*, above.

---

## 🏛️ Pillar V: Structural Purity & Complexity Control

### 1. Flat Hierarchy & Maximum 2-Level Nesting
- **Maximum nesting depth**: Code blocks must never exceed two (2) levels of indentation inside any function.
- **Guard Clauses & Bouncer Pattern**: Handle edge cases, errors, and validation exits immediately at the top of functions using `let-else` (`let Some(val) = opt else { return ... };`), the `?` operator, and early returns.
- Deeply nested `if { if { match { ... } } }` structures are strictly forbidden. The happy path must always run flat along the left margin.

### 2. Elimination of Procedural Branching (Anti-`if-else`)
- **Prohibition of `if/else` ladders**: Procedural `if / else if / else` chains are forbidden across domain boundaries.
- **Exhaustive Matching**: State dispatch must use exhaustive `match` expressions.
- **No Lazy Wildcards**: Using catch-all wildcards (`_ => ...`) in `match` over domain enums (`BumpLevel`, `ManifestKind`, `Drift`) is forbidden. Every variant must be explicitly handled so that new additions trigger compile-time errors.
- **Railway-Oriented Combinators**: Error handling and optional values must be composed via standard combinators (`map`, `and_then`, `unwrap_or_else`) rather than procedural conditions.

### 3. Polymorphic Validation over Conditionals
- Business rules, manifest synchronization, and hook validations must rely on polymorphic dispatch (via Rust traits such as `ManifestEditor`, `Validator`, or typed enum dispatch), never on ad-hoc conditional branches.
- Components must validate themselves upon parsing (*Parse, Don't Validate*).

### 4. Atomic Primitives & Strict Single Responsibility
- **Function Budget**: Functions must not exceed 35–40 lines of effective code. A function does exactly one thing: compute, parse, validate, or perform boundary I/O. Mixing responsibilities is forbidden.
- **Module Budget & No Legacy `mod.rs`**: Production source files (`src/**/*.rs`) target $\le$ 300 lines with a strict hard ceiling of $\le$ 400 lines, **enforced as a ratchet** by `tests/source_line_budget.rs`. When a file approaches 350 lines, decomposition into focused submodules via the Facade pattern (`foo.rs` + `foo/`) is **mandatory as a judgement call**: whether `foo.rs` should become `foo.rs` + `foo/` depends on where its seams are, which no line count can decide, so review owns it. Using legacy `mod.rs` files inside `src/` is strictly forbidden; modern Rust 2018/2021+ sibling files (`foo.rs` alongside `foo/`) are required.
- **Thin CLI `main.rs` Budget**: $\le$ 50 lines (ideal $\le$ 20 lines). It only parses CLI arguments and delegates immediately to runner dispatch.
- **Integration Test Suite Budget**: $\le$ 500 lines per domain suite (`tests/e2e_*.rs`), sharing common test fixtures and setup helpers in `tests/common/mod.rs`.
- Functions must be pure, deterministic, and composable without hidden side-effects.

### 5. Invariants by Construction (Type-State & No Primitive Obsession)
- Critical workflows must represent valid states through the type system (Type-State pattern: `UnverifiedPlan -> ValidatedPlan -> StagedPlan`), making invalid transitions unrepresentable at compile time.
- Primitive Obsession is forbidden: Domain-significant concepts (e.g., Git tags, commit hashes, paths) must use typed *Newtypes* rather than bare `String` primitives.

### 6. Single Source of Truth & Zero Cross-Subcommand Duplication
- **Single Canonical Domain Authority**: Critical domain heuristics (such as tag prefix normalization, changelog context assembly, author resolution, and primary manifest version deduction) must reside in exactly one canonical domain module (`crate::git`, `crate::changelog::context`, `crate::config`).
- **Prohibition of Command-Level Logic Duplication**: CLI subcommands (`bump`, `doctor`, `changelog`, `open`) must never re-implement or clone parsing, filtering, or normalization heuristics. All subcommands must consume the same shared domain services.

---

## 🔌 Pillar VI: Microkernel & Plugin Isolation Boundaries

### 1. Dual-Runtime Parity & Single Protocol
- The plugin engine must enforce a unified IPC contract across both execution runtimes: sandboxed WebAssembly (WASM via Extism/Wasmtime) and native process executables/scripts.
- Communication across the host-guest boundary must rely exclusively on referentially transparent, schema-versioned JSON DTOs (`src/plugin/dto/`). Plugins must never access shared ambient memory or process pointers.

### 2. Zero Ambient Authority (Explicit Capability Grants)
- Plugins run with zero default authority.
- Sandboxed WASM plugins must never access the host filesystem, network, or environment variables unless explicitly declared under `[plugins.<name>.permissions]` in `cutver.toml`.
- Native process plugins must execute within strict timeout bounds (`timeout_seconds`), failing closed if execution hangs.

### 3. Fail-Closed Lifecycle Guarantees
- **In-Memory Pre-Flight (`on_pre_bump`)**: Must execute strictly during the in-memory compute phase, prior to initiating any filesystem I/O or opening a `MutationTransaction`. Any plugin error or timeout aborts the pipeline immediately, leaving the workspace in an untouched state.
- **Transactional Rollback (`on_post_bump`)**: Must execute within the `MutationTransaction` boundary before the Git commit. If a post-mutation plugin fails, the RAII transaction drop guard must restore all modified manifests and unstaged Git files to their exact pre-run bytes.

### 4. Zero Footprint Regressions
- The WebAssembly runtime engine (Extism) must remain gated behind the optional Cargo feature flag `plugins` (`cargo build --features plugins`).
- **Minimal CLI installations without plugins must reach no WebAssembly runtime at all**: without `--features plugins`, the resolved dependency graph contains neither `extism` nor `wasmtime`. That shape invariant replaced the `~4 MB` binary-size budget, which nothing ever executed — a budget with no threshold, no tool and no check, while the documentation quoted three different numbers. It is **enforced** in `tests/lean_dependency_graph.rs` (the feature wiring) and by a `cargo tree` step in `scripts/check.sh` and in `ci.yml` (the graph itself).
- **Size is reported, never enforced.** Binary size is a channel cost — container layers, download — and startup latency is the user cost; both are measured per release and published, and neither is a merge gate. The number is gone; the measurement is not.

---

## 🧭 Pillar VII: Scope & Evolution

### 1. What `cutver` May Become
The pillars above govern how well something is built; this one governs whether it belongs. Growth is welcome — the tool earns its size — but scope is a decision, not an accretion, so a feature can be cleanly implemented and still be out of scope.

- **GitHub is a channel, never an identity.** Every integration with a code host degrades to stdio and exit codes: `cutver` works in a repository with no remote, no token and no network, and a CI integration is a consumer of the machine contract rather than a dependency of the core.
- **The core grows no HTTP client and no provider SDK.** Network access exists only behind the `plugins` feature, for the plugin registry and its install path, and never as a general-purpose capability of the core.
- **Plugins extend the four typed slots and nothing else**: `ManifestV1`, `LifecycleV1`, `ChangelogV1`, `VersioningV1`. A capability outside them is a core feature proposal, not a plugin.

Crossing this boundary is a **judgement** call for the maintainer, stated in the pull request and recorded in the changelog, never an implementation detail discovered in review.

### 2. Surface Compatibility & Deprecation
`schemaVersion` implies a policy that had never been written down. Every machine surface — `bump`, `publish`, `attest`, `init`, `plan`, `doctor`, `changelog`, `plugin` JSON, exit codes, the plan file and the attestation files — follows it:

- **Additive within a schema version.** New fields may appear; existing fields keep their name, type, meaning and position, because field order is part of the contract.
- **A break is a major bump with a deprecation cycle**, not a release: the old surface keeps working and warns, and the removal happens in a later release than the rename. The channel is live (#272): one table of deprecated spellings in `src/config/deprecations.rs`, a `warning: …` line on stderr always, the same sentence as a `::warning::` annotation under CI, and the sentence as data in `doctor --json`.
- **A surface is never removed in the release that renames it.** Renames take two releases at minimum.
- Configuration compatibility is a separate promise, stated for `cutver.toml` in **Pillar IV.2**.

---

## What the 1.0 Freeze Covers

`1.0` is a compatibility milestone: the release that turns every documented surface into a promise. What it freezes is
what a consumer parses and depends on — the machine surfaces, the configuration promise, the release transaction and
the plugin ABI. The judgement obligations above are not that, so they are classified here instead of being argued at
the release. A condition that a person can satisfy by declaring it satisfied is not a condition.

| Judgement obligation | Where it lives | In the freeze | Why |
| :--- | :--- | :--- | :--- |
| Whether `foo.rs` becomes `foo.rs` + `foo/` | `350-line facade trigger`; `Module Budget & No Legacy mod.rs` | **Outside** | Internal code organization: the seam is a design choice no consumer can observe, and the machine-decidable half (the enforced $\le$ 400 ceiling) is already enforced. |
| Whether a narrative page still tells the truth | `Documentation freshness` | **Outside** | No machine can read whether a sentence is still true. Presence and parseability are already enforced by `cli_reference_parity` and `documentation_examples`, which is as far as a test can honestly go. |
| Which object a check reads | `One object per check` | **Outside** | Review owns it, and it stays a defect whatever the version number: a green check answering a question nobody asked is not a promise a consumer can hold this release to. |
| The ambient state a subprocess inherits | `No inherited ambient state` | **Outside** | The `tests/e2e_*` suites catch a regression of this shape without proving the property, so nothing about it can be frozen as a surface a consumer parses. |
| Crossing the Pillar VII scope boundary | `What cutver May Become` | **Outside** | A product decision, stated in the pull request and recorded in the changelog. Growth is welcome and scope is decided; a version number does not declare it. |

Two rules keep this list honest, and `tests/freeze_scope_list.rs` enforces both:

- **Every judgement obligation is classified.** The pin derives each one from this document — the `Judgement` row of the
  limits table, and every judgement-worded clause inside a pillar — and fails when one is not named by a row above,
  reporting the clause and its line.
- **No invented classification.** A row naming an obligation this document does not carry fails too, so the list cannot
  drift into decoration as the contract changes underneath it.
