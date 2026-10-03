use crate::changelog::Error;
use crate::changelog::context::{InterpolationContext, ReleaseContext};

/// Create a configured MiniJinja environment with custom globals.
pub fn create_environment() -> minijinja::Environment<'static> {
    let mut env = minijinja::Environment::new();
    env.set_auto_escape_callback(|_| minijinja::AutoEscape::None);
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
