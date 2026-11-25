use super::commands::{BumpLevel, ChangelogCommands, Cli, Commands, Conclusion};
use super::normalize::normalize_args;
use super::plugin::PluginCommands;
use clap::Parser;
use std::path::PathBuf;

mod plugin_args;

#[test]
fn bump_patch_defaults() {
    let cli = Cli::try_parse_from(["cutver", "bump", "patch"]).unwrap();
    let Commands::Bump {
        level,
        dry_run,
        skip_preflight,
        first_release,
        defer_publish,
        json,
        report,
        ..
    } = cli.command
    else {
        panic!("expected bump")
    };
    assert_eq!(level, BumpLevel::Patch);
    assert!(!dry_run);
    assert!(skip_preflight.is_empty());
    assert!(!first_release);
    assert!(!defer_publish);
    assert!(!json);
    assert!(report.is_none());

    assert!(cli.config.is_none());
}

#[test]
fn bump_minor_dry_run_and_skip_preflight() {
    let cli = Cli::try_parse_from([
        "cutver",
        "bump",
        "minor",
        "--dry-run",
        "--skip-preflight",
        "tests",
        "--skip-preflight",
        "build",
    ])
    .unwrap();
    let Commands::Bump {
        level,
        dry_run,
        skip_preflight,
        first_release,
        defer_publish,
        json,
        report,
        ..
    } = cli.command
    else {
        panic!("expected bump")
    };
    assert_eq!(level, BumpLevel::Minor);
    assert!(dry_run);
    assert_eq!(skip_preflight, vec!["tests", "build"]);
    assert!(!first_release);
    assert!(!defer_publish);
    assert!(!json);
    assert!(report.is_none());
}

#[test]
fn bump_auto_cli() {
    let cli = Cli::try_parse_from(["cutver", "bump", "auto"]).unwrap();
    let Commands::Bump {
        level,
        dry_run,
        skip_preflight,
        first_release,
        defer_publish,
        json,
        report,
        ..
    } = cli.command
    else {
        panic!("expected bump")
    };
    assert_eq!(level, BumpLevel::Auto);
    assert!(!dry_run);
    assert!(skip_preflight.is_empty());
    assert!(!first_release);
    assert!(!defer_publish);
    assert!(!json);
    assert!(report.is_none());
}

#[test]
fn bump_defer_publish_parses_and_defaults_to_false() {
    let cli = Cli::try_parse_from(["cutver", "bump", "auto"]).unwrap();
    let Commands::Bump { defer_publish, .. } = cli.command else {
        panic!("expected bump")
    };
    assert!(!defer_publish);

    let cli = Cli::try_parse_from(["cutver", "bump", "patch", "--defer-publish"]).unwrap();
    let Commands::Bump {
        level, defer_publish, ..
    } = cli.command
    else {
        panic!("expected bump")
    };
    assert_eq!(level, BumpLevel::Patch);
    assert!(defer_publish);
}

#[test]
fn bump_json_flag_parses_and_defaults_to_false() {
    let cli = Cli::try_parse_from(["cutver", "bump", "patch"]).unwrap();
    let Commands::Bump { json, .. } = cli.command else {
        panic!("expected bump")
    };
    assert!(!json);

    let cli = Cli::try_parse_from(["cutver", "bump", "patch", "--json"]).unwrap();
    let Commands::Bump { json, .. } = cli.command else {
        panic!("expected bump")
    };
    assert!(json);
}

