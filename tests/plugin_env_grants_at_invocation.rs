//! Issue #300: the declared `[plugins.<name>.permissions] env` grants are resolved when the plugin
//! is invoked, not while the configuration is loaded.
//!
//! The premise is a command that only inspects the configuration: a release runner (and most
//! developer machines) has no `GITHUB_TOKEN` exported for a step that never invokes the plugin, so
//! `cutver doctor` and `cutver plugin restore` must run there and *say* the grant is unset. The
//! fail-closed guarantee of #264 keeps its ground at the boundary it belongs to — invocation.
//!
//! Every test spawns the binary, because the defect is that a command refuses to run.

mod common;

use common::{FixtureGuard, cutver_command};
use std::process::Output;

/// The key no fixture sets: it is what makes `doctor` and `restore` fail before #300 is fixed.
const UNSET_GRANT: &str = "CUTVER_TEST_UNSET_DOCTOR_GRANT";

/// A process plugin declaring one environment grant. A process runtime keeps this suite runnable in
/// the lean build, where the `plugins` feature (and with it the WASM driver) is off.
const UNSET_PROCESS_GRANT: &str = r#"[version]
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[plugins.probe]
runtime = "process"
command = "probe-plugin"
capabilities = ["changelog.v1"]

[plugins.probe.permissions]
env = ["CUTVER_TEST_UNSET_DOCTOR_GRANT"]
"#;

/// The same plugin, declaring a key every host sets, so the other answer is reachable too.
const SET_PROCESS_GRANT: &str = r#"[version]
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[plugins.probe]
runtime = "process"
command = "probe-plugin"
capabilities = ["changelog.v1"]

[plugins.probe.permissions]
env = ["PATH"]
"#;

fn workspace(name: &str, config: &str) -> FixtureGuard {
    let guard = FixtureGuard::new(name);
    let fixture = guard.fixture();
    fixture.write("package.json", "{\n  \"version\": \"1.2.0\"\n}\n");
    fixture.write("cutver.toml", config);
    guard
}

