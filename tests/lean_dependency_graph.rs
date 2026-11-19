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
//! The measurement has a home of its own: the release workflow reports every artifact's size in its
//! run summary and gates nothing on it. That half is asserted here too, because "the measurement does
//! not go" is only true while something still measures.
//!
//! Covers S1 and S10.

use std::path::PathBuf;

const MANIFEST: &str = "Cargo.toml";
const GATE: &str = "scripts/check.sh";
const CI: &str = ".github/workflows/ci.yml";
const RELEASE: &str = ".github/workflows/release.yml";
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

/// The steps of a workflow, split the way YAML splits them: a step opens at a `- ` item indented six
/// spaces (a job's `steps:` list) and runs to the next one. A bare line scan cannot tell a step from
/// a comment or from text in the next step, which is how a workflow assertion silently stops testing
/// anything.
fn step_blocks(workflow: &str) -> Vec<Vec<&str>> {
    let mut blocks: Vec<Vec<&str>> = Vec::new();
    for line in workflow.lines() {
        let indent = line.len() - line.trim_start().len();
        if indent == 6 && line.trim_start().starts_with("- ") {
            blocks.push(vec![line]);
        } else if let Some(open) = blocks.last_mut() {
            open.push(line);
        }
    }
    blocks
}

/// Pillar VI.4 ends with "the number is gone; the measurement is not", and this is the measurement's
/// only binding: the workflow that publishes the binaries writes each artifact's size into the run
/// summary and derives nothing else from it. Two claims, both required. If the step disappears, size
/// is neither enforced nor reported — the declaration-without-its-check that S1 removed. If a
/// threshold comes back, the clause says the opposite of what the pipeline does.
#[test]
fn the_release_reports_artifact_size_and_never_gates_on_it() {
    let release = repo_text(RELEASE);
    let blocks = step_blocks(&release);

    let reporters: Vec<&Vec<&str>> = blocks
        .iter()
        .filter(|block| block.iter().any(|line| line.contains("GITHUB_STEP_SUMMARY")))
        .collect();
    assert!(
        !reporters.is_empty(),
        "{RELEASE} must write the artifacts' sizes into $GITHUB_STEP_SUMMARY: the ~4 MB budget was \
         dropped for being a number nothing measured, so dropping the measurement too would keep \
         neither the number nor the evidence that replaced it"
    );

    // Both build jobs report, and each table says which build it came from: the defect being fixed
    // was one number standing in for two builds and three targets.
    let split = release
        .find("\n  build-plugins:")
        .expect("the release workflow must keep a lean `build` job and a `build-plugins` job");
    let (lean_job, plugins_job) = release.split_at(split);
    for (job, body, label) in [
        ("build", lean_job, "Lean build"),
        ("build-plugins", plugins_job, "Plugin-enabled build"),
    ] {
        let reporter = blocks
            .iter()
            .find(|block| body.contains(block[0]) && block.iter().any(|line| line.contains("GITHUB_STEP_SUMMARY")))
            .unwrap_or_else(|| {
                panic!(
                    "{RELEASE} must report sizes in the `{job}` job; no step there writes \
                     $GITHUB_STEP_SUMMARY"
                )
            });
        assert!(
            reporter.iter().any(|line| line.contains("wc -c")),
            "the `{job}` size step must measure with `wc -c` (`stat` and `ls -l` disagree across the \
             three runners); it reads {reporter:?}"
        );
        assert!(
            reporter.iter().any(|line| line.trim() == "shell: bash"),
            "the `{job}` size step must set `shell: bash`: the Windows runner defaults to `pwsh`, where \
             `wc` does not exist, and the step must still report rather than fail"
        );
        assert!(
            reporter.iter().any(|line| line.contains(label)),
            "the `{job}` size table must name the build (`{label}`): one figure covering both builds \
             is the confusion the docs carried; it reads {reporter:?}"
        );
        // Report only. A comparison here, or an exit status derived from the number, is the merge
        // gate the clause removed.
        for line in reporter {
            for gate in ["-gt", "-ge", "-lt", "-le", "exit ", "threshold"] {
                assert!(
                    !line.contains(gate),
                    "the `{job}` size step must report and nothing else, but it reads {line:?}: a \
                     threshold is the merge gate Pillar VI.4 removed, so size is never a pass/fail"
                );
            }
        }
    }

    // Nothing anywhere in the release path may turn a measured size into a pass/fail either. Comments
    // are skipped: a commented-out gate enforces nothing, and the rule being written down next to its
    // check is what keeps the next reader from adding one back.
    for (number, line) in release.lines().enumerate() {
        if line.trim_start().starts_with('#') {
            continue;
        }
        let lowered = line.to_lowercase();
        for marker in [
            "max_size",
            "max-size",
            "size_limit",
            "size-limit",
            "budget",
            "threshold",
        ] {
            assert!(
                !lowered.contains(marker),
                "{RELEASE}:{} reads {line:?}, which reintroduces a size limit. Size is measured and \
                 published per release, never enforced",
                number + 1
            );
        }
    }
}

