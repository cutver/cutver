//! Unit tests for the plan document, its serialized order and the phase list. The end-to-end
//! behaviour of `cutver plan` lives in `tests/e2e_plan.rs`.

use super::*;

/// The document one read-only plan renders, with the parts a unit test cannot reach (git, the
/// changelog) filled in by hand.
fn document() -> ReleasePlan {
    ReleasePlan {
        read_only: true,
        level: "minor".into(),
        current_version: Version::new(0, 19, 0),
        next_version: Version::new(0, 20, 0),
        tag: "v0.20.0".into(),
        floating_tag: Some("v0".into()),
        floating_tag_skipped: None,
        commit: "abc1234".into(),
        branch: Some("main".into()),
        commit_message: "chore(release): v0.20.0".into(),
        notes: "## [0.20.0] - 2026-11-18\n\n### Features\n".into(),
        changelog: Some(PlanChangelog {
            path: "CHANGELOG.md".into(),
            prepends: "0.20.0".into(),
        }),
        manifests: vec![PlanManifest {
            path: "Cargo.toml".into(),
            from: "0.19.0".into(),
            to: "0.20.0".into(),
        }],
        preflight: vec![
            preflight::Step {
                name: "test".into(),
                command: "cargo test".into(),
                timeout: Some(1200),
                skipped: false,
            },
            preflight::Step {
                name: "lint".into(),
                command: "cargo clippy".into(),
                timeout: None,
                skipped: true,
            },
        ],
        phases: vec![PlanPhase {
            name: "validate".into(),
            mutates: false,
            steps: vec!["tag 'v0.20.0' is a valid git ref".into()],
        }],
        artifacts: Vec::new(),
        gates: Vec::new(),
        blockers: vec!["release tag already exists locally (points at abc1234); delete it".into()],
        source: "Cargo.toml".into(),
        root_dir: PathBuf::from("/workspace"),
        rationale: None,
        interp_ctx: InterpolationContext::build(
            "0.20.0",
            Some("0.19.0"),
            "v0.20.0",
            None,
            "minor",
            "2026-11-18",
            Some("main"),
            None,
        ),
        auto_commits: None,
        changes: Vec::new(),
        changelog_plan: None,
        publish_push: true,
        publish_commands: vec!["cargo publish".into()],
        first_release: false,
        warnings: Vec::new(),
        plugin_manager: None,
    }
}

#[test]
fn the_document_carries_exactly_the_documented_keys_in_order() {
    // The order is read from the rendered bytes, not from a `Value`: `serde_json::Value` sorts
    // its object's keys, so it cannot witness an order at all.
    let json = serde_json::to_string(&document()).expect("the plan serializes");
    const KEYS: [&str; 17] = [
        "read_only",
        "level",
        "current_version",
        "next_version",
        "tag",
        "floating_tag",
        "commit",
        "branch",
        "commit_message",
        "notes",
        "changelog",
        "manifests",
        "preflight",
        "phases",
        "artifacts",
        "gates",
        "blockers",
    ];
    let mut positions: Vec<(usize, &str)> = KEYS
        .iter()
        .map(|key| {
            let at = json
                .find(&format!("\"{key}\":"))
                .unwrap_or_else(|| panic!("the document carries `{key}`: {json}"));
            (at, *key)
        })
        .collect();
    positions.sort();
    assert_eq!(
        positions.into_iter().map(|(_, key)| key).collect::<Vec<_>>(),
        KEYS.to_vec(),
        "{json}"
    );
    assert!(json.starts_with('{'), "{json}");
}

#[test]
fn versions_are_strings_and_preflight_entries_carry_no_host_budget() {
    let json = serde_json::to_value(document()).expect("the plan serializes");

    assert_eq!(json["current_version"], "0.19.0");
    assert_eq!(json["next_version"], "0.20.0");
    assert_eq!(json["read_only"], true);
    assert_eq!(
        json["preflight"],
        serde_json::json!([
            { "name": "test", "command": "cargo test", "skipped": false },
            { "name": "lint", "command": "cargo clippy", "skipped": true },
        ])
    );
}

#[test]
fn artifacts_and_gates_are_empty_arrays_a_consumer_can_code_against() {
    let json = serde_json::to_value(document()).expect("the plan serializes");

    assert_eq!(json["artifacts"], serde_json::json!([]));
    assert_eq!(json["gates"], serde_json::json!([]));
}

