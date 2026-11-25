//! Source line budget over `src/**/*.rs`: `CONTRACT.md:174`'s hard ceiling of 400 lines, enforced as
//! a ratchet instead of a wish.
//!
//! Sixteen files were already over the ceiling when this landed, the worst at 1308 lines
//! (`odd/tasks/cutver-local-gate.md` L6: enforce the ceiling, do not remediate it — refactoring them
//! mid-roadmap would produce two enormous diffs that build nothing). So the ceiling binds in three
//! directions, and only tightens:
//!
//! - a file over the ceiling that is not in [`ALLOWLIST`] fails, naming it and its count;
//! - an allowlisted file that grows past the count recorded for it fails: the recorded number is a
//!   cap, never a target to raise;
//! - an allowlisted file that now fits inside the ceiling fails until its entry is deleted, so the
//!   allowlist can shrink but never grow.
//!
//! The 350-line facade trigger in the same contract clause is deliberately *not* mechanical:
//! whether `foo.rs` should decompose into `foo.rs` + `foo/` depends on where its seams are, and
//! choosing a decomposition is a human judgement no line count can make. The ratchet owns arithmetic
//! at `cargo test` time; the facade rule stays with review.
//!
//! Sibling of `tests/ci_workflow_contract.rs`, which mirrors `ci.yml`'s documentation ceiling the same
//! way: same file walking, no network, no workflow step. Covers S4.

use std::path::{Path, PathBuf};

/// `CONTRACT.md:174`: production source files target <= 300 lines with a strict hard ceiling of 400.
const CEILING: usize = 400;

/// Every file that was already over the ceiling when the ratchet landed, with the line count it had
/// on that commit. Sorted by path, so a reviewer diffing this table sees one insertion or one deletion
/// per file.
const ALLOWLIST: [(&str, usize); 14] = [
    ("src/changelog/context/tests.rs", 1308),
    ("src/changelog/extract/tests.rs", 563),
    ("src/changelog/render/tests.rs", 755),
    ("src/cli/args/tests.rs", 738),
    ("src/cli/runner/changelog.rs", 696),
    ("src/cli/runner/tests.rs", 950),
    ("src/config/discovery.rs", 467),
    ("src/config/path.rs", 464),
    ("src/config/types.rs", 693),
    ("src/config/validation.rs", 704),
    ("src/git/tests.rs", 1267),
    ("src/plugin/manage/restore.rs", 408),
    ("src/plugin/manage/update.rs", 405),
    ("src/plugin/manager/tests.rs", 719),
];

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Repository-relative, always with `/`, so the allowlist reads the same on every host.
fn repo_relative(path: &Path) -> String {
    path.strip_prefix(repo_root())
        .unwrap_or(path)
        .components()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<String>>()
        .join("/")
}

fn line_count(path: &Path) -> usize {
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("a source file must be readable, {} failed: {err}", path.display()));
    text.lines().count()
}

/// Every `src/**/*.rs` file with its line count, sorted by path.
fn source_files() -> Vec<(String, usize)> {
    let mut found = Vec::new();
    collect_sources(&repo_root().join("src"), &mut found);
    found.sort();
    found
}

fn collect_sources(dir: &Path, found: &mut Vec<(String, usize)>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_sources(&path, found);
            continue;
        }
        if path.extension().is_some_and(|extension| extension == "rs") {
            found.push((repo_relative(&path), line_count(&path)));
        }
    }
}

/// The cap recorded for a path, if any.
fn recorded(name: &str) -> Option<usize> {
    ALLOWLIST
        .iter()
        .find(|(allowed, _)| *allowed == name)
        .map(|(_, cap)| *cap)
}

#[test]
fn no_source_file_climbs_over_the_ceiling() {
    let over: Vec<String> = source_files()
        .into_iter()
        .filter(|(name, lines)| *lines > CEILING && recorded(name).is_none())
        .map(|(name, lines)| format!("{name} has {lines}"))
        .collect();
    assert!(
        over.is_empty(),
        "the hard ceiling is {CEILING} lines (CONTRACT.md:174); these files are over it and are not \
         allowlisted: {over:?}. Decompose one into `foo.rs` + `foo/` (the Facade pattern) or move code \
         out of it. Adding it to ALLOWLIST is not the fix: the allowlist exists only for debt that \
         predates the ratchet, and it can never grow"
    );
}

#[test]
fn an_allowlisted_file_may_not_grow() {
    let grown: Vec<String> = ALLOWLIST
        .iter()
        .filter_map(|(name, cap)| {
            let lines = line_count(&repo_root().join(name));
            (lines > *cap).then(|| format!("{name} was {cap}, is now {lines}"))
        })
        .collect();
    assert!(
        grown.is_empty(),
        "an allowlisted file may shrink but never grow: the recorded count is a cap (CONTRACT.md:174) \
         and this ratchet only tightens. Pull each file back to at most its recorded count: {grown:?}. \
         If the file now fits inside the {CEILING}-line ceiling, delete its ALLOWLIST entry instead of \
         raising it"
    );
}

#[test]
fn the_allowlist_may_only_shrink() {
    let mut stale: Vec<String> = Vec::new();
    for (name, cap) in ALLOWLIST {
        let path = repo_root().join(name);
        if !path.is_file() {
            stale.push(format!("{name} no longer exists and is still allowlisted for {cap}"));
            continue;
        }
        let lines = line_count(&path);
        if lines <= CEILING {
            stale.push(format!(
                "{name} is {lines}, inside the {CEILING}-line ceiling, still allowlisted for {cap}"
            ));
        }
    }
    assert!(
        stale.is_empty(),
        "the allowlist may only shrink; these entries have to go: {stale:?}. A file that fits inside \
         the {CEILING}-line ceiling (CONTRACT.md:174) is no longer debt, so delete its ALLOWLIST entry \
         in the same commit that shrank it — leaving it there hands the next change a budget nobody \
         approved"
    );
}
