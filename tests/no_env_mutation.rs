//! No source under `src/**` may mutate the process environment.
//!
//! `std::env::set_var` and `std::env::remove_var` are `unsafe` in edition 2024 because they are
//! undefined behaviour while another thread reads the environment, and this crate's test binary
//! does exactly that: `shell_env()` resolves its grants through the same resolver, the driver tests
//! spawn children that read their configuration, and `--lib` runs every unit test as a thread of one
//! process. A fixture that set the key itself therefore failed once on macOS and passed on a rerun
//! of the same commit (#344) — the value was dropped, not stale, and a suite that fails one run in
//! N teaches re-running instead of reading.
//!
//! The scope is the whole crate (#352), not `src/plugin/**` (#344): the writers #344 left behind
//! lived in `src/cli` and `src/changelog`, and a pin that only watches the module a bug was found
//! in watches the bug, not the class. Every reader is still a boundary and keeps reading the real
//! environment; what moved is the value a test controls: `env_grants_from_lookup`,
//! `PluginManager::from_config_with_lookup`, `ProcessDriver::new`'s grants,
//! `resolve_browser_command_with_env`, `run_external_with_path` and
//! `create_environment_with_env_lookup` each take it as an argument.
//!
//! This file is the pin: it walks the sources rather than trusting review, and it reports how many
//! files it scanned so a walk that reached nothing cannot pass.

use std::path::{Path, PathBuf};

/// The two calls that mutate the process-wide environment.
const MUTATORS: [&str; 2] = ["set_var", "remove_var"];

/// The floor the walk must clear. `src` holds 168 sources today; the number sits well below that so
/// ordinary files coming and going do not trip it, and far above zero so a walk that reached
/// nothing — the one failure a source-scanning test can hide — cannot pass.
const MINIMUM_SCANNED: usize = 120;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Collects every `.rs` file under `dir`, recursively, in a stable order.
fn rust_sources(dir: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_sources(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
}

/// One mutator mention: the file, its 1-based line, and the call it names.
#[derive(Debug, PartialEq, Eq)]
struct Finding {
    file: String,
    line: usize,
    calls: &'static str,
}

impl std::fmt::Display for Finding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{} contains `{}`", self.file, self.line, self.calls)
    }
}

/// Repository-relative with `/`, so a finding reads the same on every host.
fn relative(path: &Path) -> String {
    path.strip_prefix(repo_root())
        .unwrap_or(path)
        .components()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<String>>()
        .join("/")
}

/// Walks `root` and returns how many files it read plus every mutator mention it found.
fn scan(root: &Path) -> (usize, Vec<Finding>) {
    let mut paths = Vec::new();
    rust_sources(root, &mut paths);
    paths.sort();

    let mut findings = Vec::new();
    for path in &paths {
        let text = std::fs::read_to_string(path)
            .unwrap_or_else(|err| panic!("a source file must be readable, {} failed: {err}", path.display()));
        for (index, line) in text.lines().enumerate() {
            // Prose is not a call: this crate's sources talk about the two mutators to explain why
            // they are banned, and a walk that reported documentation would be taught to stop
            // documenting. Only a line that is code can mutate anything.
            if line.trim_start().starts_with("//") {
                continue;
            }
            for calls in MUTATORS {
                if line.contains(calls) {
                    findings.push(Finding {
                        file: relative(path),
                        line: index + 1,
                        calls,
                    });
                }
            }
        }
    }

    (paths.len(), findings)
}

#[test]
fn no_source_mutates_the_process_environment() {
    let root = repo_root().join("src");
    let (scanned, findings) = scan(&root);

    assert!(
        scanned >= MINIMUM_SCANNED,
        "the walk must reach the crate sources; it scanned {scanned} files under {}, so it proves \
         nothing (#352)",
        root.display()
    );
    let found: Vec<String> = findings.iter().map(ToString::to_string).collect();
    assert!(
        found.is_empty(),
        "no source under src/ may mutate the process environment (#352): `set_var` and `remove_var` \
         are undefined behaviour while another thread reads the environment, and this crate's test \
         binary does. Inject the value instead of setting the key — `env_grants_from_lookup`, \
         `PluginManager::from_config_with_lookup`, a literal map for `ProcessDriver::new`'s grants, \
         `resolve_browser_command_with_env`, `run_external_with_path`, or \
         `create_environment_with_env_lookup`. Scanned {scanned} files; found: {found:?}"
    );
}

#[test]
fn the_scan_catches_a_planted_occurrence() {
    let dir = tempfile::tempdir().expect("a scratch directory in the OS temp dir");
    let nested = dir.path().join("driver/nested");
    std::fs::create_dir_all(&nested).expect("nested scratch directory");

    std::fs::write(dir.path().join("clean.rs"), "fn fixture() {}\n").expect("clean fixture");
    std::fs::write(
        dir.path().join("prose.rs"),
        "/// This module never calls `std::env::set_var`, and says so on purpose.\npub fn fixture() {}\n",
    )
    .expect("prose fixture");
    std::fs::write(
        nested.join("planted.rs"),
        "fn fixture() {\n    // planted by the teeth test\n    unsafe { std::env::set_var(\"K\", \"V\") }\n}\n",
    )
    .expect("planted fixture");

    let (scanned, findings) = scan(dir.path());

    assert_eq!(scanned, 3, "every fixture must be read, recursion included");
    let found: Vec<String> = findings.iter().map(ToString::to_string).collect();
    assert_eq!(
        found.len(),
        1,
        "exactly the planted line must be reported, and documentation that names the mutator must \
         not be: got {found:?}"
    );
    assert!(
        found[0].ends_with("planted.rs:3 contains `set_var`"),
        "the finding must name the file and the line: {found:?}"
    );
}
