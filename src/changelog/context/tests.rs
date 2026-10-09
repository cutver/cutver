use super::assemble::*;
use super::enrich::*;
use super::parse::*;
use super::types::*;
use crate::conventional::ConventionalCommit;

#[test]
fn test_assemble_release_context() {
    let changelog_config = crate::config::Changelog::default();
    let commits = vec![ConventionalCommit::parse("feat: add feature").unwrap()];
    let params = AssembleContextParams {
        version: "1.2.0",
        prev_version: Some("1.1.0"),
        tag: "v1.2.0",
        prev_tag: Some("v1.1.0"),
        date: "2025-01-01",
        repository: Some("https://github.com/owner/repo".into()),
        parsed_commits: &commits,
        raw_commits: None,
        contributors: vec!["Alice".into()],
        first_time_contributors: vec![],
        changelog_config: &changelog_config,
    };
    let ctx = assemble_release_context(params);
    assert_eq!(ctx.version, "1.2.0");
    assert_eq!(ctx.previous_version.as_deref(), Some("1.1.0"));
    assert_eq!(ctx.tag, "v1.2.0");
    assert_eq!(ctx.previous_tag.as_deref(), Some("v1.1.0"));
    assert_eq!(ctx.date, "2025-01-01");
    assert_eq!(ctx.repository.as_deref(), Some("https://github.com/owner/repo"));
    assert_eq!(ctx.contributors, vec!["Alice"]);
    assert_eq!(ctx.features, "- add feature");
}

#[test]
fn test_first_time_contributors_flag_present_before_vs_new() {
    let changelog_config = crate::config::Changelog::default();
    let commits = vec![];
    let params = AssembleContextParams {
        version: "2.0.0",
        prev_version: Some("1.0.0"),
        tag: "v2.0.0",
        prev_tag: Some("v1.0.0"),
        date: "2026-01-01",
        repository: Some("https://github.com/owner/repo".into()),
        parsed_commits: &commits,
        raw_commits: None,
        contributors: vec!["Alice".into(), "Bob".into()],
        // Bob was absent before this window, so he is flagged; Alice is a veteran.
        first_time_contributors: vec!["Bob".into()],
        changelog_config: &changelog_config,
    };
    let ctx = assemble_release_context(params);
    assert_eq!(ctx.first_time_contributors, vec!["Bob".to_string()]);

    let req = ctx.to_changelog_render_request("/workspace");
    assert_eq!(req.repository.as_deref(), Some("https://github.com/owner/repo"));
    assert_eq!(
        req.compare_url.as_deref(),
        Some("https://github.com/owner/repo/compare/v1.0.0...v2.0.0")
    );
    assert!(!req.is_prerelease);

    let alice = req.contributors.iter().find(|c| c.name == "Alice").unwrap();
    let bob = req.contributors.iter().find(|c| c.name == "Bob").unwrap();
    assert!(
        !alice.is_first_contribution,
        "a contributor present before must not be flagged"
    );
    assert!(bob.is_first_contribution, "a contributor absent before must be flagged");
}

#[test]
fn test_filter_commits_release_commits() {
    let commits = vec![
        ConventionalCommit::parse("chore(release): v1.0.0").unwrap(),
        ConventionalCommit::parse("chore(deps): update foo").unwrap(),
        ConventionalCommit::parse("chore: clean up").unwrap(),
        ConventionalCommit::parse("chore: release: 1.0.0").unwrap(),
        ConventionalCommit::parse("fix: v2.0.0 bug").unwrap(),
        ConventionalCommit::parse("feat: add feature").unwrap(),
    ];

    let filtered = filter_commits(&commits, true, &[]);
    let descriptions: Vec<&str> = filtered.iter().map(|c| c.description.trim()).collect();
    assert_eq!(descriptions, vec!["update foo", "clean up", "add feature"]);

    let unfiltered = filter_commits(&commits, false, &[]);
    assert_eq!(unfiltered.len(), commits.len());
}

