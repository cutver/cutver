//! The field names `$GITHUB_OUTPUT` receives (S2).
//!
//! They are the names the machine payload already carries, which is what lets the release workflow
//! stop parsing the report: `bump` feeds the keys `.github/workflows/release.yml` extracts with `jq`
//! today, `channel` included, `plan` feeds the four a planning step reads, and `publish` mirrors
//! `bump`'s `tag` and `commit` and adds only what the closing phase knows.

use crate::bump::plan::ReleasePlan;
use crate::bump::{PublishSummary, Summary};
use crate::semver_bump::channel_of;

use super::text::one_line;

/// One field of a run: its documented key and its value, `None` when the run has none.
pub type Field = (&'static str, Option<String>);

/// The keys `bump` feeds, in the order the payload documents them.
///
/// `channel` is the channel the version this run produced carries, read from the version itself
/// ([`channel_of`]) rather than from the flag: a stable cut reports `stable`, and a prerelease reports
/// the identifier in its suffix, so no downstream job has to parse a semver suffix to learn it.
pub fn bump(finished: &Summary) -> Vec<Field> {
    vec![
        ("released", Some(finished.released().to_string())),
        ("deferred", Some(finished.deferred.to_string())),
        ("version", Some(finished.next.to_string())),
        ("channel", Some(channel_of(&finished.next).to_string())),
        ("tag", Some(finished.tag.clone())),
        ("commit", Some(finished.commit.clone())),
    ]
}

/// The keys `plan` feeds, with `channel` meaning what it means in `bump`.
pub fn plan(release: &ReleasePlan) -> Vec<Field> {
    vec![
        ("next_version", Some(release.next_version.to_string())),
        ("channel", Some(channel_of(&release.next_version).to_string())),
        ("tag", Some(release.tag.clone())),
        ("commit", Some(release.commit.clone())),
    ]
}

/// The keys `publish` feeds: `bump`'s `tag` and `commit`, where they mean the same thing, plus what
/// only the closing phase can know.
///
/// The three that report an action are read against `dry_run`, because the summary a `--dry-run`
/// produces still describes the plan it printed: `publish_push_command` and `floating_tag` are filled
/// from the pushes and the tag move it planned, and `publish_commands` lists commands that did not
/// run. A dry run published nothing, so all three say exactly that.
pub fn publish(finished: &PublishSummary) -> Vec<Field> {
    let published = !finished.dry_run;
    let ran = if published { finished.publish_commands.len() } else { 0 };
    vec![
        ("tag", Some(finished.tag.clone())),
        ("commit", Some(finished.commit.clone())),
        (
            "tag_pushed",
            Some((published && finished.publish_push_command.is_some()).to_string()),
        ),
        (
            "floating_tag_moved",
            Some((published && finished.floating_tag.is_some()).to_string()),
        ),
        ("publish_commands_run", Some(ran.to_string())),
    ]
}

/// Renders the `key=value` lines.
///
/// A field the run does not have is omitted rather than written empty (S2) — a `--dry-run` bump
/// created no commit, so it writes no `commit` line — and every value is condensed to one line,
/// because the file is one line per field and a value must not be able to add another.
pub fn render(fields: &[Field]) -> String {
    let mut out = String::new();
    for (key, value) in fields {
        let Some(value) = value else {
            continue;
        };
        let value = one_line(value);
        if value.is_empty() {
            continue;
        }
        out.push_str(key);
        out.push('=');
        out.push_str(&value);
        out.push('\n');
    }
    out
}
