//! `cutver doctor` reports what it resolved (#268 S1, S5, S6) and the plugin-manager load failure it
//! used to discard (D1 of `odd/tasks/cutver-doctor-grants-and-swallowed-reads.md`).
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

/// The environment key no fixture sets: the driver constructor fails on it, which is D1's scenario.
const UNSET_GRANT: &str = "CUTVER_TEST_UNSET_DOCTOR_GRANT";

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

/// S1, S5, S6: one row per grant, carrying the three values, the plugin name and the capability set,
/// and stating whether the grant landed inside the workspace.
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
            "probe [changelog.v1]: \"assets\" -> {root}/assets -> {root}/assets (inside the workspace)"
        )),
        "the row carries plugin name, capability set, declared, resolved, canonical and where it \
         landed: {rendered}"
    );
    assert!(
        rendered.contains(&format!(
            "probe [changelog.v1]: \"generated\" -> {root}/generated -> unresolved (the directory does not exist yet)"
        )),
        "a grant that resolves inside the workspace to a directory that is not there says so \
         instead of pretending to be outside it: {rendered}"
    );
}

/// S6: the machine surface carries the same three values, so a consumer that never reads the
/// dashboard can act on what was resolved.
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

    let generated = &plugin["grants"][1];
    assert_eq!(generated["declared"], "generated", "{document}");
    assert_eq!(generated["canonical"], Value::Null, "{document}");
    assert_eq!(generated["location"], "unresolved", "{document}");
}

/// D1: `PluginManager::from_config` resolves `permissions.env` while it builds the drivers, so a host
/// with a declared grant unset (the #300 scenario) cannot construct the manager. `doctor` used to
/// discard that with `.ok()`; it reports the typed error as its own output instead — a dashboard row
/// and the `plugin_load_error` field — and neither `ok` nor an exit code is derived from it.
///
/// This is asserted on the library surface, because on that host `bump::doctor` already fails the
/// command before the dashboard is reached (`src/bump/exec/doctor.rs:17` propagates the same
/// construction); that failure is #300's own bug and out of scope here.
#[test]
fn a_plugin_the_manager_could_not_build_is_reported_as_output_instead_of_being_discarded() {
    let guard = workspace("grants-load-failure", UNSET_ENV_GRANT);
    let fixture = guard.fixture();
    let loaded = config::load(fixture.dir.join("cutver.toml")).expect("the fixture loads");

    let (plugins, load_error) = plugin_section(&loaded);
    let reported = load_error
        .as_deref()
        .unwrap_or_else(|| panic!("a manager that cannot be built is reported, not dropped"));
    assert!(
        reported.contains(&format!("declares the environment grant '{UNSET_GRANT}'")),
        "the report carries the typed error: {reported}"
    );
    assert_eq!(
        plugins.iter().map(|plugin| plugin.name.as_str()).collect::<Vec<_>>(),
        vec!["probe"],
        "what could be resolved is still reported next to the failure"
    );

    let rendered = render_dashboard(&loaded, false, &Theme::new(false));
    assert!(
        rendered.contains("could not load:") && rendered.contains(UNSET_GRANT),
        "the dashboard states the failure in its own row instead of leaving a gap: {rendered}"
    );

    let mut report = DoctorReport::new(Vec::new(), None);
    (report.plugins, report.plugin_load_error) = plugin_section(&loaded);
    assert!(report.ok, "a load failure is reported, never turned into drift");
    let json = render_report_json(&report).expect("the report serializes");
    let parsed: Value = serde_json::from_str(&json).expect("the report is valid JSON");
    assert!(
        parsed["plugin_load_error"]
            .as_str()
            .is_some_and(|message| message.contains(UNSET_GRANT)),
        "the failure travels as a field a runner can read: {json}"
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
