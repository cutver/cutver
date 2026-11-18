//! Runner for `cutver plugin info`.

use std::time::Duration;

use crate::cli::args::PluginInfoArgs;
use crate::cli::machine::FailureCode;
use crate::cli::runner::dispatch::print_error;
use crate::cli::style::Theme;
use crate::plugin::registry::{RegistryIndexUrl, UreqTransport, get_plugin_info};
use crate::plugin::types::PluginName;

const INFO_TIMEOUT: Duration = Duration::from_secs(30);

/// The machine form of `plugin info`: the entry's own keys behind the schema version every JSON
/// surface opens with (see `src/cli/json.rs`).
fn render_info_json(entry: &crate::plugin::registry::RegistryEntryJson<'_>) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&crate::cli::json::Versioned::new(&entry))
}

/// Runs `cutver plugin info` in a plugin-enabled build.
pub fn run_info(args: &PluginInfoArgs) -> i32 {
    let name = match PluginName::new(&args.name) {
        Ok(n) => n,
        Err(err) => {
            print_error(FailureCode::Plugin, err);
            return 1;
        }
    };

    let index = match args.registry.as_deref() {
        Some(base) => match RegistryIndexUrl::parse(base) {
            Ok(idx) => idx,
            Err(err) => {
                print_error(FailureCode::Plugin, err);
                return 1;
            }
        },
        None => RegistryIndexUrl::default_index(),
    };

    let transport = UreqTransport::new(INFO_TIMEOUT);

    let entry = match get_plugin_info(&name, &index, &transport) {
        Ok(entry) => entry,
        Err(err) => {
            print_error(FailureCode::Plugin, err);
            return 1;
        }
    };

    if args.json {
        let json_repr = crate::plugin::registry::RegistryEntryJson::from(&entry);
        match render_info_json(&json_repr) {
            Ok(json_str) => println!("{json_str}"),
            Err(err) => {
                print_error(
                    FailureCode::Plugin,
                    format!("failed to serialize the plugin entry to JSON: {err}"),
                );
                return 1;
            }
        }
        return 0;
    }

    let theme = Theme::stdout();

    println!("{}: {}", theme.bold("Plugin"), theme.accent(entry.name.as_str()));
    println!("  {:<14} {}", theme.bold("Tier:"), entry.tier.as_str());
    println!("  {:<14} {}", theme.bold("Author:"), entry.author);
    println!("  {:<14} {}", theme.bold("Description:"), entry.description);
    println!("  {:<14} {}", theme.bold("Repository:"), entry.repository.as_str());

    let caps: Vec<&str> = entry.capabilities.iter().map(|c| c.as_str()).collect();
    println!("  {:<14} {}", theme.bold("Capabilities:"), caps.join(", "));

    println!("  {:<14}", theme.bold("Releases:"));
    for release in &entry.releases {
        println!(
            "    {} {} ({})",
            theme.accent("•"),
            theme.bold(release.version.to_string()),
            release.sha256.as_str()
        );
    }

    0
}

#[cfg(test)]
mod tests {
    use crate::plugin::registry::{RegistryEntryJson, RegistryReleaseJson};

    use super::*;

    #[test]
    fn the_entry_is_a_versioned_object_with_the_entry_keys_in_order() {
        let entry = RegistryEntryJson {
            name: "github-publish",
            description: "Publish a GitHub release",
            author: "cutver",
            tier: "official",
            repository: "https://github.com/cutver/plugins",
            capabilities: vec!["on_post_release"],
            releases: vec![RegistryReleaseJson {
                version: "0.2.1".into(),
                url: "https://example.invalid/github-publish-0.2.1.wasm",
                sha256: "abc",
            }],
        };

        let json = render_info_json(&entry).expect("the entry serializes");

        assert!(
            json.trim_start().starts_with("{\n  \"schemaVersion\": 1,"),
            "the schema version is the first key: {json}"
        );
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("the entry is valid JSON");
        assert_eq!(parsed["schemaVersion"], 1, "{json}");
        assert_eq!(parsed["name"], "github-publish", "{json}");
        assert_eq!(parsed["releases"][0]["version"], "0.2.1", "{json}");
    }
}
