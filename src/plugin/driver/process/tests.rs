//! Behaviour tests for the process plugin driver.
//!
//! The isolation tests fail closed on an environment that is not the one the driver promises: the
//! child runs with `env_clear()` plus the named system baseline, so a key outside that baseline that
//! the driver was not handed must never appear in what the plugin prints (issue #264). Since #344
//! nothing here mutates the process environment: the map is built in the test and passed to the
//! driver, and `tests/process_driver_env_isolation.rs` proves the clearing itself against keys this
//! process really carries.

use super::*;
use crate::plugin::driver::resolve_env_grants;
use crate::plugin::types::PluginCall;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[test]
fn test_unsupported_capability() {
    let name = PluginName::new("echo-plugin").unwrap();
    let driver = ProcessDriver::new(name.clone(), root(), "echo", vec![], BTreeMap::new(), 5, HashSet::new());
    let invocation = PluginInvocation::new(PluginCall::ManifestRead, &serde_json::json!({})).unwrap();
    let err = driver.invoke(&invocation).unwrap_err();
    match err {
        PluginError::UnsupportedCapability {
            name: err_name,
            capability,
        } => {
            assert_eq!(err_name, name);
            assert_eq!(capability, "manifest.v1");
        }
        other => panic!("expected UnsupportedCapability, got {other:?}"),
    }
}

#[test]
#[cfg_attr(windows, ignore)]
fn test_successful_invocation_echo() {
    let name = PluginName::new("cat-plugin").unwrap();
    let mut caps = HashSet::new();
    caps.insert(Capability::ManifestV1);

    let (cmd, args) = if cfg!(windows) {
        (
            "cmd".to_string(),
            vec!["/C".to_string(), "findstr".to_string(), "^".to_string()],
        )
    } else {
        ("cat".to_string(), vec![])
    };

    let driver = ProcessDriver::new(name, root(), cmd, args, shell_env(), 5, caps);
    let invocation = PluginInvocation::new(
        PluginCall::ManifestWrite,
        &serde_json::json!({"action": "bump", "version": "1.0.0"}),
    )
    .unwrap();
    let output = driver.invoke(&invocation).unwrap();
    assert_eq!(output, serde_json::to_vec(&invocation).unwrap());
}

#[test]
fn test_execution_failed_non_zero_exit() {
    let name = PluginName::new("fail-plugin").unwrap();
    let mut caps = HashSet::new();
    caps.insert(Capability::LifecycleV1);

    let (cmd, args) = if cfg!(windows) {
        (
            "cmd".to_string(),
            vec!["/C".to_string(), "dir non_existing_file_for_test_12345".to_string()],
        )
    } else {
        (
            "sh".to_string(),
            vec!["-c".to_string(), "echo 'failing now' >&2; exit 42".to_string()],
        )
    };

    let driver = ProcessDriver::new(name.clone(), root(), cmd, args, shell_env(), 5, caps);
    let invocation = PluginInvocation::new(PluginCall::PreBump, &serde_json::json!({})).unwrap();
    let err = driver.invoke(&invocation).unwrap_err();

    match err {
        PluginError::ExecutionFailed {
            name: err_name,
            capability,
            exit_code,
            stderr,
        } => {
            assert_eq!(err_name, name);
            assert_eq!(capability, "lifecycle.v1");
            if !cfg!(windows) {
                assert_eq!(exit_code, 42);
                assert!(stderr.contains("failing now"));
            } else {
                assert_ne!(exit_code, 0);
            }
        }
        other => panic!("expected ExecutionFailed, got {other:?}"),
    }
}

