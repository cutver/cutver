//! The descendant fixtures: a plugin that overruns its budget while a child of it writes a marker,
//! kept apart because `process/tests.rs` is held to the 400-line hard ceiling (CONTRACT.md:174).

use super::root;
use crate::plugin::driver::{PluginDriver, ProcessDriver, resolve_env_grants};
use crate::plugin::dto::PluginInvocation;
use crate::plugin::error::PluginError;
use crate::plugin::types::{Capability, PluginCall, PluginName};
use std::collections::{BTreeMap, HashSet};
use std::time::{Duration, Instant};

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
pub(super) fn assert_deadline_reaches_the_descendant(plugin: &str, label: &str, payload: serde_json::Value) {
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
    assert!(
        elapsed < Duration::from_secs(5),
        "the budget bounds the exchange, took {elapsed:?}"
    );
    std::thread::sleep(Duration::from_secs(4));
    let now = std::time::SystemTime::now();
    let age = std::fs::metadata(&marker)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|wrote| now.duration_since(wrote).ok())
        .map(|d| format!("{} ms", d.as_millis()));
    let survived = marker.exists();
    let _ = std::fs::remove_file(&marker);
    assert!(
        !survived,
        "a descendant outlived the invocation, writing {} {} before this check: the deadline must reach the whole tree",
        marker.display(),
        age.as_deref().unwrap_or("an unknown time")
    );
}
