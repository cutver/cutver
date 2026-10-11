use std::collections::HashSet;

use toml_edit::{InlineTable, Item, Table, Value};

use super::types::{Config, ConfigError, Manifest, default_strategy};

pub(crate) fn deduce_current_source(config: &mut Config) -> Result<(), ConfigError> {
    if config.manifest.is_empty() {
        return Err(ConfigError::NoManifestsDeclared);
    }
    let candidates: Vec<&Manifest> = config.manifest.iter().filter(|m| m.primary).collect();
    if candidates.len() > 1 {
        return Err(ConfigError::MultiplePrimaryManifests);
    }
    let primary_path = candidates.first().map(|m| m.path.as_str());
    if config.version.current_source.is_empty() {
        if let Some(p) = primary_path {
            config.version.current_source = p.to_string();
        } else {
            config.version.current_source = config.manifest[0].path.to_string();
        }
    } else if let Some(p) = primary_path
        && config.version.current_source != p
    {
        return Err(ConfigError::ConflictingPrimaryManifest(
            config.version.current_source.clone(),
            p.to_string(),
        ));
    }
    if config.version.strategy.is_empty() {
        config.version.strategy = default_strategy();
    }
    Ok(())
}

pub(crate) fn validate(config: &Config) -> Result<(), ConfigError> {
    let mut seen = HashSet::new();
    for m in &config.manifest {
        if !seen.insert(&m.path) {
            return Err(ConfigError::DuplicateManifestPath(m.path.to_string()));
        }
    }
    if !config.manifest.iter().any(|m| m.path == config.version.current_source) {
        return Err(ConfigError::CurrentSourceNotFound(
            config.version.current_source.clone(),
        ));
    }
    Ok(())
}

/// Accepted top-level keys of `cutver.toml`.
const ROOT_KEYS: &[&str] = &[
    "project",
    "version",
    "manifest",
    "preflight",
    "changelog",
    "git",
    "hooks",
    "publish",
    "plugins",
];
const PROJECT_KEYS: &[&str] = &["name"];
const VERSION_KEYS: &[&str] = &["current_source", "source", "strategy", "plugin"];
/// Union of every `ManifestKind` variant's keys. The flattened enum owns these,
/// so the schema pass cannot know which kind a given table declares without
/// duplicating serde's tag dispatch; the union keeps every valid key accepted.
const MANIFEST_KEYS: &[&str] = &[
    "path",
    "primary",
    "kind",
    "field",
    "version_name_field",
    "version_code_field",
    "pattern",
    "replacement",
    "table",
    "plugin",
];
const CHANGELOG_KEYS: &[&str] = &[
    "path",
    "format",
    "mode",
    "entry_template",
    "template",
    "template_file",
    "full_template",
    "header_template",
    "include_scopes",
    "fallback_entry",
    "ignore_release_commits",
    "ignore_scopes",
    "plugin",
];
const GIT_KEYS: &[&str] = &[
    "tag_prefix",
    "commit_message",
    "require_clean_tree",
    "require_branch",
    "floating_major_tag",
];
const HOOKS_KEYS: &[&str] = &["pre_bump", "post_bump"];
const PUBLISH_KEYS: &[&str] = &["push", "commands", "default_timeout"];
const PLUGIN_KEYS: &[&str] = &[
    "runtime",
    "source",
    "hash",
    "command",
    "capabilities",
    "events",
    "manifest_match",
    "permissions",
    "timeout_seconds",
];
const PERMISSIONS_KEYS: &[&str] = &["network", "env", "filesystem"];

/// Rejects any key the loader does not know, naming the enclosing table and the
/// offending key. `[preflight]` keys are arbitrary user-named steps, so that
/// table is intentionally exempt.
///
/// Call this only after serde has accepted the document, so that malformed
/// values keep reporting their existing `ConfigError::Parse` errors.
pub(crate) fn reject_unknown_keys(text: &str) -> Result<(), ConfigError> {
    let doc = text.parse::<toml_edit::DocumentMut>()?;
    let root = doc.as_table();
    for (key, value) in root.iter() {
        if !ROOT_KEYS.contains(&key) {
            return Err(first_unknown(value, key));
        }
        match key {
            "project" => check_section(value, PROJECT_KEYS, "project")?,
            "version" => check_section(value, VERSION_KEYS, "version")?,
            "changelog" => check_section(value, CHANGELOG_KEYS, "changelog")?,
            "git" => check_section(value, GIT_KEYS, "git")?,
            "hooks" => check_section(value, HOOKS_KEYS, "hooks")?,
            "publish" => check_section(value, PUBLISH_KEYS, "publish")?,
            "manifest" => check_manifests(value)?,
            "plugins" => check_plugins(value)?,
            // `[preflight]` accepts arbitrary step names (see module docs).
            "preflight" => {}
            _ => unreachable!("ROOT_KEYS guards this dispatch"),
        }
    }
    Ok(())
}

