//! `cutver doctor` reports what it resolved (#268 S1, S5, S6), including the environment grants a
//! host cannot produce (#300).
//!
//! A `[plugins.<name>.permissions] filesystem` entry used to be overwritten with the resolved guest
//! path while the document was loading, so the declared spelling was destroyed before anything
//! downstream could print it, and the canonical host directory the sandbox really opens lived only
//! as a local in the WASM driver. `config::discovery` records all three values now, and the
//! diagnosis prints them: `declared -> resolved -> canonical`, with the plugin name and its
//! capability set.
//!
//! Half of this suite reads the loaded `Config` through the library, because S6 asks the declared
//! spelling to *survive* resolution; the other half spawns the binary, because S6 also asks the
//! rendered output — human and `--json` — to carry it.

mod common;

use std::path::Path;

use common::{FixtureGuard, cutver_command};
use cutver::cli::doctor::{DoctorReport, plugin_section, render_dashboard, render_report_json};
use cutver::cli::style::Theme;
use cutver::config;
use cutver::plugin::PluginName;
use serde_json::Value;

/// A plugin declaring two grants: one directory that exists, and one that does not exist yet, so the
/// two answers the report can give are both reachable — `inside the workspace` and `unresolved`.
const TWO_GRANTS: &str = r#"[version]
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
filesystem = ["assets", "generated"]
"#;

/// The same grant on the runtime that enforces it: a WASM plugin, whose artifact this fixture does
/// not ship, because the row is what S2 governs and the build failure is reported separately.
const WASM_GRANT: &str = r#"[version]
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[plugins.sandbox]
runtime = "wasm"
source = "sandbox.wasm"
capabilities = ["changelog.v1"]

[plugins.sandbox.permissions]
filesystem = ["assets"]
"#;

/// The environment key no fixture sets: the driver resolves it at invocation, so a host that does
/// not export it is the #300 scenario.
const UNSET_GRANT: &str = "CUTVER_TEST_UNSET_DOCTOR_GRANT";

/// The clause a `permissions.filesystem` row carries when the declared runtime never reads it.
const INERT: &str = "inert: runtime = \"process\" applies only permissions.env";

const UNSET_ENV_GRANT: &str = r#"[version]
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

fn workspace(name: &str, config: &str) -> FixtureGuard {
    let guard = FixtureGuard::new(name);
    let fixture = guard.fixture();
    fixture.write("package.json", "{\n  \"version\": \"1.2.0\"\n}\n");
    fixture.write(
        "assets/keep.txt",
        "a granted directory has to exist to be canonicalized\n",
    );
    fixture.write("cutver.toml", config);
    guard
}

/// A host path the way `config::discovery` reports every configured path: canonical, forward-slashed
/// and without the Windows verbatim prefix.
fn host_path(path: &Path) -> String {
    let text = std::fs::canonicalize(path)
        .unwrap_or_else(|err| panic!("{} must exist to be canonicalized: {err}", path.display()))
        .to_string_lossy()
        .replace('\\', "/");
    text.strip_prefix("//?/").map(str::to_string).unwrap_or(text)
}

fn doctor_json(dir: &Path) -> (i32, Value, String) {
    let output = cutver_command(dir)
        .args(["doctor", "--json"])
        .output()
        .expect("failed to execute the cutver binary");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let code = output.status.code().expect("the process exited");
    let document = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|err| panic!("stdout must be one JSON object, got {stdout:?}: {err}; stderr: {stderr}"));
    (code, document, stderr)
}

