# Contributing to cutver

One page, read before your first pull request. Every rule here is enforced by a file in this repository;
where a rule is judgement instead, the sentence says so.

## Setup

Rust, edition 2024, `rust-version` as declared in `Cargo.toml`. Clone, then:

```bash
cargo build --locked --workspace
sh scripts/check.sh
```

Then read the [architectural and quality contract](CONTRACT.md). It is the list a review will hold your diff
against: pure domain logic, atomic file writes, no `unwrap()` or `expect()` outside tests, no `if/else`
ladders, module and function budgets, one canonical home per heuristic.

## The gate

`sh scripts/check.sh` is one command and one verdict. It runs from any directory, writes only inside a
private temporary directory it removes, and exits `0` only when every step it could run passed.

| Step | What it runs |
| :--- | :--- |
| toolchain | `rustc -V`, `cargo -V` |
| build | `cargo build --locked --workspace --all-targets` |
| test | `cargo test --locked --workspace` |
| test (plugins) | `cargo test --locked --workspace --features plugins` |
| clippy ×2 | `cargo clippy --locked --workspace --all-targets -- -D warnings`, then the same `--features plugins` |
| clippy (linux) ×2 | the same two again with `--target x86_64-unknown-linux-gnu` |
| format | `cargo fmt --all --check` |

Steps run in the workflow's order and the gate stops at the first failure; the steps after it are reported
as *not reached*, never as passed. The test steps run with `GITHUB_ACTIONS`, `GITHUB_OUTPUT` and
`GITHUB_STEP_SUMMARY` set, because a suite that only passes without them passes a lie.

The summary has three buckets — passed, failed, **not run here** — and a skipped step is not a pass. On a
non-Linux host without the `x86_64-unknown-linux-gnu` target, or without the C cross-compiler `zstd-sys`
needs, the two linux clippy steps land in that third bucket with their reason. The linux target is the one
that type-checks what a `#[cfg(unix)]` file hides on Windows, so a change that touches platform-gated code
is not verified until it runs somewhere it can.

The gate does not replace CI entirely: CI also runs `cargo publish --dry-run --locked`, the runners for the
operating systems this host is not, and the same suites inside a hosted runner's environment. Run the publish
dry-run yourself after touching `Cargo.toml` or what the crate ships; the platform legs are what CI adds.

## The three budgets

1. **200 lines per document** — enforced. `every_document_stays_under_the_line_ceiling` in
   `tests/ci_workflow_contract.rs` walks `README.md`, `CONTRIBUTING.md` and `docs/**/*.md`, and `ci.yml` runs
   the same loop as a step. Target 100–150; decompose into a focused page before you reach 200. `CONTRACT.md`
   is the one named exemption, and its reason sits next to it in the test rather than in an omission from a
   glob. Crossing 150 obliges a decision: split the page, or say in the pull request why it is one thing —
   the cheapest way to satisfy a ceiling is to delete the qualifiers, and the qualifiers are the semantics.
2. **400 lines per source file** — enforced as a ratchet by `tests/source_line_budget.rs`. Sixteen files
   predate the ceiling and are allowlisted with the count they had: an entry may shrink but never grow, and a
   file that now fits inside 400 fails until its entry is deleted. Adding an entry is never the fix.
3. **350 lines** — the facade trigger, a judgement call. Whether `foo.rs` should become `foo.rs` + `foo/`
   depends on where its seams are, so no test decides it; a reviewer will.

## Documentation examples

Every `cutver …` line inside a `bash`-tagged fence is a transcript: `tests/documentation_examples.rs` parses it
against the real CLI and fails if a reader could not paste it. Redirection, pipes and a trailing `# comment`
are stripped first, and nothing is executed.

A syntax diagram — `cutver plugin add <NAME> [--registry <BASE>]` — is not a transcript, because the reader
has to fill it in. Those lines belong in a `text`-tagged fence, where they are not parsed. The tag is the
claim: `text` is checked to contain a metavariable, so it cannot quietly become the place real examples go to
avoid the parser.

## Writing

Prose here is read by people who did not write the code, so it is written for them:

- **Name the actor and the action before the mechanism.** "The plugin drivers now need the project folder"
  says more than "gain a workspace-root parameter".
- **Explain a term once, where it appears**, in the reader's words. An attestation is "a recorded
  authorization"; the moving major tag is explained by what it does to `uses: cutver/setup@v0`, not by its
  name.
