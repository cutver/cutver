use super::*;
use crate::semver_bump::Bump;

#[test]
fn parse_simple_feat() {
    let commit = ConventionalCommit::parse("feat: add auto bump").unwrap();
    assert_eq!(commit.commit_type, "feat");
    assert_eq!(commit.scope, None);
    assert!(!commit.is_breaking);
    assert_eq!(commit.description, "add auto bump");
    assert_eq!(commit.body, None);
    assert!(commit.footers.is_empty());
}

#[test]
fn parse_with_scope() {
    let commit = ConventionalCommit::parse("fix(parser): handle empty strings").unwrap();
    assert_eq!(commit.commit_type, "fix");
    assert_eq!(commit.scope, Some("parser".to_string()));
    assert!(!commit.is_breaking);
    assert_eq!(commit.description, "handle empty strings");
    assert_eq!(commit.body, None);
    assert!(commit.footers.is_empty());
}

#[test]
fn parse_with_nested_or_special_scope() {
    let commit = ConventionalCommit::parse("chore(deps-dev): bump toml from 0.8 to 0.9").unwrap();
    assert_eq!(commit.commit_type, "chore");
    assert_eq!(commit.scope, Some("deps-dev".to_string()));
    assert!(!commit.is_breaking);

    let commit2 = ConventionalCommit::parse("feat(ui/button): add icon support").unwrap();
    assert_eq!(commit2.scope, Some("ui/button".to_string()));
}

#[test]
fn parse_breaking_change_exclamation() {
    let commit = ConventionalCommit::parse("feat!: redesign CLI arguments").unwrap();
    assert_eq!(commit.commit_type, "feat");
    assert_eq!(commit.scope, None);
    assert!(commit.is_breaking);
    assert_eq!(commit.description, "redesign CLI arguments");

    let commit_scoped = ConventionalCommit::parse("refactor(core)!: drop legacy API").unwrap();
    assert_eq!(commit_scoped.commit_type, "refactor");
    assert_eq!(commit_scoped.scope, Some("core".to_string()));
    assert!(commit_scoped.is_breaking);
    assert_eq!(commit_scoped.description, "drop legacy API");
}

#[test]
fn parse_with_body() {
    let msg = "feat: add support for toml\n\nToml is used across rust ecosystems.\nIt provides clean syntax.";
    let commit = ConventionalCommit::parse(msg).unwrap();
    assert_eq!(commit.commit_type, "feat");
    assert_eq!(
        commit.body,
        Some("Toml is used across rust ecosystems.\nIt provides clean syntax.".to_string())
    );
    assert!(commit.footers.is_empty());
    assert!(!commit.is_breaking);
}

#[test]
fn parse_with_body_and_footers() {
    let msg = r#"fix: handle missing tag gracefully

When a repository has no prior tags, git describe fails with non-zero exit.
We now return None and inspect all commits up to HEAD.

Signed-off-by: Developer <dev@example.com>
Fixes: #42"#;
    let commit = ConventionalCommit::parse(msg).unwrap();
    assert_eq!(commit.commit_type, "fix");
    assert_eq!(
        commit.body,
        Some(
            "When a repository has no prior tags, git describe fails with non-zero exit.\nWe now return None and inspect all commits up to HEAD."
                .to_string()
        )
    );
    assert_eq!(
        commit.footers,
        vec![
            ("Signed-off-by".to_string(), "Developer <dev@example.com>".to_string()),
            ("Fixes".to_string(), "#42".to_string()),
        ]
    );
    assert!(!commit.is_breaking);
}

#[test]
fn parse_breaking_change_in_footer() {
    let msg = "refactor: rename config fields\n\nBREAKING CHANGE: current_source renamed to source";
    let commit = ConventionalCommit::parse(msg).unwrap();
    assert_eq!(commit.commit_type, "refactor");
    assert!(commit.is_breaking);
    assert_eq!(
        commit.footers,
        vec![(
            "BREAKING CHANGE".to_string(),
            "current_source renamed to source".to_string()
        )]
    );
    assert_eq!(commit.body, None);

    let msg_hyphen = "refactor: change defaults\n\nBREAKING-CHANGE: requires explicit opt-in";
    let commit_hyphen = ConventionalCommit::parse(msg_hyphen).unwrap();
    assert!(commit_hyphen.is_breaking);
    assert_eq!(
        commit_hyphen.footers,
        vec![("BREAKING-CHANGE".to_string(), "requires explicit opt-in".to_string())]
    );
}

