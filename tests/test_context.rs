use cutver::changelog::build_context;
use cutver::changelog::render_template;

#[test]
fn test_semver_breakdown_and_forge_in_template() {
    let ctx = build_context(
        "1.2.3-beta.1+build.42",
        None,
        "v1.2.3-beta.1+build.42",
        None,
        "2026-03-30",
        Some("https://github.com/my-org/my-repo".to_string()),
        &[],
        vec![],
        true,
        "None",
    );

    let template = "v{{ major }}.{{ minor }}.{{ patch }} [prerelease: {{ is_prerelease }}, pre: {{ prerelease }}, build: {{ build }}] on {{ forge }} by {{ owner }}/{{ repo }} (alt: {{ repo_owner }}/{{ repo_name }})";
    let rendered = render_template(template, &ctx).expect("render should succeed");
    assert_eq!(
        rendered,
        "v1.2.3 [prerelease: True, pre: beta.1, build: build.42] on github by my-org/my-repo (alt: my-org/my-repo)"
    );
}