/// S2, S3, S5: resolution keeps the declared spelling, the guest path it becomes and the canonical
/// host directory behind it, and it is these three that anything downstream reads.
#[test]
fn the_declared_spelling_survives_resolution_next_to_the_guest_path_and_the_canonical_host() {
    let guard = workspace("grants-resolved", TWO_GRANTS);
    let fixture = guard.fixture();
    let loaded = config::load(fixture.dir.join("cutver.toml")).expect("the fixture loads");
    let probe = PluginName::new("probe").expect("the fixture names the plugin");
    let plugin = loaded.plugins.get(&probe).expect("the fixture declares the plugin");

    let grants = &plugin.permissions.filesystem_grants;
    assert_eq!(
        grants.iter().map(|grant| grant.declared.as_str()).collect::<Vec<_>>(),
        vec!["assets", "generated"],
        "the spelling the document declared is retained, in declaration order"
    );

    let root = host_path(&fixture.dir);
    assert_eq!(
        grants[0].resolved,
        format!("{root}/assets"),
        "the guest path is the declared entry resolved against the workspace root"
    );
    assert_eq!(
        plugin.permissions.filesystem,
        Vec::from([grants[0].resolved.clone(), grants[1].resolved.clone()]),
        "`filesystem` keeps carrying the guest path, which is what the driver mounts"
    );

    let canonical_assets = host_path(&fixture.dir.join("assets"));
    assert_eq!(
        grants[0].canonical.as_deref(),
        Some(canonical_assets.as_str()),
        "the canonical host directory is retained instead of being recomputed and dropped"
    );
    assert_eq!(
        grants[1].canonical, None,
        "a directory that does not exist yet cannot be canonicalized, which is a third answer"
    );
}

/// S1, S5, S6, and #367 S2: one row per grant, carrying the three values, the plugin name and the
/// capability set, whether the grant landed inside the workspace, and — because this fixture is a
/// `runtime = "process"` plugin, whose driver reads neither `filesystem` nor `network` — that the
/// runtime ignores the grant instead of applying it.
#[test]
fn the_dashboard_prints_one_row_per_grant_with_its_plugin_capabilities_and_three_values() {
    let guard = workspace("grants-dashboard", TWO_GRANTS);
    let fixture = guard.fixture();
    let loaded = config::load(fixture.dir.join("cutver.toml")).expect("the fixture loads");
    let theme = Theme::new(false);

    let rendered = render_dashboard(&loaded, false, &theme);

    let grant_rows = rendered
        .lines()
        .filter(|line| line.trim_start().starts_with("Grant"))
        .count();
    assert_eq!(
        grant_rows, 2,
        "every `permissions.filesystem` entry gets its own row, and no other row is a grant: {rendered}"
    );

    let root = host_path(&fixture.dir);
    assert!(
        rendered.contains(&format!(
            "probe [changelog.v1]: \"assets\" -> {root}/assets -> {root}/assets (inside the workspace) -> {INERT}"
        )),
        "the row carries plugin name, capability set, declared, resolved, canonical and where it \
         landed, and states that the process runtime ignores it: {rendered}"
    );
    assert!(
        rendered.contains(&format!(
            "probe [changelog.v1]: \"generated\" -> {root}/generated -> unresolved (the directory does not exist yet) -> {INERT}"
        )),
        "a grant that resolves inside the workspace to a directory that is not there says so \
         instead of pretending to be outside it, and stays inert: {rendered}"
    );
}

