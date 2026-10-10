//! Runner for `cutver plugin add` (decisions D14-D19).
//!
//! Plan first (no mutation, one registry fetch), print provenance, and only
//! apply when `--dry-run` is absent. The real `ureq` transport is the only
//! network path; the plan/apply split is the same one the hermetic library
//! tests drive with an injected transport.

use std::path::PathBuf;

use crate::cli::args::PluginAddArgs;
use crate::cli::runner::dispatch::{load_config_with_path, print_error};
use crate::cli::style::Theme;
use crate::plugin::PluginCacheRoot;
use crate::plugin::error::PluginError;
use crate::plugin::install::{AddOptions, InstallOutcome, InstallPlan, apply_add, plan_add};
use crate::plugin::registry::{ARTIFACT_TIMEOUT, RegistryIndexUrl, UreqTransport};
use crate::plugin::types::PluginName;

/// Runs `cutver plugin add`, returning the process exit code.
pub fn run_add(config_override: Option<PathBuf>, args: &PluginAddArgs) -> i32 {
    let name = match PluginName::new(args.name.as_str()) {
        Ok(name) => name,
        Err(error) => {
            print_error(error);
            return 1;
        }
    };
    let index = match resolve_index(args.registry.as_deref()) {
        Ok(index) => index,
        Err(error) => {
            print_error(error);
            return 1;
        }
    };
    let (_, config_path) = match load_config_with_path(config_override) {
        Ok(pair) => pair,
        Err(code) => return code,
    };
    let cache = match PluginCacheRoot::resolve() {
        Ok(cache) => cache,
        Err(error) => {
            print_error(error);
            return 1;
        }
    };

    let transport = UreqTransport::new(ARTIFACT_TIMEOUT);
    let plan = match plan_add(&config_path, &name, &add_options(args), &index, &cache, &transport) {
        Ok(plan) => plan,
        Err(error) => {
            print_error(error);
            return 1;
        }
    };
    print_plan(&plan);

    if args.dry_run {
        return print_diff(&plan);
    }

    match apply_add(&plan, &transport) {
        Ok(outcome) => {
            print_outcome(&outcome);
            0
        }
        Err(error) => {
            print_error(error);
            1
        }
    }
}

/// Resolves the registry base, defaulting to the canonical Cutver index.
fn resolve_index(base: Option<&str>) -> Result<RegistryIndexUrl, PluginError> {
    match base {
        Some(base) => RegistryIndexUrl::parse(base),
        None => Ok(RegistryIndexUrl::default_index()),
    }
}

/// Projects the CLI arguments onto the install options.
fn add_options(args: &PluginAddArgs) -> AddOptions {
    AddOptions {
        version: args.version.clone(),
        events: args.events.clone(),
        manifest_match: args.manifest_match.clone(),
        force: args.force,
    }
}

/// Prints the resolved plan with its provenance (Pillar III.4).
fn print_plan(plan: &InstallPlan) {
    let theme = Theme::stdout();
    println!(
        "{} plugin {} {}",
        theme.success_icon(),
        theme.accent(&plan.name),
        plan.version
    );
    println!("  index:    {}", plan.index);
    println!("  artifact: {}", plan.artifact_url);
    println!("  digest:   {}", plan.digest);
}

/// Prints the dry-run config diff and reports success.
fn print_diff(plan: &InstallPlan) -> i32 {
    println!("{}", plan.diff.trim_end());
    0
}

/// Prints what was installed and where.
fn print_outcome(outcome: &InstallOutcome) {
    let theme = Theme::stdout();
    println!(
        "{} Installed plugin {} {}",
        theme.success_icon(),
        theme.accent(&outcome.name),
        outcome.version
    );
    println!("  config:   {}", outcome.config_path.display());
    println!("  artifact: {}", outcome.artifact_path.display());
    println!("  index:    {}", outcome.index);
    println!("  digest:   {}", outcome.digest);
}
