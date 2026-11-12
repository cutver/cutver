use crate::changelog::Error;
use crate::changelog::context::{InterpolationContext, ReleaseContext};
use minijinja::Value;
use std::collections::BTreeMap;

fn env_global(key: &str) -> Option<String> {
    std::env::var(key).ok()
}

fn group_by_scope_filter(seq: Vec<Value>) -> Value {
    let mut named_groups: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    let mut other_items: Vec<Value> = Vec::new();

    for item in seq {
        let scope_str = item.get_attr("scope").ok().and_then(|v| v.as_str().map(String::from));
        match scope_str {
            Some(s) if !s.trim().is_empty() => {
                named_groups.entry(s).or_default().push(item);
            }
            _ => {
                other_items.push(item);
            }
        }
    }

    let mut result: Vec<(String, Vec<Value>)> = named_groups.into_iter().collect();
    if !other_items.is_empty() {
        result.push((String::new(), other_items));
    }
    Value::from_serialize(result)
}

/// Canonical order for the type sections a template renders.
///
/// A release should read the same way every time, so sections follow this order instead of the
/// order the commits happened to land in. A type outside the list sorts alphabetically after
/// these, and each group keeps the casing it arrived with.
const TYPE_ORDER: &[&str] = &[
    "feat", "fix", "perf", "refactor", "docs", "chore", "build", "ci", "test", "style", "revert",
];

fn type_sort_key(type_str: &str) -> (usize, String) {
    let lower = type_str.to_ascii_lowercase();
    let rank = TYPE_ORDER
        .iter()
        .position(|known| *known == lower)
        .unwrap_or(usize::MAX);
    (rank, lower)
}

fn type_attr(item: &Value) -> String {
    item.get_attr("type")
        .ok()
        .or_else(|| item.get_attr("commit_type").ok())
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default()
}

fn group_by_type_filter(seq: Vec<Value>) -> Value {
    let mut groups: BTreeMap<String, Vec<Value>> = BTreeMap::new();

    for item in seq {
        let type_str = type_attr(&item);

        groups.entry(type_str).or_default().push(item);
    }

    let mut res: Vec<(String, Vec<Value>)> = groups.into_iter().collect();
    res.sort_by_key(|(type_str, _)| type_sort_key(type_str));
    Value::from_serialize(res)
}

/// The canonical release sections, in the order a release reads them.
///
/// `chore`, `build`, `ci`, `test`, `style` and `revert` are one Maintenance section: grouping by
/// literal type would emit that heading once per type, which is what a release must not do.
const SECTIONS: &[(&str, &str)] = &[
    ("feat", "✨ Features"),
    ("fix", "🐛 Fixes"),
    ("perf", "⚡ Performance"),
    ("refactor", "🔄 Refactoring"),
    ("docs", "📚 Documentation"),
    ("chore", "🛠️ Maintenance"),
    ("build", "🛠️ Maintenance"),
    ("ci", "🛠️ Maintenance"),
    ("test", "🛠️ Maintenance"),
    ("style", "🛠️ Maintenance"),
    ("revert", "🛠️ Maintenance"),
];

/// Maps a conventional type to its section label and the section's rank.
fn section_for(type_str: &str) -> (usize, String) {
    let lower = type_str.to_ascii_lowercase();
    match SECTIONS.iter().position(|(known, _)| *known == lower) {
        Some(index) => (index, SECTIONS[index].1.to_string()),
        None => (usize::MAX, capitalise(type_str)),
    }
}

fn capitalise(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn group_by_section_filter(seq: Vec<Value>) -> Value {
    let mut sections: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    let mut ranks: BTreeMap<String, usize> = BTreeMap::new();

    for item in seq {
        let (rank, label) = section_for(&type_attr(&item));
        ranks.entry(label.clone()).or_insert(rank);
        sections.entry(label).or_default().push(item);
    }

    let mut res: Vec<(String, Vec<Value>)> = sections.into_iter().collect();
    res.sort_by_key(|(label, _)| (ranks[label], label.to_ascii_lowercase()));
    Value::from_serialize(res)
}

/// Create a configured MiniJinja environment with custom globals.
pub fn create_environment() -> minijinja::Environment<'static> {
    let mut env = minijinja::Environment::new();
    env.set_auto_escape_callback(|_| minijinja::AutoEscape::None);
    env.add_function("env", env_global);
    env.add_filter("group_by_scope", group_by_scope_filter);
    env.add_filter("group_by_type", group_by_type_filter);
    env.add_filter("group_by_section", group_by_section_filter);
    env
}

/// Helper to normalize legacy `{version}` and `{tag}` tokens to MiniJinja syntax `{{ version }}` and `{{ tag }}`.
///
/// Only standalone single braces are converted: occurrences of `{{` or `}}` are preserved intact.
pub fn normalize_legacy_tokens(template: &str) -> String {
    let mut result = String::with_capacity(template.len() + 16);
    let chars: Vec<char> = template.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        if chars[i] == '{' {
            if i + 1 < len && chars[i + 1] == '{' {
                // Double opening brace `{{`, preserve as-is
                result.push('{');
                result.push('{');
                i += 2;
                continue;
            }

            // Check if this is `{version}`
            let remaining: String = chars[i..].iter().collect();
            if remaining.starts_with("{version}") {
                result.push_str("{{ version }}");
                i += "{version}".len();
                continue;
            } else if remaining.starts_with("{tag}") {
                result.push_str("{{ tag }}");
                i += "{tag}".len();
                continue;
            } else {
                result.push(chars[i]);
                i += 1;
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    result
}

/// Interpolate a string using MiniJinja with an `InterpolationContext`.
///
/// Supports backward compatibility with `{version}` and `{tag}` tokens, converting them
/// automatically to MiniJinja expressions. If template rendering fails, returns `Error::TemplateRender`
/// without swallowing errors.
pub fn interpolate_string(template: &str, context: &InterpolationContext) -> Result<String, Error> {
    let normalized = normalize_legacy_tokens(template);
    let env = create_environment();
    let val = minijinja::Value::from_serialize(context);
    env.render_str(&normalized, val).map_err(|e| Error::TemplateRender {
        detail: format!("{e:#}"),
    })
}

/// Render release notes using a MiniJinja template string and release context.
pub fn render_template(template_str: &str, context: &ReleaseContext) -> Result<String, Error> {
    let rendered = render_source(template_str, context)?;
    Ok(rendered.trim().to_string())
}

/// Render a template for **source generation**, keeping the output byte-exact.
///
/// [`render_template`] trims, which is right for a changelog body and wrong for a generated
/// source file: a `version.h` or `version.ts` must be reproducible byte for byte, including its
/// trailing newline. Both entry points share [`create_environment`], so filters and globals cannot
/// drift apart.
pub fn render_source(template_str: &str, context: &ReleaseContext) -> Result<String, Error> {
    let mut env = create_environment();
    // MiniJinja drops a template's trailing newline by default, as Jinja2 does. A generated source
    // file must end exactly where its template says it ends, so the newline is kept here and only
    // here: the changelog path trims anyway and is unaffected.
    env.set_keep_trailing_newline(true);
    let val = minijinja::Value::from_serialize(context);
    env.render_str(template_str, val).map_err(|e| Error::TemplateRender {
        detail: format!("{e:#}"),
    })
}
