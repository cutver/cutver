# Cutver Architectural & Quality Contract

> **Status**: Active / Canonical Law  
> **Scope**: Applies to all contributions, refactors, features, and automated agents in the `cutver` ecosystem.

---

## Vision & Objective

`cutver` is a mission-critical release orchestration engine. It touches canonical Git histories, mutates package manifests across diverse ecosystems, generates changelogs, tags releases, and triggers production deployment hooks.

Because developers and automated CI/CD pipelines entrust `cutver` with their canonical versioning and release workflows, software quality in this project is not an afterthought—it is a non-negotiable prerequisite.

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

### 2. Backward Compatibility & Responsible Deprecation
- Configuration syntax in `cutver.toml` must maintain backwards compatibility.
- Deprecated syntax must remain functional with clear migration warnings emitted in `cutver doctor` and `cutver init --update`.
- Breaking schema changes require a major version bump and automated migration tooling.

### 3. Quality Gates (Fail-Closed)
- `cargo clippy --all-targets -- -D warnings`: Zero warnings tolerated.
- `cargo fmt -- --check`: Canonical formatting strictly enforced.
- Minimal dependency footprint: External crates require architectural justification. If a feature can be implemented cleanly with standard library tools or existing approved dependencies (`serde`, `minijinja`, `semver`), no new dependency may be introduced.

### 4. Cognitive Load & Work-Unit Commits
- Changes should be broken down into atomic, reviewable work-unit commits (`feat:`, `fix:`, `refactor:`, `test:`, `docs:`).
- Code, tests, and documentation must accompany the behavior within the same commit.
- Pull requests must strive to remain under 300–400 lines of effective code to protect human review focus.

### 5. Cognitive Documentation & Human-Centered Design
- **Strict Cognitive Line Budgets**: All repository documentation files (`README.md`, `docs/**/*.md`) must adhere to human attention horizons (Nielsen Norman 3–5 minute reading threshold). Target: 100–150 lines per document; strict hard ceiling $\le$ 200 lines. Documents approaching limits must decompose into focused submodules under `docs/`.
- **Direct Companion Tone**: Documentation speaks directly from engineer to engineer. Lead with executable snippets, explain concisely afterwards, and highlight gotchas via visible callouts. Marketing hype (*"blazing fast"*, *"revolutionary"*) and corporate bureaucratic prose are strictly forbidden.
- **Semantic Link Hygiene**: Hyperlink anchor text must communicate conceptual destinations and value. Exposing raw file paths (`docs/...`), file extensions (`.md`), or generic phrases (*"click here"*) in visible link text is strictly forbidden.
- **Domain-First Abstraction**: Frame features around user outcomes (e.g. *"Release Templates"*, *"Manifest Synchronization"*), treating underlying implementation libraries (e.g. MiniJinja, toml_edit) as secondary technical notes.

### 6. Transversal Skill & Workflow Governance
- **Proactive Identification**: When project-specific adaptations, recurring workflow patterns, or transversal capabilities emerge with reusability potential across repositories, agents must proactively surface them as candidate skills.
- **Explicit Rationale ("The Why")**: Every skill proposal must articulate the concrete problem it solves, why it is transversal, and the value it brings.
- **Human Authorization Gate**: Agents must never author, generate, or register new skills unprompted. Implementation requires explicit human authorization.
- **Precondition of Stability**: Skill creation can only be proposed or initiated after the active feature work is 100% functional, verified, and green.

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
- **Module Budget**: Production source files (`src/**/*.rs`) target $\le$ 300 lines with a strict hard ceiling of $\le$ 400 lines. When a file approaches 350 lines, decomposition into focused submodules via the Facade pattern (`foo.rs` + `foo/`) is mandatory.
- **Thin CLI `main.rs` Budget**: $\le$ 50 lines (ideal $\le$ 20 lines). It only parses CLI arguments and delegates immediately to runner dispatch.
- **Integration Test Suite Budget**: $\le$ 500 lines per domain suite (`tests/e2e_*.rs`), sharing common test fixtures and setup helpers in `tests/common/mod.rs`.
- Functions must be pure, deterministic, and composable without hidden side-effects.

### 5. Invariants by Construction (Type-State & No Primitive Obsession)
- Critical workflows must represent valid states through the type system (Type-State pattern: `UnverifiedPlan -> ValidatedPlan -> StagedPlan`), making invalid transitions unrepresentable at compile time.
- Primitive Obsession is forbidden: Domain-significant concepts (e.g., Git tags, commit hashes, paths) must use typed *Newtypes* rather than bare `String` primitives.

### 6. Single Source of Truth & Zero Cross-Subcommand Duplication
- **Single Canonical Domain Authority**: Critical domain heuristics (such as tag prefix normalization, changelog context assembly, author resolution, and primary manifest version deduction) must reside in exactly one canonical domain module (`crate::git`, `crate::changelog::context`, `crate::config`).
- **Prohibition of Command-Level Logic Duplication**: CLI subcommands (`bump`, `doctor`, `changelog`, `open`) must never re-implement or clone parsing, filtering, or normalization heuristics. All subcommands must consume the same shared domain services.

