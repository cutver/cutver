use super::*;
use crate::bump::Touched;
use semver::Version;

fn dummy_summary() -> Summary {
    Summary {
        source: "Cargo.toml".into(),
        current: Version::new(0, 8, 0),
        next: Version::new(0, 9, 0),
        dry_run: true,
        preflight: vec![crate::preflight::Step {
            name: "tests".into(),
            command: "cargo test".into(),
            timeout: None,
            skipped: false,
        }],
        touched: vec![Touched {
            path: "Cargo.toml".into(),
            old: "0.8.0".into(),
            new: "0.9.0".into(),
        }],
        changelog: Some("CHANGELOG.md".into()),
        commit_message: "chore(release): v0.9.0 [skip ci]".into(),
        commit: "abc1234".into(),
        tag: "v0.9.0".into(),
        tag_skipped: false,
        floating_tag: None,
        pre_bump: None,
        post_bump: None,
        warnings: Vec::new(),
        publish_push: true,
        publish_push_command: Some("git push origin main --tags".into()),
        publish_commands: vec!["cargo publish".into()],
        deferred: false,
        rationale: None,
        root_dir: None,
    }
}

#[test]
fn test_simulation_banner_formatting() {
    let theme = Theme::new(false);
    let banner = render_simulation_banner(&theme);
    assert!(banner.contains("┌─ SIMULATION MODE ─"));
    assert!(banner.contains("│ No files, commits, or git tags will be modified."));
    assert!(banner.ends_with("┘"));
}

#[test]
fn test_tree_rendering_structure() {
    let theme = Theme::new(false);
    let summary = dummy_summary();
    let plan = render_release_plan(&summary, &theme);

    assert!(plan.contains("Release Plan: 0.8.0 -> 0.9.0"));
    assert!(plan.contains("├── Manifests"));
    assert!(plan.contains("│   └── Cargo.toml (0.8.0 -> 0.9.0)"));
    assert!(plan.contains("├── Preflight"));
    assert!(plan.contains("│   └── tests: cargo test"));
    assert!(plan.contains("├── Changelog"));
    assert!(plan.contains("│   └── CHANGELOG.md (prepends 0.9.0)"));
    assert!(plan.contains("├── Git"));
    assert!(plan.contains("│   ├── commit: chore(release): v0.9.0 [skip ci]"));
    assert!(plan.contains("│   └── tag: v0.9.0"));
    assert!(plan.contains("└── Publish"));
    assert!(plan.contains("    ├── push: git push origin main --tags"));
    assert!(plan.contains("    └── commands:"));
    assert!(plan.contains("        └── cargo publish"));
}

/// #326 D1a: a run that released nothing says so, and marks the commit and the tag it did not
/// create, instead of rendering a release that never happened.
#[test]
fn test_a_no_op_release_says_so_and_marks_what_it_skipped() {
    let theme = Theme::new(false);
    let mut summary = dummy_summary();
    summary.current = Version::new(0, 9, 0);
    summary.touched = Vec::new();
    summary.changelog = None;
    summary.commit = String::new();
    summary.tag_skipped = true;
    summary.publish_push = false;
    summary.publish_push_command = None;
    summary.publish_commands = Vec::new();

    let plan = render_release_plan(&summary, &theme);

    assert!(plan.contains("Release Plan: 0.9.0 -> 0.9.0"), "{plan}");
    assert!(plan.contains("├── No-op"), "the tree says nothing was released: {plan}");
    assert!(plan.contains("nothing was written, committed or tagged"), "{plan}");
    assert!(
        plan.contains("commit: chore(release): v0.9.0 [skip ci] [SKIPPED]"),
        "the commit it did not create is marked: {plan}"
    );
    assert!(
        plan.contains("tag: v0.9.0 [SKIPPED]"),
        "the tag it did not create is marked: {plan}"
    );
}

/// A first release tags the version the manifests already declare, so its summary also carries
/// `current == next` — and it is a release, not a no-op (#326 D1a).
#[test]
fn test_a_release_that_does_not_move_the_version_is_not_a_no_op() {
    let theme = Theme::new(false);
    let mut summary = dummy_summary();
    summary.current = summary.next.clone();

    let plan = render_release_plan(&summary, &theme);

    assert!(
        !plan.contains("No-op"),
        "a release that only tags the declared version is a release: {plan}"
    );
    assert!(plan.contains("tag: v0.9.0"), "{plan}");
}

#[test]
fn test_relativize_path_nested() {
    let root = std::path::Path::new("/workspace/project");
    assert_eq!(
        relativize_path("/workspace/project/crates/app/Cargo.toml", Some(root)),
        "crates/app/Cargo.toml"
    );
}

#[test]
fn test_relativize_path_root() {
    let root = std::path::Path::new("/workspace/project");
    assert_eq!(
        relativize_path("/workspace/project/Cargo.toml", Some(root)),
        "Cargo.toml"
    );
}

#[test]
fn test_relativize_path_outside() {
    let root = std::path::Path::new("/workspace/project");
    assert_eq!(
        relativize_path("/other/location/Cargo.toml", Some(root)),
        "/other/location/Cargo.toml"
    );
    assert_eq!(relativize_path("Cargo.toml", None), "Cargo.toml");
}

#[test]
fn test_relativize_path_backslash_normalization() {
    let root = std::path::Path::new("C:\\repo");
    assert_eq!(
        relativize_path("C:\\repo\\src\\Cargo.toml", Some(root)),
        "src/Cargo.toml"
    );
    let root_forward = std::path::Path::new("C:/repo");
    assert_eq!(
        relativize_path("C:\\repo\\src\\Cargo.toml", Some(root_forward)),
        "src/Cargo.toml"
    );
}

/// The no-op reaches no push and runs no command, so its tree must not advertise them: a plan that
/// says `push: true` for a push it never makes is the same lie as a report that says it released.
#[test]
fn test_a_no_op_does_not_advertise_a_push_it_never_makes() {
    let theme = Theme::new(false);
    let mut summary = dummy_summary();
    summary.current = Version::new(0, 9, 0);
    summary.touched = Vec::new();
    summary.changelog = None;
    summary.commit = String::new();
    summary.tag_skipped = true;
    summary.publish_push = true;
    summary.publish_push_command = None;
    summary.publish_commands = vec!["cargo publish".into()];

    let plan = render_release_plan(&summary, &theme);

    assert!(
        plan.contains("push: true [SKIPPED]"),
        "the push it did not make is marked: {plan}"
    );
    assert!(
        plan.contains("cargo publish [SKIPPED]"),
        "and so is every command it did not run: {plan}"
    );
}
