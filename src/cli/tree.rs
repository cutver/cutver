use crate::bump::Summary;
use crate::cli::style::Theme;

/// Renders the simulation mode banner box for `--dry-run`.
pub fn render_simulation_banner(theme: &Theme) -> String {
    let title = format!("{} {}", theme.info_icon(), theme.bold("SIMULATION MODE"));
    let top = format!("┌─ {title} {}┐", theme.warning("─".repeat(48)));
    let middle = format!(
        "│ {}│",
        pad_right("No files, commits, or git tags will be modified.", 67)
    );
    let bottom = format!("└{}┘", theme.warning("─".repeat(68)));

    format!("{top}\n{middle}\n{bottom}")
}

fn pad_right(text: &str, width: usize) -> String {
    let count = width.saturating_sub(text.len());
    format!("{text}{}", " ".repeat(count))
}

fn format_plan_header(summary: &Summary, theme: &Theme) -> String {
    let icon = if summary.dry_run {
        theme.info_icon()
    } else {
        theme.success_icon()
    };
    let bump_desc = match &summary.rationale {
        Some(rat) => rat.summary(),
        None => "bump".to_string(),
    };
    let transition = format!("{} {} {}", summary.current, theme.arrow(), theme.accent(&summary.next));
    let title = format!("Release Plan: {transition} ({bump_desc})");
    format!("{icon} {title}")
}

fn render_manifests_node(summary: &Summary, theme: &Theme, is_last: bool) -> Vec<String> {
    if summary.touched.is_empty() {
        return Vec::new();
    }
    let mut lines = Vec::new();
    let prefix = branch_prefix(is_last);
    let child_prefix = child_indent(is_last);
    lines.push(format!("{prefix}Manifests"));

    let count = summary.touched.len();
    for (i, t) in summary.touched.iter().enumerate() {
        let leaf = branch_prefix(i + 1 == count);
        let arrow = theme.arrow();
        let target = theme.accent(&t.new);
        lines.push(format!("{child_prefix}{leaf}{} ({} {arrow} {target})", t.path, t.old));
    }
    lines
}

fn render_preflight_node(summary: &Summary, theme: &Theme, is_last: bool) -> Vec<String> {
    if summary.preflight.is_empty() {
        return Vec::new();
    }
    let mut lines = Vec::new();
    let prefix = branch_prefix(is_last);
    let child_prefix = child_indent(is_last);
    lines.push(format!("{prefix}Preflight"));

    let count = summary.preflight.len();
    for (i, step) in summary.preflight.iter().enumerate() {
        let leaf = branch_prefix(i + 1 == count);
        let marker = if step.skipped {
            format!(" {}", theme.muted("[SKIPPED]"))
        } else {
            String::new()
        };
        lines.push(format!("{child_prefix}{leaf}{}: {}{marker}", step.name, step.command));
    }
    lines
}

fn render_changelog_node(summary: &Summary, theme: &Theme, is_last: bool) -> Vec<String> {
    let Some(ref cl) = summary.changelog else {
        return Vec::new();
    };
    let prefix = branch_prefix(is_last);
    let child_prefix = child_indent(is_last);
    let leaf = branch_prefix(true);
    let target = theme.accent(&summary.next);
    vec![
        format!("{prefix}Changelog"),
        format!("{child_prefix}{leaf}{cl} (prepends {target})"),
    ]
}

fn render_git_node(summary: &Summary, theme: &Theme, is_last: bool) -> Vec<String> {
    let mut items = Vec::new();
    items.push(format!("commit: {}", summary.commit_message));
    if summary.tag_skipped {
        items.push(format!(
            "tag: {} {}",
            theme.accent(&summary.tag),
            theme.muted("[SKIPPED]")
        ));
    } else {
        items.push(format!("tag: {}", theme.accent(&summary.tag)));
    }
    if let Some(ref ft) = summary.floating_tag {
        items.push(format!("floating tag: {}", theme.accent(ft)));
    }
    render_section_with_items("Git", &items, is_last)
}

fn render_section_with_items(title: &str, items: &[String], is_last: bool) -> Vec<String> {
    if items.is_empty() {
        return Vec::new();
    }
    let mut lines = Vec::new();
    let prefix = branch_prefix(is_last);
    let child_prefix = child_indent(is_last);
    lines.push(format!("{prefix}{title}"));

    let count = items.len();
    for (i, item) in items.iter().enumerate() {
        let leaf = branch_prefix(i + 1 == count);
        lines.push(format!("{child_prefix}{leaf}{item}"));
    }
    lines
}

