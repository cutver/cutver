use crate::bump::{self, Summary};
use crate::cli::args::BumpLevel;
use crate::config;

use super::dispatch::print_error;

pub fn run_bump(
    config: &config::Config,
    level: BumpLevel,
    dry_run: bool,
    skip_preflight: &[String],
    first_release: bool,
) -> i32 {
    match bump::run_with_first_release(config, level, dry_run, skip_preflight, first_release) {
        Ok(summary) => {
            print_bump_summary(&summary);
            0
        }
        Err(e) => {
            print_error(e);
            1
        }
    }
}

pub fn print_bump_summary(summary: &Summary) {
    crate::cli::tree::print_tree_summary(summary);
}

#[allow(dead_code)]
pub fn print_summary(summary: &Summary) {
    print_bump_summary(summary);
}
