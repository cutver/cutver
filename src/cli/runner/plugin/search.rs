//! Runner for `cutver plugin search`.

use std::time::Duration;

use crate::cli::args::PluginSearchArgs;
use crate::cli::runner::dispatch::print_error;
use crate::cli::style::Theme;
use crate::plugin::registry::{RegistryIndexUrl, UreqTransport, search_plugins};

const SEARCH_TIMEOUT: Duration = Duration::from_secs(30);

/// Runs `cutver plugin search` in a plugin-enabled build.
pub fn run_search(args: &PluginSearchArgs) -> i32 {
    let index = match args.registry.as_deref() {
        Some(base) => match RegistryIndexUrl::parse(base) {
            Ok(idx) => idx,
            Err(err) => {
                print_error(err);
                return 1;
            }
        },
        None => RegistryIndexUrl::default_index(),
    };

    let transport = UreqTransport::new(SEARCH_TIMEOUT);

    let entries = match search_plugins(args.query.as_deref(), &index, &transport) {
        Ok(res) => res,
        Err(err) => {
            print_error(err);
            return 1;
        }
    };

    if args.json {
        let json_entries: Vec<crate::plugin::registry::RegistryEntryJson> = entries
            .iter()
            .map(crate::plugin::registry::RegistryEntryJson::from)
            .collect();
        match serde_json::to_string_pretty(&json_entries) {
            Ok(json_str) => println!("{json_str}"),
            Err(err) => {
                eprintln!("Failed to format JSON: {err}");
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
