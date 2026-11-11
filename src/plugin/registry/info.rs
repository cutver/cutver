//! Query operations for inspecting and searching registry plugin entries.

use serde::Serialize;

use crate::plugin::error::PluginError;
use crate::plugin::registry::{RegistryEntry, RegistryIndexUrl, Transport, resolve_entry_with};
use crate::plugin::types::PluginName;

/// Official canonical plugins published in the Cutver registry.
pub const CANONICAL_PLUGINS: &[&str] = &["github-publish", "github-releases"];

/// Serializable projection of a registry entry for JSON CLI outputs.
#[derive(Debug, Clone, Serialize)]
pub struct RegistryEntryJson<'a> {
    pub name: &'a str,
    pub description: &'a str,
    pub author: &'a str,
    pub tier: &'a str,
    pub repository: &'a str,
    pub capabilities: Vec<&'a str>,
    pub releases: Vec<RegistryReleaseJson<'a>>,
}

/// Serializable projection of a single release for JSON CLI outputs.
#[derive(Debug, Clone, Serialize)]
pub struct RegistryReleaseJson<'a> {
    pub version: String,
    pub url: &'a str,
    pub sha256: &'a str,
}

impl<'a> From<&'a RegistryEntry> for RegistryEntryJson<'a> {
    fn from(entry: &'a RegistryEntry) -> Self {
        Self {
            name: entry.name.as_str(),
            description: &entry.description,
            author: &entry.author,
            tier: entry.tier.as_str(),
            repository: entry.repository.as_str(),
            capabilities: entry.capabilities.iter().map(|c| c.as_str()).collect(),
            releases: entry
                .releases
                .iter()
                .map(|r| RegistryReleaseJson {
                    version: r.version.to_string(),
                    url: r.url.as_str(),
                    sha256: r.sha256.as_str(),
                })
                .collect(),
        }
    }
}

/// Fetches and validates detailed information about a single plugin from the registry.
pub fn get_plugin_info(
    name: &PluginName,
    index: &RegistryIndexUrl,
    transport: &dyn Transport,
) -> Result<RegistryEntry, PluginError> {
    resolve_entry_with(index, name, transport)
}

/// Searches the registry index for plugins matching `query`.
///
/// If `query` is `None` or empty, returns all discovered registry entries.
/// Matches case-insensitively against name, description, author, capability names, and tier.
pub fn search_plugins(
    query: Option<&str>,
    index: &RegistryIndexUrl,
    transport: &dyn Transport,
) -> Result<Vec<RegistryEntry>, PluginError> {
    let clean_query = query.map(|q| q.trim().to_lowercase()).filter(|q| !q.is_empty());

    let mut entries = Vec::new();

    for plugin_name_str in CANONICAL_PLUGINS {
        let name = match PluginName::new(*plugin_name_str) {
            Ok(n) => n,
            Err(_) => continue,
        };

        if let Ok(entry) = resolve_entry_with(index, &name, transport) {
            if let Some(ref q) = clean_query {
                let matches_name = entry.name.as_str().to_lowercase().contains(q);
                let matches_desc = entry.description.to_lowercase().contains(q);
                let matches_author = entry.author.to_lowercase().contains(q);
                let matches_tier = entry.tier.as_str().to_lowercase().contains(q);
                let matches_caps = entry.capabilities.iter().any(|c| c.as_str().to_lowercase().contains(q));

                if matches_name || matches_desc || matches_author || matches_tier || matches_caps {
                    entries.push(entry);
                }
            } else {
                entries.push(entry);
            }
        }
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::error::TransportError;
    use std::collections::HashMap;

    struct MockTransport {
        responses: HashMap<String, Vec<u8>>,
    }

    impl MockTransport {
        fn new() -> Self {
            Self {
                responses: HashMap::new(),
            }
        }

        fn with(mut self, url: impl Into<String>, body: impl Into<Vec<u8>>) -> Self {
            self.responses.insert(url.into(), body.into());
            self
        }
    }

    impl Transport for MockTransport {
        fn fetch(&self, url: &str, _cap_bytes: u64) -> Result<Vec<u8>, TransportError> {
            self.responses
                .get(url)
                .cloned()
                .ok_or_else(|| TransportError::TransportHttpStatus {
                    url: url.to_string(),
                    status: 404,
                })
        }
    }

    fn sample_entry(name: &str, desc: &str, cap: &str) -> String {
        format!(
            "name = \"{name}\"\n\
             description = \"{desc}\"\n\
             author = \"cutver\"\n\
             tier = \"official\"\n\
             repository = \"https://github.com/cutver/plugins\"\n\
             capabilities = [\"{cap}\"]\n\n\
             [[release]]\n\
             version = \"0.1.0\"\n\
             url = \"https://example.com/{name}.wasm\"\n\
             sha256 = \"2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824\"\n"
        )
    }

    #[test]
    fn get_plugin_info_resolves_metadata() {
        let transport = MockTransport::new().with(
            "https://raw.githubusercontent.com/cutver/registry/main/plugins/github-publish.toml",
            sample_entry("github-publish", "Release publisher", "lifecycle.v1"),
        );

        let index = RegistryIndexUrl::default_index();
        let name = PluginName::new("github-publish").unwrap();
        let entry = get_plugin_info(&name, &index, &transport).unwrap();

        assert_eq!(entry.name.as_str(), "github-publish");
        assert_eq!(entry.description, "Release publisher");
        assert_eq!(entry.capabilities[0].as_str(), "lifecycle.v1");
        assert_eq!(entry.releases.len(), 1);
    }

    #[test]
    fn search_plugins_filters_by_query() {
        let transport = MockTransport::new()
            .with(
                "https://raw.githubusercontent.com/cutver/registry/main/plugins/github-publish.toml",
                sample_entry("github-publish", "Release publisher", "lifecycle.v1"),
            )
            .with(
                "https://raw.githubusercontent.com/cutver/registry/main/plugins/github-releases.toml",
                sample_entry("github-releases", "Changelog renderer", "changelog.v1"),
            );

        let index = RegistryIndexUrl::default_index();

        let all = search_plugins(None, &index, &transport).unwrap();
        assert_eq!(all.len(), 2);

        let publish_only = search_plugins(Some("publish"), &index, &transport).unwrap();
        assert_eq!(publish_only.len(), 1);
        assert_eq!(publish_only[0].name.as_str(), "github-publish");

        let changelog_caps = search_plugins(Some("changelog.v1"), &index, &transport).unwrap();
        assert_eq!(changelog_caps.len(), 1);
        assert_eq!(changelog_caps[0].name.as_str(), "github-releases");

        let non_matching = search_plugins(Some("nonexistent-token"), &index, &transport).unwrap();
        assert_eq!(non_matching.len(), 0);
    }
}