#[test]
fn test_filter_commits_ignore_scopes() {
    let commits = vec![
        ConventionalCommit::parse("feat(cli): new flag").unwrap(),
        ConventionalCommit::parse("fix(core): bugfix").unwrap(),
        ConventionalCommit::parse("docs: update readme").unwrap(),
    ];

    let filtered = filter_commits(&commits, false, &[String::from("cli")]);
    assert_eq!(filtered.len(), 2);
    assert_eq!(filtered[0].commit_type, "fix");
    assert_eq!(filtered[1].commit_type, "docs");
}

#[test]
fn test_format_commit_line_and_bullet() {
    let mut commit = ConventionalCommit::parse("feat(cli): add flag (#42)").unwrap();
    commit.footers.push(("Fixes".into(), "#101".into()));

    let raw = crate::git::RawCommit {
        hash: "1234567890abcdef1234567890abcdef12345678".to_string(),
        short_hash: "1234567".to_string(),
        author_name: "Alice Smith".to_string(),
        author_email: "alice@example.com".to_string(),
        message: "feat(cli): add flag (#42)\n\nFixes #101".to_string(),
    };

    let ctx = enrich_commit_context(
        CommitContext::from(&commit),
        Some(&raw),
        Some(&commit),
        Some("https://github.com/Row0902/cutver"),
    );

    assert_eq!(ctx.commit_type, "feat");
    assert_eq!(ctx.scope.as_deref(), Some("cli"));
    assert_eq!(ctx.description, "add flag (#42)");
    assert_eq!(ctx.clean_description, "add flag");
    assert_eq!(ctx.pr_number, Some(42));
    assert_eq!(ctx.pr_url.as_deref(), Some("https://github.com/Row0902/cutver/pull/42"));
    assert_eq!(ctx.issue_numbers, vec![101]);
    assert_eq!(ctx.author.as_deref(), Some("Alice Smith"));
    assert_eq!(ctx.short_hash.as_deref(), Some("1234567"));
    assert_eq!(
        ctx.commit_url.as_deref(),
        Some("https://github.com/Row0902/cutver/commit/1234567890abcdef1234567890abcdef12345678")
    );

    let formatted = format_commit_line(&ctx, true);
    let expected = "**cli**: add flag in [#42](https://github.com/Row0902/cutver/pull/42) ([1234567](https://github.com/Row0902/cutver/commit/1234567890abcdef1234567890abcdef12345678)) by @Alice Smith";
    assert_eq!(formatted, expected);
    assert_eq!(ctx.line, formatted);
    assert_eq!(ctx.bullet, format!("- {formatted}"));

    let json = serde_json::to_string(&ctx).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["type"], "feat");
    assert_eq!(value["commit_type"], "feat");
    assert_eq!(value["scope"], "cli");
    assert_eq!(value["clean_description"], "add flag");
    assert_eq!(value["pr_number"], 42);
    assert_eq!(value["pr_url"], "https://github.com/Row0902/cutver/pull/42");
    assert_eq!(value["line"], formatted);
    assert_eq!(value["bullet"], format!("- {formatted}"));

    let json_input = serde_json::json!({
        "type": "feat",
        "scope": "cli",
        "description": "add flag (#42)",
        "line": formatted,
        "bullet": format!("- {formatted}")
    });
    let deserialized: CommitContext = serde_json::from_value(json_input).unwrap();
    assert_eq!(deserialized.line, formatted);
    assert_eq!(deserialized.bullet, format!("- {formatted}"));
}

#[test]
fn test_format_commit_line_without_urls() {
    let ctx = CommitContext {
        commit_type: "fix".to_string(),
        scope: None,
        description: "resolve bug".to_string(),
        clean_description: "resolve bug".to_string(),
        is_breaking: false,
        body: None,
        breaking_description: None,
        hash: None,
        short_hash: Some("abc1234".to_string()),
        author: Some("Bob".to_string()),
        author_email: None,
        pr_number: Some(10),
        pr_url: None,
        issue_numbers: vec![],
        commit_url: None,
        line: String::new(),
        bullet: String::new(),
    };
    let formatted = format_commit_line(&ctx, true);
    assert_eq!(formatted, "resolve bug in #10 (abc1234) by @Bob");
}

