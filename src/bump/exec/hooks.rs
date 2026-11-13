//! The `[hooks]` shell steps of a release, split out of the pipeline to keep that file inside the
//! module budget (Pillar V.4). Behavior is unchanged: `pre_bump` runs before any file is written,
//! `post_bump` runs after the manifests are applied and stages the lockfiles it touched.

use std::path::Path;

use crate::bump::Error;
use crate::bump::exec::command::{format_command, is_known_lockfile};
use crate::changelog;
use crate::config::Config;
use crate::git;

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
    let status = shell(&config.root_dir, &cmd).map_err(|e| Error::PreBumpHookSpawn {
        command: cmd.clone(),
        source: e,
    })?;
    if !status.success() {
        return Err(Error::PreBumpHookFailed { command: cmd, status });
    }
    Ok(Some(cmd))
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
    let status = shell(&config.root_dir, &cmd).map_err(|e| Error::PostBumpHookSpawn {
        command: cmd.clone(),
        source: e,
    })?;
    if !status.success() {
        return Err(Error::PostBumpHookFailed { command: cmd, status });
    }
    stage_modified_lockfiles(repo, paths_to_stage)?;
    Ok(Some(cmd))
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

fn shell(dir: &Path, command: &str) -> Result<std::process::ExitStatus, std::io::Error> {
    let (shell, flag) = if cfg!(windows) { ("cmd", "/C") } else { ("sh", "-c") };
    std::process::Command::new(shell)
        .arg(flag)
        .arg(command)
        .current_dir(dir)
        .status()
}