fn render_publish_node(summary: &Summary, _theme: &Theme, is_last: bool) -> Vec<String> {
    let has_push = summary.publish_push || summary.publish_push_command.is_some();
    let has_cmds = !summary.publish_commands.is_empty();
    if !has_push && !has_cmds {
        return Vec::new();
    }

    let mut lines = Vec::new();
    lines.push(format!("{}Publish", branch_prefix(is_last)));
    let child_prefix = child_indent(is_last);

    let push_leaf = branch_prefix(!has_cmds);
    if let Some(cmd) = &summary.publish_push_command {
        lines.push(format!("{child_prefix}{push_leaf}push: {cmd}"));
    } else if summary.publish_push {
        lines.push(format!("{child_prefix}{push_leaf}push: true"));
    }

    if has_cmds {
        append_publish_commands(&summary.publish_commands, child_prefix, &mut lines);
    }
    lines
}

fn append_publish_commands(commands: &[String], parent_prefix: &str, lines: &mut Vec<String>) {
    lines.push(format!("{parent_prefix}└── commands:"));
    let sub_indent = format!("{parent_prefix}    ");
    let count = commands.len();
    for (i, cmd) in commands.iter().enumerate() {
        let leaf = branch_prefix(i + 1 == count);
        lines.push(format!("{sub_indent}{leaf}{cmd}"));
    }
}

fn branch_prefix(is_last: bool) -> &'static str {
    if is_last { "└── " } else { "├── " }
}

fn child_indent(is_last: bool) -> &'static str {
    if is_last { "    " } else { "│   " }
}

fn collect_tree_sections(summary: &Summary, theme: &Theme) -> Vec<Vec<String>> {
    let has_publish =
        summary.publish_push || summary.publish_push_command.is_some() || !summary.publish_commands.is_empty();

    let mut sections = Vec::new();
    if !summary.touched.is_empty() {
        sections.push(render_manifests_node(summary, theme, false));
    }
    if !summary.preflight.is_empty() {
        sections.push(render_preflight_node(summary, theme, false));
    }
    if summary.changelog.is_some() {
        sections.push(render_changelog_node(summary, theme, false));
    }
    sections.push(render_git_node(summary, theme, !has_publish));
    if has_publish {
        sections.push(render_publish_node(summary, theme, true));
    }
    sections
}

/// Renders the complete release plan tree string.
pub fn render_release_plan(summary: &Summary, theme: &Theme) -> String {
    let mut out = format_plan_header(summary, theme);
    for sec in collect_tree_sections(summary, theme) {
        for line in sec {
            out.push('\n');
            out.push_str(&line);
        }
    }
    out
}

/// Prints the release plan and simulation banner if in dry-run mode.
pub fn print_tree_summary(summary: &Summary) {
    let theme = Theme::stdout();
    if summary.dry_run {
        println!("{}\n", render_simulation_banner(&theme));
    }
    println!("{}", render_release_plan(summary, &theme));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bump::Touched;
    use semver::Version;

    fn dummy_summary() -> Summary {
        Summary {
            source: "Cargo.toml".into(),
            current: Version::new(0, 8, 0),
            next: Version::new(0, 9, 0),
            dry_run: true,
            preflight: vec![crate::preflight::Step {
                name: "tests".into(),
                command: "cargo test".into(),
                timeout: None,
                skipped: false,
            }],
            touched: vec![Touched {
                path: "Cargo.toml".into(),
                old: "0.8.0".into(),
                new: "0.9.0".into(),
            }],
            changelog: Some("CHANGELOG.md".into()),
            commit_message: "chore(release): v0.9.0 [skip ci]".into(),
            tag: "v0.9.0".into(),
            tag_skipped: false,
            floating_tag: None,
            post_bump: None,
            publish_push: true,
            publish_push_command: Some("git push origin main --tags".into()),
            publish_commands: vec!["cargo publish".into()],
            rationale: None,
        }
    }

    #[test]
    fn test_simulation_banner_formatting() {
        let theme = Theme::new(false);
        let banner = render_simulation_banner(&theme);
        assert!(banner.contains("┌─ ℹ SIMULATION MODE ─"));
        assert!(banner.contains("│ No files, commits, or git tags will be modified."));
        assert!(banner.ends_with("┘"));
    }

    #[test]
    fn test_tree_rendering_structure() {
        let theme = Theme::new(false);
        let summary = dummy_summary();
        let plan = render_release_plan(&summary, &theme);

        assert!(plan.contains("Release Plan: 0.8.0 -> 0.9.0"));
        assert!(plan.contains("├── Manifests"));
        assert!(plan.contains("│   └── Cargo.toml (0.8.0 -> 0.9.0)"));
        assert!(plan.contains("├── Preflight"));
        assert!(plan.contains("│   └── tests: cargo test"));
        assert!(plan.contains("├── Changelog"));
        assert!(plan.contains("│   └── CHANGELOG.md (prepends 0.9.0)"));
        assert!(plan.contains("├── Git"));
        assert!(plan.contains("│   ├── commit: chore(release): v0.9.0 [skip ci]"));
        assert!(plan.contains("│   └── tag: v0.9.0"));
        assert!(plan.contains("└── Publish"));
        assert!(plan.contains("    ├── push: git push origin main --tags"));
        assert!(plan.contains("    └── commands:"));
        assert!(plan.contains("        └── cargo publish"));
    }
}
