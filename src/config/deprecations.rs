//! The deprecation table and the scan that finds a deprecated spelling in `cutver.toml`.
//!
//! `CONTRACT.md` Pillar IV.2 requires deprecated syntax to keep working **and** to warn, and Pillar
//! VII.2 requires a surface to be deprecated before it is removed, with a warning the consumer can
//! act on. One row per deprecated spelling is the whole registry: the sentence a consumer reads is
//! rendered from the row, never written at a call site, so a rename cannot ship without saying what
//! it replaced and which release removes it.
//!
//! The scan is structural — it walks the parsed document — so a deprecated spelling is found where
//! it is used, never where it happens to appear in a comment or an unrelated value.

use toml_edit::{DocumentMut, Item, Table, Value};

use super::types::ConfigError;

/// The release that removes every spelling this table has deprecated.
pub const REMOVAL_VERSION: &str = "1.0.0";

/// One deprecated spelling: where it lives, what replaced it, and the release that removes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Deprecation {
    /// The table the spelling lives in, as a reader of `cutver.toml` writes it.
    pub surface: &'static str,
    /// The spelling itself: a key, or a key and its value when the value is what changed.
    pub spelling: &'static str,
    /// The spelling that replaces it.
    pub canonical: &'static str,
    /// The release that removes the deprecated spelling.
    pub removal: &'static str,
}

impl Deprecation {
    /// The one sentence this entry warns with:
    /// `<surface>.<spelling> is deprecated, use <canonical>; it will be removed in <removal>`.
    pub fn message(&self) -> String {
        format!(
            "{}.{} is deprecated, use {}; it will be removed in {}",
            self.surface, self.spelling, self.canonical, self.removal
        )
    }
}

/// Every spelling the loader still accepts but the contract has deprecated.
///
/// The three below are `#[serde(alias)]` renames that used to resolve silently, which is exactly the
/// shape Pillar IV.2 forbids: they keep working, and now they say so.
pub const DEPRECATIONS: &[Deprecation] = &[
    Deprecation {
        surface: "[version]",
        spelling: "source",
        canonical: "current_source",
        removal: REMOVAL_VERSION,
    },
    Deprecation {
        surface: "[[manifest]]",
        spelling: "kind = \"toml\"",
        canonical: "\"cargo-package\"",
        removal: REMOVAL_VERSION,
    },
    Deprecation {
        surface: "[[manifest]]",
        spelling: "kind = \"pyproject-toml\"",
        canonical: "\"pyproject\"",
        removal: REMOVAL_VERSION,
    },
];

/// Finds every deprecated spelling the document carries, in table order.
pub fn deprecations_in(text: &str) -> Result<Vec<&'static Deprecation>, ConfigError> {
    let doc = text.parse::<DocumentMut>()?;
    Ok(deprecations_in_document(&doc))
}

/// Scans a document and says what it found, on both channels, once per process.
///
/// This is the loader's whole call into the channel: a deprecated spelling that still works is a
/// warning, never an error, so the scan reports and returns instead of failing.
pub fn report(text: &str) -> Result<Vec<&'static Deprecation>, ConfigError> {
    let found = deprecations_in(text)?;
    crate::warnings::deprecations(&found);
    Ok(found)
}

/// The same scan for a caller that already parsed the document, so nothing can fail twice.
pub fn deprecations_in_document(doc: &DocumentMut) -> Vec<&'static Deprecation> {
    in_document(doc)
}

/// The table entry for one exact spelling, or nothing when the table stopped carrying it.
fn entry(surface: &str, spelling: &str) -> Option<&'static Deprecation> {
    DEPRECATIONS
        .iter()
        .find(|entry| entry.surface == surface && entry.spelling == spelling)
}

fn in_document(doc: &DocumentMut) -> Vec<&'static Deprecation> {
    let mut found = Vec::new();
    if version_names_source(doc.get("version"))
        && let Some(deprecation) = entry("[version]", "source")
    {
        found.push(deprecation);
    }
    for kind in manifest_kinds(doc.get("manifest")) {
        let spelling = format!("kind = \"{kind}\"");
        if let Some(deprecation) = entry("[[manifest]]", &spelling)
            && !found.contains(&deprecation)
        {
            found.push(deprecation);
        }
    }
    found
}

/// True when `[version]` names its source with the legacy key, in either table form.
fn version_names_source(item: Option<&Item>) -> bool {
    let table = match item {
        Some(Item::Table(table)) => table,
        Some(Item::Value(Value::InlineTable(table))) => return table.contains_key("source"),
        _ => return false,
    };
    table.contains_key("source")
}

/// Every `kind` value the document declares, for both `[[manifest]]` tables and the inline array.
fn manifest_kinds(item: Option<&Item>) -> Vec<String> {
    match item {
        Some(Item::ArrayOfTables(tables)) => tables.iter().filter_map(kind_of_table).collect(),
        Some(Item::Value(Value::Array(entries))) => entries.iter().filter_map(kind_of_value).collect(),
        Some(Item::Value(Value::InlineTable(single))) => {
            single.get("kind").and_then(kind_of_value).into_iter().collect()
        }
        _ => Vec::new(),
    }
}

fn kind_of_table(table: &Table) -> Option<String> {
    match table.get("kind")? {
        Item::Value(value) => kind_of_value(value),
        _ => None,
    }
}

fn kind_of_value(value: &Value) -> Option<String> {
    match value {
        Value::String(kind) => Some(kind.value().to_string()),
        Value::InlineTable(table) => table.get("kind").and_then(kind_of_value),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_entry_names_its_surface_replacement_and_removal() {
        for deprecation in DEPRECATIONS {
            let message = deprecation.message();
            assert!(message.starts_with(deprecation.surface), "{message}");
            assert!(message.contains(deprecation.spelling), "{message}");
            assert!(message.contains(deprecation.canonical), "{message}");
            assert!(
                message.ends_with(deprecation.removal),
                "a deprecation without a removal version is a warning nobody can act on: {message}"
            );
        }
    }

    #[test]
    fn a_spelling_mentioned_only_in_a_comment_is_not_a_deprecation() {
        let document = r#"# source = "package.json" is the old spelling
[version]
current_source = "package.json"
"#;
        assert!(deprecations_in(document).unwrap().is_empty());
    }
}