#[test]
fn test_timeout_isolation() {
    let name = PluginName::new("sleep-plugin").unwrap();
    let mut caps = HashSet::new();
    caps.insert(Capability::ChangelogV1);

    let (cmd, args) = if cfg!(windows) {
        (
            "powershell".to_string(),
            vec!["-Command".to_string(), "Start-Sleep -Seconds 5".to_string()],
        )
    } else {
        ("sleep".to_string(), vec!["5".to_string()])
    };

    let driver = ProcessDriver::new(name.clone(), root(), cmd, args, shell_env(), 1, caps);
    let invocation = PluginInvocation::new(PluginCall::ChangelogRender, &serde_json::json!({})).unwrap();
    let start = Instant::now();
    let err = driver.invoke(&invocation).unwrap_err();
    let elapsed = start.elapsed();

    assert!(elapsed.as_secs() >= 1);
    assert!(elapsed.as_secs() < 4);

    match err {
        PluginError::Timeout {
            name: err_name,
            capability,
            timeout_secs,
        } => {
            assert_eq!(err_name, name);
            assert_eq!(capability, "changelog.v1");
            assert_eq!(timeout_secs, 1);
        }
        other => panic!("expected Timeout, got {other:?}"),
    }
}

#[test]
fn test_getters_and_debug() {
    let name = PluginName::new("custom-plugin").unwrap();
    let mut caps = HashSet::new();
    caps.insert(Capability::VersioningV1);
    let mut env = BTreeMap::new();
    env.insert("FOO".to_string(), "BAR".to_string());

    let driver = ProcessDriver::new(
        name.clone(),
        root(),
        "custom-cmd",
        vec!["--flag".to_string()],
        env.clone(),
        10,
        caps.clone(),
    );

    assert_eq!(driver.name(), &name);
    assert_eq!(driver.command(), "custom-cmd");
    assert_eq!(driver.args(), &["--flag".to_string()]);
    assert_eq!(driver.grants().resolve(&name).unwrap(), env);
    assert_eq!(driver.timeout_seconds(), 10);
    assert_eq!(driver.root_dir(), root());
    assert_eq!(driver.capabilities(), &caps);
    assert!(format!("{driver:?}").contains("custom-cmd"));
}

/// Prints the child's whole environment with the interpreter the platform already has.
fn env_dump_command() -> (String, Vec<String>) {
    if cfg!(windows) {
        ("cmd".to_string(), vec!["/C".to_string(), "set".to_string()])
    } else {
        ("/usr/bin/env".to_string(), Vec::new())
    }
}

/// The workspace root every fixture driver runs its child in.
///
/// The OS temp directory: absolute, present on both platforms, and nothing here asserts what is in
/// it. These fixtures drive one invocation and assert what the child receives, so the directory is
/// theirs to name — and naming one is the point (#367 S1): there is no root-less constructor left
/// for a fixture to reach for, and the behavioural half of the guarantee is
/// `tests/plugin_process_cwd.rs`.
fn root() -> PathBuf {
    std::env::temp_dir()
}

/// The minimum environment the platform's shell genuinely needs once the driver clears the rest.
///
/// These keys are the tool's own requirements, not ambient authority: `powershell.exe` reads
/// `SystemRoot` while loading the managed engine and exits with `8009001d` without it, and the
/// POSIX fixtures (`sh`, `sleep`, `cat`) are resolved through `PATH`. Isolation is asserted
/// separately, by the absence of a key that is *not* granted.
fn shell_env() -> BTreeMap<String, String> {
    let keys: Vec<String> = if cfg!(windows) {
        vec!["SystemRoot".to_string()]
    } else {
        vec!["PATH".to_string()]
    };
    let plugin = PluginName::new("shell-fixture").unwrap();
    resolve_env_grants(&plugin, &keys).expect("the platform shell needs these keys to start")
}

