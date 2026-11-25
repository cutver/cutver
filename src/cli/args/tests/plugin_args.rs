//! The `plugin` subcommand's own arguments: the flag a list run prints as data, and the artifact a
//! removal keeps. Extracted from `src/cli/args/tests.rs` when the prerelease channel (#326 gap 6)
//! needed that file's line budget (allowlisted at 738, `CONTRACT.md:174`).

use super::*;

#[test]
fn plugin_list_parses_with_and_without_json() {
    let cli = Cli::try_parse_from(["cutver", "plugin", "list"]).unwrap();
    let Commands::Plugin {
        command: PluginCommands::List { json },
    } = cli.command
    else {
        panic!("expected plugin list");
    };
    assert!(!json);

    let cli = Cli::try_parse_from(["cutver", "plugin", "list", "--json"]).unwrap();
    let Commands::Plugin {
        command: PluginCommands::List { json },
    } = cli.command
    else {
        panic!("expected plugin list --json");
    };
    assert!(json);
}

#[test]
fn plugin_remove_parses_name_and_keep_artifact() {
    let cli = Cli::try_parse_from(["cutver", "plugin", "remove", "demo"]).unwrap();
    let Commands::Plugin {
        command: PluginCommands::Remove { name, keep_artifact },
    } = cli.command
    else {
        panic!("expected plugin remove");
    };
    assert_eq!(name, "demo");
    assert!(!keep_artifact);

    let cli = Cli::try_parse_from(["cutver", "plugin", "remove", "demo", "--keep-artifact"]).unwrap();
    let Commands::Plugin {
        command: PluginCommands::Remove { keep_artifact, .. },
    } = cli.command
    else {
        panic!("expected plugin remove --keep-artifact");
    };
    assert!(keep_artifact);
}

#[test]
fn plugin_is_declared_in_every_build_and_hidden_only_without_the_feature() {
    use clap::CommandFactory;

    let command = Cli::command();
    let plugin = command.find_subcommand("plugin").expect("plugin must be declared");
    assert_eq!(plugin.is_hide_set(), !cfg!(feature = "plugins"));
}
