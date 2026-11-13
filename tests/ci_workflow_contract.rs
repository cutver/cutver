//! Contract tests binding the `main` ruleset to the workflows that produce its two required status
//! contexts: `CI gate` (`.github/workflows/ci.yml`) and `Exactly one type label`
//! (`.github/workflows/pr-labels.yml`).
//!
//! The ruleset lives outside the repository (id 27202161, enforcement `active`,
//! `bypass_actors: []`), so no test can exercise it. What a test can hold is the edge every context
//! name, trigger and `needs`/`env` wire depends on, matched with the indentation the YAML actually
//! has. `contains()` is deliberately not used: it also passes on the wrong nesting level, on a
//! comment, and on text appended to the end of the file, which is how a text contract silently
//! stops testing anything.
//!
//! Guards #257 (risk item 5: delivery/CI). Covers S1 (exact context), S2 (job-level `if: always()`),
//! S3 (`needs` covers every existing job), S4 (each leg wired to its own result), S5 (unfiltered
//! `pull_request` trigger), S6 (label job context + event set).

use std::path::PathBuf;

const CI: &str = ".github/workflows/ci.yml";
const LABELS: &str = ".github/workflows/pr-labels.yml";

/// The exact literal GitHub reports as the check context and the ruleset requires. Renaming either
/// side alone leaves the context permanently "Expected": every pull request unmergeable, nothing red.
const GATE_CONTEXT: &str = "CI gate";
const LABEL_CONTEXT: &str = "Exactly one type label";
/// Every leg whose result the umbrella job collapses.
const LEGS: [&str; 3] = ["test", "test-plugins", "publish-dry-run"];

fn repo_text(relative: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative);
    let raw = std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
    raw.replace("\r\n", "\n") // line endings are a checkout setting
}

fn is_job_key(line: &str) -> bool {
    line.strip_prefix("  ")
        .is_some_and(|rest| !rest.starts_with(' ') && !rest.trim().is_empty())
}

/// Every job id declared under `jobs:`, in order.
fn job_ids(ci: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let mut in_jobs = false;
    for line in ci.lines() {
        if !in_jobs {
            in_jobs = line == "jobs:";
            continue;
        }
        if !is_job_key(line) {
            continue;
        }
        let rest = line.trim();
        if rest.starts_with('#') {
            continue;
        }
        if let Some(id) = rest.strip_suffix(':') {
            ids.push(id.to_string());
        }
    }
    ids
}

/// The lines of one job body: strictly between its key and the next job key.
///
/// Anchored to the `jobs:` section and to exact two-space indentation, so a same-named key
/// elsewhere (a step env var, a workflow section after `jobs:`) cannot stand in for the job.
fn job_lines<'a>(ci: &'a str, id: &str) -> Vec<&'a str> {
    let marker = "\njobs:\n";
    let jobs_at = ci
        .find(marker)
        .unwrap_or_else(|| panic!("{CI} must declare a `jobs:` section"))
        + marker.len();
    let body: Vec<&str> = ci[jobs_at..].lines().collect();
    let key = format!("  {id}:");
    let start = body
        .iter()
        .position(|line| *line == key)
        .unwrap_or_else(|| panic!("{CI} must declare the `{id}` job id"));
    let mut out = Vec::new();
    for line in &body[start + 1..] {
        if is_job_key(line) {
            break;
        }
        out.push(*line);
    }
    out
}

fn has_line(lines: &[&str], needle: &str) -> bool {
    lines.contains(&needle)
}

/// The value after `          KEY:` on an exact ten-space-indented line.
fn indented<'a>(lines: &[&'a str], prefix: &str) -> Vec<&'a str> {
    lines.iter().filter_map(|line| line.strip_prefix(prefix)).collect()
}

#[test]
fn the_gate_job_reports_the_exact_required_context() {
    let ci = repo_text(CI);
    let gate = job_lines(&ci, "gate");
    assert!(
        has_line(&gate, &format!("    name: {GATE_CONTEXT}")),
        "the `gate` job must declare exactly `    name: {GATE_CONTEXT}`: the ruleset requires that \
         literal context, so a suffix or a rename leaves every pull request waiting on a check that \
         never reports"
    );
    // Exactly one job may carry the required context, or which one satisfies the ruleset becomes a
    // function of evaluation order.
    let carriers: Vec<String> = job_ids(&ci)
        .into_iter()
        .filter(|id| has_line(&job_lines(&ci, id), &format!("    name: {GATE_CONTEXT}")))
        .collect();
    assert_eq!(
        carriers,
        vec!["gate".to_string()],
        "exactly the `gate` job may report `{GATE_CONTEXT}`"
    );
}

#[test]
fn the_gate_runs_unconditionally_and_covers_every_job() {
    let ci = repo_text(CI);
    let gate = job_lines(&ci, "gate");
    assert!(
        has_line(&gate, "    if: always()"),
        "`if: always()` must sit at job level (four spaces): on a step it cannot rescue the job, \
         which a failing need skips, and a skipped required check is neither green nor red"
    );
    assert!(
        has_line(&gate, &format!("    needs: [{}]", LEGS.join(", "))),
        "the gate must need exactly [{}] on one line",
        LEGS.join(", ")
    );

    // A leg added to `ci.yml` but left out of `needs` is silently unrequired: its failures run red
    // in the pull request and block nothing. Coverage is therefore checked against the jobs that
    // exist, not against a remembered list.
    let declared = job_ids(&ci);
    for leg in LEGS {
        assert!(
            declared.contains(&leg.to_string()),
            "the gate needs `{leg}`, which must be a real job id in {CI}"
        );
    }
    let ungated: Vec<&String> = declared
        .iter()
        .filter(|id| *id != "gate" && !LEGS.contains(&id.as_str()))
        .collect();
    assert!(
        ungated.is_empty(),
        "{CI} declares {ungated:?}, which the gate does not need: a failure there would not block a \
         merge. Add it to `needs` (and to LEGS) or delete it"
    );
}

