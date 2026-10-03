use std::path::Path;
use std::time::{Duration, Instant};

use super::*;
use crate::bump::Error;

#[test]
#[cfg(unix)]
fn test_run_publish_command_times_out() {
    let temp = std::env::temp_dir();
    let start = Instant::now();
    let res = run_publish_command("sleep 5", &temp, Some(1));
    assert!(start.elapsed() < Duration::from_secs(3));
    match res {
        Err(Error::PublishCommandTimeout {
            command,
            timeout,
            elapsed_ms,
        }) => {
            assert_eq!(command, "sleep 5");
            assert_eq!(timeout, 1);
            assert!(elapsed_ms >= 1000);
        }
        other => panic!("expected PublishCommandTimeout, got {:?}", other),
    }
}

#[test]
fn test_known_lockfiles_matched() {
    for lockfile in command::KNOWN_LOCKFILES {
        assert!(is_known_lockfile(Path::new(lockfile)));
        assert!(is_known_lockfile(Path::new("subdir").join(lockfile)));
        assert!(is_known_lockfile(Path::new("deep/nested/path").join(lockfile)));
    }

    assert!(is_known_lockfile(Path::new("bun.lock")));
    assert!(is_known_lockfile(Path::new("uv.lock")));
    assert!(is_known_lockfile(Path::new("pdm.lock")));
    assert!(is_known_lockfile(Path::new("backend/uv.lock")));
    assert!(is_known_lockfile(Path::new("frontend/bun.lock")));
}

#[test]
fn test_arbitrary_files_not_matched() {
    let non_lockfiles = [
        "unrelated.txt",
        "Cargo.toml",
        "package.json",
        "release.toml",
        "cutver.toml",
        "Cargo.lock.backup",
        "not-Cargo.lock",
        "lockfile",
        "gradle.lock",
        "poetry.lock.bak",
        "",
    ];
    for f in non_lockfiles {
        assert!(!is_known_lockfile(Path::new(f)));
    }
}
