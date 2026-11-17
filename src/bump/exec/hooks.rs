//! The `[hooks]` shell steps of a release, split out of the pipeline to keep that file inside the
//! module budget (Pillar V.4). Behavior is unchanged: `pre_bump` runs before any file is written,
//! `post_bump` runs after the manifests are applied and stages the lockfiles it touched.
//!
//! Both hooks run through the shared bounded spawn (audit F-16), so a hook that outlives
//! `[hooks] default_timeout` is killed together with its process tree instead of holding the
//! release open forever. No timeout key means no bound, exactly as `[preflight]` and `[publish]`
//! treat their own `default_timeout`.

use std::io;
use std::path::Path;
use std::process::ExitStatus;
use std::time::Duration;

use crate::bump::Error;
use crate::bump::exec::command::{format_command, is_known_lockfile};
use crate::changelog;
use crate::config::Config;
use crate::git;
use crate::preflight::{Ended, run_supervised};

/// Runs `[hooks] pre_bump`, which must precede every write so a failing hook leaves the workspace
/// untouched. A dry run reports the rendered command instead of running it.
pub(crate) fn run_pre_bump_hook(
    config: &Config,
    interp_ctx: &changelog::InterpolationContext,
    dry_run: bool,
) -> Result<Option<String>, Error> {
    let Some(raw_pre_bump) = &config.hooks.pre_bump else {
        return Ok(None);
    };
    let cmd = format_command(raw_pre_bump, interp_ctx)?;
    if dry_run {
        return Ok(Some(cmd));
    }
    match run_hook(config, &cmd) {
        Ok(()) => Ok(Some(cmd)),
        Err(HookFailure::Spawn(source)) => Err(Error::PreBumpHookSpawn { command: cmd, source }),
        Err(HookFailure::Status(status)) => Err(Error::PreBumpHookFailed { command: cmd, status }),
        Err(HookFailure::TimedOut { timeout, elapsed_ms }) => Err(Error::PreBumpHookTimeout {
            command: cmd,
            timeout,
            elapsed_ms,
        }),
    }
}

/// Runs `[hooks] post_bump` and stages the lockfiles it rewrote, so the release commit carries
/// them. A dry run reports the rendered command instead of running it.
pub(crate) fn run_post_bump_hook(
    config: &Config,
    repo: &Path,
    interp_ctx: &changelog::InterpolationContext,
    dry_run: bool,
    paths_to_stage: &mut Vec<String>,
) -> Result<Option<String>, Error> {
    let Some(raw_post_bump) = &config.hooks.post_bump else {
        return Ok(None);
    };
    let cmd = format_command(raw_post_bump, interp_ctx)?;
    if dry_run {
        return Ok(Some(cmd));
    }
    match run_hook(config, &cmd) {
        Ok(()) => {}
        Err(HookFailure::Spawn(source)) => return Err(Error::PostBumpHookSpawn { command: cmd, source }),
        Err(HookFailure::Status(status)) => return Err(Error::PostBumpHookFailed { command: cmd, status }),
        Err(HookFailure::TimedOut { timeout, elapsed_ms }) => {
            return Err(Error::PostBumpHookTimeout {
                command: cmd,
                timeout,
                elapsed_ms,
            });
        }
    }
    stage_modified_lockfiles(repo, paths_to_stage)?;
    Ok(Some(cmd))
}

/// A hook that did not end in success. The pre/post callers own the wording, so the shared step
/// only reports which of the three ways it went wrong.
enum HookFailure {
    Spawn(io::Error),
    Status(ExitStatus),
    TimedOut { timeout: u64, elapsed_ms: u128 },
}

/// Runs one hook in the workspace root under `[hooks] default_timeout`.
fn run_hook(config: &Config, command: &str) -> Result<(), HookFailure> {
    let timeout = config.hooks.default_timeout.map(Duration::from_secs);
    match run_supervised(command, Some(&config.root_dir), timeout) {
        Ok(Ended::Exited(status)) if status.success() => Ok(()),
        Ok(Ended::Exited(status)) => Err(HookFailure::Status(status)),
        Ok(Ended::TimedOut { timeout, elapsed }) => Err(HookFailure::TimedOut {
            timeout: timeout.as_secs(),
            elapsed_ms: elapsed.as_millis(),
        }),
        Err(e) => Err(HookFailure::Spawn(e.into_io())),
    }
}

/// Collects the lockfiles the post_bump hook rewrote, without duplicating a path already staged.
fn stage_modified_lockfiles(repo: &Path, paths_to_stage: &mut Vec<String>) -> Result<(), Error> {
    for file in git::status_files(repo).map_err(Error::Git)? {
        let rel = Path::new(&file);
        if !is_known_lockfile(rel) {
            continue;
        }
        let already_staged = paths_to_stage
            .iter()
            .any(|staged| Path::new(staged) == rel || Path::new(staged) == repo.join(rel));
        if !already_staged {
            paths_to_stage.push(file);
        }
    }
    Ok(())
}