#[test]
fn every_collapsed_leg_is_wired_to_its_own_result() {
    let ci = repo_text(CI);
    let gate = job_lines(&ci, "gate");

    // `          VAR: ${{ needs.<job>.result }}`
    let mut wires: Vec<(String, String)> = Vec::new();
    for line in &gate {
        let Some(rest) = line.strip_prefix("          ") else {
            continue;
        };
        let Some((var, value)) = rest.split_once(": ${{ needs.") else {
            continue;
        };
        let Some(job) = value.strip_suffix(".result }}") else {
            continue;
        };
        assert!(
            var.chars().all(|c| c.is_ascii_uppercase() || c == '_'),
            "unexpected env var `{var}`"
        );
        wires.push((var.to_string(), job.to_string()));
    }
    assert_eq!(
        wires.len(),
        LEGS.len(),
        "the gate must wire exactly {} need results into env vars, found {wires:?}",
        LEGS.len()
    );
    let mut wired_jobs: Vec<&str> = wires.iter().map(|(_, job)| job.as_str()).collect();
    wired_jobs.sort_unstable();
    let mut expected = LEGS.to_vec();
    expected.sort_unstable();
    assert_eq!(
        wired_jobs, expected,
        "each leg must be wired to its own env var: a copy-pasted `needs.<job>.result` leaves one \
         leg's failures unasserted while the gate still looks green"
    );

    // `          test "$VAR" = success` for every wired var, and never the looser comparison.
    let mut asserted: Vec<&str> = indented(&gate, "          test \"$")
        .into_iter()
        .filter_map(|rest| rest.strip_suffix("\" = success"))
        .collect();
    asserted.sort_unstable();
    let mut wired_vars: Vec<&str> = wires.iter().map(|(var, _)| var.as_str()).collect();
    wired_vars.sort_unstable();
    assert_eq!(
        asserted, wired_vars,
        "every wired result must be asserted with `= success`: a skipped or cancelled need also \
         reports non-success, so `!= failure` would pass it"
    );
    assert!(
        !gate.iter().any(|line| line.contains("!= failure")),
        "the gate must compare against `success`, never `!= failure`"
    );
}

#[test]
fn the_pr_trigger_produces_checks_for_every_pull_request() {
    let ci = repo_text(CI);
    let jobs_at = ci.find("\njobs:\n").expect("jobs section");
    let triggers = &ci[..jobs_at];
    assert!(
        has_line(&triggers.lines().collect::<Vec<_>>(), "  pull_request:"),
        "{CI} must run on `pull_request:`: a required context only exists where its workflow runs"
    );
    // `  pull_request:` must stay bare. A `paths`, `paths-ignore` or `branches` filter under it
    // means pull requests that touch nothing in the filter produce no check at all, and the ruleset
    // waits on a context that will never arrive.
    let mut after_pr = false;
    for line in triggers.lines() {
        if line == "  pull_request:" {
            after_pr = true;
            continue;
        }
        if after_pr && is_job_key(line) {
            after_pr = false;
        }
        if !after_pr || !line.starts_with("    ") || line.trim_start().starts_with('#') {
            continue;
        }
        let key = line.trim();
        for filter in ["paths:", "paths-ignore:", "branches:", "branches-ignore:", "types:"] {
            assert!(
                !key.starts_with(filter),
                "`pull_request` must stay unfiltered: `{filter}` under it means some pull requests \
                 never report `{GATE_CONTEXT}` and can never merge"
            );
        }
    }
}

#[test]
fn the_label_check_reports_the_exact_required_context_on_a_fresh_pull_request() {
    let labels = repo_text(LABELS);
    let job = job_lines(&labels, "type-label");
    assert!(
        has_line(&job, &format!("    name: {LABEL_CONTEXT}")),
        "the label job must declare exactly `    name: {LABEL_CONTEXT}`: the ruleset requires that \
         literal context"
    );

    let types = labels
        .lines()
        .find(|line| line.trim_start().starts_with("types:"))
        .unwrap_or_else(|| panic!("{LABELS} must declare the pull_request types it answers"));
    let inner = types
        .split_once('[')
        .and_then(|(_, rest)| rest.split_once(']'))
        .map(|(inner, _)| inner)
        .unwrap_or_else(|| panic!("{LABELS} types must be an inline list, found: {types}"));
    let mut events: Vec<&str> = inner.split(',').map(str::trim).filter(|e| !e.is_empty()).collect();
    events.sort_unstable();
    // `opened` is what keeps a required check out of "Expected" on a fresh pull request, and it
    // carries labels applied in the same `gh pr create`. The others re-answer when it can change.
    assert_eq!(
        events,
        vec!["labeled", "opened", "reopened", "synchronize", "unlabeled"],
        "{LABELS} must answer exactly these events; changing them changes which pull requests ever \
         report `{LABEL_CONTEXT}`. Update this list and the ruleset together"
    );
}
