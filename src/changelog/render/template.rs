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

/// Heading of the section that holds the breaking entries, and of no other section.
///
/// Breaking is not a conventional type, so [`SECTIONS`] cannot list it; it is published from here
/// so a template never learns a second label from anywhere else.
pub(crate) const BREAKING_SECTION_HEADING: &str = "⚠️ Breaking Changes";

/// Maps a conventional type to its section label and that label's rank.
///
/// The rank belongs to the label, not to the type: every Maintenance type shares one heading, so a
/// section sorts where the first of its types appears instead of splitting per literal type.
pub(crate) fn section_for(type_str: &str) -> (usize, String) {
    let lower = type_str.to_ascii_lowercase();
    match SECTIONS.iter().position(|(known, _)| *known == lower) {
        Some(index) => {
            let label = SECTIONS[index].1;
            let rank = SECTIONS
                .iter()
                .position(|(_, known_label)| *known_label == label)
                .unwrap_or(index);
            (rank, label.to_string())
        }
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
    let mut sections: BTreeMap<String, (usize, Vec<Value>)> = BTreeMap::new();

    for item in seq {
        let (rank, label) = section_for(&type_attr(&item));
        sections.entry(label).or_insert_with(|| (rank, Vec::new())).1.push(item);
    }

    let mut res: Vec<(usize, String, Vec<Value>)> = sections
        .into_iter()
        .map(|(label, (rank, items))| (rank, label, items))
        .collect();
    res.sort_by_key(|(rank, label, _)| (*rank, label.to_ascii_lowercase()));
    Value::from_serialize(
        res.into_iter()
            .map(|(_, label, items)| (label, items))
            .collect::<Vec<_>>(),
    )
}

/// Create a configured MiniJinja environment with custom globals.
pub fn create_environment() -> minijinja::Environment<'static> {
    create_environment_with_env_lookup(env_global)
}

/// Create the same environment with the `env()` global resolved through `lookup`.
///
/// The process environment is read at exactly one place, [`create_environment`]; everything below
/// the boundary takes the lookup as an argument, so a rendering test can state what the environment
/// says without writing to a variable every other thread in the test binary can read (#352).
///
/// `env()` takes a variable name and an optional default (#358): one argument renders an absent
/// variable as undefined, which is what shipped, and two render the default instead.
pub fn create_environment_with_env_lookup<F>(lookup: F) -> minijinja::Environment<'static>
where
    F: Fn(&str) -> Option<String> + Send + Sync + 'static,
{
    let mut env = minijinja::Environment::new();
    env.set_auto_escape_callback(|_| minijinja::AutoEscape::None);
    let lookup = std::sync::Arc::new(lookup);
    env.add_function("env", move |args: minijinja::value::Rest<minijinja::Value>| {
        env_from_lookup(&*lookup, &args)
    });

    env.add_filter("group_by_scope", group_by_scope_filter);
    env.add_filter("group_by_type", group_by_type_filter);
    env.add_filter("group_by_section", group_by_section_filter);
    env
}

/// The `env()` global behind the environment above: `<name>` plus an optional `<default>`.
///
/// The default applies **only** when the variable is absent. A set variable always wins, including
/// an empty one, because the lookup already treats presence as the question it answers. Arity and
/// types are checked here rather than left to miniJinja, so a template written against a different
/// engine gets a message naming the call instead of a coercion.
fn env_from_lookup<F>(
    lookup: &F,
    args: &minijinja::value::Rest<minijinja::Value>,
) -> Result<minijinja::Value, minijinja::Error>
where
    F: Fn(&str) -> Option<String> + ?Sized,
{
    let missing = || {
        minijinja::Error::new(
            minijinja::ErrorKind::MissingArgument,
            "env() needs a variable name: `env(\"NAME\")` or `env(\"NAME\", \"default\")`",
        )
    };
    let name = args.0.first().ok_or_else(missing)?;
    let name = name.as_str().ok_or_else(|| {
        minijinja::Error::new(
            minijinja::ErrorKind::InvalidOperation,
            "env() takes the variable name as a string",
        )
    })?;

    match args.0.len() {
        1 => Ok(lookup(name).map_or(minijinja::Value::UNDEFINED, minijinja::Value::from)),
        2 => Ok(match lookup(name) {
            Some(value) => minijinja::Value::from(value),
            None => args.0[1].clone(),
        }),
        given => Err(minijinja::Error::new(
            minijinja::ErrorKind::TooManyArguments,
            format!("env() takes a variable name and an optional default, {given} arguments given"),
        )),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The `env()` global reads through the injected lookup, never the process: `cargo test --lib`
    /// runs every unit test as a thread of one binary, so a rendering fixture must not write a key
    /// another test reads (#352).
    #[test]
    fn the_env_global_reads_from_the_injected_lookup() {
        let render = |lookup: fn(&str) -> Option<String>| {
            create_environment_with_env_lookup(lookup)
                .render_str(r#"{{ env("CUTVER_TEST_ENV_VAR") }}"#, minijinja::Value::UNDEFINED)
                .expect("the env global renders")
        };

        fn keeps(key: &str) -> Option<String> {
            (key == "CUTVER_TEST_ENV_VAR").then(|| "cutver_awesome_val".to_string())
        }

        assert_eq!(render(keeps), "cutver_awesome_val");

        // Control: the same template through a lookup that answers nothing. An absent key renders as
        // minijinja's `None` because the global returns `Option`, which is the behaviour this seam
        // leaves untouched; what the control proves is that the value above came from the lookup and
        // not from whatever the process happens to hold.
        assert_ne!(render(|_| None), "cutver_awesome_val");
    }

    /// #358: the authoring reference publishes `env("VAR", "default")`, and that page ships inside
    /// the crates.io tarball, so a call the engine rejects is a defect in every published artefact
    /// rather than a stale website.
    #[test]
    fn the_env_global_takes_an_optional_default() {
        let render = |lookup: fn(&str) -> Option<String>| {
            create_environment_with_env_lookup(lookup)
                .render_str(
                    r#"{{ env("CUTVER_TEST_ENV_VAR", "fallback") }}"#,
                    minijinja::Value::UNDEFINED,
                )
                .expect("the env global renders with a default")
        };

        fn keeps(key: &str) -> Option<String> {
            (key == "CUTVER_TEST_ENV_VAR").then(|| "cutver_awesome_val".to_string())
        }

        assert_eq!(
            render(keeps),
            "cutver_awesome_val",
            "a set variable wins over the default"
        );
        assert_eq!(render(|_| None), "fallback", "an absent variable renders the default");
    }

    /// S1: arity and type are checked by the global, not left to the engine, so a template copied
    /// from another ecosystem fails naming the call it wrote rather than with a coercion.
    #[test]
    fn the_env_global_rejects_a_call_it_cannot_answer() {
        let render = |template: &str| {
            create_environment_with_env_lookup(|_| None)
                .render_str(template, minijinja::Value::UNDEFINED)
                .expect_err("the call is refused")
                .to_string()
        };

        let too_many = render(r#"{{ env("A", "b", "c") }}"#);
        assert!(
            too_many.contains("env() takes a variable name and an optional default"),
            "{too_many}"
        );

        let no_args = render("{{ env() }}");
        assert!(no_args.contains("env() needs a variable name"), "{no_args}");

        let not_a_string = render("{{ env(1) }}");
        assert!(
            not_a_string.contains("env() takes the variable name as a string"),
            "{not_a_string}"
        );
    }
}