/// S6 and #367 S2: the machine surface carries the same three values, so a consumer that never
/// reads the dashboard can act on what was resolved, and the same statement about what the runtime
/// does with the grant — the plugin's runtime, and the effect it has on the entry.
#[test]
fn the_json_surface_carries_the_same_three_values_per_grant() {
    let guard = workspace("grants-json", TWO_GRANTS);
    let fixture = guard.fixture();

    let (code, document, stderr) = doctor_json(&fixture.dir);

    assert_eq!(code, 0, "a clean diagnosis exits 0; stderr: {stderr}");
    assert_eq!(document["ok"], true, "{document}");
    assert_eq!(
        document["plugin_load_error"],
        Value::Null,
        "a plugin that built is not a load failure: {document}"
    );

    let plugin = &document["plugins"][0];
    assert_eq!(plugin["name"], "probe", "{document}");
    assert_eq!(
        plugin["capabilities"],
        serde_json::json!(["changelog.v1"]),
        "{document}"
    );
    assert_eq!(
        plugin["runtime"], "process",
        "the record names the runtime that decides what a grant means: {document}"
    );

    let root = host_path(&fixture.dir);
    let assets = &plugin["grants"][0];
    assert_eq!(assets["declared"], "assets", "{document}");
    assert_eq!(assets["resolved"], format!("{root}/assets"), "{document}");
    assert_eq!(
        assets["canonical"],
        host_path(&fixture.dir.join("assets")),
        "the canonical host directory travels as the third value: {document}"
    );
    assert_eq!(assets["location"], "inside", "{document}");
    assert_eq!(
        assets["effect"], "ignored",
        "a runtime = \"process\" plugin does not apply a filesystem grant, and the machine surface \
         says so: {document}"
    );

    let generated = &plugin["grants"][1];
    assert_eq!(generated["declared"], "generated", "{document}");
    assert_eq!(generated["canonical"], Value::Null, "{document}");
    assert_eq!(generated["location"], "unresolved", "{document}");
    assert_eq!(generated["effect"], "ignored", "{document}");
}

/// #300: a declared environment grant the host cannot produce is a row, not a load failure.
///
/// The previous decision (D1 of `odd/tasks/cutver-doctor-grants-and-swallowed-reads.md`) existed
/// because construction resolved `permissions.env`, so an unset grant could only be reported as "the
/// manager could not be built". Construction resolves nothing now, so there is no load failure left
/// to report, and the state travels per key — the one the driver will find when it is invoked.
#[test]
fn an_unset_grant_is_reported_as_its_own_row_not_as_a_load_failure() {
    let guard = workspace("grants-load-failure", UNSET_ENV_GRANT);
    let fixture = guard.fixture();
    let loaded = config::load(fixture.dir.join("cutver.toml")).expect("the fixture loads");

    let (plugins, load_error) = plugin_section(&loaded);
    assert!(
        load_error.is_none(),
        "an unset grant is not a construction failure: {load_error:?}"
    );
    assert_eq!(
        plugins.iter().map(|plugin| plugin.name.as_str()).collect::<Vec<_>>(),
        vec!["probe"],
        "what was resolved is still reported next to the failure"
    );
    let grant = &plugins[0].env_grants[0];
    assert_eq!(grant.key, UNSET_GRANT, "the declared key travels as declared");
    assert!(!grant.set, "the host cannot produce it, and the report says so");

    let rendered = render_dashboard(&loaded, false, &Theme::new(false));
    assert!(
        rendered.contains(&format!("\"{UNSET_GRANT}\" -> unset")),
        "the dashboard states the resolution in its own row: {rendered}"
    );

    let mut report = DoctorReport::new(Vec::new(), None);
    (report.plugins, report.plugin_load_error) = plugin_section(&loaded);
    assert!(report.ok, "a declared grant is not drift");
    let json = render_report_json(&report).expect("the report serializes");
    let parsed: Value = serde_json::from_str(&json).expect("the report is valid JSON");
    assert_eq!(
        parsed["plugin_load_error"],
        Value::Null,
        "a grant the host cannot produce is not a failed load: {json}"
    );
    assert_eq!(
        parsed["plugins"][0]["env_grants"][0]["set"], false,
        "the JSON carries the same answer the dashboard renders: {json}"
    );
    assert_eq!(parsed["ok"], true, "reporting is not drift: {json}");
}

/// The section the report is built from is also readable on its own, so the runner and the dashboard
/// cannot disagree about what was resolved.
#[test]
fn the_plugin_section_reports_every_plugin_in_name_order() {
    let guard = workspace("grants-section", TWO_GRANTS);
    let fixture = guard.fixture();
    let loaded = config::load(fixture.dir.join("cutver.toml")).expect("the fixture loads");

    let (plugins, load_error) = plugin_section(&loaded);

    assert_eq!(
        plugins.iter().map(|plugin| plugin.name.as_str()).collect::<Vec<_>>(),
        vec!["probe"],
        "one entry per declared plugin"
    );
    assert_eq!(load_error, None, "this workspace builds its drivers");
}