/// The body of one job: from its two-space `id:` line to the next job key at the same indent. The
/// previous test slices the workflow by text and can be fooled by a step that moved into a job that
/// never fills `dist/`; this walks the job boundaries the file actually declares.
fn job_body<'a>(workflow: &'a str, id: &str) -> Vec<&'a str> {
    let header = format!("  {id}:");
    let mut body = Vec::new();
    let mut inside = false;
    for line in workflow.lines() {
        if line == header {
            inside = true;
            body.push(line);
            continue;
        }
        if !inside {
            continue;
        }
        let is_job_key =
            line.len() > 2 && line.starts_with("  ") && !line.starts_with("   ") && line.trim_end().ends_with(':');
        if is_job_key {
            break;
        }
        body.push(line);
    }
    body
}

/// S1: every job that hands `dist/*` to `upload-artifact` reports its size, after the step that fills
/// `dist/` and before the upload, measuring `dist/` itself. A report placed earlier writes an empty
/// table and still exits 0, which is a heading, not a measurement — and a heading is the shape of the
/// defect that let a size claim outlive the version it was measured on.
#[test]
fn every_asset_uploading_job_reports_after_packaging() {
    let release = repo_text(RELEASE);
    let ids: Vec<String> = release
        .lines()
        .filter(|line| line.len() > 2 && line.starts_with("  ") && !line.starts_with("   "))
        .filter_map(|line| line.trim_end().strip_suffix(':').map(|id| id.trim().to_string()))
        .collect();
    let mut checked = 0;
    for id in &ids {
        let body = job_body(&release, id);
        if !body.iter().any(|line| line.trim() == "path: dist/*") {
            continue;
        }
        checked += 1;
        let report = body
            .iter()
            .position(|line| !line.trim_start().starts_with('#') && line.contains("GITHUB_STEP_SUMMARY"))
            .unwrap_or_else(|| panic!("job `{id}` uploads `dist/*` but no step writes $GITHUB_STEP_SUMMARY"));
        let package = body
            .iter()
            .rposition(|line| line.contains("- name: Package ("))
            .expect("a packaging step that fills dist/");
        let upload = body
            .iter()
            .position(|line| line.contains("- name: Upload artifact"))
            .expect("an upload step");
        assert!(
            package < report && report < upload,
            "job `{id}`: the report is at {report} and must sit between Package ({package}) and Upload ({upload});              earlier than the packaging it reports an empty table and exits 0"
        );
        let step = &body[report..upload];
        assert!(
            step.iter().any(|line| line.contains(">> \"$GITHUB_STEP_SUMMARY\"")),
            "job `{id}`: the summary write must not be commented out; it reads {step:?}"
        );
        assert!(
            step.iter()
                .any(|line| line.trim_start() == "for artifact in dist/*; do"),
            "job `{id}`: the report must measure the archives the job uploads, in `dist/`; it reads {step:?}"
        );
    }
    assert_eq!(checked, 2, "both build jobs upload `dist/*`; found {checked}");
}

/// S1: a gate is not a word. `[ "$bytes" \> 4000000 ]` is a threshold carrying none of the names the
/// reporter-block scan looks for, so no line that mentions a measured size may compare it.
#[test]
fn no_comparison_touches_a_measured_size() {
    let release = repo_text(RELEASE);
    for (number, line) in release.lines().enumerate() {
        let code = line.trim_start();
        if code.starts_with('#') {
            continue;
        }
        let lowered = code.to_lowercase();
        if !(lowered.contains("bytes") || lowered.contains("size")) {
            continue;
        }
        for operator in ["-gt", "-ge", "-lt", "-le", r"\>", r"\<"] {
            assert!(
                !code.contains(operator),
                "{RELEASE}:{} compares a measured size ({operator}): {code:?}. Size is reported, never gated",
                number + 1
            );
        }
    }
}