#[test]
fn parse_breaking_change_in_body() {
    let msg = "fix: change error codes\n\nNotice: this is a BREAKING CHANGE: error codes are now numeric.";
    let commit = ConventionalCommit::parse(msg).unwrap();
    assert_eq!(commit.commit_type, "fix");
    assert!(commit.is_breaking);
}

#[test]
fn parse_multiline_footer() {
    let msg = r#"feat: new plugin system

BREAKING CHANGE: old plugin API removed
  Plugins must now implement PluginV2 trait.
  See migration guide for details.
Signed-off-by: Maintainer <maintainer@example.com>"#;
    let commit = ConventionalCommit::parse(msg).unwrap();
    assert!(commit.is_breaking);
    assert_eq!(commit.footers.len(), 2);
    assert_eq!(commit.footers[0].0, "BREAKING CHANGE");
    assert!(
        commit.footers[0]
            .1
            .contains("Plugins must now implement PluginV2 trait.")
    );
    assert_eq!(commit.footers[1].0, "Signed-off-by");
    assert_eq!(commit.footers[1].1, "Maintainer <maintainer@example.com>");
}

#[test]
fn parse_invalid_messages_return_none() {
    assert_eq!(ConventionalCommit::parse(""), None);
    assert_eq!(ConventionalCommit::parse("   "), None);
    assert_eq!(ConventionalCommit::parse("feat"), None);
    assert_eq!(ConventionalCommit::parse("feat:"), None);
    assert_eq!(ConventionalCommit::parse("feat: "), None);
    assert_eq!(ConventionalCommit::parse("feat(): empty scope"), None);
    assert_eq!(ConventionalCommit::parse("(scope): no type"), None);
    assert_eq!(
        ConventionalCommit::parse("Merge pull request #123 from user/branch"),
        None
    );
    assert_eq!(ConventionalCommit::parse("WIP on main: 1234567 some commit"), None);
    assert_eq!(ConventionalCommit::parse("feat(bad\nscope): newline in scope"), None);
    assert_eq!(ConventionalCommit::parse("invalid type: description"), None);
}

#[test]
fn deduce_bump_major_precedence() {
    let commits = vec![
        ConventionalCommit::parse("fix: small fix").unwrap(),
        ConventionalCommit::parse("feat: shiny feature").unwrap(),
        ConventionalCommit::parse("feat!: breaking feature").unwrap(),
    ];
    assert_eq!(deduce_bump(&commits), Bump::Major);

    let commits_footer = vec![
        ConventionalCommit::parse("fix: small fix\n\nBREAKING CHANGE: breaks stuff").unwrap(),
        ConventionalCommit::parse("feat: feature").unwrap(),
    ];
    assert_eq!(deduce_bump(&commits_footer), Bump::Major);
}

#[test]
fn deduce_bump_minor_precedence() {
    let commits = vec![
        ConventionalCommit::parse("fix: small fix").unwrap(),
        ConventionalCommit::parse("feat: shiny feature").unwrap(),
        ConventionalCommit::parse("docs: update readme").unwrap(),
        ConventionalCommit::parse("chore: bump deps").unwrap(),
    ];
    assert_eq!(deduce_bump(&commits), Bump::Minor);
}

#[test]
fn deduce_bump_patch_for_fixes_and_chores() {
    let commits = vec![
        ConventionalCommit::parse("fix: small fix").unwrap(),
        ConventionalCommit::parse("perf: speed up loop").unwrap(),
        ConventionalCommit::parse("chore: bump deps").unwrap(),
        ConventionalCommit::parse("docs: fix typo").unwrap(),
    ];
    assert_eq!(deduce_bump(&commits), Bump::Patch);

    let empty: Vec<ConventionalCommit> = Vec::new();
    assert_eq!(deduce_bump(&empty), Bump::Patch);
}

#[test]
fn parse_and_deduce_bump_with_mixed_messages() {
    let messages = [
        "Merge branch 'main' into feature",
        "feat: implement conventional commits",
        "fix: off by one error",
        "random non-conventional commit",
    ];
    let (bump, parsed) = parse_and_deduce_bump(&messages);
    assert_eq!(bump, Bump::Minor);
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0].commit_type, "feat");
    assert_eq!(parsed[1].commit_type, "fix");
}

