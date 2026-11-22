//! The system baseline a `runtime = "process"` child receives (#369 S1).
//!
//! `env_clear()` alone is the wrong boundary for an authoring surface: a Python plugin that never
//! declared anything gets no `PATH`, no `HOME`, no `TMPDIR` and, on Windows, no `SystemRoot` — the
//! last one stops a managed engine from starting at all. `tests/process_driver_env_isolation.rs`
//! pins the other half (a key outside the baseline still never crosses), and
//! `src/plugin/driver/process/tests.rs` pins that a declared grant arrives. This suite pins the
//! baseline itself: which keys it covers, that an unset key is omitted rather than emptied, and that
//! a declared grant still wins over it.

use std::collections::{BTreeMap, HashSet};

use cutver::plugin::driver::{ProcessDriver, SYSTEM_ENV_BASELINE};
use cutver::plugin::dto::PluginInvocation;
use cutver::plugin::types::PluginName;
use cutver::plugin::{Capability, PluginCall, PluginDriver, PluginError};

/// Name shapes the baseline must never contain: a baseline is ambient convenience, not authority.
/// The check catches the shapes it names; a credential named after none of them (`GH_PAT`, say) is
/// caught by review, which is why `SYSTEM_ENV_BASELINE` stays a short, hand-written list.
const SECRET_SHAPES: [&str; 6] = ["TOKEN", "SECRET", "PASSWORD", "CREDENTIAL", "KEY", "JWT"];

/// The dump command: the child **drains its envelope** and then prints its whole environment, one
/// `key=value` per line.
///
/// Draining first is not decoration. `/usr/bin/env` and `set` answer without ever reading stdin, so the
/// host's envelope write races the child's exit and can land on a closed pipe — `StdinWriteFailed`,
/// `BrokenPipe` — on any host where the child wins. That is the trap
/// `docs/guides/plugin-process-runtime.md` documents for plugin authors, and this fixture walked into
/// it: it passed on Windows and ubuntu and failed on the macOS leg of PR #370.
fn env_dump_command() -> (String, Vec<String>) {
    if cfg!(windows) {
        ("cmd".to_string(), vec!["/C".to_string(), "more >nul & set".to_string()])
    } else {
        (
            "/bin/sh".to_string(),
            vec!["-c".to_string(), "cat >/dev/null; exec /usr/bin/env".to_string()],
        )
    }
}

/// The keys of the baseline this host really sets, in the order the constant declares them.
fn baseline_keys_present() -> Vec<&'static str> {
    SYSTEM_ENV_BASELINE
        .iter()
        .copied()
        .filter(|key| std::env::var_os(key).is_some())
        .collect()
}

fn name() -> PluginName {
    PluginName::new("env-baseline").unwrap()
}

fn capabilities() -> HashSet<Capability> {
    HashSet::from([Capability::ManifestV1])
}

/// Runs the dump through a driver whose only grants are `granted`, and returns what the child
/// printed. Nothing here mutates the process environment (#344).
fn dump(granted: BTreeMap<String, String>) -> String {
    let (command, args) = env_dump_command();
    let driver = ProcessDriver::new(name(), std::env::temp_dir(), command, args, granted, 10, capabilities());
    let invocation = PluginInvocation::new(PluginCall::ManifestRead, &serde_json::json!({})).unwrap();
    let printed: Result<Vec<u8>, PluginError> = driver.invoke(&invocation);
    // The dump itself is never formatted into a message: it may contain credentials the parent
    // environment legitimately holds.
    String::from_utf8_lossy(&printed.expect("the environment dump must run")).to_string()
}

/// Every value the child printed for `key`, matched case-insensitively because Windows variable
/// names are (`set` echoes the casing the variable was created with, which is `Path`, not `PATH`).
fn values_printed(printed: &str, key: &str) -> Vec<String> {
    printed
        .lines()
        .filter_map(|line| line.split_once('='))
        .filter(|(name, _)| name.eq_ignore_ascii_case(key))
        .map(|(_, value)| value.to_string())
        .collect()
}

#[test]
fn the_baseline_reaches_a_child_that_declared_nothing() {
    let present = baseline_keys_present();
    assert!(
        !present.is_empty(),
        "this test needs at least one baseline key set in the host environment; none of \
         {SYSTEM_ENV_BASELINE:?} is"
    );

    let printed = dump(BTreeMap::new());

    for key in present {
        assert!(
            !values_printed(&printed, key).is_empty(),
            "'{key}' is a baseline key and is set in this process, so a child that declared \
             nothing still receives it: env_clear() plus a documented system baseline, not \
             env_clear() alone"
        );
    }
}

#[test]
fn a_baseline_key_the_host_does_not_set_is_omitted_never_emptied() {
    let printed = dump(BTreeMap::new());

    for key in SYSTEM_ENV_BASELINE {
        let values = values_printed(&printed, key);
        assert!(
            !values.iter().any(|value| value.is_empty()),
            "the baseline must omit '{key}' when the host does not set it, never hand over an \
             empty string: an empty value hides a broken environment, and fail-closed is the \
             declared grants' job"
        );
    }

    for key in SYSTEM_ENV_BASELINE
        .iter()
        .copied()
        .filter(|key| std::env::var_os(key).is_none())
    {
        assert!(
            values_printed(&printed, key).is_empty(),
            "'{key}' is not set in this process, so it must not appear in the child at all"
        );
    }
}

#[test]
fn a_declared_grant_wins_over_the_baseline() {
    let granted = BTreeMap::from([("PATH".to_string(), "/cutver/grant/wins".to_string())]);

    let printed = dump(granted);
    let paths = values_printed(&printed, "PATH");

    assert_eq!(
        paths,
        vec!["/cutver/grant/wins".to_string()],
        "a declared permissions.env key is the author's answer for that key, so it replaces the \
         baseline value instead of colliding with it"
    );
}

#[test]
fn the_baseline_names_no_secret_shaped_key() {
    for key in SYSTEM_ENV_BASELINE {
        let upper = key.to_ascii_uppercase();
        assert!(
            !upper.ends_with("_KEY"),
            "'{key}' is granted to every process plugin without being declared; a *_KEY shape is \
             a secret and can never be part of the baseline"
        );
        for shape in SECRET_SHAPES {
            assert!(
                !upper.contains(shape),
                "'{key}' carries the secret shape {shape}: the baseline is ambient convenience, \
                 not authority"
            );
        }
    }
}
