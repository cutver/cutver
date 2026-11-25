//! `ETXTBSY` on the dispatch path: a fixture script whose write handle is still open cannot be
//! `exec`ed.
//!
//! The kernel's refusal is the right answer for a file genuinely held for writing, and
//! `execute_binary` still reports it. What made #375 a flake is that the window can belong to
//! *someone else*: `cargo test` runs a suite in threads of one process, so any thread's `fork`
//! duplicates an open write descriptor into an unrelated child, where it outlives the write that
//! created it. These two tests hold the descriptor themselves, which makes the window
//! deterministic instead of load-dependent — the failing CI job could not be reproduced here at
//! all, because the fixture is a `#!/bin/sh` script and this host is Windows.

use std::fs::{self, File, OpenOptions};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use super::temp_dir;

/// Writes the `#!/bin/sh` fixture and marks it executable, the same shape the dispatch test uses.
fn script(dir: &Path) -> PathBuf {
    let path = dir.join("cutver-mock");
    fs::write(&path, "#!/bin/sh\nexit 42\n").unwrap();
    let mut perms = fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms).unwrap();
    path
}

/// Holds the descriptor the kernel objects to. Dropping it is what lets `exec` succeed.
fn hold_open_for_writing(path: &Path) -> File {
    OpenOptions::new().write(true).open(path).unwrap()
}

/// The search path is the directory alone: `resolve_binary` splits it with `std::env::split_paths`,
/// and a temp directory never contains the separator.
fn search(dir: &Path) -> Option<std::ffi::OsString> {
    Some(dir.as_os_str().to_os_string())
}

#[test]
fn a_write_handle_released_before_the_last_attempt_still_executes() {
    let dir = temp_dir("cutver-runner-ext-busy-retry");
    let script_path = script(&dir);
    let handle = hold_open_for_writing(&script_path);

    let released = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(15));
        drop(handle);
    });

    let code = crate::cli::runner::run_external_with_path(&["mock".to_string()], search(&dir));

    released.join().unwrap();
    assert_eq!(
        code, 42,
        "a write handle released while the call is retrying must not fail the dispatch"
    );
}

#[test]
fn a_write_handle_held_for_the_whole_call_is_reported_after_a_bounded_wait() {
    let dir = temp_dir("cutver-runner-ext-busy-held");
    let script_path = script(&dir);
    let handle = hold_open_for_writing(&script_path);

    let started = Instant::now();
    let code = crate::cli::runner::run_external_with_path(&["mock".to_string()], search(&dir));
    let elapsed = started.elapsed();

    drop(handle);
    assert_eq!(code, 1, "a file genuinely held for writing must still fail, not run");
    assert!(
        elapsed < Duration::from_secs(2),
        "the retry is bounded: a held file must not stall the dispatch (took {elapsed:?})"
    );
}