#[test]
fn parse_and_deduce_bump_defaults_to_patch_on_empty_or_non_conventional() {
    let empty: Vec<&str> = Vec::new();
    let (bump, parsed) = parse_and_deduce_bump(&empty);
    assert_eq!(bump, Bump::Patch);
    assert!(parsed.is_empty());

    let non_conventional = ["initial commit", "WIP", "Update README.md"];
    let (bump, parsed) = parse_and_deduce_bump(&non_conventional);
    assert_eq!(bump, Bump::Patch);
    assert!(parsed.is_empty());
}

#[test]
fn breaking_change_footer_with_space_before_colon() {
    let msg1 = "fix: edge case\n\nBREAKING CHANGE : database schema dropped";
    let c1 = ConventionalCommit::parse(msg1).expect("parses conventional commit");
    assert!(c1.is_breaking);
    assert_eq!(c1.footers[0].0, "BREAKING CHANGE");
    assert_eq!(c1.footers[0].1, "database schema dropped");

    let msg2 = "fix: edge case\n\nBREAKING-CHANGE : database schema dropped";
    let c2 = ConventionalCommit::parse(msg2).expect("parses conventional commit");
    assert!(c2.is_breaking);
    assert_eq!(c2.footers[0].0, "BREAKING-CHANGE");
    assert_eq!(c2.footers[0].1, "database schema dropped");
}

#[test]
fn deduce_rationale_major_with_breaking_sample() {
    let commits = vec![
        ConventionalCommit::parse("feat(api)!: remove v1 endpoints").unwrap(),
        ConventionalCommit::parse("feat: add v2 endpoints").unwrap(),
        ConventionalCommit::parse("fix: handle null").unwrap(),
    ];
    let rationale = deduce_rationale(&commits);
    assert_eq!(rationale.level, Bump::Major);
    assert_eq!(rationale.breaking_count, 1);
    assert_eq!(rationale.feat_count, 2);
    assert_eq!(rationale.fix_count, 1);
    assert_eq!(rationale.other_count, 0);
    assert_eq!(
        rationale.breaking_sample.as_deref(),
        Some("feat(api)!: remove v1 endpoints")
    );
    assert_eq!(
        rationale.summary(),
        "major (deduced from breaking change: feat(api)!: remove v1 endpoints)"
    );
    assert_eq!(rationale.to_string(), rationale.summary());
}

#[test]
fn deduce_rationale_minor_mixed_commits() {
    let commits = vec![
        ConventionalCommit::parse("feat: add cli flag").unwrap(),
        ConventionalCommit::parse("feat(parser): add parser option").unwrap(),
        ConventionalCommit::parse("fix: off-by-one in loop").unwrap(),
        ConventionalCommit::parse("chore: update deps").unwrap(),
    ];
    let rationale = deduce_rationale(&commits);
    assert_eq!(rationale.level, Bump::Minor);
    assert_eq!(rationale.breaking_count, 0);
    assert_eq!(rationale.feat_count, 2);
    assert_eq!(rationale.fix_count, 1);
    assert_eq!(rationale.other_count, 1);
    assert_eq!(rationale.breaking_sample, None);
    assert_eq!(rationale.summary(), "minor (deduced from 2 features, 1 bugfix)");
}

#[test]
fn deduce_rationale_patch_fixes_and_chores() {
    let commits = vec![
        ConventionalCommit::parse("fix: repair glitch").unwrap(),
        ConventionalCommit::parse("chore: lint cleanup").unwrap(),
    ];
    let rationale = deduce_rationale(&commits);
    assert_eq!(rationale.level, Bump::Patch);
    assert_eq!(rationale.breaking_count, 0);
    assert_eq!(rationale.feat_count, 0);
    assert_eq!(rationale.fix_count, 1);
    assert_eq!(rationale.other_count, 1);
    assert_eq!(rationale.summary(), "patch (deduced from 1 bugfix, 1 other change)");
}

#[test]
fn deduce_rationale_patch_empty_or_no_commits() {
    let empty: Vec<ConventionalCommit> = Vec::new();
    let rationale = deduce_rationale(&empty);
    assert_eq!(rationale.level, Bump::Patch);
    assert_eq!(rationale.breaking_count, 0);
    assert_eq!(rationale.feat_count, 0);
    assert_eq!(rationale.fix_count, 0);
    assert_eq!(rationale.other_count, 0);
    assert_eq!(rationale.summary(), "patch (default - no breaking changes or features)");
}
