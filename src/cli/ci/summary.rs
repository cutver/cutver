//! The single markdown block a run appends to `$GITHUB_STEP_SUMMARY` (S3).

use super::fields::Field;
use super::text::code_span;

/// Renders the block: a heading, then one bullet per field the run has.
///
/// Every value goes through [`code_span`], so it can neither close its own span nor open a line, and
/// the bullet list is the run's fixed field list, so the block is bounded however long the material
/// behind a value was. A changelog entry is never copied here: the summary is a brief, not a dump.
pub fn render(title: &str, fields: &[Field]) -> String {
    let mut block = format!("### {title}\n\n");
    for (key, value) in fields {
        let Some(value) = value else {
            continue;
        };
        if value.trim().is_empty() {
            continue;
        }
        block.push_str("- ");
        block.push_str(key);
        block.push_str(": ");
        block.push_str(&code_span(value));
        block.push('\n');
    }
    block
}
