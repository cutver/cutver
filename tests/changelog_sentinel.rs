//! S5 and S6 (#307): the reviewed-none footer is review provenance, never release content.
//!
//! `NO-BREAKING-CHANGE: <why>` exists so a release can say "we looked, and this range is compatible".
//! Two things follow from that, and both are pinned here against the template the project ships:
//!
//! * the sentinel never reaches the rendered body — it is not a bullet, not a migration note, not a
//!   heading — while a range that really declares `BREAKING CHANGE:` still renders the warning block
//!   and its per-commit `> **Migration**:` line;
//! * the sentinel does not turn a compatible commit into a breaking one: before #307 the footer was
//!   detected as `BREAKING-CHANGE:` by substring, which rendered a "Breaking changes" block whose
//!   migration read "the endpoint is additive and the plugin environment is unchanged".

use cutver::changelog::{build_context, render_body, render_template};
use cutver::config::Changelog;
use cutver::conventional::ConventionalCommit;
use cutver::init::scaffold::DEFAULT_RELEASE_TEMPLATE;

const SENTINEL: &str = "NO-BREAKING-CHANGE:";
const REVIEWED_NONE: &str = "NO-BREAKING-CHANGE: the endpoint is additive and the plugin environment is unchanged";
const MIGRATION: &str = "BREAKING CHANGE: grants are now required; `env_clear()` leaves only the allowlist";

fn context(commits: &[ConventionalCommit]) -> cutver::changelog::ReleaseContext {
    build_context(
        "0.20.0",
        Some("0.19.0"),
        "v0.20.0",
        Some("v0.19.0"),
        "2026-11-21",
        Some("https://example.test/acme/cutver".to_string()),
        commits,
        vec!["ana".to_string()],
        false,
        "1.99",
    )
}

/// The rendered body of the shipped template plus the conventional renderer, both checked.
fn bodies(commits: &[ConventionalCommit]) -> Vec<String> {
    let ctx = context(commits);
    let template = render_template(DEFAULT_RELEASE_TEMPLATE, &ctx).expect("the default template renders");
    let conventional = render_body(&Changelog::default(), commits, Some(&ctx)).expect("the body renders");
    vec![template, conventional]
}

/// S5: a range that declares both a breaking change and the reviewed-none footer.
#[test]
fn the_sentinel_never_reaches_the_notes_of_a_range_that_declares_both() {
    let commits = vec![
        ConventionalCommit::parse(&format!(
            "docs(api): record the compatibility review\n\n{REVIEWED_NONE}"
        ))
        .expect("parses"),
        ConventionalCommit::parse(&format!("refactor(plugin): rewrite the handshake\n\n{MIGRATION}")).expect("parses"),
        ConventionalCommit::parse("feat(api): add the gallery endpoint").expect("parses"),
    ];

    for body in bodies(&commits) {
        assert!(
            !body.contains(SENTINEL),
            "the reviewed-none footer is provenance, not content: {body}"
        );
        assert!(
            !body.contains("the endpoint is additive"),
            "the sentinel's rationale is not release content either: {body}"
        );
    }

    let template = render_template(DEFAULT_RELEASE_TEMPLATE, &context(&commits)).expect("the default template renders");
    assert!(
        template.contains("> [!WARNING]"),
        "a declared breaking change still renders the warning block: {template}"
    );
    assert!(
        template.contains("> **Migration**: grants are now required"),
        "and its per-commit migration line: {template}"
    );
}

/// S5 and S6: the reviewed-none footer alone is not a breaking change — no warning block, no
/// migration line, and nothing of the footer in the body.
#[test]
fn a_reviewed_none_footer_is_not_a_breaking_change() {
    let commits = vec![
        ConventionalCommit::parse("feat(api): add the gallery endpoint").expect("parses"),
        ConventionalCommit::parse(&format!("docs(api): record the review\n\n{REVIEWED_NONE}")).expect("parses"),
    ];

    let template = render_template(DEFAULT_RELEASE_TEMPLATE, &context(&commits)).expect("the default template renders");
    assert!(
        !template.contains("> [!WARNING]"),
        "a compatible range renders no breaking-changes block: {template}"
    );
    assert!(!template.contains("**Migration**"), "and no migration line: {template}");
    for body in bodies(&commits) {
        assert!(!body.contains(SENTINEL), "{body}");
    }
}

/// S6: prose that merely names the sentinel is neither a footer nor a breaking change.
#[test]
fn prose_naming_the_sentinel_is_not_a_declaration() {
    let commit = ConventionalCommit::parse(
        "docs(api): record the review\n\nThe review asked whether\nNO-BREAKING-CHANGE: would be the honest\ndeclaration here, and left it for the next release to write.",
    )
    .expect("parses");

    assert!(
        commit.footers.is_empty(),
        "prose is not a footer block: {:?}",
        commit.footers
    );
    assert!(
        !commit.is_breaking,
        "a prose mention is not a breaking change: {commit:?}"
    );

    let template =
        render_template(DEFAULT_RELEASE_TEMPLATE, &context(&[commit])).expect("the default template renders");
    assert!(!template.contains("> [!WARNING]"), "{template}");
    assert!(!template.contains(SENTINEL), "{template}");
}