/// #367 S2, PRESERVE: the WASM runtime really does open the directory a grant names, so its rows
/// keep the text they had and `--json` says the grant is applied.
///
/// Read through the library, not the binary: this fixture declares a WASM plugin whose artifact it
/// does not ship, and the row is what S2 governs — the artifact failure is reported as a load error
/// of its own and does not reach the grant rendering.
#[test]
fn a_wasm_grant_is_reported_as_applied_and_its_row_stays_unchanged() {
    let guard = workspace("grants-wasm", WASM_GRANT);
    let fixture = guard.fixture();
    let loaded = config::load(fixture.dir.join("cutver.toml")).expect("the fixture loads");

    let root = host_path(&fixture.dir);
    let rendered = render_dashboard(&loaded, false, &Theme::new(false));
    assert!(
        rendered.contains(&format!(
            "sandbox [changelog.v1]: \"assets\" -> {root}/assets -> {root}/assets (inside the workspace)\n"
        )),
        "an applied grant's row ends at the location, exactly as before: {rendered}"
    );
    assert!(
        !rendered.contains("inert"),
        "nothing is inert for the WASM runtime: {rendered}"
    );

    let mut report = DoctorReport::new(Vec::new(), None);
    (report.plugins, report.plugin_load_error) = plugin_section(&loaded);
    let json = render_report_json(&report).expect("the report serializes");
    let parsed: Value = serde_json::from_str(&json).expect("the report is valid JSON");
    assert_eq!(parsed["plugins"][0]["runtime"], "wasm", "{json}");
    assert_eq!(parsed["plugins"][0]["grants"][0]["effect"], "applied", "{json}");
}

/// The two rows the dashboard had before this change, and the fixed-width cell they are rendered
/// into. They moved here from the unit tests of `src/cli/doctor.rs`, which is a ratchet entry in
/// `tests/source_line_budget.rs` and may shrink but never grow.
mod dashboard {
    use cutver::cli::doctor::{render_dashboard, render_grid_row};
    use cutver::cli::style::Theme;
    use cutver::config::{Config, Manifest, ManifestKind, ManifestPath, PreflightCommand, VersionSection};
    use std::path::PathBuf;

    #[test]
    fn test_render_grid_row_padding() {
        let theme = Theme::new(false);
        let mut out = String::new();
        render_grid_row(&mut out, "Configuration", "valid (cutver.toml)", &theme);
        assert_eq!(out, "  Configuration   valid (cutver.toml)\n");
    }

    #[test]
    fn test_render_dashboard_layout() {
        let theme = Theme::new(false);
        let config = Config {
            root_dir: PathBuf::from("."),
            version: VersionSection {
                current_source: "Cargo.toml".into(),
                strategy: "manual".into(),
                plugin: None,
            },
            manifest: vec![Manifest {
                path: ManifestPath::parse("Cargo.toml").unwrap(),
                package: None,
                primary: true,
                kind: ManifestKind::CargoPackage,
            }],
            preflight: vec![(
                "test".into(),
                PreflightCommand {
                    command: "cargo test".into(),
                    timeout: None,
                },
            )],
            ..Default::default()
        };

        let rendered = render_dashboard(&config, true, &theme);
        assert!(rendered.contains("✔ cutver.toml is valid\n\n"));
        assert!(rendered.contains("  Configuration   valid (cutver.toml)\n"));
        assert!(rendered.contains("  Manifests       1 tracked (Cargo.toml @ "));
        assert!(rendered.contains("  Preflight       1 step defined (test: cargo test)\n"));
        assert!(rendered.contains("  Changelog       consistent with Git tags\n"));
        assert!(
            rendered.contains("  Plugins         0 declared\n"),
            "a document without plugins still reports the section, so its absence is readable: {rendered}"
        );
    }
}
