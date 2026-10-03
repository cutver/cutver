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

fn group_by_type_filter(seq: Vec<Value>) -> Value {
    let mut ordered_types: Vec<String> = Vec::new();
    let mut groups: BTreeMap<String, Vec<Value>> = BTreeMap::new();

    for item in seq {
        let type_str = item
            .get_attr("type")
            .ok()
            .or_else(|| item.get_attr("commit_type").ok())
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_default();

        if !groups.contains_key(&type_str) {
            ordered_types.push(type_str.clone());
        }
        groups.entry(type_str).or_default().push(item);
    }

    let res: Vec<(String, Vec<Value>)> = ordered_types
        .into_iter()
        .filter_map(|t| groups.remove(&t).map(|items| (t, items)))
        .collect();
    Value::from_serialize(res)
}

/// Create a configured MiniJinja environment with custom globals.
pub fn create_environment() -> minijinja::Environment<'static> {
    let mut env = minijinja::Environment::new();
    env.set_auto_escape_callback(|_| minijinja::AutoEscape::None);
    env.add_function("env", env_global);
    env.add_filter("group_by_scope", group_by_scope_filter);
    env.add_filter("group_by_type", group_by_type_filter);
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
    let env = create_environment();
    let val = minijinja::Value::from_serialize(context);
    let rendered = env.render_str(template_str, val).map_err(|e| Error::TemplateRender {
        detail: format!("{e:#}"),
    })?;
    Ok(rendered.trim().to_string())
}
