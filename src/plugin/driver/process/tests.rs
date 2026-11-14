//! Behaviour tests for the process plugin driver.
//!
//! The isolation tests fail closed on an environment that is not the one the driver promises:
//! the child runs with `env_clear()`, so a key set in the parent but not granted must never
//! appear in what the plugin prints (issue #264).

use super::*;
use crate::plugin::driver::resolve_env_grants;
use crate::plugin::types::PluginCall;

#[test]
fn test_unsupported_capability() {
    let name = PluginName::new("echo-plugin").unwrap();
    let driver = ProcessDriver::new(name.clone(), "echo", vec![], BTreeMap::new(), 5, HashSet::new());
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

    let driver = ProcessDriver::new(name, cmd, args, shell_env(), 5, caps);
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

    let driver = ProcessDriver::new(name.clone(), cmd, args, shell_env(), 5, caps);
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

    let driver = ProcessDriver::new(name.clone(), cmd, args, shell_env(), 1, caps);
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
        "custom-cmd",
        vec!["--flag".to_string()],
        env.clone(),
        10,
        caps.clone(),
    );

    assert_eq!(driver.name(), &name);
    assert_eq!(driver.command(), "custom-cmd");
    assert_eq!(driver.args(), &["--flag".to_string()]);
    assert_eq!(driver.env(), &env);
    assert_eq!(driver.timeout_seconds(), 10);
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

/// S1: the granted key reaches the plugin and an ungranted parent key does not.
#[test]
fn test_only_granted_env_keys_reach_the_plugin() {
    const GRANTED: &str = "CUTVER_TEST_GRANTED_KEY";
    const WITHHELD: &str = "CUTVER_TEST_WITHHELD_SECRET";
    unsafe {
        std::env::set_var(GRANTED, "granted-value");
        std::env::set_var(WITHHELD, "withheld-value");
    }

    let name = PluginName::new("env-plugin").unwrap();
    let mut caps = HashSet::new();
    caps.insert(Capability::ManifestV1);
    let (cmd, args) = env_dump_command();

    let granted = resolve_env_grants(&name, &[GRANTED.to_string()]).expect("the granted key is set");
    let driver = ProcessDriver::new(name, cmd, args, granted, 5, caps);
    let invocation = PluginInvocation::new(PluginCall::ManifestRead, &serde_json::json!({})).unwrap();
    let output = driver.invoke(&invocation).expect("the environment dump must run");

    unsafe {
        std::env::remove_var(GRANTED);
        std::env::remove_var(WITHHELD);
    }

    let printed = String::from_utf8_lossy(&output);
    assert!(
        printed.contains("CUTVER_TEST_GRANTED_KEY=granted-value"),
        "the granted key must reach the plugin: {printed}"
    );
    assert!(
        !printed.contains("CUTVER_TEST_WITHHELD_SECRET"),
        "a parent key that is not granted must never reach the plugin: {printed}"
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

    let driver = ProcessDriver::new(name.clone(), cmd, args, shell_env(), 1, caps);
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

/// The minimum environment a shell needs when it must also find an external child command.
///
/// `ping` and `sleep` are external programs, so the platform shell needs `PATH` on both
/// platforms; Windows additionally needs `SystemRoot` to start at all.
fn descendant_env() -> BTreeMap<String, String> {
    let keys: Vec<String> = if cfg!(windows) {
        vec!["SystemRoot".to_string(), "PATH".to_string()]
    } else {
        vec!["PATH".to_string()]
    };
    let plugin = PluginName::new("orphan-fixture").unwrap();
    resolve_env_grants(&plugin, &keys).expect("the platform shell needs these keys to start")
}

/// S3: the whole exchange -- write, wait, drain -- stays inside `timeout_seconds`. A plugin that
/// leaves a descendant holding the stdin pipe must not extend the caller's wait, because the
/// driver kills only the direct child and the descendant keeps the pipe open after it.
#[test]
fn test_a_descendant_holding_stdin_cannot_outlive_the_timeout() {
    let name = PluginName::new("orphan-plugin").unwrap();
    let mut caps = HashSet::new();
    caps.insert(Capability::LifecycleV1);

    let (cmd, args) = if cfg!(windows) {
        (
            "cmd".to_string(),
            vec![
                "/C".to_string(),
                "start /b ping -n 25 127.0.0.1 >NUL & ping -n 25 127.0.0.1".to_string(),
            ],
        )
    } else {
        (
            "sh".to_string(),
            vec!["-c".to_string(), "sleep 25 & sleep 25".to_string()],
        )
    };

    let driver = ProcessDriver::new(name.clone(), cmd, args, descendant_env(), 1, caps);
    // Larger than any OS pipe buffer, so the write cannot complete while nobody drains stdin.
    let payload = serde_json::json!({ "blob": "x".repeat(2 * 1024 * 1024) });
    let invocation = PluginInvocation::new(PluginCall::PreBump, &payload).unwrap();

    let start = Instant::now();
    let err = driver.invoke(&invocation).unwrap_err();
    let elapsed = start.elapsed();

    match err {
        PluginError::Timeout {
            name: err_name,
            timeout_secs,
            ..
        } => {
            assert_eq!(err_name, name);
            assert_eq!(timeout_secs, 1);
        }
        other => panic!("expected Timeout, got {other:?}"),
    }
    assert!(
        elapsed < Duration::from_secs(5),
        "a descendant of the killed child must not extend the caller's wait, took {elapsed:?}"
    );
}

/// S4: a declared but unset grant is an error naming the plugin and the key, never an empty
/// string handed to the plugin. `resolve_env_grants` is the authority the process factory and the
/// WASM manifest builder both resolve their grants through.
#[test]
fn test_declared_but_unset_grant_names_plugin_and_key() {
    const KEY: &str = "CUTVER_TEST_UNSET_GRANT_KEY";
    unsafe {
        std::env::remove_var(KEY);
    }

    let name = PluginName::new("grant-plugin").unwrap();
    let err = resolve_env_grants(&name, &[KEY.to_string()]).expect_err("an unset grant must fail");
    let message = err.to_string();

    match err {
        PluginError::UnsetEnvGrant {
            name: err_name,
            key: err_key,
        } => {
            assert_eq!(err_name, name);
            assert_eq!(err_key, KEY);
        }
        other => panic!("expected UnsetEnvGrant, got {other:?}"),
    }
    assert!(
        message.contains("grant-plugin"),
        "message must name the plugin: {message}"
    );
    assert!(message.contains(KEY), "message must name the key: {message}");
    assert!(
        message.contains("[plugins.grant-plugin.permissions]"),
        "message must name the grant it failed to resolve: {message}"
    );
}
