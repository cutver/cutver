//! S1 (#367): a process plugin's child runs in the resolved workspace root, never in the directory
//! the caller happens to be in (`CONTRACT.md:154-155`, "Every subprocess names its working directory
//! explicitly and never inherits the caller's").
//!
//! `ProcessDriver::child_command` set no `current_dir`, so the child inherited `cutver`'s directory: a
//! plugin that opens a relative template, or imports a module beside itself, resolved it against
//! wherever the user ran the release. The fixture below names a workspace root that is *not* this
//! test's working directory, so a child that inherits cannot be mistaken for a correct one — the
//! difference between the two is the whole assertion.
//!
//! Only the printed directory is compared, never an error message: asserting a platform's wording for
//! a missing command is how a suite passes on one host and fails on the other two.

mod common;

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use common::FixtureGuard;
use cutver::plugin::driver::ProcessDriver;
use cutver::plugin::dto::PluginInvocation;
use cutver::plugin::types::PluginName;
use cutver::plugin::{Capability, PluginCall, PluginDriver};

/// A plugin that drains its envelope and then prints the directory it was started in.
///
/// Draining stdin first is the documented exchange (`docs/guides/plugin-process-runtime.md`): a child
/// that answers or exits before reading the envelope can lose its output to a closed pipe, and the
/// outcome would then depend on which side finished first.
fn cwd_plugin() -> (String, Vec<String>) {
    if cfg!(windows) {
        (
            "cmd".to_string(),
            vec!["/C".to_string(), "findstr . >nul & cd".to_string()],
        )
    } else {
        (
            "sh".to_string(),
            vec!["-c".to_string(), "cat >/dev/null; pwd".to_string()],
        )
    }
}

fn name() -> PluginName {
    PluginName::new("cwd-plugin").unwrap()
}

/// The one directory a path really names, so two spellings of it compare equal: `cmd` prints the
/// directory as the process sees it, and a short 8.3 name is the same directory as its long form.
fn identity(path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();
    fs::canonicalize(path).unwrap_or_else(|err| panic!("{} must be a directory that exists: {err}", path.display()))
}

fn capabilities() -> HashSet<Capability> {
    HashSet::from([Capability::LifecycleV1])
}

/// S1, S5: the workspace root the driver was built with is where the child runs, and the equality is
/// not vacuous — an inheriting child prints the caller's directory, which the last assertion proves
/// is a different one.
#[test]
fn the_child_runs_in_the_workspace_root_rather_than_the_callers_directory() {
    let guard = FixtureGuard::new("process-cwd");
    let root = identity(&guard.fixture().dir);
    let caller = identity(std::env::current_dir().expect("this test knows the directory it runs in"));
    assert_ne!(
        root, caller,
        "the fixture root must differ from the caller's directory, or the assertion below proves nothing"
    );

    let (command, args) = cwd_plugin();
    let driver = ProcessDriver::new(name(), root.clone(), command, args, BTreeMap::new(), 30, capabilities());

    let invocation = PluginInvocation::new(PluginCall::PreBump, &serde_json::json!({})).unwrap();
    let answer = driver.invoke(&invocation).expect("the plugin prints its directory");
    let printed = String::from_utf8_lossy(&answer).to_string();

    assert_eq!(
        identity(printed.trim()),
        root,
        "the child must run in the workspace root {}, not in the caller's directory {}; it printed {printed:?}",
        root.display(),
        caller.display()
    );
}

/// The premise the test above rests on: the same command, spawned the way the driver used to spawn it
/// — inheriting the caller's directory — does print the caller's directory. Without this half, a
/// plugin that simply failed to print anything would make the first half unfalsifiable.
#[test]
fn the_same_command_reports_the_callers_directory_when_it_inherits_it() {
    let (command, args) = cwd_plugin();
    let output = Command::new(command)
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("the probe command starts")
        .wait_with_output()
        .expect("the probe command is waited on");
    let printed = String::from_utf8_lossy(&output.stdout).to_string();

    let caller = identity(std::env::current_dir().expect("this test knows the directory it runs in"));
    assert_eq!(
        identity(printed.trim()),
        caller,
        "a child with no working directory of its own reports the caller's directory, which is the \
         behaviour S1 removes; it printed {printed:?}"
    );
}