#[test]
fn test_build_context_full() {
    let commits = vec![
        ConventionalCommit::parse("feat(cli): add template flag").unwrap(),
        ConventionalCommit::parse("fix: small fix").unwrap(),
        ConventionalCommit::parse("feat!: breaking api change").unwrap(),
        ConventionalCommit::parse("chore(release): v1.2.0").unwrap(),
    ];
    let contributors = vec!["Alice".to_string(), "Bob".to_string()];
    let ctx = build_context_with_filter(
        "1.2.0",
        Some("1.1.0"),
        "v1.2.0",
        Some("v1.1.0"),
        "2026-03-30",
        Some("https://github.com/Row0902/cutver".to_string()),
        &commits,
        contributors,
        true,
        "Maintenance and updates.",
        true,
        &[],
    );

    assert_eq!(ctx.version, "1.2.0");
    assert_eq!(ctx.previous_version.as_deref(), Some("1.1.0"));
    assert_eq!(ctx.tag, "v1.2.0");
    assert_eq!(ctx.previous_tag.as_deref(), Some("v1.1.0"));
    assert_eq!(ctx.date, "2026-03-30");
    assert_eq!(
        ctx.compare_url.as_deref(),
        Some("https://github.com/Row0902/cutver/compare/v1.1.0...v1.2.0")
    );
    assert_eq!(ctx.repository.as_deref(), Some("https://github.com/Row0902/cutver"));
    assert_eq!(ctx.features, "- **cli**: add template flag");
    assert_eq!(ctx.fixes, "- small fix");
    assert_eq!(ctx.breaking, "- breaking api change");
    assert_eq!(ctx.contributors, vec!["Alice", "Bob"]);
    assert_eq!(ctx.commits.len(), 3);
}

#[test]
fn test_build_context_empty_commits() {
    let ctx = build_context_with_filter(
        "1.0.0",
        None,
        "v1.0.0",
        None,
        "2026-03-30",
        None,
        &[],
        vec![],
        true,
        "No changes recorded.",
        true,
        &[],
    );

    assert_eq!(ctx.version, "1.0.0");
    assert_eq!(ctx.all_changes, "- No changes recorded.");
    assert!(ctx.commits.is_empty());
}

#[test]
fn test_enrich_commit_context_metadata() {
    let commit = ConventionalCommit::parse("feat: shiny thing (#42)").unwrap();
    let raw = crate::git::RawCommit {
        hash: "abcdef1234567890abcdef1234567890abcdef12".to_string(),
        short_hash: "abcdef1".to_string(),
        author_name: "Alice".to_string(),
        author_email: "alice@example.com".to_string(),
        message: "feat: shiny thing (#42)".to_string(),
    };

    let ctx = enrich_commit_context(
        CommitContext::from(&commit),
        Some(&raw),
        Some(&commit),
        Some("https://github.com/org/repo"),
    );

    assert_eq!(ctx.author.as_deref(), Some("Alice"));
    assert_eq!(ctx.author_email.as_deref(), Some("alice@example.com"));
    assert_eq!(ctx.short_hash.as_deref(), Some("abcdef1"));
    assert_eq!(
        ctx.commit_url.as_deref(),
        Some("https://github.com/org/repo/commit/abcdef1234567890abcdef1234567890abcdef12")
    );
    assert_eq!(ctx.pr_number, Some(42));
    assert_eq!(ctx.pr_url.as_deref(), Some("https://github.com/org/repo/pull/42"));
    assert_eq!(ctx.clean_description, "shiny thing");
}

#[test]
fn test_enrich_commit_context_gitlab_url() {
    let commit = ConventionalCommit::parse("fix: patch bug").unwrap();
    let raw = crate::git::RawCommit {
        hash: "1111111111111111111111111111111111111111".to_string(),
        short_hash: "1111111".to_string(),
        author_name: "Bob".to_string(),
        author_email: "bob@gitlab.com".to_string(),
        message: "fix: patch bug\n\nMerge pull request #99".to_string(),
    };

    let ctx = enrich_commit_context(
        CommitContext::from(&commit),
        Some(&raw),
        Some(&commit),
        Some("https://gitlab.com/group/project"),
    );

    assert_eq!(ctx.pr_number, Some(99));
    assert_eq!(
        ctx.pr_url.as_deref(),
        Some("https://gitlab.com/group/project/-/merge_requests/99")
    );
    assert_eq!(
        ctx.commit_url.as_deref(),
        Some("https://gitlab.com/group/project/-/commit/1111111111111111111111111111111111111111")
    );
}

