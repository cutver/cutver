use crate::changelog::build_context;
use crate::changelog::context::InterpolationContext;
use crate::changelog::render::body::{render_body, render_body_with_context};
use crate::changelog::render::template::{interpolate_string, normalize_legacy_tokens, render_template};
use crate::config::Changelog;
use crate::conventional::ConventionalCommit;

#[test]
fn test_render_template_basic() {
    let commits = vec![
        ConventionalCommit::parse("feat(ui): cool button").unwrap(),
        ConventionalCommit::parse("fix: crash on start").unwrap(),
    ];
    let ctx = build_context(
        "1.0.0",
        None,
        "v1.0.0",
        None,
        "2026-03-30",
        None,
        &commits,
        vec!["Alice".into()],
        true,
        "Maintenance and updates.",
    );

    let tpl = "Release {{ version }} ({{ date }}):\n{{ features }}\n{{ fixes }}";
    let res = render_template(tpl, &ctx).unwrap();
    assert_eq!(
        res,
        "Release 1.0.0 (2026-03-30):\n- **ui**: cool button\n- crash on start"
    );
}

#[test]
fn test_render_template_no_html_escape() {
    let commits = vec![ConventionalCommit::parse("feat: add <script> & *markdown* tags").unwrap()];
    let ctx = build_context(
        "1.0.0",
        None,
        "v1.0.0",
        None,
        "2026-03-30",
        Some("https://github.com/Row0902/cutver".into()),
        &commits,
        vec![],
        true,
        "Maintenance and updates.",
    );

    let tpl = "Changes:\n{{ features }}\nCompare: <{{ repository }}>";
    let res = render_template(tpl, &ctx).unwrap();
    // Crucial invariant: characters must not be HTML escaped (< > &)
    assert!(res.contains("<script>"));
    assert!(res.contains('&'));
    assert!(res.contains("<https://github.com/Row0902/cutver>"));
}

#[test]
fn test_render_body_template_mode() {
    let config = Changelog {
        mode: "template".into(),
        entry_template: "Static release notes template.\n".into(),
        ..Default::default()
    };
    let commits = vec![ConventionalCommit::parse("feat: something").unwrap()];
    let body = render_body(&config, &commits, None);
    assert_eq!(body, "Static release notes template.");
}

#[test]
fn test_render_body_inline_template() {
    let config = Changelog {
        template: Some("Version {{ version }}:\n{{ features }}".into()),
        ..Default::default()
    };
    let commits = vec![ConventionalCommit::parse("feat: new cool thing").unwrap()];
    let ctx = build_context(
        "1.0.0",
        None,
        "v1.0.0",
        None,
        "2026-03-30",
        None,
        &commits,
        vec![],
        true,
        "Maintenance and updates.",
    );
    let body = render_body(&config, &commits, Some(&ctx));
    assert!(body.starts_with("Version 1.0.0"));
    assert!(body.contains("- new cool thing"));
}

#[test]
fn test_render_body_with_context_helper() {
    let config = Changelog {
        template: Some("Version {{ version }}:\n{{ features }}".into()),
        ..Default::default()
    };
    let commits = vec![ConventionalCommit::parse("feat: helper feature").unwrap()];
    let ctx = build_context(
        "2.5.0",
        None,
        "v2.5.0",
        None,
        "2026-03-30",
        None,
        &commits,
        vec![],
        true,
        "Maintenance and updates.",
    );
    let body = render_body_with_context(&config, &commits, &ctx);
    assert!(body.starts_with("Version 2.5.0"));
    assert!(body.contains("- helper feature"));
}

#[test]
fn test_render_body_conventional_groups() {
    let config = Changelog::default();
    let commits = vec![
        ConventionalCommit::parse("feat: new feature").unwrap(),
        ConventionalCommit::parse("fix: bug fix").unwrap(),
        ConventionalCommit::parse("perf: speedup").unwrap(),
        ConventionalCommit::parse("refactor: cleanup").unwrap(),
        ConventionalCommit::parse("docs: update guide").unwrap(),
        ConventionalCommit::parse("chore: bump deps").unwrap(),
        ConventionalCommit::parse("custom: something else").unwrap(),
        ConventionalCommit::parse("feat!: breaking change").unwrap(),
    ];
    let body = render_body(&config, &commits, None);
    let expected = "\
### ⚠️ Breaking Changes
- breaking change

### Features
- new feature

### Bug Fixes
- bug fix

### Performance Improvements
- speedup

### Refactoring
- cleanup

### Documentation
- update guide

### Maintenance
- bump deps

### Other Changes
- something else";
    assert_eq!(body, expected);
}

