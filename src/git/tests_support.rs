//! Shared test fixture helpers, compiled only for tests.
//!
//! `init_test_repo` is used by the `bump` and `cli::runner` test modules as well as this one, so it
//! lives in its own file behind `#[cfg(test)]` instead of inside the module root.

use std::path::Path;
use std::process::Command;

/// Initializes a Git repository with signing disabled and a fixed identity, so every test that
/// needs a commit history starts from the same baseline.
pub fn init_test_repo(dir: impl AsRef<Path>) {
    let dir = dir.as_ref();
    let run = |args: &[&str]| {
        let status = Command::new("git")
            .current_dir(dir)
            .args(args)
            .status()
            .expect("failed to execute git");
        assert!(status.success(), "git command failed: {:?}", args);
    };
    run(&["init"]);
    run(&["config", "user.email", "test@example.com"]);
    run(&["config", "user.name", "Test User"]);
    run(&["config", "commit.gpgsign", "false"]);
    run(&["config", "tag.gpgsign", "false"]);
}
