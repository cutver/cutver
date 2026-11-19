//! The field names `$GITHUB_OUTPUT` receives (S2).
//!
//! They are the names the machine payload already carries, which is what lets the release workflow
//! stop parsing the report: `bump` feeds the five keys `.github/workflows/release.yml` extracts with
//! `jq` today, and `plan` feeds the three a planning step reads.

use crate::bump::Summary;
use crate::bump::plan::ReleasePlan;

use super::text::one_line;

/// One field of a run: its documented key and its value, `None` when the run has none.
pub type Field = (&'static str, Option<String>);

/// The keys `bump` feeds, in the order the payload documents them.
pub fn bump(finished: &Summary) -> Vec<Field> {
    vec![
        ("released", Some((!finished.dry_run).to_string())),
        ("deferred", Some(finished.deferred.to_string())),
        ("version", Some(finished.next.to_string())),
        ("tag", Some(finished.tag.clone())),
        ("commit", Some(finished.commit.clone())),
    ]
}

/// The keys `plan` feeds.
pub fn plan(release: &ReleasePlan) -> Vec<Field> {
    vec![
        ("next_version", Some(release.next_version.to_string())),
        ("tag", Some(release.tag.clone())),
        ("commit", Some(release.commit.clone())),
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
