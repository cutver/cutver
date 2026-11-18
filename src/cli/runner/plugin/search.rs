//! Runner for `cutver plugin search`.

use std::time::Duration;

use crate::cli::args::PluginSearchArgs;
use crate::cli::machine::FailureCode;
use crate::cli::runner::dispatch::print_error;
use crate::cli::style::Theme;
use crate::plugin::registry::{RegistryIndexUrl, UreqTransport, search_plugins};

const SEARCH_TIMEOUT: Duration = Duration::from_secs(30);

/// The machine form of `plugin search`.
///
/// The list used to be a bare array, which has no room for a version; the object wrapper is the
/// breaking change `CHANGELOG.md` declares, and the reason every surface now opens with
/// `schemaVersion` (see `src/cli/json.rs`).
#[derive(serde::Serialize)]
struct SearchJson<'a> {
    plugins: Vec<crate::plugin::registry::RegistryEntryJson<'a>>,
}

fn render_search_json(entries: &[crate::plugin::registry::RegistryEntryJson<'_>]) -> Result<String, serde_json::Error> {
    let payload = SearchJson {
        plugins: entries.to_vec(),
    };
    serde_json::to_string_pretty(&crate::cli::json::Versioned::new(&payload))
}

/// Runs `cutver plugin search` in a plugin-enabled build.
pub fn run_search(args: &PluginSearchArgs) -> i32 {
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

    let transport = UreqTransport::new(SEARCH_TIMEOUT);

    let entries = match search_plugins(args.query.as_deref(), &index, &transport) {
        Ok(res) => res,
        Err(err) => {
            print_error(FailureCode::Plugin, err);
            return 1;
        }
    };

    if args.json {
        let json_entries: Vec<crate::plugin::registry::RegistryEntryJson> = entries
            .iter()
            .map(crate::plugin::registry::RegistryEntryJson::from)
            .collect();
        match render_search_json(&json_entries) {
            Ok(json_str) => println!("{json_str}"),
            Err(err) => {
                print_error(
                    FailureCode::Plugin,
                    format!("failed to serialize the search result to JSON: {err}"),
                );
                return 1;
            }
        }
        return 0;
    }

    let theme = Theme::stdout();

    if entries.is_empty() {
        if let Some(q) = &args.query {
            println!("No plugins found matching query '{q}'.");
        } else {
            println!("No plugins found in registry.");
        }
        return 0;
    }

    println!("Matching plugins in registry ({}):\n", entries.len());

    for entry in &entries {
        let caps: Vec<&str> = entry.capabilities.iter().map(|c| c.as_str()).collect();
        let latest_ver = entry
            .releases
            .iter()
            .filter(|r| r.version.pre.is_empty())
            .max_by_key(|r| &r.version)
            .map(|r| r.version.to_string())
            .unwrap_or_else(|| "none".to_string());

        println!(
            "  {} [{}] ({})",
            theme.bold(entry.name.as_str()),
            entry.tier.as_str(),
            caps.join(", ")
        );
        println!("    {}", entry.description);
        println!(
            "    {} {} • {} {}",
            theme.muted("Latest:"),
            latest_ver,
            theme.muted("Repo:"),
            entry.repository.as_str()
        );
        println!();
    }

    0
}

#[cfg(test)]
mod tests {
    use crate::plugin::registry::{RegistryEntryJson, RegistryReleaseJson};

    use super::*;

    fn entry() -> RegistryEntryJson<'static> {
        RegistryEntryJson {
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
        }
    }

    #[test]
    fn the_search_result_is_a_versioned_object_not_a_bare_array() {
        let json = render_search_json(&[entry()]).expect("the search result serializes");

        assert!(
            json.trim_start().starts_with("{\n  \"schemaVersion\": 1,"),
            "the schema version is the first key: {json}"
        );
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("the result is valid JSON");
        assert_eq!(parsed["schemaVersion"], 1, "{json}");
        assert_eq!(parsed["plugins"][0]["name"], "github-publish", "{json}");
        assert!(parsed["plugins"].is_array(), "the list lives under `plugins`: {json}");
    }

    #[test]
    fn an_empty_search_result_is_an_empty_list_and_not_a_null() {
        let json = render_search_json(&[]).expect("the search result serializes");

        let parsed: serde_json::Value = serde_json::from_str(&json).expect("the result is valid JSON");
        assert_eq!(parsed["plugins"].as_array().map(Vec::len), Some(0), "{json}");
    }
}
