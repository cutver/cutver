//! The escaping rules the CI channel applies to text cutver did not derive itself (S3).
//!
//! Two sinks need them: the runner's output file is one `key=value` per line, and its step summary is
//! markdown that a changelog entry or a configured tag prefix must not be able to restructure.

/// The longest value the channel writes into one summary bullet, in characters.
///
/// The block is bounded by construction — a fixed field list, each value this long — so no run can
/// dump a document into the step summary (S3).
pub const MAX_VALUE_CHARS: usize = 200;

/// Condenses `value` to one line.
///
/// Runs of whitespace become a single space and control characters are dropped, so a value can
/// neither open a markdown line of its own — a heading, a list item, a fence — nor carry a terminal
/// escape sequence into the rendering.
pub fn one_line(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut pending_space = false;
    for ch in value.chars() {
        if ch.is_whitespace() {
            pending_space = true;
            continue;
        }
        if ch.is_control() {
            continue;
        }
        if pending_space && !out.is_empty() {
            out.push(' ');
        }
        pending_space = false;
        out.push(ch);
    }
    out
}

/// Renders `value` as a markdown code span no content can close.
///
/// The fence is one backtick longer than the longest backtick run inside the value, and a value that
/// starts or ends with a backtick is padded by a space on each side, which is what CommonMark
/// requires for the span to render as code. The value is truncated to [`MAX_VALUE_CHARS`] first so
/// the block cannot grow with the text it describes.
pub fn code_span(value: &str) -> String {
    let text = truncate(&one_line(value));
    let fence = "`".repeat(longest_backtick_run(&text) + 1);
    if text.starts_with('`') || text.ends_with('`') {
        return format!("{fence} {text} {fence}");
    }
    format!("{fence}{text}{fence}")
}

/// Truncates `text` to [`MAX_VALUE_CHARS`] characters, marking the cut.
fn truncate(text: &str) -> String {
    if text.chars().count() <= MAX_VALUE_CHARS {
        return text.to_string();
    }
    let mut cut: String = text.chars().take(MAX_VALUE_CHARS).collect();
    cut.push('…');
    cut
}

/// The length of the longest run of backticks in `text`.
fn longest_backtick_run(text: &str) -> usize {
    let mut longest = 0;
    let mut current = 0;
    for ch in text.chars() {
        if ch == '`' {
            current += 1;
            longest = longest.max(current);
        } else {
            current = 0;
        }
    }
    longest
}
