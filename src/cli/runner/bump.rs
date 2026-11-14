//! `cutver bump`: runs the release and reports it — as a tree, as JSON on stdout, or into a file.
//!
//! `--json` prints the release result as one compact JSON object on stdout. It is the last line
//! cutver writes there, but it is not necessarily the only one: `[preflight]` steps and the git
//! subprocesses inherit stdout and print above it. `--report <PATH>` is the flag for a machine that
//! needs the object alone: it writes the same JSON to that file and nothing else, atomically.

use std::path::{Path, PathBuf};

use crate::atomic;
use crate::bump::{self, Summary};
use crate::cli::args::BumpLevel;
use crate::config;

use super::dispatch::print_error;

/// Where the release result goes. `json` and `report` are independent: either, both, or neither.
#[derive(Debug, Default)]
pub struct BumpOutput {
    pub json: bool,
    pub report: Option<PathBuf>,
}

pub fn run_bump(
    config: &config::Config,
    level: BumpLevel,
    dry_run: bool,
    skip_preflight: &[String],
    first_release: bool,
    defer_publish: bool,
    output: BumpOutput,
) -> i32 {
    let outcome = if defer_publish {
        bump::run_deferred_publish(config, level, dry_run, skip_preflight, first_release)
    } else {
        bump::run_with_first_release(config, level, dry_run, skip_preflight, first_release)
    };
    match outcome {
        Ok(summary) => match emit(&summary, &output) {
            Ok(()) => 0,
            Err(e) => {
                print_error(e);
                1
            }
        },
        Err(e) => {
            print_error(e);
            1
        }
    }
}

/// The machine-readable result of a release run.
#[derive(serde::Serialize)]
struct SummaryJson<'a> {
    released: bool,
    deferred: bool,
    version: &'a str,
    tag: &'a str,
    commit: &'a str,
    floating_tag: Option<&'a str>,
    publish_push_command: Option<&'a str>,
    publish_commands: &'a [String],
}

/// Why the machine-readable release result could not be produced or written.
#[derive(Debug, thiserror::Error)]
pub enum ReportError {
    #[error(
        "could not serialize the release result to JSON: {source}\n  Where: `--json`/`--report`, after the release finished; the release itself is unaffected\n  Fix: report this as a cutver bug, since the release result holds only strings, booleans and lists of strings, and re-run without `--json`/`--report` to print the plan tree instead."
    )]
    Serialize {
        #[source]
        source: serde_json::Error,
    },
    #[error(
        "could not write the release report to '{path}': {source}\n  Where: `--report <PATH>`, after release '{tag}' finished; a failure here never changes what the release did or did not publish\n  Fix: create the missing directory or choose a writable path and re-run with the same `--report`. The report is written atomically, so an existing file at that path is intact and never half-written."
    )]
    Write {
        path: String,
        tag: String,
        #[source]
        source: atomic::Error,
    },
}

/// Renders the release result as compact JSON. Field order is part of the contract, so the shape is
/// a struct rather than a map.
pub fn render_summary_json(summary: &Summary) -> Result<String, ReportError> {
    let version = summary.next.to_string();
    let fields = SummaryJson {
        released: !summary.dry_run,
        deferred: summary.deferred,
        version: &version,
        tag: &summary.tag,
        commit: &summary.commit,
        floating_tag: summary.floating_tag.as_deref(),
        publish_push_command: summary.publish_push_command.as_deref(),
        publish_commands: &summary.publish_commands,
    };
    serde_json::to_string(&fields).map_err(|source| ReportError::Serialize { source })
}

/// Writes the same object `--json` prints to `path`: one line, one trailing newline, nothing else.
/// Written through the atomic boundary so a failure never leaves a partial report behind.
pub fn write_report(summary: &Summary, path: &Path) -> Result<(), ReportError> {
    let object = render_summary_json(summary)?;
    atomic::write_atomic(path, format!("{object}\n")).map_err(|source| ReportError::Write {
        path: path.display().to_string(),
        tag: summary.tag.clone(),
        source,
    })
}

/// Prints the release result on stdout (tree or JSON) and writes the report file when asked.
fn emit(summary: &Summary, output: &BumpOutput) -> Result<(), ReportError> {
    if output.json {
        println!("{}", render_summary_json(summary)?);
    } else {
        print_bump_summary(summary);
    }
    if let Some(path) = &output.report {
        write_report(summary, path)?;
    }
    Ok(())
}

pub fn print_bump_summary(summary: &Summary) {
    crate::cli::tree::print_tree_summary(summary);
}

#[allow(dead_code)]
pub fn print_summary(summary: &Summary) {
    print_bump_summary(summary);
}