#[test]
fn publish_parses_its_arguments_and_requires_the_tag() {
    let cli = Cli::try_parse_from(["cutver", "publish", "--tag", "v1.3.0"]).unwrap();
    let Commands::Publish {
        tag,
        commit,
        notes_file,
        require_gate,
        dry_run,
        ..
    } = cli.command
    else {
        panic!("expected publish")
    };
    assert_eq!(tag, "v1.3.0");
    assert!(commit.is_none());
    assert!(notes_file.is_none());
    assert!(require_gate.is_empty(), "no gate is required by default");
    assert!(!dry_run);

    let cli = Cli::try_parse_from([
        "cutver",
        "publish",
        "--tag",
        "v1.3.0",
        "--commit",
        "abc1234",
        "--notes-file",
        "notes.md",
        "--require-gate",
        "ci",
        "--require-gate",
        "security",
        "--dry-run",
    ])
    .unwrap();
    let Commands::Publish {
        tag,
        commit,
        notes_file,
        require_gate,
        dry_run,
        ..
    } = cli.command
    else {
        panic!("expected publish")
    };
    assert_eq!(tag, "v1.3.0");
    assert_eq!(commit.as_deref(), Some("abc1234"));
    assert_eq!(notes_file, Some(PathBuf::from("notes.md")));
    assert_eq!(
        require_gate,
        vec!["ci".to_string(), "security".to_string()],
        "`--require-gate` repeats and keeps its order"
    );
    assert!(dry_run);

    assert!(
        Cli::try_parse_from(["cutver", "publish"]).is_err(),
        "the tag is required"
    );
}

#[test]
fn bump_report_parses_its_path_and_is_independent_of_json() {
    let cli = Cli::try_parse_from(["cutver", "bump", "patch"]).unwrap();
    let Commands::Bump { report, .. } = cli.command else {
        panic!("expected bump")
    };
    assert!(report.is_none(), "no report file is written unless asked");

    let cli = Cli::try_parse_from(["cutver", "bump", "patch", "--report", "out/release.json"]).unwrap();
    let Commands::Bump { report, json, .. } = cli.command else {
        panic!("expected bump")
    };
    assert_eq!(report, Some(PathBuf::from("out/release.json")));
    assert!(!json, "--report must not imply --json");

    let cli = Cli::try_parse_from(["cutver", "bump", "patch", "--report", "r.json", "--json"]).unwrap();
    let Commands::Bump { report, json, .. } = cli.command else {
        panic!("expected bump")
    };
    assert_eq!(report, Some(PathBuf::from("r.json")));
    assert!(json, "both may be passed together");
}

#[test]
fn bump_auto_dry_run() {
    let cli = Cli::try_parse_from(["cutver", "bump", "auto", "--dry-run"]).unwrap();
    let Commands::Bump { level, dry_run, .. } = cli.command else {
        panic!("expected bump")
    };
    assert_eq!(level, BumpLevel::Auto);
    assert!(dry_run);
}

#[test]
fn plan_defaults_to_auto_and_the_plan_tree() {
    let cli = Cli::try_parse_from(["cutver", "plan"]).unwrap();
    let Commands::Plan {
        level,
        json,
        skip_preflight,
        first_release,
        ..
    } = cli.command
    else {
        panic!("expected plan")
    };
    assert_eq!(level, BumpLevel::Auto);
    assert!(!json, "the default output is the human plan tree");
    assert!(skip_preflight.is_empty());
    assert!(!first_release);
}

#[test]
fn plan_takes_a_level_json_and_skipped_preflight_steps() {
    let cli = Cli::try_parse_from([
        "cutver",
        "plan",
        "minor",
        "--json",
        "--skip-preflight",
        "tests",
        "--skip-preflight",
        "lint",
        "--first-release",
    ])
    .unwrap();
    let Commands::Plan {
        level,
        json,
        skip_preflight,
        first_release,
        ..
    } = cli.command
    else {
        panic!("expected plan")
    };
    assert_eq!(level, BumpLevel::Minor);
    assert!(json);
    assert_eq!(skip_preflight, vec!["tests", "lint"]);
    assert!(first_release);
}

#[test]
fn plan_accepts_the_visible_first_release_alias() {
    let cli = Cli::try_parse_from(["cutver", "plan", "--fr"]).unwrap();
    let Commands::Plan { first_release, .. } = cli.command else {
        panic!("expected plan")
    };
    assert!(first_release);
}

