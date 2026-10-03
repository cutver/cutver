use std::fs;
use std::io;
use std::path::Path;
use std::process::Child;
use std::thread;
use std::time::{Duration, Instant};

use crate::bump::Error;

pub fn read(path: impl AsRef<Path>) -> Result<String, Error> {
    let path = path.as_ref();
    fs::read_to_string(path).map_err(|e| Error::Read {
        path: path.display().to_string(),
        source: e,
    })
}

pub const KNOWN_LOCKFILES: &[&str] = &[
    "Cargo.lock",
    "package-lock.json",
    "pnpm-lock.yaml",
    "yarn.lock",
    "bun.lockb",
    "bun.lock",
    "gradle.lockfile",
    "poetry.lock",
    "Pipfile.lock",
    "composer.lock",
    "uv.lock",
    "pdm.lock",
];

pub fn is_known_lockfile(path: impl AsRef<Path>) -> bool {
    path.as_ref()
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|name| KNOWN_LOCKFILES.contains(&name))
}

const POLL: Duration = Duration::from_millis(50);

pub fn run_publish_command(command: &str, current_dir: &Path, timeout_secs: Option<u64>) -> Result<(), Error> {
    let timeout = timeout_secs.map(Duration::from_secs);
    let start = Instant::now();
    let mut child = spawn_command(command, current_dir).map_err(|e| Error::PublishCommandSpawn {
        command: command.to_string(),
        source: e,
    })?;

    loop {
        match child.try_wait().map_err(|e| Error::PublishCommandSpawn {
            command: command.to_string(),
            source: e,
        })? {
            Some(status) => {
                if status.success() {
                    return Ok(());
                }
                return Err(Error::PublishCommandFailed {
                    command: command.to_string(),
                    status,
                });
            }
            None => {
                if let Some(limit) = timeout {
                    let elapsed = start.elapsed();
                    if elapsed >= limit {
                        kill_tree(&mut child);
                        return Err(Error::PublishCommandTimeout {
                            command: command.to_string(),
                            timeout: limit.as_secs(),
                            elapsed_ms: elapsed.as_millis(),
                        });
                    }
                    thread::sleep(POLL.min(limit - elapsed));
                } else {
                    thread::sleep(POLL);
                }
            }
        }
    }
}

pub fn spawn_command(command: &str, current_dir: &Path) -> Result<Child, io::Error> {
    let (shell, flag) = shell();
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        unsafe {
            std::process::Command::new(shell)
                .arg(flag)
                .arg(command)
                .current_dir(current_dir)
                .pre_exec(|| {
                    let _ = setpgid(0, 0);
                    Ok(())
                })
                .spawn()
        }
    }
    #[cfg(not(unix))]
    {
        std::process::Command::new(shell)
            .arg(flag)
            .arg(command)
            .current_dir(current_dir)
            .spawn()
    }
}

pub fn kill_tree(child: &mut Child) {
    #[cfg(unix)]
    unsafe {
        let _ = killpg(child.id() as i32, SIGKILL);
    }
    #[cfg(windows)]
    {
        let pid = child.id().to_string();
        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/T", "/PID", pid.as_str()])
            .output();
        let _ = child.kill();
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = child.kill();
    }
    let _ = child.wait();
}

#[cfg(unix)]
const SIGKILL: i32 = 9;
#[cfg(unix)]
unsafe extern "C" {
    fn setpgid(pid: i32, pgid: i32) -> i32;
    fn killpg(pgrp: i32, sig: i32) -> i32;
}

#[cfg(unix)]
pub fn shell() -> (&'static str, &'static str) {
    ("sh", "-c")
}
#[cfg(windows)]
pub fn shell() -> (&'static str, &'static str) {
    ("cmd", "/C")
}
#[cfg(not(any(unix, windows)))]
pub fn shell() -> (&'static str, &'static str) {
    ("sh", "-c")
}
