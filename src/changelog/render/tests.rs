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
    let body = render_body(&config, &commits, None).unwrap();
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
    let body = render_body(&config, &commits, Some(&ctx)).unwrap();
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
    let body = render_body_with_context(&config, &commits, &ctx).unwrap();
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
    let body = render_body(&config, &commits, None).unwrap();
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
    let body_with_scopes = render_body(&config_with_scopes, &commits, None).unwrap();
    assert!(body_with_scopes.contains("- **core**: parser rewrite"));
    assert!(body_with_scopes.contains("- general fix"));

    let config_without_scopes = Changelog {
        include_scopes: false,
        ..Default::default()
    };
    let body_without_scopes = render_body(&config_without_scopes, &commits, None).unwrap();
    assert!(body_without_scopes.contains("- parser rewrite"));
    assert!(!body_without_scopes.contains("**core**"));
}

#[test]
fn test_render_body_fallback_when_empty() {
    let config = Changelog {
        fallback_entry: "Custom fallback notes.".into(),
        ..Default::default()
    };
    let body = render_body(&config, &[], None).unwrap();
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

#[test]
fn test_minijinja_filters_and_env_global() {
    unsafe {
        std::env::set_var("CUTVER_TEST_ENV_VAR", "cutver_awesome_val");
    }

    let commits = vec![
        ConventionalCommit::parse("feat(api): add endpoint\n\nBody details.").unwrap(),
        ConventionalCommit::parse("fix: patch bug").unwrap(),
        ConventionalCommit::parse("feat(core): core engine\n\nBREAKING CHANGE: broke api").unwrap(),
        ConventionalCommit::parse("fix(api): fix endpoint typo").unwrap(),
    ];

    let ctx = build_context(
        "2.0.0",
        None,
        "v2.0.0",
        None,
        "2026-04-01",
        None,
        &commits,
        vec![],
        true,
        "fallback",
    );

    let tpl = r#"
Env: {{ env("CUTVER_TEST_ENV_VAR") }}
Date: {{ year }}-{{ month }}-{{ day }}
{% for scope, scoped_commits in commits | group_by_scope %}
Scope: [{{ scope }}]
{% for c in scoped_commits %}
  - {{ c.type }}: {{ c.description }} | body: {{ c.body }} | breaking: {{ c.breaking_description }}
{% endfor %}
{% endfor %}
{% for type, typed_commits in commits | group_by_type %}
Type: [{{ type }}]
{% for c in typed_commits %}
  - {{ c.description }}
{% endfor %}
{% endfor %}
"#;

    let res = render_template(tpl, &ctx).unwrap();
    assert!(res.contains("Env: cutver_awesome_val"));
    assert!(res.contains("Date: 2026-4-1"));

    // Scopes should be ordered: "api", "core", then "" (other)
    let pos_api = res.find("Scope: [api]").unwrap();
    let pos_core = res.find("Scope: [core]").unwrap();
    let pos_empty = res.find("Scope: []").unwrap();
    assert!(pos_api < pos_core);
    assert!(pos_core < pos_empty);

    // Check body and breaking_description in rendered output
    assert!(res.contains("body: Body details."));
    assert!(res.contains("breaking: broke api"));

    // Types ordered as first seen: feat, then fix
    let pos_feat = res.find("Type: [feat]").unwrap();
    let pos_fix = res.find("Type: [fix]").unwrap();
    assert!(pos_feat < pos_fix);
}

#[derive(Debug)]
struct MockChangelogDriver {
    name: crate::plugin::types::PluginName,
    output_body: String,
}

impl crate::plugin::driver::PluginDriver for MockChangelogDriver {
    fn name(&self) -> &crate::plugin::types::PluginName {
        &self.name
    }

    fn invoke(&self, capability: &str, payload: &[u8]) -> Result<Vec<u8>, crate::plugin::PluginError> {
        assert_eq!(capability, "changelog.v1");
        let req: crate::plugin::dto::ChangelogRenderRequest =
            serde_json::from_slice(payload).expect("valid changelog request payload");
        assert_eq!(req.version, "1.0.0");
        let res = crate::plugin::dto::ChangelogRenderResponse {
            body: self.output_body.clone(),
        };
        Ok(serde_json::to_vec(&res).unwrap())
    }
}

#[test]
fn test_render_body_with_plugin_mock() {
    use crate::changelog::render::body::render_body_with_plugin;
    use crate::plugin::PluginManager;
    use crate::plugin::types::{Capability, PluginConfig, PluginName, RuntimeKind};
    use std::collections::HashMap;

    let p_name = PluginName::new("mock-formatter").unwrap();
    let driver = Box::new(MockChangelogDriver {
        name: p_name.clone(),
        output_body: "### Custom Notes\n* Handled by mock plugin".into(),
    });

    let config_plugin = PluginConfig {
        runtime: RuntimeKind::Process,
        source: None,
        hash: None,
        command: Some("mock".into()),
        capabilities: vec![Capability::ChangelogV1],
        events: vec![],
        manifest_match: vec![],
        permissions: Default::default(),
        timeout_seconds: None,
    };

    let mut drivers = HashMap::new();
    drivers.insert(p_name.clone(), driver as Box<dyn crate::plugin::driver::PluginDriver>);
    let mut configs = HashMap::new();
    configs.insert(p_name.clone(), config_plugin);

    let manager = PluginManager::new(drivers, configs);

    let changelog_cfg = Changelog {
        format: "plugin".into(),
        plugin: Some("mock-formatter".into()),
        ..Default::default()
    };

    let commits = vec![ConventionalCommit::parse("feat: add mock feature").unwrap()];
    let ctx = build_context(
        "1.0.0",
        None,
        "v1.0.0",
        None,
        "2026-04-10",
        None,
        &commits,
        vec![],
        true,
        "fallback",
    );

    // 1. Explicit plugin name
    let body = render_body_with_plugin(&changelog_cfg, &commits, Some(&ctx), Some(&manager)).unwrap();
    assert_eq!(body, "### Custom Notes\n* Handled by mock plugin");

    // 2. Automatic plugin resolution
    let changelog_auto_cfg = Changelog {
        format: "plugin".into(),
        plugin: None,
        ..Default::default()
    };
    let body_auto = render_body_with_plugin(&changelog_auto_cfg, &commits, Some(&ctx), Some(&manager)).unwrap();
    assert_eq!(body_auto, "### Custom Notes\n* Handled by mock plugin");

    // 3. Missing plugin manager returns Error::PluginManagerRequired
    let err = render_body_with_plugin(&changelog_cfg, &commits, Some(&ctx), None).unwrap_err();
    assert!(matches!(err, crate::changelog::Error::PluginManagerRequired));
}
