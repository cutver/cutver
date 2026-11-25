//! The CI output channel: what a run tells the runner (#326, phase 4A).
//!
//! Three sinks, and none of them is stdout:
//!
//! - `$GITHUB_OUTPUT` receives the field names the machine payload already carries, one `key=value`
//!   per line, appended and never truncated ([`fields`]).
//! - `$GITHUB_STEP_SUMMARY` receives one bounded markdown block per run ([`summary`]).
//! - a failure writes one `::error::` annotation to **stderr**, before the human line or the machine
//!   envelope (L6).
//!
//! Nothing here may write to stdout (L5). `bump --json` documents that its object is the last line
//! rather than the only one, but `plan --json`, `doctor --json` and `changelog --json` are parsed
//! whole, and one extra byte would corrupt them. S5's invariant — stdout is byte-for-byte the same
//! whether CI is detected or not — is therefore a property of this module writing nowhere else.
//!
//! Detection is resolved once, at the top of the run, into a [`CiContext`]. The pure resolver takes
//! an injected lookup, so a test builds the context directly instead of mutating the process
//! environment, which `std::env::set_var` makes unsafe and racy under parallel tests (Pillar IV.1,
//! the same boundary as `crate::plugin::cache`).

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::bump::PublishSummary;
use crate::bump::Summary;
use crate::bump::plan::ReleasePlan;

mod fields;
mod summary;
#[cfg(test)]
mod tests;
mod text;

/// Set to `true` by the runner on GitHub Actions.
pub const ACTIONS_ENV: &str = "GITHUB_ACTIONS";
/// The file a step appends its outputs to, one `key=value` per line.
pub const OUTPUT_ENV: &str = "GITHUB_OUTPUT";
/// The file a step appends its markdown summary to.
pub const SUMMARY_ENV: &str = "GITHUB_STEP_SUMMARY";

/// What the invocation asked for, before the environment is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CiMode {
    /// No flag: the environment decides.
    Detect,
    /// `--ci`: the channel is on whenever the runner named a file, `GITHUB_ACTIONS` or not.
    Force,
    /// `--no-ci`: the channel is off, annotations included.
    Off,
}

impl CiMode {
    /// Reads the two flags. Clap refuses both at once, so `(true, true)` never reaches here; it is
    /// answered as "on", because a caller that asked for the channel twice did not ask it off.
    pub fn from_flags(ci: bool, no_ci: bool) -> Self {
        match (ci, no_ci) {
            (false, false) => Self::Detect,
            (true, false) => Self::Force,
            (false, true) => Self::Off,
            (true, true) => Self::Force,
        }
    }
}

/// The channel one run resolved: which files it may append to, and whether it may annotate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CiContext {
    detected: bool,
    output: Option<PathBuf>,
    summary: Option<PathBuf>,
}

impl CiContext {
    /// The pure resolver: every input comes from `lookup`, so no test needs the process environment.
    ///
    /// Detection is `GITHUB_ACTIONS=true` **and** at least one of the two file variables naming a
    /// path (S1), where the flag value is read trimmed and case-insensitively. No other provider is
    /// detected: a second one is a later table, not a guessed heuristic. Writability is proven by the
    /// write, not guessed here, so resolution opens nothing.
    pub fn from_env_lookup(lookup: impl Fn(&str) -> Option<String>, mode: CiMode) -> Self {
        let output = file_named(&lookup, OUTPUT_ENV);
        let summary = file_named(&lookup, SUMMARY_ENV);
        let named = output.is_some() || summary.is_some();
        let detected = match mode {
            CiMode::Off => false,
            // `--ci` is the caller's own declaration, so it needs neither the flag variable nor a
            // sink: the annotation follows it, and the two file sinks are filled when named.
            CiMode::Force => true,
            CiMode::Detect => named && is_true(lookup(ACTIONS_ENV)),
        };
        if !detected {
            return Self::disabled();
        }
        Self {
            detected,
            output,
            summary,
        }
    }

    /// Boundary constructor binding the pure resolver to the process environment.
    pub fn resolve(mode: CiMode) -> Self {
        Self::from_env_lookup(|key| std::env::var(key).ok(), mode)
    }

    /// The channel a run that selected none has: no sink, no annotation (S6).
    pub const fn disabled() -> Self {
        Self {
            detected: false,
            output: None,
            summary: None,
        }
    }

    /// Whether this run may write the runner's sinks and annotate a failure.
    pub fn detected(&self) -> bool {
        self.detected
    }

    /// Appends `key=value` lines to `$GITHUB_OUTPUT`, when the run named that file (S2).
    fn append_outputs(&self, fields: &[fields::Field]) {
        let Some(path) = &self.output else {
            return;
        };
        append(path, &fields::render(fields));
    }

