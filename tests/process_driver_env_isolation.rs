//! The child runs with `env_clear()` plus the named system baseline (#264, #344, #369 S1): a key
//! outside that baseline, and not granted, never reaches it — and that assertion has to be able to
//! fail.
//!
//! `src/plugin/driver/process/tests.rs` proves a granted key arrives and an invented one does not.
//! An invented key is absent from the parent environment too, so that half would stay green if the
//! driver stopped clearing: it says what the map contains, not what the child inherited. This suite
//! is the other half — it names keys that really are in the parent environment and asserts they are
//! not in the child, then runs the same command inheriting the environment to prove the name is
//! visible there. Without that second half the first one cannot distinguish clearing from luck.
//!
//! The names deliberately exclude every key of `SYSTEM_ENV_BASELINE` (no `PATH`, `HOME`, `TEMP`,
//! `LANG`): those reach the child on purpose, and
//! `tests/process_driver_env_baseline.rs` is the suite that pins them.

use std::collections::{BTreeMap, HashSet};
use std::process::Command;

use cutver::plugin::driver::ProcessDriver;
use cutver::plugin::dto::PluginInvocation;
use cutver::plugin::types::PluginName;
use cutver::plugin::{Capability, PluginCall, PluginDriver, PluginError};

/// The dump command: the child prints its whole environment, one `key=value` per line.
fn env_dump_command() -> (String, Vec<String>) {
    if cfg!(windows) {
        ("cmd".to_string(), vec!["/c".to_string(), "set".to_string()])
    } else {
        ("/usr/bin/env".to_string(), Vec::new())
    }
}

/// Keys the parent process is expected to carry, which neither the baseline nor the driver supplies.
fn ambient_candidates() -> Vec<&'static str> {
    if cfg!(windows) {
        vec!["USERNAME", "COMPUTERNAME", "PROCESSOR_ARCHITECTURE"]
    } else {
        vec!["PWD", "SHELL", "LOGNAME", "OLDPWD"]
    }
}

/// The keys that are really set in this process, so an absent key cannot stand in for a cleared one.
fn ambient_keys_present() -> Vec<&'static str> {
    ambient_candidates()
        .into_iter()
        .filter(|key| std::env::var_os(key).is_some())
        .collect()
}

fn name() -> PluginName {
    PluginName::new("env-isolation").unwrap()
}

fn capabilities() -> HashSet<Capability> {
    HashSet::from([Capability::ManifestV1])
}

fn invoke(driver: &ProcessDriver) -> Result<Vec<u8>, PluginError> {
    let invocation = PluginInvocation::new(PluginCall::ManifestRead, &serde_json::json!({})).unwrap();
    driver.invoke(&invocation)
}

/// The driver the test drives: the granted map is the only environment beyond the system baseline.
fn granted() -> BTreeMap<String, String> {
    BTreeMap::from([("CUTVER_ENV_ISOLATION".to_string(), "explicit".to_string())])
}

#[test]
fn the_child_inherits_nothing_the_driver_was_not_given() {
    let present = ambient_keys_present();
    assert!(
        !present.is_empty(),
        "this test needs at least one ambient key to name; the environment carries none of {:?}",
        ambient_candidates()
    );

    let (cmd, args) = env_dump_command();
    let driver = ProcessDriver::new(name(), std::env::temp_dir(), cmd, args, granted(), 10, capabilities());
    let printed = String::from_utf8_lossy(&invoke(&driver).expect("the environment dump must run")).to_string();

    assert!(
        printed.contains("CUTVER_ENV_ISOLATION=explicit"),
        "the granted key must reach the child; the dump is withheld because it may contain credentials"
    );
    for key in present {
        assert!(
            !printed.contains(&format!("{key}=")),
            "'{key}' was not granted and must not reach the child: the driver clears the environment \
             and then applies only the system baseline and the granted keys"
        );
    }
}

/// The half that stops the test above from being vacuous: the same command, inheriting the parent
/// environment, does show those keys. So their absence in the driver's child is the driver's doing.
#[test]
fn the_same_command_inherits_those_keys_when_nothing_clears_them() {
    let present = ambient_keys_present();
    assert!(
        !present.is_empty(),
        "the environment carries none of {:?}",
        ambient_candidates()
    );

    let (cmd, args) = env_dump_command();
    let output = Command::new(cmd)
        .args(args)
        .output()
        .expect("the environment dump must run");
    let printed = String::from_utf8_lossy(&output.stdout).to_string();

    for key in present {
        assert!(
            printed.contains(&format!("{key}=")),
            "'{key}' is set in this process, so a child that inherits shows it: without this half the \
             isolation test could pass on a cleared environment it never had"
        );
    }
}
