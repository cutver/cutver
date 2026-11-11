//! Runner for `cutver plugin info`.

use std::time::Duration;

use crate::cli::args::PluginInfoArgs;
use crate::cli::runner::dispatch::print_error;
use crate::cli::style::Theme;
use crate::plugin::registry::{RegistryIndexUrl, UreqTransport, get_plugin_info};
use crate::plugin::types::PluginName;

const INFO_TIMEOUT: Duration = Duration::from_secs(30);

/// Runs `cutver plugin info` in a plugin-enabled build.
pub fn run_info(args: &PluginInfoArgs) -> i32 {
    let name = match PluginName::new(&args.name) {
        Ok(n) => n,
        Err(err) => {
            print_error(err);
            return 1;
        }
    };

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

    let transport = UreqTransport::new(INFO_TIMEOUT);

    let entry = match get_plugin_info(&name, &index, &transport) {
        Ok(entry) => entry,
        Err(err) => {
            print_error(err);
            return 1;
        }
    };

    if args.json {
        let json_repr = crate::plugin::registry::RegistryEntryJson::from(&entry);
        match serde_json::to_string_pretty(&json_repr) {
            Ok(json_str) => println!("{json_str}"),
            Err(err) => {
                eprintln!("Failed to format JSON: {err}");
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
