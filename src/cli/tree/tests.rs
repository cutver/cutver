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