#[test]
fn plan_has_no_dry_run_flag_because_it_never_writes() {
    assert!(
        Cli::try_parse_from(["cutver", "plan", "--dry-run"]).is_err(),
        "`plan` is what `--dry-run` means, so the flag has nothing left to ask for"
    );
}

#[test]
fn the_two_ci_flags_are_global_so_every_subcommand_can_opt_out() {
    let cli = Cli::try_parse_from(["cutver", "plan", "--ci"]).unwrap();
    assert!(cli.ci, "`--ci` turns the channel on");
    assert!(!cli.no_ci);

    let cli = Cli::try_parse_from(["cutver", "doctor", "--no-ci"]).unwrap();
    assert!(!cli.ci);
    assert!(cli.no_ci, "`--no-ci` turns the channel off on any subcommand");
    assert!(matches!(cli.command, Commands::Doctor { .. }));

    let before = Cli::try_parse_from(["cutver", "--no-ci", "changelog", "latest"]).unwrap();
    assert!(before.no_ci, "a global flag is accepted before the subcommand too");

    assert!(
        Cli::try_parse_from(["cutver", "bump", "--ci", "--no-ci"]).is_err(),
        "`--ci` and `--no-ci` are opposite answers to one question"
    );
}

#[test]
fn doctor_subcommand() {
    let cli = Cli::try_parse_from(["cutver", "doctor"]).unwrap();
    assert!(matches!(cli.command, Commands::Doctor { .. }));
}

#[test]
fn doctor_defaults() {
    let cli = Cli::try_parse_from(["cutver", "doctor"]).unwrap();
    let Commands::Doctor { check_changelog, json } = cli.command else {
        panic!("expected doctor");
    };
    assert!(!check_changelog);
    assert!(!json, "the human dashboard is the default surface");
}

#[test]
fn doctor_with_json_keeps_the_exit_code_meanings_out_of_the_flag() {
    let cli = Cli::try_parse_from(["cutver", "doctor", "--json", "--check-changelog"]).unwrap();
    let Commands::Doctor { check_changelog, json } = cli.command else {
        panic!("expected doctor");
    };
    assert!(json);
    assert!(check_changelog, "--json does not change what is checked");
}

#[test]
fn doctor_with_check_changelog() {
    let cli = Cli::try_parse_from(["cutver", "doctor", "--check-changelog"]).unwrap();
    let Commands::Doctor { check_changelog, json } = cli.command else {
        panic!("expected doctor");
    };
    assert!(check_changelog);
    assert!(!json);
}

#[test]
fn changelog_latest_defaults() {
    let cli = Cli::try_parse_from(["cutver", "changelog", "latest"]).unwrap();
    let Commands::Changelog {
        command:
            ChangelogCommands::Latest {
                include_header,
                path,
                template,
                json,
            },
    } = cli.command
    else {
        panic!("expected changelog latest");
    };
    assert!(!include_header);
    assert!(path.is_none());
    assert!(template.is_none());
    assert!(!json);
}

#[test]
fn changelog_latest_with_options() {
    let cli = Cli::try_parse_from([
        "cutver",
        "changelog",
        "latest",
        "--include-header",
        "--path",
        "docs/HISTORY.md",
        "--template",
        "release.j2",
    ])
    .unwrap();
    let Commands::Changelog { command } = cli.command else {
        panic!("expected changelog");
    };
    assert_eq!(
        command,
        ChangelogCommands::Latest {
            include_header: true,
            path: Some(PathBuf::from("docs/HISTORY.md")),
            template: Some(PathBuf::from("release.j2")),
            json: false,
        }
    );

    let cli_short = Cli::try_parse_from(["cutver", "changelog", "latest", "-H", "-p", "custom.md"]).unwrap();
    let Commands::Changelog { command: command_short } = cli_short.command else {
        panic!("expected changelog");
    };
    assert_eq!(
        command_short,
        ChangelogCommands::Latest {
            include_header: true,
            path: Some(PathBuf::from("custom.md")),
            template: None,
            json: false,
        }
    );

    let cli_json = Cli::try_parse_from(["cutver", "changelog", "latest", "--json"]).unwrap();
    let Commands::Changelog { command: command_json } = cli_json.command else {
        panic!("expected changelog");
    };
    assert_eq!(
        command_json,
        ChangelogCommands::Latest {
            include_header: false,
            path: None,
            template: None,
            json: true,
        }
    );
}