#[test]
fn test_linear_raw_commits_distinct_attribution_for_duplicates() {
    let commits = vec![
        ConventionalCommit::parse("fix: identical commit message").unwrap(),
        ConventionalCommit::parse("fix: identical commit message").unwrap(),
    ];

    let raw_commits = vec![
        crate::git::RawCommit {
            hash: "1111111111111111111111111111111111111111".to_string(),
            short_hash: "1111111".to_string(),
            author_name: "Alice".to_string(),
            author_email: "alice@example.com".to_string(),
            message: "fix: identical commit message".to_string(),
        },
        crate::git::RawCommit {
            hash: "2222222222222222222222222222222222222222".to_string(),
            short_hash: "2222222".to_string(),
            author_name: "Bob".to_string(),
            author_email: "bob@example.com".to_string(),
            message: "fix: identical commit message".to_string(),
        },
    ];

    let ctx = build_context_with_raw_and_filter(
        "1.0.0",
        None,
        "v1.0.0",
        None,
        "2026-03-30",
        None,
        &commits,
        Some(&raw_commits),
        vec![],
        Vec::new(),
        false,
        "fallback",
        false,
        &[],
    );

    assert_eq!(ctx.commits.len(), 2);
    assert_eq!(
        ctx.commits[0].hash.as_deref(),
        Some("1111111111111111111111111111111111111111")
    );
    assert_eq!(ctx.commits[0].author.as_deref(), Some("Alice"));
    assert_eq!(
        ctx.commits[1].hash.as_deref(),
        Some("2222222222222222222222222222222222222222")
    );
    assert_eq!(ctx.commits[1].author.as_deref(), Some("Bob"));
}

#[test]
fn test_clean_description_strips_trailing_pr_number() {
    assert_eq!(
        strip_trailing_pr_number("add exciting feature (#42)"),
        "add exciting feature"
    );
    assert_eq!(
        strip_trailing_pr_number("fix issue with (#12) in core (#99)"),
        "fix issue with (#12) in core"
    );
    assert_eq!(
        strip_trailing_pr_number("no pr reference in description"),
        "no pr reference in description"
    );
    assert_eq!(
        strip_trailing_pr_number("invalid pr suffix (#abc)"),
        "invalid pr suffix (#abc)"
    );

    let c = ConventionalCommit::parse("feat: add feature (#100)").unwrap();
    let ctx = CommitContext::from(&c);
    assert_eq!(ctx.description, "add feature (#100)");
    assert_eq!(ctx.clean_description, "add feature");
}

#[test]
fn test_chained_pr_deduplication() {
    // 1. Test strip_trailing_pr_numbers directly
    let (clean, prs) = strip_trailing_pr_numbers("feat: something (#85) (#90)");
    assert_eq!(clean, "feat: something");
    assert_eq!(prs, vec![90, 85]);

    // 2. Test CommitContext::from
    let c = ConventionalCommit::parse("feat: something (#85) (#90)").unwrap();
    let ctx = CommitContext::from(&c);
    assert_eq!(ctx.description, "something (#85) (#90)");
    assert_eq!(ctx.clean_description, "something");
    assert_eq!(ctx.pr_number, Some(90));
    assert_eq!(ctx.issue_numbers, vec![85]);

    // 3. Test enrich_commit_context with chained PR numbers
    let raw = crate::git::RawCommit {
        hash: "1234567890abcdef1234567890abcdef12345678".to_string(),
        short_hash: "1234567".to_string(),
        author_name: "Author".to_string(),
        author_email: "author@example.com".to_string(),
        message: "feat: something (#85) (#90)".to_string(),
    };
    let enriched = enrich_commit_context(
        CommitContext::from(&c),
        Some(&raw),
        Some(&c),
        Some("https://github.com/owner/repo"),
    );
    assert_eq!(enriched.clean_description, "something");
    assert_eq!(enriched.pr_number, Some(90));
    assert_eq!(enriched.issue_numbers, vec![85]);
    assert_eq!(
        enriched.pr_url.as_deref(),
        Some("https://github.com/owner/repo/pull/90")
    );
    // Ensure no duplicate (#85) or (#90) in the rendered line
    assert_eq!(
        enriched.line,
        "something in [#90](https://github.com/owner/repo/pull/90) ([1234567](https://github.com/owner/repo/commit/1234567890abcdef1234567890abcdef12345678)) by @Author"
    );
}