    /// Appends one markdown block to `$GITHUB_STEP_SUMMARY`, when the run named that file (S3).
    fn append_summary(&self, block: &str) {
        let Some(path) = &self.summary else {
            return;
        };
        append(path, block);
    }
}

/// The channel this run resolved. Written once by [`select`], read by every sink and failure site;
/// one value is enough because one process runs one command.
static CHANNEL: OnceLock<CiContext> = OnceLock::new();

/// Resolves the channel for `mode`, once, at the top of the run.
pub fn select(mode: CiMode) {
    let _ = CHANNEL.set(CiContext::resolve(mode));
}

/// The channel this run resolved, or the disabled one when nothing selected it.
fn channel() -> &'static CiContext {
    CHANNEL.get_or_init(CiContext::disabled)
}

/// Writes the failure annotation when CI is detected, independently of machine mode (S4).
///
/// The annotation carries the human text on one line: a runner reads a command annotation to the end
/// of its line, so `%`, CR and LF are percent-escaped the way its command syntax requires. The
/// caller prints the human line or the machine envelope after it, on its own line, so a reader that
/// parses stderr keeps working unchanged.
pub fn annotate_error(message: &str) {
    if !channel().detected() {
        return;
    }
    eprintln!("::error::{}", escape_command_data(message));
}

/// Writes the deprecation annotation when CI is detected, independently of machine mode (#272).
///
/// A deprecation is not a failure: the command keeps working, the exit code does not move, and the
/// human line stays on stderr next to the annotation. The annotation is what a runner turns into a
/// visible notice on the pull request or the run.
pub fn annotate_warning(message: &str) {
    if !channel().detected() {
        return;
    }
    eprintln!("::warning::{}", escape_command_data(message));
}

/// Everything the channel carries for a finished `bump`: the outputs, then the summary.
pub fn on_bump_success(finished: &Summary) {
    let channel = channel();
    if !channel.detected() {
        return;
    }
    let fields = fields::bump(finished);
    channel.append_outputs(&fields);
    channel.append_summary(&summary::render("cutver bump", &fields));
}

/// Everything the channel carries for a computed `plan`: the outputs, then the summary.
///
/// The summary carries the level and the blocker count from the document, but not the blockers
/// themselves: the block is a brief, and the document has the rest.
pub(crate) fn on_plan_success(plan: &ReleasePlan) {
    let channel = channel();
    if !channel.detected() {
        return;
    }
    let fields = fields::plan(plan);
    channel.append_outputs(&fields);
    let mut brief = vec![("level", Some(plan.level.clone()))];
    brief.extend(fields);
    brief.push(("blockers", Some(plan.blockers.len().to_string())));
    channel.append_summary(&summary::render("cutver plan", &brief));
}

/// Everything the channel carries for a finished `publish`: the outputs, then the summary.
///
/// The closing phase is the irreversible one, so its fields report what it did and not what it
/// planned: a `--dry-run` writes `false` for every action and no command as having run.
pub(crate) fn on_publish_success(finished: &PublishSummary) {
    let channel = channel();
    if !channel.detected() {
        return;
    }
    let fields = fields::publish(finished);
    channel.append_outputs(&fields);
    channel.append_summary(&summary::render("cutver publish", &fields));
}

/// Appends `text` to `path`, creating the file when the runner named a path that does not exist yet.
///
/// A sink that cannot be written degrades to a warning: it is bookkeeping for the runner, not the
/// release, so failing the run over it would turn a finished release into a failed one.
fn append(path: &Path, text: &str) {
    use std::io::Write;
    if text.is_empty() {
        return;
    }
    let written = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut file| file.write_all(text.as_bytes()));
    if let Err(source) = written {
        crate::cli::style::warn_degradation(format!("could not write '{}': {source}", path.display()));
    }
}

/// Reads a path-valued variable: blank means "not named", so an empty value opens nothing.
fn file_named(lookup: &impl Fn(&str) -> Option<String>, key: &str) -> Option<PathBuf> {
    let value = lookup(key)?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(PathBuf::from(trimmed))
}

/// Whether a flag variable says `true`, which is the only value GitHub Actions writes.
fn is_true(value: Option<String>) -> bool {
    value.is_some_and(|value| value.trim().eq_ignore_ascii_case("true"))
}

/// Escapes the characters a workflow command annotation treats as structure.
fn escape_command_data(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '%' => escaped.push_str("%25"),
            '\r' => escaped.push_str("%0D"),
            '\n' => escaped.push_str("%0A"),
            other => escaped.push(other),
        }
    }
    escaped
}