- **One idea per sentence.** A semicolon holding two claims is a sentence to split.
- **Numbers instead of adjectives**: `1.3 MB against 5.0 MB`, never "much smaller"; "fifteen seconds",
  never "fast".
- **Translate the concept, never the identifier.** `latest`, `--channel` and command names stay verbatim in
  any language, because a reader who translates them cannot find them in the guide.
- **Clarity is not softening.** The limit, the residual and the case that cannot be recovered are stated as
  plainly as the gain. A rewrite that drops a caveat is a regression.

The same register covers release notes, pull-request bodies and anything copied out of the repository into a
chat: a summary first, and no markdown once it leaves these files.

## Commits

Conventional Commits, `type(scope): subject`, with the types the pull-request labels also use: `feat`, `fix`,
`docs`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `style`, `revert`.

The reason is mechanical: `cutver bump auto` parses those commits. A `feat` deduces a minor bump, a `fix` or
`perf` or `refactor` a patch, and `!` or a `BREAKING CHANGE:` footer a major. The same parse groups the
changelog entry. A commit that lies about its type ships a version number that lies with it — the release
commit itself follows the same grammar, `chore(release): v<version> [skip ci]`, from `[git] commit_message`.

Keep code, tests and documentation in the same work-unit commit, and keep the pull request under roughly
300–400 lines of effective diff. Past that a reviewer reads the shape and skips the detail.

A release's notes may live in the tree at `.github/releases/<tag>.md` — `v0.20.0.md` — in English, committed
before the dispatch, so they are reviewed in the pull request that produced the release. `release.yml` uses that
file when it exists and renders the template otherwise: optional, never a step a release cannot start without.

## The pull request

Pull requests open with [the review checklist](.github/PULL_REQUEST_TEMPLATE.md) already in the
body: five boxes, ticked only with evidence, in the order a reviewer reads them. A vulnerability is not
a pull request: it goes through [the private advisory form](https://github.com/cutver/cutver/security/advisories/new),
the only channel [the security page](SECURITY.md) names.

Two checks are required on `main`: **`CI gate`** and **`Exactly one type label`**. The ruleset that requires
them lives outside this repository, which is why the names are pinned by exact string in
`tests/ci_workflow_contract.rs` — rename one side alone and every pull request waits forever on a check that
never reports.

`CI gate` collapses three legs (`test`, `test-plugins`, `publish-dry-run`) into one status, so a new job in
`ci.yml` must be added to the gate's `needs` and to that test's `LEGS`, or its failures block nothing.

**The label trap:** the check requires *exactly one* `type:*` label, and it answers `opened` as well as
`labeled`. On `opened` the labels are read from the event payload, and on a brand-new pull request there are
none yet — `gh pr create --label type:fix` creates the pull request first and applies the label second, so it
is not an escape hatch. Every pull request therefore shows **two** runs of this check: a red `opened` and a
green `labeled`. That is expected. The latest run is the one that counts, so the pull request still reports
clean, and the `opened` answer exists so a required check never sits in *Expected* on a fresh pull request.

What is not expected is a `labeled` run that is still red: that means zero `type:*` labels or two of them.

```bash
gh pr create --label type:fix --title "fix(git): unstage the index on error" --body "…"
```

Label it, then read the green run and not the red one. If it fails again after the label, the message says
which count it objects to.

## Tests

A behaviour change starts with a test that fails for the intended reason, and you watch it fail before you
write the implementation. A test that has never failed has not been shown to test anything.

- Integration tests own an isolated directory (`tests/common` builds one with `tempfile::tempdir()`), never
  the ambient repository, the user's `~/.gitconfig`, or a real network. Time-dependent output takes a date or
  a clock, never the wall clock.
- The plugins feature is a separate compilation. `tests/wasm_plugin.rs` opens with
  `#![cfg(feature = "plugins")]`, so `cargo test` alone never compiles the WebAssembly path: run both
  `cargo test --locked --workspace` and the same with `--features plugins`, which is what the gate does.
- Contracts that live in YAML or in a line count are tested as code, not trusted to a comment:
  `tests/ci_workflow_contract.rs`, `tests/container_workflow_contract.rs`, `tests/source_line_budget.rs`,
  `tests/machine_error_contract.rs`. When a workflow or an output format changes, the test changes in the same
  commit — and if a test only fails in CI, move the check into the suite or into the gate so the next person
  sees it in seconds.

Design intent behind the gate and the ratchet: [The Local Gate & Source Line Budget](docs/internals/local-gate.md).