#[test]
fn test_parse_repo_forge() {
    let meta = parse_repo_forge("https://github.com/Row0902/cutver.git");
    assert_eq!(meta.forge.as_deref(), Some("github"));
    assert_eq!(meta.owner.as_deref(), Some("Row0902"));
    assert_eq!(meta.repo.as_deref(), Some("cutver"));

    let meta = parse_repo_forge("git@github.com:Row0902/cutver.git");
    assert_eq!(meta.forge.as_deref(), Some("github"));
    assert_eq!(meta.owner.as_deref(), Some("Row0902"));
    assert_eq!(meta.repo.as_deref(), Some("cutver"));

    let meta = parse_repo_forge("ssh://git@gitlab.com/group/subgroup/project.git");
    assert_eq!(meta.forge.as_deref(), Some("gitlab"));
    assert_eq!(meta.owner.as_deref(), Some("group/subgroup"));
    assert_eq!(meta.repo.as_deref(), Some("project"));

    let meta = parse_repo_forge("https://bitbucket.org/team/repo");
    assert_eq!(meta.forge.as_deref(), Some("bitbucket"));
    assert_eq!(meta.owner.as_deref(), Some("team"));
    assert_eq!(meta.repo.as_deref(), Some("repo"));

    let meta = parse_repo_forge("https://codeberg.org/user/repo.git");
    assert_eq!(meta.forge.as_deref(), Some("codeberg"));
    assert_eq!(meta.owner.as_deref(), Some("user"));
    assert_eq!(meta.repo.as_deref(), Some("repo"));

    let meta = parse_repo_forge("https://git.sr.ht/~user/repo");
    assert_eq!(meta.forge.as_deref(), Some("sourcehut"));
    assert_eq!(meta.owner.as_deref(), Some("~user"));
    assert_eq!(meta.repo.as_deref(), Some("repo"));

    let meta = parse_repo_forge("");
    assert_eq!(meta.forge, None);
    assert_eq!(meta.owner, None);
    assert_eq!(meta.repo, None);
}

#[test]
fn test_release_context_semver_breakdown() {
    let ctx = build_context_with_filter(
        "1.2.3-alpha.1+20230101",
        None,
        "v1.2.3-alpha.1+20230101",
        None,
        "2026-03-30",
        Some("https://github.com/org/repo".to_string()),
        &[],
        vec![],
        true,
        "entry",
        true,
        &[],
    );

    assert_eq!(ctx.major, 1);
    assert_eq!(ctx.minor, 2);
    assert_eq!(ctx.patch, 3);
    assert!(ctx.is_prerelease);
    assert_eq!(ctx.prerelease.as_deref(), Some("alpha.1"));
    assert_eq!(ctx.build.as_deref(), Some("20230101"));
    assert_eq!(ctx.repo_owner.as_deref(), Some("org"));
    assert_eq!(ctx.repo_name.as_deref(), Some("repo"));
    assert_eq!(ctx.forge.as_deref(), Some("github"));
}

#[test]
fn test_release_context_non_semver_fallback() {
    let ctx = build_context_with_filter(
        "custom-version-2026",
        None,
        "custom-version-2026",
        None,
        "2026-03-30",
        None,
        &[],
        vec![],
        true,
        "entry",
        true,
        &[],
    );

    assert_eq!(ctx.major, 0);
    assert_eq!(ctx.minor, 0);
    assert_eq!(ctx.patch, 0);
    assert!(!ctx.is_prerelease);
    assert_eq!(ctx.prerelease, None);
    assert_eq!(ctx.build, None);
    assert_eq!(ctx.repo_owner, None);
    assert_eq!(ctx.repo_name, None);
    assert_eq!(ctx.forge, None);
}