/// Runs `cutver <args>` in `dir` and returns the whole exchange.
fn run(guard: &FixtureGuard, args: &[&str]) -> Output {
    let mut command = cutver_command(&guard.fixture().dir);
    command.args(args);
    command.output().expect("the cutver binary runs")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// S1: a configuration check runs on a host that has no such secret, and it names the key it could
/// not resolve instead of refusing to diagnose the workspace.
#[test]
fn doctor_succeeds_with_an_unset_grant_and_says_the_grant_is_unset() {
    let guard = workspace("grant-unset-doctor", UNSET_PROCESS_GRANT);

    let output = run(&guard, &["doctor"]);
    let out = stdout(&output);

    assert!(
        output.status.success(),
        "a configuration check must not need production secrets: stdout={out} stderr={}",
        stderr(&output)
    );
    assert!(
        stderr(&output).is_empty(),
        "a successful doctor writes no stderr: {}",
        stderr(&output)
    );
    assert!(
        out.contains(UNSET_GRANT),
        "the row names the declared key a runner has to look at: {out}"
    );
    assert!(
        out.contains(&format!("\"{UNSET_GRANT}\" -> unset")),
        "the row states the resolution, not just the declaration: {out}"
    );
    assert!(
        !out.contains("could not load:"),
        "an unset grant is a resolved finding, not a manager that failed to build: {out}"
    );
}

/// S1, the other answer: a grant the host does set is reported as set, and never as its value — a
/// diagnosis prints declarations, not secrets.
#[test]
fn doctor_reports_a_set_grant_without_printing_its_value() {
    let guard = workspace("grant-set-doctor", SET_PROCESS_GRANT);
    let path_value = std::env::var("PATH").expect("the test host sets PATH");

    let output = run(&guard, &["doctor"]);
    let out = stdout(&output);

    assert!(output.status.success(), "stdout={out} stderr={}", stderr(&output));
    assert!(out.contains("\"PATH\" -> set"), "a set grant is reported as set: {out}");
    assert!(
        !path_value.is_empty() && !out.contains(&path_value),
        "the granted value stays private; doctor prints the declaration only: {out}"
    );
}

/// S1 travels as data too, so a runner reads the same answer the dashboard prints.
#[test]
fn doctor_json_carries_the_state_of_every_declared_grant() {
    let guard = workspace("grant-json-doctor", UNSET_PROCESS_GRANT);

    let output = run(&guard, &["doctor", "--json"]);
    let out = stdout(&output);
    assert!(output.status.success(), "stdout={out} stderr={}", stderr(&output));

    let parsed: serde_json::Value = serde_json::from_str(&out).expect("doctor --json prints an object");
    let plugin = &parsed["plugins"][0];
    assert_eq!(plugin["name"], "probe", "the plugin travels by name: {out}");
    assert_eq!(plugin["env_grants"][0]["key"], UNSET_GRANT, "{out}");
    assert_eq!(
        plugin["env_grants"][0]["set"], false,
        "the JSON states the same resolution the row does: {out}"
    );
}

/// S5: no construction path resolves a grant. The manager builds drivers; resolution belongs to the
/// invocation, which is the only moment the map is handed over. The scan states both halves, so it
/// cannot pass by finding nothing.
#[test]
fn no_construction_path_resolves_the_declared_grants() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let manager = root.join("src/plugin/manager");

    let mut scanned = 0;
    for entry in std::fs::read_dir(&manager).expect("the manager module is readable") {
        let path = entry.expect("a directory entry").path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("a readable source");
        for forbidden in ["env_grants_from_lookup", "resolve_env_grants"] {
            assert!(
                !text.contains(forbidden),
                "{} calls `{forbidden}`: building a driver records the declaration and resolves \
                 nothing, so a configuration check runs without the secret (#300 S5)",
                path.display()
            );
        }
        scanned += 1;
    }
    assert!(
        scanned >= 2,
        "the walk must reach the factory and the core; it scanned {scanned} files under {}",
        manager.display()
    );

    let process = std::fs::read_to_string(root.join("src/plugin/driver/process.rs")).expect("readable");
    assert!(
        process.contains(".grants.resolve("),
        "the process driver resolves the declared grants when it builds the child command"
    );
    let wasm = std::fs::read_to_string(root.join("src/plugin/driver/wasm.rs")).expect("readable");
    assert!(
        wasm.contains("grants.resolve("),
        "the WASM driver resolves them at instantiation, which is its invocation boundary"
    );
}

/// S2: `cutver plugin restore` restores artifacts. It never invokes a plugin, so an unset grant in
/// the declaration it reads must not stop it — the release workflow's step has no token to give.
#[cfg(feature = "plugins")]
#[test]
fn plugin_restore_succeeds_with_the_grant_unset() {
    let guard = FixtureGuard::new("grant-unset-restore");
    let fixture = guard.fixture();
    fixture.write("package.json", "{\n  \"version\": \"1.2.0\"\n}\n");
    fixture.write(
        "cutver.toml",
        r#"[version]
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[plugins.probe]
runtime = "wasm"
source = "probe.wasm"
capabilities = ["changelog.v1"]

[plugins.probe.permissions]
env = ["CUTVER_TEST_UNSET_DOCTOR_GRANT"]
"#,
    );
    // A local artifact is `AlreadyPresent`, so the command is hermetic: it never reaches a registry.
    fixture.write("probe.wasm", "\0asm");

    let output = run(&guard, &["plugin", "restore"]);
    let out = stdout(&output);

    assert!(
        output.status.success(),
        "restoring artifacts does not invoke the plugin, so it does not need the grant: \
         stdout={out} stderr={}",
        stderr(&output)
    );
    assert!(
        out.contains("already present") || out.contains("Already present") || out.contains("0 restored"),
        "the command reports what it did: {out}"
    );
}
