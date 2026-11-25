use crate::bump::Summary;
use crate::bump::plan::ReleasePlan;
use crate::cli::path::relativize_path;
use crate::cli::style::Theme;
use crate::semver_bump;

/// Renders the simulation mode banner box for `--dry-run`.
pub fn render_simulation_banner(theme: &Theme) -> String {
    let title = theme.bold("SIMULATION MODE");
    let top = format!("┌─ {title} {}┐", theme.warning("─".repeat(50)));
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
    let title = format!(
        "Release Plan: {transition} ({bump_desc}{})",
        channel_transition(summary)
    );
    format!("{icon} {title}")
}

/// The channel the transition leaves and the one it lands on, in words, or nothing at all when they
/// are the same: approving a release on a channel is not reading suffixes by eye (#326 D3).
///
/// It reads the two versions the header already prints, so a report about a version the tool did not
/// cut is never relabelled by it.
fn channel_transition(summary: &Summary) -> String {
    let from = semver_bump::channel_of(&summary.current);
    let to = semver_bump::channel_of(&summary.next);
    if from == to {
        String::new()
    } else {
        format!(", channel: {from} -> {to}")
    }
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
        let display_path = relativize_path(&t.path, summary.root_dir.as_deref());
        lines.push(format!(
            "{child_prefix}{leaf}{display_path} ({} {arrow} {target})",
            t.old
        ));
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
    let display_path = relativize_path(cl, summary.root_dir.as_deref());
    vec![
        format!("{prefix}Changelog"),
        format!("{child_prefix}{leaf}{display_path} (prepends {target})"),
    ]
}

fn render_git_node(summary: &Summary, theme: &Theme, is_last: bool) -> Vec<String> {
    let mut items = Vec::new();
    let skipped = if summary.no_op() {
        format!(" {}", theme.muted("[SKIPPED]"))
    } else {
        String::new()
    };
    items.push(format!("commit: {}{skipped}", summary.commit_message));
    items.push(render_tag_line(summary, theme));
    if let Some(ref ft) = summary.floating_tag {
        items.push(format!("floating tag: {}", theme.accent(ft)));
    }
    render_section_with_items("Git", &items, is_last)
}

/// States what actually exists: a tag that was created, one a dry run skipped, or one a deferred
/// run left to `cutver publish` (#259).
fn render_tag_line(summary: &Summary, theme: &Theme) -> String {
    let tag = theme.accent(&summary.tag);
    match (summary.deferred, summary.tag_skipped) {
        (true, _) => format!("tag: {tag} {}", theme.muted("[DEFERRED]")),
        (false, true) => format!("tag: {tag} {}", theme.muted("[SKIPPED]")),
        (false, false) => format!("tag: {tag}"),
    }
}

/// Renders one tree section: a title followed by its leaves. Shared by the release plan and the
/// `cutver publish` outcome so both use the same glyphs.
pub(crate) fn render_section_with_items(title: &str, items: &[String], is_last: bool) -> Vec<String> {
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

fn render_publish_node(summary: &Summary, theme: &Theme, is_last: bool) -> Vec<String> {
    let has_push = summary.publish_push || summary.publish_push_command.is_some();
    let has_cmds = !summary.publish_commands.is_empty();
    if !has_push && !has_cmds && !summary.deferred {
        return Vec::new();
    }

    let mut lines = Vec::new();
    lines.push(format!("{}Publish", branch_prefix(is_last)));
    let child_prefix = child_indent(is_last);

    if summary.deferred {
        append_deferred_publication(summary, theme, child_prefix, &mut lines);
        return lines;
    }

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

/// The deferred run did push the branch, and names the command that finishes the release, so the
/// plan never looks like a release that simply skipped its tag (#259).
fn append_deferred_publication(summary: &Summary, theme: &Theme, child_prefix: &str, lines: &mut Vec<String>) {
    if let Some(cmd) = &summary.publish_push_command {
        lines.push(format!("{child_prefix}├── push: {cmd}"));
    } else if summary.publish_push {
        lines.push(format!("{child_prefix}├── push: true"));
    }
    let finish = format!("cutver publish --tag {}", summary.tag);
    lines.push(format!("{child_prefix}└── deferred: {}", theme.accent(&finish)));
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
    let has_publish = summary.publish_push
        || summary.publish_push_command.is_some()
        || !summary.publish_commands.is_empty()
        || summary.deferred;

    let mut sections = Vec::new();
    if summary.no_op() {
        sections.push(render_no_op_node(summary, theme, false));
    }
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

fn render_no_op_node(summary: &Summary, theme: &Theme, is_last: bool) -> Vec<String> {
    if !summary.no_op() {
        return Vec::new();
    }
    render_section_with_items(
        "No-op",
        &[format!(
            "nothing was written, committed or tagged: {} is the version the request yields",
            theme.accent(&summary.next)
        )],
        is_last,
    )
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
    for warning in &summary.warnings {
        crate::cli::style::warn_degradation(warning);
    }
    if summary.dry_run {
        println!("{}\n", render_simulation_banner(&theme));
    }
    println!("{}", render_release_plan(summary, &theme));
}

/// Renders `cutver plan`: the very tree the dry run prints, because it is the very release, then the
/// blockers a real `bump` would abort on (#326 S7).
///
/// Reusing [`render_release_plan`] instead of a second renderer is what keeps a plan and a rehearsal
/// from describing the same release differently; the blockers are the one thing a dry run cannot
/// print, because in a dry run they are an abort.
pub(crate) fn render_plan(plan: &ReleasePlan, theme: &Theme) -> String {
    // The plan reports the tag it would not create, exactly as the run would: one renderer, two
    // readers, and a no-op is not a release that happens to skip a tag (#326 D1a).
    let mut summary = plan.summary();
    summary.tag_skipped = plan.no_op();
    let mut out = render_release_plan(&summary, theme);
    out.push_str(&format!(
        "\n\n{}",
        theme.muted("Nothing was written, staged, committed, tagged or pushed.")
    ));
    if plan.blockers.is_empty() {
        return out;
    }
    out.push_str(&format!(
        "\n\n{}",
        theme.warning(format!(
            "A real bump would abort on {} blocker(s):",
            plan.blockers.len()
        ))
    ));
    for blocker in &plan.blockers {
        out.push_str(&format!("\n  {} {blocker}", theme.error("•")));
    }
    out.push_str(&format!(
        "\n\n{}",
        theme.muted("`cutver plan` exits 0 anyway: the release decision is the caller's.")
    ));
    out
}

/// Prints the plan tree, its degradation warnings and its blockers.
pub(crate) fn print_plan(plan: &ReleasePlan) {
    let theme = Theme::stdout();
    for warning in &plan.warnings {
        crate::cli::style::warn_degradation(warning);
    }
    println!("{}", render_plan(plan, &theme));
}

#[cfg(test)]
mod tests;
