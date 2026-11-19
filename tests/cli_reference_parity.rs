//! Surface parity: every subcommand and flag the binary accepts is documented in `docs/reference/`.
//!
//! The documentation rule this enforces (`odd/tasks/contract-reevaluation.md` S7) has three parts, and
//! only one of them is mechanical. This is that one: the flag surface is readable from the clap
//! definition and from the reference pages, so agreement between them is arithmetic, not judgement.
//! A new `attest` with no reference section is a defect a machine can report.
//!
//! What it deliberately does **not** do: check that the prose around a flag is still true. No test can
//! verify a sentence, so the narrative pages stay a review obligation and the contract says so instead
//! of pretending otherwise.
//!
//! Scope is `docs/reference/**` and not one file, because the ceiling forces those pages to split
//! (`CONTRACT.md`'s documentation clause): where a command is documented is a layout decision, that it
//! is documented somewhere is the contract.
//!
//! Covers S7 and S10.

use clap::CommandFactory;
use cutver::cli::args::Cli;
use std::path::{Path, PathBuf};

const REFERENCE_DIR: &str = "docs/reference";

/// Arguments clap adds by itself, so no page is expected to list them. `--version` is only generated
/// for the root command, which is why it is filtered by whether the command declares a version rather
/// than by name alone: `plugin add --version` is a real flag a page must document.
const GENERATED: [&str; 2] = ["help", "version"];

fn reference_pages() -> Vec<(String, String)> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(REFERENCE_DIR);
    let mut pages = Vec::new();
    collect(&root, &mut pages);
    pages.sort();
    assert!(
        !pages.is_empty(),
        "{REFERENCE_DIR} must contain the reference pages; found none"
    );
    pages
}

fn collect(dir: &Path, pages: &mut Vec<(String, String)>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, pages);
            continue;
        }
        if path.extension().is_some_and(|extension| extension == "md") {
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|err| panic!("a reference page must be readable, {} failed: {err}", path.display()));
            let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            pages.push((name, text.replace("\r\n", "\n")));
        }
    }
}

/// The body of the `### `cutver <name>`` section across every reference page, so a command may be
/// documented on whichever page its family ended up on.
fn section(pages: &[(String, String)], name: &str) -> Option<String> {
    let heading = format!("### `cutver {name}`");
    for (_, text) in pages {
        let mut body = String::new();
        let mut inside = false;
        for line in text.lines() {
            if line.starts_with("## ") || line.starts_with("### ") {
                if inside {
                    return Some(body);
                }
                inside = line.trim_end() == heading;
                continue;
            }
            if inside {
                body.push_str(line);
                body.push('\n');
            }
        }
        if inside {
            return Some(body);
        }
    }
    None
}

/// Every long flag a command accepts, including the ones its nested subcommands declare: `plugin`'s
/// page documents `--keep-artifact`, which belongs to `plugin remove`.
fn long_flags(command: &clap::Command, flags: &mut Vec<String>) {
    let generates_version = command.get_version().is_some();
    for argument in command.get_arguments() {
        let Some(long) = argument.get_long() else { continue };
        if GENERATED.contains(&long) && (long != "version" || generates_version) {
            continue;
        }
        flags.push(format!("--{long}"));
    }
    for nested in command.get_subcommands() {
        long_flags(nested, flags);
    }
}

/// The subcommands a parity check can hold: everything the binary accepts, minus the catch-all that
/// stands in for `cutver-<name>` binaries on `$PATH` and has no surface of its own.
fn documented_subcommands() -> Vec<String> {
    let root = Cli::command();
    root.get_subcommands()
        .map(clap::Command::get_name)
        .filter(|name| !name.eq_ignore_ascii_case("external"))
        .map(str::to_string)
        .collect()
}

#[test]
fn every_subcommand_has_a_reference_section() {
    let pages = reference_pages();
    let missing: Vec<String> = documented_subcommands()
        .into_iter()
        .filter(|name| section(&pages, name).is_none())
        .collect();
    assert!(
        missing.is_empty(),
        "these subcommands exist in the clap tree and have no `### `cutver <name>`` section anywhere under \
         {REFERENCE_DIR}: {missing:?}. A surface nobody documented is the drift S7 exists to stop"
    );
}

#[test]
fn every_flag_appears_in_its_own_command_s_section() {
    let pages = reference_pages();
    let root = Cli::command();
    let mut undocumented = Vec::new();
    for name in documented_subcommands() {
        let Some(command) = root.find_subcommand(&name) else {
            continue;
        };
        let Some(body) = section(&pages, &name) else { continue }; // reported by the test above
        let mut flags = Vec::new();
        long_flags(command, &mut flags);
        flags.sort();
        flags.dedup();
        for flag in flags {
            if !body.contains(&flag) {
                undocumented.push(format!("cutver {name} {flag}"));
            }
        }
    }
    assert!(
        undocumented.is_empty(),
        "these flags are accepted by the binary and absent from the section documenting their command: \
         {undocumented:?}"
    );
}