#[test]
fn test_release_context_serialization_aliases() {
    let ctx = build_context_with_filter(
        "1.2.3-beta.2+build.42",
        None,
        "v1.2.3-beta.2+build.42",
        None,
        "2026-03-30",
        Some("https://github.com/my-org/my-project".to_string()),
        &[],
        vec![],
        true,
        "entry",
        true,
        &[],
    );

    let json = serde_json::to_string(&ctx).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(value["major"], 1);
    assert_eq!(value["minor"], 2);
    assert_eq!(value["patch"], 3);
    assert_eq!(value["is_prerelease"], true);
    assert_eq!(value["prerelease"], "beta.2");
    assert_eq!(value["build"], "build.42");
    assert_eq!(value["repo_owner"], "my-org");
    assert_eq!(value["owner"], "my-org");
    assert_eq!(value["repo_name"], "my-project");
    assert_eq!(value["repo"], "my-project");
    assert_eq!(value["forge"], "github");
    assert_eq!(value["project_name"], "my-project");
    assert_eq!(value["project"], "my-project");

    let minimal_json = serde_json::json!({
        "version": "2.0.1-rc.1",
        "tag": "v2.0.1-rc.1",
        "date": "2026-03-30",
        "repository": "https://gitlab.com/group/subgroup/my-app.git"
    });
    let deserialized: ReleaseContext = serde_json::from_value(minimal_json).unwrap();
    assert_eq!(deserialized.major, 2);
    assert_eq!(deserialized.minor, 0);
    assert_eq!(deserialized.patch, 1);
    assert!(deserialized.is_prerelease);
    assert_eq!(deserialized.prerelease.as_deref(), Some("rc.1"));
    assert_eq!(deserialized.repo_owner.as_deref(), Some("group/subgroup"));
    assert_eq!(deserialized.repo_name.as_deref(), Some("my-app"));
    assert_eq!(deserialized.forge.as_deref(), Some("gitlab"));
    assert_eq!(deserialized.project_name.as_deref(), Some("my-app"));
}

#[test]
fn test_interpolation_context_build_and_serialization() {
    let ctx = InterpolationContext::build(
        "1.2.3-rc.1+build.123",
        Some("1.2.2"),
        "v1.2.3-rc.1+build.123",
        Some("v1.2.2"),
        "minor",
        "2026-03-30",
        Some("main"),
        Some("https://github.com/my-org/my-pkg"),
    );

    assert_eq!(ctx.version, "1.2.3-rc.1+build.123");
    assert_eq!(ctx.previous_version.as_deref(), Some("1.2.2"));
    assert_eq!(ctx.tag, "v1.2.3-rc.1+build.123");
    assert_eq!(ctx.previous_tag.as_deref(), Some("v1.2.2"));
    assert_eq!(ctx.bump_level, "minor");
    assert!(ctx.is_prerelease);
    assert_eq!(ctx.prerelease.as_deref(), Some("rc.1"));
    assert_eq!(ctx.build.as_deref(), Some("build.123"));
    assert_eq!(ctx.major, 1);
    assert_eq!(ctx.minor, 2);
    assert_eq!(ctx.patch, 3);
    assert_eq!(ctx.date, "2026-03-30");
    assert_eq!(ctx.branch.as_deref(), Some("main"));
    assert_eq!(ctx.repo_owner.as_deref(), Some("my-org"));
    assert_eq!(ctx.repo_name.as_deref(), Some("my-pkg"));
    assert_eq!(ctx.forge.as_deref(), Some("github"));
    assert_eq!(ctx.project_name.as_deref(), Some("my-pkg"));

    let json = serde_json::to_string(&ctx).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["owner"], "my-org");
    assert_eq!(value["repo_owner"], "my-org");
    assert_eq!(value["repo"], "my-pkg");
    assert_eq!(value["repo_name"], "my-pkg");
    assert_eq!(value["forge"], "github");
    assert_eq!(value["branch"], "main");
    assert_eq!(value["bump_level"], "minor");
    assert_eq!(value["is_prerelease"], true);
    assert_eq!(value["project"], "my-pkg");
    assert_eq!(value["project_name"], "my-pkg");

    let from_rc = InterpolationContext::from_release_context(
        &ReleaseContext {
            version: "1.0.0".into(),
            previous_version: None,
            tag: "v1.0.0".into(),
            previous_tag: None,
            date: "2026-03-30".into(),
            compare_url: None,
            repository: None,
            features: String::new(),
            fixes: String::new(),
            breaking: String::new(),
            perf: String::new(),
            refactor: String::new(),
            docs: String::new(),
            maintenance: String::new(),
            other: String::new(),
            all_changes: String::new(),
            commits: Vec::new(),
            contributors: Vec::new(),
            first_time_contributors: Vec::new(),
            major: 1,
            minor: 0,
            patch: 0,
            is_prerelease: false,
            prerelease: None,
            build: None,
            repo_owner: None,
            repo_name: None,
            forge: None,
            project_name: Some("cutver".into()),
            year: 2026,
            month: 3,
            day: 30,
        },
        "major",
        Some("main"),
    );
    assert_eq!(from_rc.project_name.as_deref(), Some("cutver"));
}