#[test]
fn test_render_body_scopes() {
    let commits = vec![
        ConventionalCommit::parse("feat(core): parser rewrite").unwrap(),
        ConventionalCommit::parse("fix: general fix").unwrap(),
    ];

    let config_with_scopes = Changelog {
        include_scopes: true,
        ..Default::default()
    };
    let body_with_scopes = render_body(&config_with_scopes, &commits, None);
    assert!(body_with_scopes.contains("- **core**: parser rewrite"));
    assert!(body_with_scopes.contains("- general fix"));

    let config_without_scopes = Changelog {
        include_scopes: false,
        ..Default::default()
    };
    let body_without_scopes = render_body(&config_without_scopes, &commits, None);
    assert!(body_without_scopes.contains("- parser rewrite"));
    assert!(!body_without_scopes.contains("**core**"));
}

#[test]
fn test_render_body_fallback_when_empty() {
    let config = Changelog {
        fallback_entry: "Custom fallback notes.".into(),
        ..Default::default()
    };
    let body = render_body(&config, &[], None);
    assert_eq!(body, "- Custom fallback notes.");
}

#[test]
fn test_render_template_with_semver_and_forge() {
    let ctx = build_context(
        "3.14.15-rc.1+exp.sha.5114f85",
        None,
        "v3.14.15-rc.1+exp.sha.5114f85",
        None,
        "2026-03-30",
        Some("https://gitlab.com/awesome-org/subgroup/super-tool".to_string()),
        &[],
        vec![],
        true,
        "None",
    );

    // Test semver components, aliases owner/repo and repo_owner/repo_name, forge
    let tpl = "Release: {{ major }}.{{ minor }}.{{ patch }} (pre: {{ prerelease }}, build: {{ build }}, is_pre: {{ is_prerelease }})\n\
               Forge: {{ forge }}, Owner: {{ owner }} ({{ repo_owner }}), Repo: {{ repo }} ({{ repo_name }})";

    let rendered = render_template(tpl, &ctx).unwrap();
    let expected = "Release: 3.14.15 (pre: rc.1, build: exp.sha.5114f85, is_pre: True)\n\
                    Forge: gitlab, Owner: awesome-org/subgroup (awesome-org/subgroup), Repo: super-tool (super-tool)";
    assert_eq!(rendered, expected);
}

#[test]
fn test_normalize_legacy_tokens() {
    assert_eq!(
        normalize_legacy_tokens("chore(release): v{version}"),
        "chore(release): v{{ version }}"
    );
    assert_eq!(
        normalize_legacy_tokens("release {tag} ({version})"),
        "release {{ tag }} ({{ version }})"
    );
    assert_eq!(
        normalize_legacy_tokens("chore(release): v{{ version }}"),
        "chore(release): v{{ version }}"
    );
    assert_eq!(
        normalize_legacy_tokens("{other} stay intact: {version}"),
        "{other} stay intact: {{ version }}"
    );
}

#[test]
fn test_interpolate_string_with_legacy_and_minijinja() {
    let ctx = InterpolationContext::build(
        "1.2.3-rc.1",
        Some("1.2.2"),
        "v1.2.3-rc.1",
        Some("v1.2.2"),
        "minor",
        "2026-03-30",
        Some("main"),
        Some("https://github.com/my-org/my-repo"),
    );

    // Legacy single brace
    let legacy = "chore: v{version} tag {tag}";
    assert_eq!(
        interpolate_string(legacy, &ctx).unwrap(),
        "chore: v1.2.3-rc.1 tag v1.2.3-rc.1"
    );

    // Modern MiniJinja variables, forge, semver parts
    let modern = "release: {{ tag }} (prev: {{ previous_tag }}) [{{ repo }} by {{ owner }}] on {{ branch }} - bump: {{ bump_level }} - major: {{ major }}, minor: {{ minor }}, patch: {{ patch }}{% if is_prerelease %} (pre: {{ prerelease }}){% endif %}";
    assert_eq!(
        interpolate_string(modern, &ctx).unwrap(),
        "release: v1.2.3-rc.1 (prev: v1.2.2) [my-repo by my-org] on main - bump: minor - major: 1, minor: 2, patch: 3 (pre: rc.1)"
    );

    // Fails fast on malformed template syntax without swallowing error
    let malformed = "release: {{ tag [unclosed";
    assert!(interpolate_string(malformed, &ctx).is_err());
}