#[test]
fn changelog_show_defaults() {
    let cli = Cli::try_parse_from(["cutver", "changelog", "show", "0.2.0"]).unwrap();
    let Commands::Changelog {
        command:
            ChangelogCommands::Show {
                version,
                include_header,
                path,
                template,
                json,
            },
    } = cli.command
    else {
        panic!("expected changelog show");
    };
    assert_eq!(version, "0.2.0");
    assert!(!include_header);
    assert!(path.is_none());
    assert!(template.is_none());
    assert!(!json);
}

#[test]
fn changelog_show_with_options() {
    let cli = Cli::try_parse_from([
        "cutver",
        "changelog",
        "show",
        "v1.0.0",
        "--include-header",
        "--path",
        "docs/HISTORY.md",
        "--template",
        "notes.j2",
    ])
    .unwrap();
    let Commands::Changelog { command } = cli.command else {
        panic!("expected changelog");
    };
    assert_eq!(
        command,
        ChangelogCommands::Show {
            version: "v1.0.0".to_string(),
            include_header: true,
            path: Some(PathBuf::from("docs/HISTORY.md")),
            template: Some(PathBuf::from("notes.j2")),
            json: false,
        }
    );

    let cli_short = Cli::try_parse_from(["cutver", "changelog", "show", "0.3.1", "-H", "-p", "custom.md"]).unwrap();
    let Commands::Changelog { command: command_short } = cli_short.command else {
        panic!("expected changelog");
    };
    assert_eq!(
        command_short,
        ChangelogCommands::Show {
            version: "0.3.1".to_string(),
            include_header: true,
            path: Some(PathBuf::from("custom.md")),
            template: None,
            json: false,
        }
    );

    let cli_json = Cli::try_parse_from(["cutver", "changelog", "show", "1.5.0", "--json"]).unwrap();
    let Commands::Changelog { command: command_json } = cli_json.command else {
        panic!("expected changelog");
    };
    assert_eq!(
        command_json,
        ChangelogCommands::Show {
            version: "1.5.0".to_string(),
            include_header: false,
            path: None,
            template: None,
            json: true,
        }
    );
    assert_eq!(
        command_short,
        ChangelogCommands::Show {
            version: "0.3.1".to_string(),
            include_header: true,
            path: Some(PathBuf::from("custom.md")),
            template: None,
            json: false,
        }
    );
}

#[test]
fn config_override_global() {
    let cli = Cli::try_parse_from(["cutver", "-c", "other.toml", "doctor"]).unwrap();
    assert_eq!(cli.config, Some(PathBuf::from("other.toml")));
}

#[test]
fn init_defaults() {
    let cli = Cli::try_parse_from(["cutver", "init"]).unwrap();
    let Commands::Init {
        update,
        force,
        path,
        no_template,
    } = cli.command
    else {
        panic!("expected init");
    };
    assert!(!update);
    assert!(!force);
    assert!(!no_template);
    assert!(path.is_none());
}

#[test]
fn init_with_long_flags() {
    let cli = Cli::try_parse_from([
        "cutver",
        "init",
        "--update",
        "--force",
        "--path",
        "sub/dir",
        "--no-template",
    ])
    .unwrap();
    let Commands::Init {
        update,
        force,
        path,
        no_template,
    } = cli.command
    else {
        panic!("expected init");
    };
    assert!(update);
    assert!(force);
    assert!(no_template);
    assert_eq!(path, Some(PathBuf::from("sub/dir")));
}