#[test]
fn test_template_rendering_with_project_and_project_name() {
    let mut ctx = build_context_with_filter(
        "1.0.0",
        None,
        "v1.0.0",
        None,
        "2026-03-30",
        Some("https://github.com/my-org/my-repo".to_string()),
        &[],
        vec![],
        true,
        "entry",
        true,
        &[],
    );
    assert_eq!(ctx.project_name.as_deref(), Some("my-repo"));

    let template = "Project: {{ project_name }}, Alias: {{ project }}, Repo: {{ repo }}";
    let mut env = minijinja::Environment::new();
    env.add_template("test", template).unwrap();
    let tmpl = env.get_template("test").unwrap();
    let rendered = tmpl.render(&ctx).unwrap();
    assert_eq!(rendered, "Project: my-repo, Alias: my-repo, Repo: my-repo");

    ctx.project_name = Some("custom-project".to_string());
    let rendered = tmpl.render(&ctx).unwrap();
    assert_eq!(
        rendered,
        "Project: custom-project, Alias: custom-project, Repo: my-repo"
    );
}

#[test]
fn test_commit_context_body_and_breaking_description() {
    let msg = "feat(parser)!: rewrite parser\n\nDetailed body explaining why.\n\nBREAKING CHANGE: AST structure changed completely";
    let c = ConventionalCommit::parse(msg).unwrap();
    let ctx = CommitContext::from(&c);

    assert_eq!(ctx.body.as_deref(), Some("Detailed body explaining why."));
    assert_eq!(
        ctx.breaking_description.as_deref(),
        Some("AST structure changed completely")
    );

    let serialized = serde_json::to_value(&ctx).unwrap();
    assert_eq!(serialized["body"], "Detailed body explaining why.");
    assert_eq!(serialized["breaking_description"], "AST structure changed completely");
    assert_eq!(
        serialized["breaking_change_description"],
        "AST structure changed completely"
    );

    let deserialized: CommitContext = serde_json::from_value(serialized).unwrap();
    assert_eq!(deserialized.body.as_deref(), Some("Detailed body explaining why."));
    assert_eq!(
        deserialized.breaking_description.as_deref(),
        Some("AST structure changed completely")
    );
}

#[test]
fn test_release_context_date_parts() {
    let ctx = build_context_with_filter(
        "1.2.3",
        None,
        "v1.2.3",
        None,
        "2026-04-15",
        None,
        &[],
        vec![],
        true,
        "entry",
        true,
        &[],
    );
    assert_eq!(ctx.year, 2026);
    assert_eq!(ctx.month, 4);
    assert_eq!(ctx.day, 15);

    let serialized = serde_json::to_value(&ctx).unwrap();
    assert_eq!(serialized["year"], 2026);
    assert_eq!(serialized["month"], 4);
    assert_eq!(serialized["day"], 15);

    let deserialized: ReleaseContext = serde_json::from_value(serialized).unwrap();
    assert_eq!(deserialized.year, 2026);
    assert_eq!(deserialized.month, 4);
    assert_eq!(deserialized.day, 15);
}
