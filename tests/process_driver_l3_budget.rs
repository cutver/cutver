//! The invocation budget covers the whole exchange, including the drain (#316, L3).
//!
//! Why this lives here and not in `src/plugin/driver/process/tests.rs`: that file sits at 399 of the
//! 400-line ceiling `tests/source_line_budget.rs` enforces, and its ratchet table "can shrink but
//! never grow". A test this important does not get to be the reason a ceiling breaks.
//!
//! The case is the one #316 does *not* name: the direct child exits zero while something it started
//! still holds the inherited stdout pipe. Before `proc::run_capture`, the drain joins were unbounded
//! on that path, so the caller waited for the descendant (here ~3 s) instead of for its own budget
//! (1 s). The fix bounds the joins and deliberately does **not** kill a tree whose direct child
//! exited zero: an exit-zero child is not a failure, and giving it kill semantics is beyond #316.

use std::collections::{BTreeMap, HashSet};
use std::time::{Duration, Instant};

use cutver::plugin::driver::{ProcessDriver, resolve_env_grants};
use cutver::plugin::dto::PluginInvocation;
use cutver::plugin::types::{Capability, PluginCall, PluginName};
use cutver::plugin::{PluginDriver, PluginError};

#[test]
fn a_zero_exit_child_with_a_descendant_holding_stdout_stays_inside_the_budget() {
    // The fixture's own commands still need `PATH` (or `SystemRoot` on Windows) to be found at all:
    // both now come from the system baseline, and declaring them keeps the grant path exercised.
    let keys: Vec<String> = if cfg!(windows) {
        vec!["SystemRoot".into(), "PATH".into()]
    } else {
        vec!["PATH".into()]
    };
    let name = PluginName::new("l3-budget").unwrap();
    let env: BTreeMap<String, String> = resolve_env_grants(&name, &keys).unwrap();

    // The child returns immediately; the descendant outlives it by ~3 s and keeps the pipe open.
    let (command, args) = if cfg!(windows) {
        (
            "cmd".to_string(),
            vec![
                "/C".to_string(),
                "start /b \"\" cmd /c \"ping -n 4 127.0.0.1 >NUL & echo hi\"".to_string(),
            ],
        )
    } else {
        (
            "sh".to_string(),
            vec!["-c".to_string(), "(sleep 3; echo hi) & exit 0".to_string()],
        )
    };

    let mut capabilities = HashSet::new();
    capabilities.insert(Capability::LifecycleV1);
    let driver = ProcessDriver::new(name, std::env::temp_dir(), command, args, env, 1, capabilities);

    let invocation = PluginInvocation::new(PluginCall::PreBump, &serde_json::json!({})).unwrap();
    let start = Instant::now();
    let outcome = driver.invoke(&invocation);
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_millis(2500),
        "the drain join must be bounded by the budget, not by the descendant: took {elapsed:?}"
    );
    assert!(
        matches!(outcome, Err(PluginError::Timeout { timeout_secs: 1, .. })),
        "expected Timeout naming the 1s budget, got {outcome:?}"
    );
}