fn check_section(item: &Item, allowed: &[&str], path: &str) -> Result<(), ConfigError> {
    match item {
        Item::Table(t) => check_table_unknown(t, allowed, path),
        Item::Value(Value::InlineTable(t)) => check_inline_unknown(t, allowed, path),
        _ => Ok(()),
    }
}

fn check_table_unknown(table: &Table, allowed: &[&str], path: &str) -> Result<(), ConfigError> {
    for (key, _) in table.iter() {
        if !allowed.contains(&key) {
            return Err(ConfigError::UnknownKey {
                table: path.to_string(),
                key: key.to_string(),
            });
        }
    }
    Ok(())
}

fn check_inline_unknown(table: &InlineTable, allowed: &[&str], path: &str) -> Result<(), ConfigError> {
    for (key, _) in table.iter() {
        if !allowed.contains(&key) {
            return Err(ConfigError::UnknownKey {
                table: path.to_string(),
                key: key.to_string(),
            });
        }
    }
    Ok(())
}

fn check_manifests(item: &Item) -> Result<(), ConfigError> {
    match item {
        Item::ArrayOfTables(manifests) => {
            for manifest in manifests.iter() {
                check_table_unknown(manifest, MANIFEST_KEYS, "manifest")?;
            }
        }
        Item::Value(Value::Array(manifests)) => {
            for value in manifests.iter() {
                if let Value::InlineTable(t) = value {
                    check_inline_unknown(t, MANIFEST_KEYS, "manifest")?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn check_plugins(item: &Item) -> Result<(), ConfigError> {
    match item {
        Item::Table(plugins) => {
            for (name, plugin) in plugins.iter() {
                check_plugin_item(plugin, &format!("plugins.{name}"))?;
            }
        }
        Item::Value(Value::InlineTable(plugins)) => {
            for (name, plugin) in plugins.iter() {
                let path = format!("plugins.{name}");
                if let Value::InlineTable(t) = plugin {
                    check_inline_unknown(t, PLUGIN_KEYS, &path)?;
                    if let Some(Value::InlineTable(perms)) = t.get("permissions") {
                        check_inline_unknown(perms, PERMISSIONS_KEYS, &format!("{path}.permissions"))?;
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn check_plugin_item(item: &Item, path: &str) -> Result<(), ConfigError> {
    match item {
        Item::Table(plugin) => {
            check_table_unknown(plugin, PLUGIN_KEYS, path)?;
            if let Some(perms) = plugin.get("permissions") {
                check_section(perms, PERMISSIONS_KEYS, &format!("{path}.permissions"))?;
            }
        }
        Item::Value(Value::InlineTable(plugin)) => {
            check_inline_unknown(plugin, PLUGIN_KEYS, path)?;
            if let Some(Value::InlineTable(perms)) = plugin.get("permissions") {
                check_inline_unknown(perms, PERMISSIONS_KEYS, &format!("{path}.permissions"))?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// Builds the error for an unknown top-level key. For an unknown table the
/// first key inside it is named, so the message identifies both the table and a
/// concrete offending key. When a nested unknown table is empty there is no key
/// to name, so the message names the table instead.
fn first_unknown(item: &Item, key: &str) -> ConfigError {
    match first_unknown_key(item, key) {
        Some(UnknownItem::Key { table, key }) => ConfigError::UnknownKey { table, key },
        Some(UnknownItem::Table { table }) => ConfigError::UnknownTable { table },
        None => ConfigError::UnknownRootKey { key: key.to_string() },
    }
}

/// The first offending item found while descending an unknown top-level table.
enum UnknownItem {
    /// A concrete key inside known path `table`.
    Key { table: String, key: String },
    /// A table that contains no keys to name as the offending key.
    Table { table: String },
}

fn first_unknown_key(item: &Item, path: &str) -> Option<UnknownItem> {
    match item {
        Item::Table(t) => first_from_table(t, path),
        Item::Value(Value::InlineTable(t)) => first_from_inline(t, path),
        Item::ArrayOfTables(entries) => entries.iter().next().and_then(|t| first_from_table(t, path)),
        _ => None,
    }
}

fn first_from_table(table: &Table, path: &str) -> Option<UnknownItem> {
    let (key, value) = table.iter().next()?;
    Some(first_from_item(value, key, path))
}

fn first_from_inline(table: &InlineTable, path: &str) -> Option<UnknownItem> {
    let (key, value) = table.iter().next()?;
    let child = format!("{path}.{key}");
    Some(match value {
        Value::InlineTable(inner) => first_from_inline(inner, &child).unwrap_or(UnknownItem::Table { table: child }),
        _ => UnknownItem::Key {
            table: path.to_string(),
            key: key.to_string(),
        },
    })
}

fn first_from_item(item: &Item, key: &str, path: &str) -> UnknownItem {
    let child = format!("{path}.{key}");
    match item {
        Item::Table(inner) => first_from_table(inner, &child).unwrap_or(UnknownItem::Table { table: child }),
        Item::ArrayOfTables(entries) => entries
            .iter()
            .next()
            .and_then(|t| first_from_table(t, &child))
            .unwrap_or(UnknownItem::Table { table: child }),
        Item::Value(Value::InlineTable(inner)) => {
            first_from_inline(inner, &child).unwrap_or(UnknownItem::Table { table: child })
        }
        _ => UnknownItem::Key {
            table: path.to_string(),
            key: key.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_str(s: &str) -> Result<Config, ConfigError> {
        let mut c: Config = toml::from_str(s).map_err(ConfigError::Parse)?;
        super::reject_unknown_keys(s)?;
        let (p, d) = super::super::preflight::parse_preflight(s)?;
        c.preflight = p;
        c.preflight_default_timeout = d;
        c.root_dir = std::env::current_dir().unwrap();
        deduce_current_source(&mut c)?;
        validate(&c)?;
        Ok(c)
    }

    fn manifest(kind: &str, extra: &str) -> String {
        format!("[version]\ncurrent_source = \"a\"\n\n[[manifest]]\npath = \"a\"\nkind = \"{kind}\"\n{extra}")
    }

    #[test]
    fn validation_failures() {
        let dup = "[version]\ncurrent_source = \"a\"\n\n[[manifest]]\npath = \"a\"\nkind = \"cargo-package\"\n\n[[manifest]]\npath = \"a\"\nkind = \"json\"\nfield = \"version\"";
        let missing = "[version]\ncurrent_source = \"missing\"\n\n[[manifest]]\npath = \"a\"\nkind = \"cargo-package\"";
        assert!(matches!(load_str(dup), Err(ConfigError::DuplicateManifestPath(_))));
        assert!(matches!(load_str(missing), Err(ConfigError::CurrentSourceNotFound(_))));
        assert!(matches!(load_str(&manifest("json", "")), Err(ConfigError::Parse(_))));
        assert!(matches!(load_str(&manifest("gradle", "")), Err(ConfigError::Parse(_))));
        assert!(matches!(load_str(&manifest("regex", "")), Err(ConfigError::Parse(_))));
        assert!(matches!(load_str(&manifest("unknown", "")), Err(ConfigError::Parse(_))));
        assert!(load_str(&manifest("plugin", "")).is_ok());
        assert!(load_str(&manifest("plugin", "plugin = \"helm\"\n")).is_ok());
        assert!(matches!(
            load_str(&(manifest("cargo-package", "") + "\n[preflight]\ntests = 123")),
            Err(ConfigError::PreflightNotString(_))
        ));
    }

    #[test]
    fn omit_version_uses_first_manifest_as_source() {
        let toml = r#"
[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"
"#;
        let cfg = load_str(toml).unwrap();
        assert_eq!(cfg.version.current_source, "Cargo.toml");
        assert_eq!(cfg.version.strategy, "manual");
    }

    #[test]
    fn version_source_alias_works() {
        let toml = r#"
[version]
source = "package.json"

[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"
"#;
        let cfg = load_str(toml).unwrap();
        assert_eq!(cfg.version.current_source, "package.json");
    }

    #[test]
    fn primary_manifest_on_non_first_manifest_uses_it_as_source() {
        let toml = r#"
[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"
primary = true
"#;
        let cfg = load_str(toml).unwrap();
        assert_eq!(cfg.version.current_source, "Cargo.toml");
    }

    #[test]
    fn multiple_primary_manifests_errors() {
        let toml = r#"
[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"
primary = true

[[manifest]]
path = "package.json"
kind = "json"
field = "version"
primary = true
"#;
        let err = load_str(toml).unwrap_err();
        assert!(matches!(err, ConfigError::MultiplePrimaryManifests));
    }

    #[test]
    fn conflicting_current_source_and_primary_manifest_errors() {
        let toml = r#"
[version]
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"
primary = true
"#;
        let err = load_str(toml).unwrap_err();
        match err {
            ConfigError::ConflictingPrimaryManifest(source, primary) => {
                assert_eq!(source, "package.json");
                assert_eq!(primary, "Cargo.toml");
            }
            other => panic!("expected ConflictingPrimaryManifest, got {other:?}"),
        }
    }

    #[test]
    fn empty_manifests_returns_no_manifests_declared() {
        let toml = r#"
[git]
tag_prefix = "v"
"#;
        let err = load_str(toml).unwrap_err();
        assert!(matches!(err, ConfigError::NoManifestsDeclared));

        let toml_with_version = r#"
[version]
current_source = "Cargo.toml"
"#;
        let err = load_str(toml_with_version).unwrap_err();
        assert!(matches!(err, ConfigError::NoManifestsDeclared));
    }
}
