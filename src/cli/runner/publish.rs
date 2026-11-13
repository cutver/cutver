//! `cutver publish`: the closing phase of a release prepared by `bump --defer-publish` (#259).

use crate::bump::{self, PublishRequest, PublishSummary};
use crate::cli::style::Theme;
use crate::cli::tree;
use crate::config;

use super::dispatch::print_error;

pub fn run_publish(config: &config::Config, request: PublishRequest) -> i32 {
    match bump::run_publish(config, &request) {
        Ok(summary) => {
            print_publish_summary(&summary);
            0
        }
        Err(e) => {
            print_error(e);
            1
        }
    }
}

fn plan_header(summary: &PublishSummary, theme: &Theme) -> String {
    let (icon, title) = if summary.dry_run {
        (theme.info_icon(), "Publish Plan")
    } else {
        (theme.success_icon(), "Published")
    };
    let tag = theme.accent(&summary.tag);
    let commit = theme.muted(&summary.commit);
    format!("{icon} {title}: {tag} at {commit}")
}

/// One entry per thing the closing phase did, in the order `publish::finalize` ran them.
fn publish_sections(summary: &PublishSummary, theme: &Theme) -> Vec<(&'static str, Vec<String>)> {
    let mut sections = vec![("Tag", vec![theme.accent(&summary.tag)])];
    if let Some(floating) = &summary.floating_tag {
        sections.push(("Floating tag", vec![theme.accent(floating)]));
    }
    if let Some(command) = &summary.publish_push_command {
        let pushes = command.split(" && ").map(str::to_string).collect();
        sections.push(("Push", pushes));
    }
    if !summary.publish_commands.is_empty() {
        sections.push(("Commands", summary.publish_commands.clone()));
    }
    sections
}

/// Renders the closing phase as a tree, using the same glyphs as the release plan.
pub fn render_publish_plan(summary: &PublishSummary, theme: &Theme) -> String {
    let mut out = plan_header(summary, theme);
    let sections = publish_sections(summary, theme);
    let total = sections.len();
    for (index, (title, items)) in sections.iter().enumerate() {
        for line in tree::render_section_with_items(title, items, index + 1 == total) {
            out.push('\n');
            out.push_str(&line);
        }
    }
    out
}

pub fn print_publish_summary(summary: &PublishSummary) {
    let theme = Theme::stdout();
    if summary.dry_run {
        println!("{}\n", tree::render_simulation_banner(&theme));
    }
    println!("{}", render_publish_plan(summary, &theme));
}
