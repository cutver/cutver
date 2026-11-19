//! The lean build's dependency *shape* — what replaced `CONTRACT.md`'s ~4 MB binary budget.
//!
//! The budget was dropped on 2026-11-19 ("the tool is growing and the growth is worth it"), and the
//! distinction that survived is: **the number goes, the measurement does not.** A size target nobody
//! enforced is what let the 400-line ceiling reach sixteen violators, so the replacement had to be a
//! claim a machine can check. It is: without `--features plugins`, the resolved dependency graph
//! reaches no WebAssembly runtime. Size is then *reported* per release, never enforced
//! (`odd/tasks/contract-reevaluation.md` S1).
//!
//! The invariant has two halves with different homes, and this file is only one of them:
//!
//! - **The graph half** needs a `cargo tree` invocation that must not run nested inside `cargo test`,
//!   so it lives in `scripts/check.sh` and in `.github/workflows/ci.yml`.
//! - **The wiring half** is arithmetic over one file — which feature enables which optional
//!   dependency — and lives here, where it runs with no toolchain and no network.
//!
//! The last test in this file binds the three copies to each other, the same way
//! `tests/ci_workflow_contract.rs` binds the documentation loop: a declaration, its check, and the
//! place the check runs are one decision written three times, and drift between them is silent.
//!
//! Covers S1 and S10.

use std::path::PathBuf;

const MANIFEST: &str = "Cargo.toml";
const GATE: &str = "scripts/check.sh";
const CI: &str = ".github/workflows/ci.yml";
/// The crates the lean graph may never reach. `extism` is the direct dependency; `wasmtime` is what it
/// drags in, and naming both means a future direct `wasmtime` dependency is caught by the same rule.
const FORBIDDEN: [&str; 2] = ["extism", "wasmtime"];
/// Of those two, only `extism` appears in `[dependencies]`; `wasmtime` is transitive, so this file can
/// check its absence from the graph but nothing here can declare it. That asymmetry is why the graph
/// half of the invariant needs a real `cargo tree` and not a manifest reading.
const DECLARED: [&str; 1] = ["extism"];

fn repo_text(relative: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative);
    let raw = std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
    raw.replace("\r\n", "\n")
}

/// The body of one TOML table: strictly between its header and the next one. Anchored on the header
/// line so a same-named key in another table cannot stand in for it.
fn table<'a>(text: &'a str, header: &str) -> Vec<&'a str> {
    let marker = format!("[{header}]");
    let mut body = Vec::new();
    let mut inside = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            inside = trimmed == marker;
            continue;
        }
        if inside {
            body.push(trimmed);
        }
    }
    assert!(!body.is_empty(), "Cargo.toml must still declare a [{header}] table");
    body
}

#[test]
fn no_feature_but_plugins_reaches_a_wasm_runtime() {
    let manifest = repo_text(MANIFEST);
    let features = table(&manifest, "features");
    assert!(
        features.contains(&"default = []"),
        "the default feature set must stay empty so a plain `cargo build` is the lean build; it reads {features:?}"
    );

    for line in &features {
        let Some((name, enabled)) = line.split_once('=') else {
            continue;
        };
        if name.trim() == "plugins" {
            continue;
        }
        for forbidden in FORBIDDEN {
            assert!(
                !enabled.contains(forbidden),
                "feature `{}` reaches {forbidden}; only `plugins` may enable a WebAssembly runtime, \
                 because the lean build is what the shape invariant protects",
                name.trim()
            );
        }
    }

    let plugins = features
        .iter()
        .find_map(|line| line.strip_prefix("plugins ="))
        .expect("the `plugins` feature is what gates the WebAssembly runtime; it must exist");
    assert!(
        plugins.contains("dep:extism"),
        "the `plugins` feature must keep enabling `dep:extism`, or the gated path has no gate; it reads {plugins:?}"
    );
}

#[test]
fn every_runtime_this_manifest_declares_is_optional() {
    let manifest = repo_text(MANIFEST);
    let dependencies = table(&manifest, "dependencies");
    for declared in DECLARED {
        let declaration = dependencies
            .iter()
            .find(|line| line.starts_with(declared))
            .unwrap_or_else(|| panic!("{declared} must be declared in [dependencies], not inherited silently"));
        assert!(
            declaration.contains("optional = true"),
            "{declared} is declared `{declaration}`; without `optional = true` every build links it and \
             the shape invariant is already broken"
        );
    }
}

/// The declaration and its check are only worth what the place that runs it is worth. This repository
/// has already shipped a rule that nothing executed (`permissions.env`, #300), so the check that binds
/// the other two copies gets its own assertion instead of a comment.
#[test]
fn the_graph_check_runs_in_the_gate_and_in_ci() {
    let gate = repo_text(GATE);
    assert!(
        gate.contains("dependency-shape"),
        "scripts/check.sh must run the graph check as a named step; a gate that mirrors CI without it \
         leaves the shape invariant to a pipeline the contributor cannot run"
    );

    let ci = repo_text(CI);
    let graph_step = ci
        .lines()
        .map(str::trim)
        .find(|line| line.contains("cargo tree") && line.contains("--no-default-features"))
        .expect("ci.yml must run `cargo tree --no-default-features`, or the check has no required gate");
    for forbidden in FORBIDDEN {
        assert!(
            graph_step.contains(forbidden) || ci.contains(forbidden),
            "ci.yml's graph step must name {forbidden} among the crates it forbids; it reads {graph_step:?}"
        );
    }
}
