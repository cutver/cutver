//! A preflight step resolves relative paths against the repository root (#295).
//!
//! The process working directory is ambient authority: `cargo test` launches from the crate root, a
//! CI runner from its workspace, a long-lived process from wherever it was started. `[preflight]`
//! was the one stage that inherited it, while `[publish].commands` and the `[hooks]` commands
//! already received the root they run in — so the same command could touch a file in the repository
//! or in a stranger's directory depending on who launched cutver.
//!
//! Deliberately not `#![cfg(unix)]`: the defect is platform-independent, and a unix-gated file runs
//! zero tests on the Windows leg (see #270's note on the same blind spot).

mod common;

use std::path::Path;

use common::*;
use cutver::bump::run as bump_run;
use cutver::semver_bump::Bump;

/// A command that creates `name` at a relative path, spelled for the platform shell every preflight
/// step is run through.
fn marker_command(name: &str) -> String {
    if cfg!(windows) {
        format!("type nul > {name}")
    } else {
        format!("touch {name}")
    }
}

#[test]
fn a_preflight_step_resolves_relative_paths_against_the_repository_root() {
    assert!(
        git_available(),
        "git CLI is required for e2e tests but was not found in PATH"
    );
    const MARKER: &str = "preflight-cwd-marker.txt";

    let guard = FixtureGuard::new("preflight-root");
    let fixture = guard.fixture();
    write_fixture(&guard, &format!("[preflight]\ncheck = \"{}\"", marker_command(MARKER)));

    let cfg = load_config(fixture);
    // A failing step aborts the bump, so this expectation is what separates "the step ran" from
    // "the step was rejected".
    bump_run(&cfg, Bump::Minor, false, &[], None).expect("the preflight step exits zero");

    // Under the defect the marker landed in the process CWD. Note and remove it on both paths, so a
    // red run does not leave the repository dirty for whatever runs next.
    let in_process_cwd = Path::new(MARKER);
    let strayed = in_process_cwd.exists();
    let _ = std::fs::remove_file(in_process_cwd);

    assert!(
        !strayed,
        "the step wrote {MARKER} into the process working directory: a preflight step still \
         inherits ambient CWD authority"
    );
    assert!(
        fixture.dir.join(MARKER).exists(),
        "the step must write into the repository root ({})",
        fixture.dir.display()
    );
}
