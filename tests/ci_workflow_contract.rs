//! Contract tests binding the required status checks on `main` to the workflows that produce them.
//!
//! A repository ruleset cannot be exercised from a test, but the two properties that make it work
//! are text-visible: the umbrella job that collapses the matrix into one stable check name, and the
//! `opened` trigger that stops the label check from sitting in "Expected" on a fresh pull request.
//! Each assertion names the merge failure it prevents.
//!
//! Guards #257 (risk item 5: delivery/CI).

use std::path::PathBuf;

const CI: &str = ".github/workflows/ci.yml";
const LABELS: &str = ".github/workflows/pr-labels.yml";

fn repo_text(relative: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative);
    let raw = std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
    // Same reason as release_pipeline_contract.rs: line endings are a checkout setting.
    raw.replace("\r\n", "\n")
}

/// The body of one job: from its two-space-indented key to the next line at that indentation.
///
/// Slicing to the end of the file instead would let any later job satisfy an assertion meant for
/// this one, which is the one way a text-based contract test silently stops testing anything.
fn job_block<'a>(workflow: &'a str, id: &str) -> &'a str {
    let marker = format!("\n  {id}:\n");
    let start = workflow
        .find(&marker)
        .unwrap_or_else(|| panic!("the workflow must declare the `{id}` job"));
    let body = &workflow[start + marker.len()..];
    let end = body
        .match_indices("\n  ")
        .find(|(offset, _)| !body[offset + 1..].starts_with("   "))
        .map_or(body.len(), |(offset, _)| offset);
    &body[..end]
}

/// The `main` ruleset requires exactly one check name. Requiring the eight per-matrix names instead
/// binds branch protection to the matrix: delete a leg and its check stays expected forever, which
/// blocks every merge; add one and it is silently unrequired. Without this job the ruleset would
/// either deadlock or protect nothing.
#[test]
fn ci_declares_one_umbrella_gate_over_the_matrix() {
    let workflow = repo_text(CI);
    assert!(
        workflow.contains("\n  pull_request:\n"),
        "ci.yml must run on pull_request: a required status check only exists where it runs"
    );

    let gate = job_block(&workflow, "gate");
    assert!(
        gate.contains("name: CI gate"),
        "the gate job must be named `CI gate`: that literal string is the required check context"
    );
    assert!(
        gate.contains("needs: [test, test-plugins, publish-dry-run]"),
        "the gate must need every leg whose result it collapses"
    );
    for leg in ["test", "test-plugins", "publish-dry-run"] {
        assert!(
            workflow.contains(&format!("\n  {leg}:\n")),
            "the gate needs `{leg}`, which must be a real job id"
        );
    }
}

/// Without `if: always()` a failing need skips the gate; a skipped job reports no conclusion, so the
/// ruleset keeps waiting instead of failing, and the pull request is blocked by a check that is
/// neither green nor red. The comparison must be `= success`, never `!= failure`: a skipped or
/// cancelled need also reports non-success and would pass the looser test.
#[test]
fn ci_gate_fails_when_any_leg_did_not_succeed() {
    let workflow = repo_text(CI);
    let gate = job_block(&workflow, "gate");

    assert!(
        gate.contains("if: always()"),
        "the gate must run unconditionally, or a failing leg turns it into a silent skip"
    );
    for result in ["TEST", "TEST_PLUGINS", "PUBLISH_DRY_RUN"] {
        let assertion = format!("test \"${result}\" = success");
        assert!(
            gate.contains(&assertion),
            "the gate must assert `{assertion}` so a skipped or cancelled leg also fails it"
        );
    }
    assert!(
        !gate.contains("!= failure"),
        "a `!= failure` comparison passes a skipped or cancelled need"
    );
}

/// A required check that never reports leaves a pull request stuck in "Expected" with nothing red
/// to explain it: unmergeable and undiagnosable. The label check must therefore speak on `opened`,
/// which is also the event that carries labels applied in the same `gh pr create`.
#[test]
fn the_label_check_reports_on_a_fresh_pull_request() {
    let workflow = repo_text(LABELS);
    let line = workflow
        .lines()
        .find(|line| line.trim_start().starts_with("types:"))
        .unwrap_or_else(|| panic!("pr-labels.yml must declare the pull_request types it answers"));
    let list = line
        .split_once('[')
        .and_then(|(_, rest)| rest.split_once(']'))
        .map(|(inner, _)| inner)
        .unwrap_or_else(|| panic!("pr-labels.yml types must be an inline list, found: {line}"));

    let events: Vec<&str> = list.split(',').map(str::trim).collect();
    for required in ["opened", "labeled", "unlabeled", "synchronize", "reopened"] {
        assert!(
            events.contains(&required),
            "pr-labels.yml must answer `{required}`; a required check that never reports blocks the merge"
        );
    }
}
