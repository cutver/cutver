//! The maintenance surface is a declaration, so it ships with its check.
//!
//! `CONTRACT.md`'s Enforcement section forbids a rule without a check, and a template nobody checks
//! is a form that rots: GitHub refuses a malformed issue form without telling anyone, and the pull
//! request template can stop naming the two required contexts while still looking complete. Three
//! mechanical facts are pinned here, and nothing else is:
//!
//! - `.github/PULL_REQUEST_TEMPLATE.md` names `CI gate` and `Exactly one type label` by their exact
//!   strings — the same two `tests/ci_workflow_contract.rs` pins on the producing side, so a rename
//!   on either side alone is caught before a contributor spends an afternoon hunting a check that
//!   never reports;
//! - it keeps the five checkboxes S2 asks for, in that order and unchecked, because a ticked box is
//!   a claim about evidence and the template is read in fifteen seconds or not at all;
//! - the three issue forms are structurally what GitHub accepts, and `config.yml` exists;
//! - the one link `config.yml` carries is the private advisory form `SECURITY.md` sends a
//!   vulnerability report to. A contact link is a promise to answer somewhere, so the second link
//!   here is not a small addition: it is a channel nobody runs, and for a security report a dead
//!   address is the whole damage, because the reporter's next stop is a public issue.
//!
//! **Why no YAML dependency.** Parsing these files for real would mean a YAML crate in `Cargo.toml`,
//! and the contract makes a dependency the expensive answer: the dev-dependency list is deliberately
//! minimal and every entry is already in `Cargo.lock`. What has to be true is shallow — a top-level
//! `name:`, `description:` and `body:`, and `body` entries that each start `  - type:` with a type
//! GitHub renders — so the shape is asserted with anchored line checks. Anchoring is the point: the
//! one mistake that matters is a key nested a level too deep, which GitHub silently reads as absent,
//! and a `contains()` check would pass on it.
//!
//! Sibling of `tests/ci_workflow_contract.rs` and `tests/documentation_examples.rs`. Covers S6.

use std::path::PathBuf;

const PR_TEMPLATE: &str = ".github/PULL_REQUEST_TEMPLATE.md";
const CONFIG: &str = ".github/ISSUE_TEMPLATE/config.yml";
const SECURITY: &str = "SECURITY.md";

/// The one channel a vulnerability report may be routed to, matched literally: private vulnerability
/// reporting is enabled on the repository, so this URL is a live form rather than a guess.
const ADVISORY_URL: &str = "https://github.com/cutver/cutver/security/advisories/new";

/// The two status contexts the `main` ruleset requires, matched literally.
const GATE_CONTEXT: &str = "CI gate";
const LABEL_CONTEXT: &str = "Exactly one type label";

/// S2's five checklists, in the order they must appear. Anchored to a checkbox line rather than to
/// the file: the phrase surviving in prose while its box is deleted is exactly the failure this pins.
const CHECKLISTS: [&str; 5] = [
    "the gate was run",
    "the docs moved with the behaviour",
    "the slice boundary is stated",
    "every claim is verified",
    "a test failed before the fix",
];

/// The three forms S5 names, in that order.
const FORMS: [&str; 3] = [
    ".github/ISSUE_TEMPLATE/bug.yml",
    ".github/ISSUE_TEMPLATE/feature.yml",
    ".github/ISSUE_TEMPLATE/contract-debt.yml",
];

/// The `type:` values GitHub renders. An unknown one degrades the form to raw YAML the reporter
/// cannot fill in, with no error shown to anybody.
const FIELD_TYPES: [&str; 5] = ["markdown", "input", "textarea", "dropdown", "checkboxes"];

fn repo_text(relative: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative);
    let raw = std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
    raw.replace("\r\n", "\n") // line endings are a checkout setting
}

/// The unchecked boxes of a markdown checklist, at any nesting depth.
fn unchecked_boxes(text: &str) -> Vec<&str> {
    text.lines()
        .filter(|line| line.trim_start().starts_with("- [ ]"))
        .collect()
}

/// The ticked boxes. A template ships none: a box ticked before the work exists is a lie the
/// reviewer has to disprove.
fn checked_boxes(text: &str) -> Vec<&str> {
    text.lines()
        .filter(|line| line.trim_start().to_ascii_lowercase().starts_with("- [x]"))
        .collect()
}

/// Keys at column zero. An issue form's `name:`, `description:` and `body:` must sit there; nested
/// one level deeper they belong to the previous block and GitHub sees no such key.
fn top_level_keys(text: &str) -> Vec<&str> {
    text.lines()
        .filter(|line| !line.is_empty() && !line.starts_with(' ') && !line.starts_with('\t') && !line.starts_with('#'))
        .filter_map(|line| line.split_once(':').map(|(key, _)| key))
        .collect()
}

#[test]
fn the_pull_request_template_names_both_required_checks_exactly() {
    let template = repo_text(PR_TEMPLATE);
    for context in [GATE_CONTEXT, LABEL_CONTEXT] {
        assert!(
            template.lines().any(|line| line.contains(context)),
            "{PR_TEMPLATE} no longer names `{context}`: the `main` ruleset requires that exact \
             context, so a renamed check leaves every pull request waiting on a run that will never \
             appear, and the contributor reads the silence as their own mistake"
        );
    }
}

