//! Every relative link in the documentation resolves: the file it names exists, and the `#anchor` it
//! carries is a heading on that page.
//!
//! Splitting a page moves headings, and a moved heading breaks every link that named it. That damage is
//! invisible to every other check in this suite — the line ceiling passes, the examples still parse, the
//! page still renders — so it is the one documentation defect no test would have caught. It became worth
//! catching when two 199-line pages were split and eighteen anchored links had to be repointed by hand.
//!
//! External links are not fetched: this test is deterministic and offline by design, and a network check
//! would turn a green suite into a hostage of someone else's uptime. It holds the half that is ours.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const ROOTS: [&str; 2] = ["README.md", "docs"];

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn pages() -> Vec<PathBuf> {
    let root = repo_root();
    let mut files = vec![root.join("README.md")];
    collect(&root.join("docs"), &mut files);
    files.sort();
    assert!(
        files.len() > 1,
        "the documentation set must still be discoverable under {ROOTS:?}"
    );
    files
}

fn collect(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, files);
            continue;
        }
        if path.extension().is_some_and(|extension| extension == "md") {
            files.push(path);
        }
    }
}

fn text_of(path: &Path) -> String {
    let raw = std::fs::read_to_string(path).unwrap_or_else(|err| {
        panic!(
            "a documentation file must be readable, {} failed: {err}",
            path.display()
        )
    });
    raw.replace("\r\n", "\n")
}

/// GitHub's heading anchor for the punctuation these pages use: lowercased, everything that is not
/// alphanumeric, `_`, `-` or a space dropped, each space becoming one `-`. Spaces are not collapsed,
/// which is what makes the four-table heading end in `--publish`.
fn slug(heading: &str) -> String {
    let mut slug = String::new();
    for character in heading.trim().to_lowercase().chars() {
        if character.is_alphanumeric() || character == '_' || character == '-' {
            slug.push(character);
        } else if character.is_whitespace() {
            slug.push('-');
        }
    }
    slug.trim_matches('-').to_string()
}

/// Every anchor a page declares, in document order with GitHub's duplicate suffixing.
fn anchors_of(path: &Path) -> Vec<String> {
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    let mut anchors = Vec::new();
    for line in text_of(path).lines() {
        let Some(rest) = line.strip_prefix('#') else { continue };
        let heading = rest.trim_start_matches('#').trim();
        if heading.is_empty() {
            continue;
        }
        let base = slug(heading);
        let count = seen.entry(base.clone()).or_insert(0);
        anchors.push(if *count == 0 {
            base.clone()
        } else {
            format!("{base}-{count}")
        });
        *count += 1;
    }
    anchors
}

/// The `(target)` of every markdown link or image on a page, in order.
fn targets(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find("](") {
        let after = &rest[open + 2..];
        let Some(close) = after.find(')') else { break };
        found.push(after[..close].trim().to_string());
        rest = &after[close..];
    }
    found
}

fn relative_pages() -> Vec<(PathBuf, String)> {
    pages()
        .into_iter()
        .map(|path| {
            let text = text_of(&path);
            (path, text)
        })
        .collect()
}

#[test]
fn every_relative_link_resolves() {
    let root = repo_root();
    let mut broken = Vec::new();
    let mut checked = 0;
    for (page, text) in relative_pages() {
        let name = page
            .strip_prefix(&root)
            .unwrap_or(&page)
            .display()
            .to_string()
            .replace('\\', "/");
        for target in targets(&text) {
            // A MiniJinja placeholder is not a link: `templates/release-notes.md` shows the renderer's
            // own output syntax, and the braces are the point of the example.
            if target.is_empty()
                || target.contains("{{")
                || target.starts_with("http://")
                || target.starts_with("https://")
                || target.starts_with("mailto:")
                || target.starts_with('<')
            {
                continue;
            }
            checked += 1;
            // A bare `#anchor` points inside the same page.
            let (path_part, anchor) = match target.split_once('#') {
                Some((path, anchor)) => (path, Some(anchor)),
                None => (target.as_str(), None),
            };
            let resolved = if path_part.is_empty() {
                page.clone()
            } else {
                let base = page.parent().unwrap_or(&root);
                normalize(&base.join(path_part))
            };
            if !resolved.is_file() {
                broken.push(format!("{name}: `{target}` names a file that does not exist"));
                continue;
            }
            let Some(anchor) = anchor else { continue };
            if anchor.is_empty() {
                continue;
            }
            let declared = anchors_of(&resolved);
            if !declared.iter().any(|candidate| candidate == anchor) {
                broken.push(format!(
                    "{name}: `{target}` names an anchor the target page does not declare (it has: {declared:?})"
                ));
            }
        }
    }
    assert!(
        checked > 100,
        "the walk found only {checked} relative links; it has stopped finding the docs"
    );
    assert!(
        broken.is_empty(),
        "a split or a rename moves headings, and a moved heading breaks every link that named it — which \
         is the one documentation defect the rest of this suite cannot see. These do not resolve: {broken:#?}"
    );
}

/// `a/../b` resolved without touching the filesystem, so a link is judged where it is written.
fn normalize(path: &Path) -> PathBuf {
    let mut parts: Vec<std::ffi::OsString> = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                parts.pop();
            }
            std::path::Component::CurDir => {}
            other => parts.push(other.as_os_str().to_os_string()),
        }
    }
    parts.iter().collect()
}
