use crate::cli::style::Theme;

/// Formats and prints changelog output.
///
/// When styling is disabled (e.g. non-TTY, piped, redirected, or `NO_COLOR=1`),
/// emits the content unmodified (100% byte-for-byte raw Markdown).
///
/// When styling is enabled, renders headings, categories, bullet items, and scopes
/// with clean ANSI terminal colors.
pub fn print_changelog_output(content: &str, theme: &Theme) {
    if !theme.is_enabled() {
        println!("{content}");
        return;
    }

    for line in content.lines() {
        let styled = style_markdown_line(line, theme);
        println!("{styled}");
    }
}

/// Styles a single line of markdown for terminal display.
pub fn style_markdown_line(line: &str, theme: &Theme) -> String {
    if !theme.is_enabled() {
        return line.to_string();
    }

    if let Some(rest) = line.strip_prefix("## ") {
        return format_version_heading(rest, theme);
    }
    if let Some(rest) = line.strip_prefix("### ") {
        return format_category_heading(rest, theme);
    }
    if let Some(rest) = line.strip_prefix("- ") {
        return format_bullet_line(rest, theme);
    }
    if let Some(rest) = line.strip_prefix("* ") {
        return format_bullet_line(rest, theme);
    }
    line.to_string()
}

/// Formats a level-2 version heading: `## [vX.Y.Z] - YYYY-MM-DD` or `## vX.Y.Z`.
fn format_version_heading(heading: &str, theme: &Theme) -> String {
    let bold_accent = theme.accent(theme.bold(heading));
    format!("  {bold_accent}")
}

/// Formats a level-3 category heading: `### <Emoji> <Name>`.
fn format_category_heading(heading: &str, theme: &Theme) -> String {
    let bold_category = theme.bold(heading);
    format!("    {bold_category}")
}

/// Formats a list item: `- **scope**: description (#123)`.
fn format_bullet_line(item: &str, theme: &Theme) -> String {
    let bullet = theme.bullet();
    let body = format_item_body(item, theme);
    format!("      {bullet} {body}")
}

/// Formats the body of a bullet item, highlighting scope and dimming issue references.
fn format_item_body(body: &str, theme: &Theme) -> String {
    let (prefix, remainder) = split_scope(body, theme);
    let dimmed_remainder = dim_issue_references(&remainder, theme);
    format!("{prefix}{dimmed_remainder}")
}

/// Splits and styles an optional bold scope like `**scope**: ` or `**scope:** `.
fn split_scope(body: &str, theme: &Theme) -> (String, String) {
    let Some(after_first) = body.strip_prefix("**") else {
        return (String::new(), body.to_string());
    };
    let Some((scope_text, rest)) = after_first.split_once("**") else {
        return (String::new(), body.to_string());
    };

    let styled_scope = theme.accent(theme.bold(scope_text));
    (styled_scope, rest.to_string())
}

/// Dims issue / pull request references at the end or within the line, e.g. `(#123)`.
fn dim_issue_references(text: &str, theme: &Theme) -> String {
    let Some(open_idx) = text.rfind("(#") else {
        return text.to_string();
    };
    let Some(close_rel) = text[open_idx..].find(')') else {
        return text.to_string();
    };

    let close_idx = open_idx + close_rel + 1;
    let before = &text[..open_idx];
    let issue_part = &text[open_idx..close_idx];
    let after = &text[close_idx..];

    let dimmed_issue = theme.muted(issue_part);
    format!("{before}{dimmed_issue}{after}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uncolored_theme_returns_unmodified_line() {
        let theme = Theme::new(false);
        let line = "## [1.2.0] - 2026-03-01";
        assert_eq!(style_markdown_line(line, &theme), line);
    }

    #[test]
    fn test_version_heading_styled() {
        let theme = Theme::new(true);
        let line = "## [1.2.0] - 2026-03-01";
        let styled = style_markdown_line(line, &theme);
        assert!(styled.contains("\x1b[36m\x1b[1m[1.2.0] - 2026-03-01\x1b[0m\x1b[0m"));
    }

    #[test]
    fn test_category_heading_styled() {
        let theme = Theme::new(true);
        let line = "### Features";
        let styled = style_markdown_line(line, &theme);
        assert!(styled.contains("\x1b[1mFeatures\x1b[0m"));
    }

    #[test]
    fn test_bullet_with_scope_and_issue_styled() {
        let theme = Theme::new(true);
        let line = "- **cli**: add changelog styling (#72)";
        let styled = style_markdown_line(line, &theme);
        assert!(styled.contains("\x1b[36m\x1b[1mcli\x1b[0m\x1b[0m"));
        assert!(styled.contains("\x1b[90m(#72)\x1b[0m"));
    }

    #[test]
    fn test_bullet_without_scope() {
        let theme = Theme::new(true);
        let line = "- simple change without scope";
        let styled = style_markdown_line(line, &theme);
        assert!(styled.contains("simple change without scope"));
    }
}