#[test]
fn the_pull_request_template_keeps_the_five_checklists_in_order() {
    let template = repo_text(PR_TEMPLATE);
    let boxes = unchecked_boxes(&template);
    assert!(
        !boxes.is_empty(),
        "{PR_TEMPLATE} has no unchecked `- [ ]` boxes: the five checklists are the project's review \
         criterion, and a template that states them as prose is a page the contributor skims instead \
         of five questions they answer"
    );

    let mut cursor = 0;
    for phrase in CHECKLISTS {
        let found = boxes[cursor..]
            .iter()
            .position(|line| line.to_lowercase().contains(phrase))
            .unwrap_or_else(|| {
                panic!(
                    "{PR_TEMPLATE} must carry `{phrase}` as an unchecked checkbox after the previous \
                     one, in S2's order: the order is the review sequence — a docs claim before the \
                     gate ran is a claim nobody verified"
                )
            });
        cursor += found + 1;
    }

    assert!(
        checked_boxes(&template).is_empty(),
        "{PR_TEMPLATE} ships a ticked box: a box ticked in the template is ticked on every pull \
         request, so it says nothing about this one, and evidence is the only thing a box may mean"
    );
}

#[test]
fn every_issue_form_is_one_github_will_accept() {
    for form in FORMS {
        let text = repo_text(form);
        let keys = top_level_keys(&text);
        for required in ["name", "description", "body"] {
            assert!(
                keys.contains(&required),
                "{form} must declare `{required}:` at column zero: GitHub reads a form without it as \
                 no form at all, and the failure is silent — the page simply stops offering these \
                 fields and reporters go back to blank issues"
            );
        }

        let types: Vec<&str> = text
            .lines()
            .filter_map(|line| line.strip_prefix("  - type: "))
            .map(str::trim_end)
            .collect();
        assert!(
            !types.is_empty(),
            "{form} declares a `body:` with no `  - type:` entry: a form with no fields accepts no \
             input, which is the ceremony this template set exists to avoid"
        );
        for field_type in &types {
            assert!(
                FIELD_TYPES.contains(field_type),
                "{form} asks for an unknown field type `{field_type}`; GitHub renders none of it, so \
                 the reporter sees YAML instead of the question"
            );
        }
        assert!(
            types.iter().any(|field_type| *field_type != "markdown"),
            "{form} carries only `markdown` blocks, which ask nothing: the fields are the point — \
             each one is a question the maintainer would otherwise have to ask by hand"
        );
    }
}

#[test]
fn the_issue_config_decides_blank_issues_and_routes_only_the_private_advisory_form() {
    let config = repo_text(CONFIG);
    let decision = config
        .lines()
        .find(|line| line.starts_with("blank_issues_enabled:"))
        .unwrap_or_else(|| {
            panic!(
                "{CONFIG} must declare `blank_issues_enabled:` at column zero: the repository made a \
                 decision about blank issues, and leaving it unsaid means the default decides it"
            )
        });
    let value = decision
        .split_once(':')
        .map(|(_, rest)| rest.trim())
        .unwrap_or_default();
    assert!(
        matches!(value, "true" | "false"),
        "{CONFIG} must state a boolean for `blank_issues_enabled:`, found `{value}`"
    );
    assert!(
        config.lines().any(|line| line.starts_with('#')),
        "{CONFIG} must carry the reason for its blank-issue decision in a comment: an unexplained \
         setting is a decision nobody can revisit, which is how a template set starts outliving its \
         own argument"
    );

    let urls: Vec<&str> = config
        .lines()
        .filter_map(|line| line.trim().strip_prefix("url:"))
        .map(|rest| rest.trim().trim_matches('"'))
        .collect();
    assert!(
        urls.contains(&ADVISORY_URL),
        "{CONFIG} no longer routes a vulnerability report to {ADVISORY_URL}: {SECURITY} names the \
         private advisory form as the one channel, and private vulnerability reporting is enabled, so \
         a config that drops the link leaves the reporter to open the public issue that page forbids"
    );
    let link_keys: Vec<&str> = config
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| line.strip_prefix("- ").unwrap_or(line))
        .filter_map(|line| line.split_once(':').map(|(key, _)| key.trim()))
        .collect();
    assert!(
        link_keys.contains(&"contact_links"),
        "{CONFIG} must declare a `contact_links:` block: the vulnerability route is the one entry it \
         offers beside the three forms, and a link written in a comment is a link GitHub never renders"
    );
    for key in ["name", "about"] {
        assert!(
            link_keys.contains(&key),
            "{CONFIG}'s `contact_links` entry needs a `{key}:` key: GitHub renders neither an unnamed \
             nor an unexplained link, so the reporter sees a button with no reason to press it"
        );
    }

    let documented = format!(
        "{}{}{}",
        repo_text("README.md"),
        repo_text("CONTRIBUTING.md"),
        repo_text(SECURITY)
    );
    for url in urls {
        assert!(
            url == ADVISORY_URL,
            "{CONFIG} links to {url}, which is not the private advisory form: a contact link is a \
             promise to answer somewhere, and an address nobody runs is worse than no address — for a \
             vulnerability report it is the whole damage, because the reporter then opens a public issue"
        );
        assert!(
            documented.contains(url),
            "{CONFIG} links to {url}, which README.md, CONTRIBUTING.md and {SECURITY} all fail to \
             mention: a contact link nobody documented is a channel nobody runs, and inventing one \
             here turns an unanswered question into a 404"
        );
    }
}
