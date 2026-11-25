//! `cutver plan`: the release the workspace is about to cut, printed and not performed (#326).
//!
//! The command owns no release logic. It reads the plan the pipeline computes
//! (`crate::bump::plan`), prints it as one versioned JSON object or as the tree the dry run already
//! renders, and exits `0` even when the plan carries blockers: the release decision belongs to the
//! caller, and exit `1` is reserved for "no plan could be computed" (S4, L6).

use crate::bump::BumpLevel;
use crate::bump::plan::{ReleasePlan, read_only};
use crate::cli::args::BumpLevel as CliLevel;
use crate::cli::args::Channel as CliChannel;
use crate::cli::machine::FailureCode;
use crate::config;

use super::dispatch::print_error;

/// Renders the plan as the versioned object `cutver plan --json` emits.
///
/// Field order is part of the contract, so the object is built from a struct rather than a map;
/// `src/cli/json.rs` owns the `schemaVersion` it opens with (#326 S2).
pub(crate) fn render_plan_json(plan: &ReleasePlan) -> Result<String, serde_json::Error> {
    serde_json::to_string(&crate::cli::json::Versioned::new(plan))
}

/// Computes and prints the plan. Returns the process exit code: `0` for a plan, blocked or not.
pub(crate) fn run_plan(
    config: &config::Config,
    level: CliLevel,
    skip_preflight: &[String],
    first_release: bool,
    channel: Option<CliChannel>,
    json: bool,
) -> i32 {
    let level: BumpLevel = level.into();
    match read_only(config, level, skip_preflight, first_release, channel.map(Into::into)) {
        Ok(plan) => {
            if !json {
                crate::cli::tree::print_plan(&plan);
                crate::cli::ci::on_plan_success(&plan);
                return 0;
            }
            match render_plan_json(&plan) {
                Ok(object) => {
                    println!("{object}");
                    crate::cli::ci::on_plan_success(&plan);
                    0
                }
                Err(e) => {
                    print_error(FailureCode::Plan, e);
                    1
                }
            }
        }
        // Only "the plan could not be computed" reaches here: a blocker is data, never an error.
        Err(e) => {
            print_error(FailureCode::Plan, e);
            1
        }
    }
}