#[test]
fn the_wrapping_object_puts_the_schema_version_first() {
    let plan = document();
    let json = serde_json::to_string(&crate::cli::json::Versioned::new(&plan)).expect("the plan serializes");

    assert!(
        json.starts_with(r#"{"schemaVersion":1,"read_only":true,"level":"minor","#),
        "{json}"
    );
}

#[test]
fn the_phases_are_derived_from_the_computation_in_release_order() {
    let plan = document();
    let config = Config {
        git: crate::config::Git {
            require_clean_tree: true,
            require_branch: Some("main".into()),
            floating_major_tag: true,
            ..Default::default()
        },
        publish: crate::config::Publish {
            push: true,
            ..Default::default()
        },
        preflight: Vec::new(),
        ..Default::default()
    };

    let phases = plan.derive_phases(&config).expect("the phases render");

    let names: Vec<&str> = phases.iter().map(|phase| phase.name.as_str()).collect();
    assert_eq!(names, vec!["validate", "mutate", "publish"]);
    assert_eq!(
        phases.iter().map(|phase| phase.mutates).collect::<Vec<_>>(),
        vec![false, true, true]
    );
    assert!(
        phases[0]
            .steps
            .contains(&"release tag 'v0.20.0' is free locally and on 'origin'".to_string()),
        "the collision check is derived, not hand-written: {:?}",
        phases[0].steps
    );
    assert!(
        phases[0].steps.contains(&"preflight test: cargo test".to_string()),
        "the preflight steps come from the plan: {:?}",
        phases[0].steps
    );
    assert!(
        phases[0]
            .steps
            .contains(&"preflight lint: cargo clippy (skipped)".to_string()),
        "a skipped step says so: {:?}",
        phases[0].steps
    );
    assert!(
        phases[1]
            .steps
            .contains(&"manifest Cargo.toml: 0.19.0 -> 0.20.0".to_string()),
        "the manifests come from the computed changes: {:?}",
        phases[1].steps
    );
    assert!(
        phases[1]
            .steps
            .contains(&"changelog CHANGELOG.md: prepend 0.20.0".to_string()),
        "the changelog entry comes from the plan: {:?}",
        phases[1].steps
    );
    assert!(
        phases[2].steps.contains(&"tag v0.20.0 (annotated)".to_string()),
        "the tag this run would create: {:?}",
        phases[2].steps
    );
    assert!(
        phases[2].steps.contains(&"force-push tag 'v0'".to_string()),
        "the floating tag comes from the plan: {:?}",
        phases[2].steps
    );
    assert!(
        phases[2].steps.contains(&"publish: cargo publish".to_string()),
        "the rendered publish commands: {:?}",
        phases[2].steps
    );
}

#[test]
fn a_blocker_is_one_line() {
    let err: git::Error = git::Error::Guard {
        detail: "working tree has uncommitted changes:\n M src/bump/plan.rs\n?? notes.md".into(),
    };

    let blocker = dirty_tree_blocker(&err);

    assert!(!blocker.contains('\n'), "a blocker is one line: {blocker:?}");
    assert!(blocker.contains("require_clean_tree = false"), "{blocker}");
    assert!(blocker.contains("src/bump/plan.rs"), "{blocker}");
}

#[test]
fn the_two_shared_guards_are_the_ones_the_pipeline_aborts_on() {
    // The same functions, so the only difference between a plan and a bump is how the result is
    // interpreted: `git::require_clean_tree`/`require_branch` for a bump, `blockers` for a plan.
    let config = Config {
        git: crate::config::Git {
            require_clean_tree: false,
            require_branch: None,
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(git::require_clean_tree(&config.root_dir, config.git.require_clean_tree).is_ok());
    assert!(git::require_branch(&config.root_dir, config.git.require_branch.as_deref()).is_ok());
}

#[test]
fn a_plugin_driven_configuration_is_refused_instead_of_dispatched() {
    let config = Config {
        version: crate::config::VersionSection {
            current_source: "Cargo.toml".into(),
            strategy: "plugin".into(),
            plugin: None,
        },
        ..Default::default()
    };

    let err = reject_plugin_dependencies(&config).expect_err("a plugin strategy cannot be planned");

    let text = err.to_string();
    assert!(text.contains("strategy = \"plugin\""), "{text}");
    assert!(text.contains("cutver bump --dry-run"), "{text}");
}

/// T6 and ADR-0005's decision: a prerelease publishes exactly its own version, so the floating major
/// tag is stable-only. The decision is one function because `src/bump/exec/publish.rs` answers it again
/// for `cutver publish` — a separate process that never reads a plan — from the version it publishes.
///
/// The skip is carried, not dropped: the same value holds the line the tree prints, so a plan can never
/// report a skip it did not decide (#326 T6).
#[test]
fn the_floating_major_tag_is_stable_only() {
    use super::resolve::floating_tag;

    let config = Config {
        git: crate::config::Git {
            floating_major_tag: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let version = |text: &str| text.parse::<Version>().expect("a semver version");

    for prerelease in ["0.22.0-alpha.1", "0.22.0-beta.2", "0.22.0-rc.1", "0.22.0-nightly.3"] {
        let floating = floating_tag(&config, &version(prerelease), false);
        assert_eq!(
            floating.tag, None,
            "{prerelease} is a prerelease: `v0` must not follow it"
        );
        assert_eq!(
            floating.skipped.as_deref(),
            Some(format!(
                "the floating major tag 'v0' is not moved: {prerelease} is a prerelease, and that tag is stable-only (ADR-0005)"
            ).as_str()),
            "the skip is reported with the tag, the version and the decision behind it"
        );
    }

    let stable = floating_tag(&config, &version("0.22.0"), false);
    assert_eq!(
        stable.tag,
        Some("v0".into()),
        "a stable release still moves the floating major tag"
    );
    assert_eq!(stable.skipped, None, "a stable release has nothing to explain");
}

/// The two reasons the plan reported no floating tag before T6, unchanged — and, crucially, still
/// unremarked: a no-op or a configuration that never asked for the tag is not a skip to explain.
#[test]
fn a_no_op_or_an_unconfigured_release_still_names_no_floating_tag() {
    use super::resolve::floating_tag;

    let armed = Config {
        git: crate::config::Git {
            floating_major_tag: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let disarmed = Config::default();

    for (floating, why) in [
        (
            floating_tag(&armed, &Version::new(0, 22, 0), true),
            "a no-op releases nothing",
        ),
        (
            floating_tag(&disarmed, &Version::new(0, 22, 0), false),
            "`floating_major_tag = false` asks for no tag",
        ),
        (
            floating_tag(
                &disarmed,
                &Version::parse("0.22.0-alpha.1").expect("a semver version"),
                false,
            ),
            "`floating_major_tag = false` asks for no tag on any channel",
        ),
    ] {
        assert_eq!(floating.tag, None, "{why}, so no tag is moved");
        assert_eq!(
            floating.skipped, None,
            "{why}, so there is no deliberate skip to report"
        );
    }
}