/// S4: the granted key reaches the plugin and a key that was never granted does not.
#[test]
fn test_only_granted_env_keys_reach_the_plugin() {
    const GRANTED: &str = "CUTVER_TEST_GRANTED_KEY";
    const WITHHELD: &str = "CUTVER_TEST_WITHHELD_SECRET";

    let name = PluginName::new("env-plugin").unwrap();
    let mut caps = HashSet::new();
    caps.insert(Capability::ManifestV1);
    let (cmd, args) = env_dump_command();

    // The map is built here and handed straight to the driver, so the test never mutates the
    // process environment (#344). `WITHHELD` is a key of our own naming and is absent from the
    // parent too, so this half states what the map contains — the clearing itself is proven by
    // `tests/process_driver_env_isolation.rs`.
    let granted = BTreeMap::from([(GRANTED.to_string(), "granted-value".to_string())]);
    let driver = ProcessDriver::new(name, root(), cmd, args, granted, 5, caps);
    let invocation = PluginInvocation::new(PluginCall::ManifestRead, &serde_json::json!({})).unwrap();
    let output = driver.invoke(&invocation).expect("the environment dump must run");

    let printed = String::from_utf8_lossy(&output);
    // Neither message formats `printed`: a regression here would dump the inherited environment,
    // and on CI that environment carries the release tokens (native review, R1-001).
    assert!(
        printed.contains(&format!("{GRANTED}=granted-value")),
        "the granted key must reach the plugin; the dump is withheld because it may contain credentials"
    );
    assert!(
        !printed.contains(WITHHELD),
        "a key that is not granted must never reach the plugin; the dump is withheld because it \
         may contain credentials"
    );
}

/// S2: a plugin that never reads stdin fills the pipe and still fails closed within the timeout,
/// because the write runs concurrently with the wait instead of blocking before it.
#[test]
fn test_stdin_that_is_never_read_fails_closed_with_timeout() {
    let name = PluginName::new("mute-plugin").unwrap();
    let mut caps = HashSet::new();
    caps.insert(Capability::LifecycleV1);

    let (cmd, args) = if cfg!(windows) {
        (
            "powershell".to_string(),
            vec!["-Command".to_string(), "Start-Sleep -Seconds 10".to_string()],
        )
    } else {
        ("sleep".to_string(), vec!["10".to_string()])
    };

    let driver = ProcessDriver::new(name.clone(), root(), cmd, args, shell_env(), 1, caps);
    // Larger than any OS pipe buffer, so an unbounded blocking write could never finish.
    let payload = serde_json::json!({ "blob": "x".repeat(2 * 1024 * 1024) });
    let invocation = PluginInvocation::new(PluginCall::PreBump, &payload).unwrap();
    let start = Instant::now();
    let err = driver.invoke(&invocation).unwrap_err();
    let elapsed = start.elapsed();

    match err {
        PluginError::Timeout {
            name: err_name,
            capability,
            timeout_secs,
        } => {
            assert_eq!(err_name, name);
            assert_eq!(capability, "lifecycle.v1");
            assert_eq!(timeout_secs, 1);
        }
        other => panic!("expected Timeout, got {other:?}"),
    }
    assert!(
        elapsed < Duration::from_secs(3),
        "the write must stay inside timeout_seconds, took {elapsed:?}"
    );
}

/// The minimum environment a shell needs to find an external child command (`ping`, `sleep`);
/// Windows also needs `SystemRoot` to start at all.
fn descendant_env() -> BTreeMap<String, String> {
    let keys: Vec<String> = if cfg!(windows) {
        vec!["SystemRoot".to_string(), "PATH".to_string()]
    } else {
        vec!["PATH".to_string()]
    };
    let plugin = PluginName::new("orphan-fixture").unwrap();
    resolve_env_grants(&plugin, &keys).expect("the platform shell needs these keys to start")
}

/// A plugin that never exits inside the budget, plus one descendant writing `marker` three seconds in.
fn plugin_with_a_descendant_that_writes(marker: &std::path::Path) -> (String, Vec<String>) {
    let m = marker.display();
    let script = if cfg!(windows) {
        // The carets keep `&` and `>` for the descendant's own command line.
        format!("start /b cmd /c ping -n 4 127.0.0.1 >NUL ^& type nul ^> {m} & ping -n 25 127.0.0.1")
    } else {
        format!("(sleep 3; touch {m}) & sleep 25")
    };
    let (shell, flag) = if cfg!(windows) { ("cmd", "/C") } else { ("sh", "-c") };
    (shell.to_string(), vec![flag.to_string(), script])
}