#[test]
fn bump_first_release_flags_and_defaults() {
    let cli = Cli::try_parse_from(["cutver", "bump", "--first-release"]).unwrap();
    let Commands::Bump {
        level,
        dry_run,
        skip_preflight,
        first_release,
        defer_publish,
        json,
        report,
        ..
    } = cli.command
    else {
        panic!("expected bump")
    };
    assert_eq!(level, BumpLevel::Auto);
    assert!(!dry_run);
    assert!(skip_preflight.is_empty());
    assert!(first_release);
    assert!(!defer_publish);
    assert!(!json);
    assert!(report.is_none());

    let cli2 = Cli::try_parse_from(["cutver", "bump", "auto", "--first-release"]).unwrap();
    let Commands::Bump {
        level: l2,
        first_release: fr2,
        ..
    } = cli2.command
    else {
        panic!("expected bump")
    };
    assert_eq!(l2, BumpLevel::Auto);
    assert!(fr2);

    let cli3 = Cli::try_parse_from(["cutver", "bump", "--fr"]).unwrap();
    let Commands::Bump { first_release: fr3, .. } = cli3.command else {
        panic!("expected bump")
    };
    assert!(fr3);
}

#[test]
fn test_normalize_args() {
    let normalized = normalize_args(["cutver", "bump", "-fr"]);
    assert_eq!(normalized, vec!["cutver", "bump", "--first-release"]);

    let cli = Cli::try_parse_from(normalized).unwrap();
    let Commands::Bump { first_release, .. } = cli.command else {
        panic!("expected bump");
    };
    assert!(first_release);
    let normalized_nt = normalize_args(["cutver", "init", "-nt"]);
    assert_eq!(normalized_nt, vec!["cutver", "init", "--no-template"]);
    let cli_nt = Cli::try_parse_from(normalized_nt).unwrap();
    let Commands::Init { no_template, .. } = cli_nt.command else {
        panic!("expected init");
    };
    assert!(no_template);
}

#[test]
fn init_with_short_flags() {
    let cli = Cli::try_parse_from(["cutver", "init", "-u", "-f", "-p", "sub/dir"]).unwrap();
    let Commands::Init {
        update,
        force,
        path,
        no_template,
    } = cli.command
    else {
        panic!("expected init");
    };
    assert!(update);
    assert!(force);
    assert!(!no_template);
    assert_eq!(path, Some(PathBuf::from("sub/dir")));
}

#[test]
fn attest_parses_its_arguments_and_requires_a_conclusion() {
    let cli = Cli::try_parse_from([
        "cutver",
        "attest",
        "ci",
        "--commit",
        "abc1234",
        "--conclusion",
        "success",
    ])
    .unwrap();
    let Commands::Attest {
        gate,
        commit,
        conclusion,
        run,
    } = cli.command
    else {
        panic!("expected attest")
    };
    assert_eq!(gate, "ci");
    assert_eq!(commit, "abc1234");
    assert_eq!(conclusion, Conclusion::Success);
    assert!(run.is_none(), "the run is optional");

    let cli = Cli::try_parse_from([
        "cutver",
        "attest",
        "e2e-linux",
        "--commit",
        "abc1234",
        "--conclusion",
        "failure",
        "--run",
        "https://ci.example/run/7",
    ])
    .unwrap();
    let Commands::Attest { conclusion, run, .. } = cli.command else {
        panic!("expected attest")
    };
    assert_eq!(conclusion, Conclusion::Failure);
    assert_eq!(run.as_deref(), Some("https://ci.example/run/7"));

    // The conclusion is a closed set, and both `--commit` and `--conclusion` are required.
    assert!(Cli::try_parse_from(["cutver", "attest", "ci", "--commit", "abc1234", "--conclusion", "maybe"]).is_err());
    assert!(Cli::try_parse_from(["cutver", "attest", "ci", "--commit", "abc1234"]).is_err());
    assert!(Cli::try_parse_from(["cutver", "attest", "ci", "--conclusion", "success"]).is_err());
    assert!(Cli::try_parse_from(["cutver", "attest", "--commit", "abc1234", "--conclusion", "success"]).is_err());
}
