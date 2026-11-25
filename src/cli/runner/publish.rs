//! `cutver publish`: the closing phase of a release prepared by `bump --defer-publish` (#259).

use std::path::Path;

use thiserror::Error;

use crate::bump::{self, PublishRequest, PublishSummary};
use crate::cli::style::Theme;
use crate::cli::tree;
use crate::config;
use crate::plugin::PluginOperation;

use super::dispatch::print_error;
use crate::cli::machine::FailureCode;

#[cfg(test)]
mod tests;

pub fn run_publish(config: &config::Config, request: PublishRequest) -> i32 {
    // Refused before the clean-tree guard runs: the guard exempts a declared directory from the
    // tree, because an input is not a change in the tree, and only the request can say whether the
    // path was meant to be notes. A directory can never be.
    if let Some(path) = request.notes_file.as_deref()
        && let Err(rejected) = NotesInput::parse(path)
    {
        print_error(FailureCode::Publish, rejected);
        return 1;
    }
    match bump::run_publish(config, &request) {
        Ok(summary) => {
            // A resume dispatches no `on_post_release` plugin, so the tree names the ones it left
            // alone instead of letting the omission pass silently (#326 D2).
            let left_alone = request
                .resume
                .then(|| left_alone_line(&summary.post_release_left_alone));
            print_publish_summary(&summary, left_alone.as_deref());
            crate::cli::ci::on_publish_success(&summary);
            0
        }
        Err(e) => {
            print_error(FailureCode::Publish, e);
            1
        }
    }
}

/// Why `cutver publish` refused the path it was handed as release notes.
///
/// `--notes-file` names one file the closing phase reads and hands to the `on_post_release` plugins.
/// A path that names a directory is not that file: reading it can only fail, and exempting it from
/// the clean-tree guard would hide every change inside it, which is the opposite of what the
/// exemption exists for (#302, #326 S10).
#[derive(Debug, Error)]
enum NotesInput {
    #[error(
        "`--notes-file` names a directory, not a notes file: '{path}'\n  Where: `cutver publish`, before the clean-tree check and before the tag, the pushes and the publish commands\n  Fix: pass the release-notes file itself (`--notes-file RELEASE_NOTES.md`), or `--notes-file -` to read the notes from standard input. A directory is exempt from the clean-tree check as an input, so accepting it would hide the work inside it. Nothing was tagged, pushed or executed."
    )]
    Directory { path: String },
}

impl NotesInput {
    /// Accepts the path the caller declared, unless it names a directory that exists right now.
    fn parse(path: &Path) -> Result<(), Self> {
        if path.is_dir() {
            return Err(Self::Directory {
                path: path.display().to_string(),
            });
        }
        Ok(())
    }
}

fn plan_header(summary: &PublishSummary, theme: &Theme, resume: Option<&str>) -> String {
    let resumed = resume.is_some();
    let (icon, title) = if summary.dry_run {
        let title = if resumed { "Resume Plan" } else { "Publish Plan" };
        (theme.info_icon(), title)
    } else if resumed {
        // A resume repeats the closing steps of a release an earlier run started; it is not a fresh
        // publication, so it says what it is instead of claiming one (#326 S4, D2).
        (theme.success_icon(), "Resumed")
    } else {
        (theme.success_icon(), "Published")
    };
    let tag = theme.accent(&summary.tag);
    let commit = theme.muted(&summary.commit);
    format!("{icon} {title}: {tag} at {commit}")
}

/// One entry per thing the closing phase did, in the order `publish::finalize` ran them.
///
/// A resume adds the one section its evidence cannot show: which steps were repeated because
/// repeating them is harmless, what that means for a tag, a push or a floating major tag that was
/// already in place, and which `on_post_release` plugins it left alone (#326 D2).
fn publish_sections(summary: &PublishSummary, theme: &Theme, resume: Option<&str>) -> Vec<(&'static str, Vec<String>)> {
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
    if let Some(left_alone) = resume {
        sections.push((
            "Resume",
            vec![
                "repeated only the steps that are harmless to repeat: the release tag, its push, the floating major tag and the gates".to_string(),
                "a tag, a push or a floating major tag already in place was left as it was, and no `[publish] commands` entry ran without its `publish-<n>` attestation".to_string(),
                left_alone.to_string(),
            ],
        ));
    }
    sections
}

/// The `Resume` line naming the `on_post_release` plugins this run left alone (#326 D2).
///
/// The names come from the summary: the dispatch is the one place that knows the phase, so it returns
/// what it skipped instead of the runner loading a second manager to ask the same question.
fn left_alone_line(left_alone: &[String]) -> String {
    let event = PluginOperation::OnPostRelease.as_str();
    if left_alone.is_empty() {
        return format!("`{event}` not dispatched: this workspace declares no plugin for that event");
    }
    let mut names = left_alone.to_vec();
    names.sort();
    let named: Vec<String> = names.iter().map(|name| format!("`{name}`")).collect();
    format!(
        "`{event}` not dispatched: {} left alone, because a plugin may already have run in the attempt this resume continues (`cutver publish` without `--resume` dispatches it)",
        named.join(", ")
    )
}

/// Renders the closing phase as a tree, using the same glyphs as the release plan.
pub fn render_publish_plan(summary: &PublishSummary, theme: &Theme, resume: Option<&str>) -> String {
    let mut out = plan_header(summary, theme, resume);
    let sections = publish_sections(summary, theme, resume);
    let total = sections.len();
    for (index, (title, items)) in sections.iter().enumerate() {
        for line in tree::render_section_with_items(title, items, index + 1 == total) {
            out.push('\n');
            out.push_str(&line);
        }
    }
    out
}

pub fn print_publish_summary(summary: &PublishSummary, resume: Option<&str>) {
    let theme = Theme::stdout();
    if summary.dry_run {
        println!("{}\n", tree::render_simulation_banner(&theme));
    }
    println!("{}", render_publish_plan(summary, &theme, resume));
}