/// Runs one invocation of a plugin that starts a marker-writing descendant and asserts the deadline
/// reached the whole tree: it ended on its own budget, naming plugin, capability and limit (S8), the
/// caller never waited past it, and the descendant never wrote its marker (S3, S6).
fn assert_deadline_reaches_the_descendant(plugin: &str, label: &str, payload: serde_json::Value) {
    let mut caps = HashSet::new();
    caps.insert(Capability::LifecycleV1);
    let marker = std::env::temp_dir().join(format!("cutver-descendant-{label}-{}.marker", std::process::id()));
    let _ = std::fs::remove_file(&marker);
    let (cmd, args) = plugin_with_a_descendant_that_writes(&marker);
    let driver = ProcessDriver::new(
        PluginName::new(plugin).unwrap(),
        root(),
        cmd,
        args,
        descendant_env(),
        1,
        caps,
    );
    let invocation = PluginInvocation::new(PluginCall::PreBump, &payload).unwrap();
    let start = Instant::now();
    let err = driver.invoke(&invocation).unwrap_err();
    let elapsed = start.elapsed();
    match err {
        PluginError::Timeout {
            name,
            capability,
            timeout_secs,
        } => {
            assert_eq!(&name, driver.name());
            assert_eq!(capability, "lifecycle.v1");
            assert_eq!(timeout_secs, 1, "the invocation budget is unchanged");
        }
        other => panic!("expected Timeout, got {other:?}"),
    }
    let budget = Duration::from_secs(5);
    assert!(elapsed < budget, "the budget bounds the exchange, took {elapsed:?}");
    // The descendant was due three seconds in, after the deadline that ended the invocation.
    std::thread::sleep(Duration::from_secs(4));
    let survived = marker.exists();
    let _ = std::fs::remove_file(&marker);
    assert!(
        !survived,
        "a descendant outlived the invocation and wrote {}: the deadline must reach the whole tree",
        marker.display()
    );
}

/// S2, S6: the descendant pinned as a survivor is killed with the plugin, and a plugin that never
/// drains its stdin cannot hold the caller past the budget either.
#[test]
fn test_a_descendant_holding_stdin_cannot_outlive_the_timeout() {
    // Larger than any OS pipe buffer, so the write cannot complete while nobody drains stdin.
    let payload = serde_json::json!({ "blob": "x".repeat(2 * 1024 * 1024) });
    assert_deadline_reaches_the_descendant("orphan-plugin", "stdin", payload);
}

/// S3: a timed-out plugin whose descendant writes a marker leaves no descendant to write it. Not
/// unix-gated: the leak and its fix exist on both platforms, and a `cfg(unix)` leg proves nothing.
#[test]
fn test_a_descendant_of_a_timed_out_plugin_never_writes_its_marker() {
    assert_deadline_reaches_the_descendant("marker-plugin", "timeout", serde_json::json!({}));
}

/// S1 (#367): the command the driver builds carries the workspace root as its working directory, so
/// the child never inherits the directory of the process that built the driver.
///
/// `CONTRACT.md:154-155` asks every subprocess to name its working directory. The behavioural half —
/// that the operating system really starts the child there — is `tests/plugin_process_cwd.rs`, which
/// spawns a plugin and reads the directory it printed. This half reads the command the driver would
/// spawn, so the regression is an assertion rather than a platform surprise.
#[test]
fn test_child_command_names_the_workspace_root_as_its_working_directory() {
    let name = PluginName::new("rooted-plugin").unwrap();
    let workspace = root();
    let mut caps = HashSet::new();
    caps.insert(Capability::ManifestV1);
    let driver = ProcessDriver::new(name, workspace.clone(), "echo", vec![], BTreeMap::new(), 5, caps);

    let command = driver
        .child_command()
        .expect("a driver with no declared grant has nothing to reject");

    assert_eq!(
        command.get_current_dir(),
        Some(workspace.as_path()),
        "the workspace root is named explicitly, so the child cannot inherit the caller's directory"
    );
}

#[cfg(test)]
mod env_grants;
