//! Documented commands must parse. The second mechanical third of the documentation rule (S7).
//!
//! The rule the repository now writes down is short: *an example is a transcript or it is a claim.*
//! A `cutver …` line inside a `bash` fence is a transcript — a reader will paste it — so it is checked
//! against the real clap tree, and the check is parse-only: nothing runs, no file is touched, no git
//! command is spawned. A `cutver …` line inside a `text` fence is a syntax diagram, which is what its
//! metavariables say, and the fence tag is where that claim lives.
//!
//! The tag may not be picked to make a failure go away, so the two directions are both enforced: a
//! `bash` line must parse, and a `text` line must actually be syntax rather than a transcript hiding
//! from the parser.
//!
//! What this cannot check is whether the prose around an example is still true, and it does not try.
//! Narrative pages stay a review obligation, and the contract says so instead of pretending.
//!
//! Covers S7 and S10.

use clap::Parser;
use cutver::cli::args::Cli;
use std::path::{Path, PathBuf};

/// Where prose is shipped: the entry point and every documented page.
const ROOTS: [&str; 2] = ["README.md", "docs"];

/// Fence tags whose contents a reader pastes into a shell.
const TRANSCRIPT: [&str; 2] = ["bash", "sh"];
/// The fence tag that marks a syntax diagram: metavariables, not a command to run.
const SYNTAX: &str = "text";

#[derive(Debug)]
struct Fenced {
    file: String,
    line: usize,
    tag: String,
    text: String,
}

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

/// Every `cutver …` line that sits inside a fence, with the tag of the fence it sits in.
fn fenced_commands() -> Vec<Fenced> {
    let root = repo_root();
    let mut found = Vec::new();
    for page in pages() {
        let text = std::fs::read_to_string(&page).unwrap_or_else(|err| {
            panic!(
                "a documentation file must be readable, {} failed: {err}",
                page.display()
            )
        });
        let file = page
            .strip_prefix(&root)
            .unwrap_or(&page)
            .display()
            .to_string()
            .replace('\\', "/");
        let mut tag: Option<String> = None;
        for (index, line) in text.replace("\r\n", "\n").lines().enumerate() {
            if let Some(rest) = line.strip_prefix("```") {
                tag = match tag {
                    Some(_) => None,
                    None => Some(rest.trim().to_string()),
                };
                continue;
            }
            let Some(current) = tag.as_deref() else { continue };
            if line.trim_start().starts_with("cutver ") {
                found.push(Fenced {
                    file: file.clone(),
                    line: index + 1,
                    tag: current.to_string(),
                    text: line.trim().to_string(),
                });
            }
        }
    }
    assert!(
        !found.is_empty(),
        "no fenced `cutver` example was found; the walk has stopped finding the docs"
    );
    found
}

/// A shell line reduced to the arguments the binary would receive: quoting resolved, trailing
/// comment dropped, and anything a shell would consume before the command starts cut away.
fn arguments(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    for ch in line.chars() {
        if let Some(open) = quote {
            if ch == open {
                quote = None;
            } else {
                current.push(ch);
            }
            continue;
        }
        match ch {
            '\'' | '"' => quote = Some(ch),
            // A `#` at a token boundary starts a comment; mid-token it is a version or a ref.
            '#' if current.is_empty() => break,
            // Redirection, a pipe or a heredoc belong to the shell, not to the binary.
            '>' | '|' | '<' => break,
            whitespace if whitespace.is_whitespace() => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            other => current.push(other),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

/// `--help` and `--version` are answers, not parse failures: clap reports them as errors because its
/// normal job is to print and exit, and a transcript that asks for them is a valid transcript.
fn parses(tokens: &[String]) -> Result<(), String> {
    match Cli::try_parse_from(tokens) {
        Ok(_) => Ok(()),
        Err(err) => match err.kind() {
            clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion => Ok(()),
            _ => Err(err.to_string().lines().next().unwrap_or_default().to_string()),
        },
    }
}

#[test]
fn every_fenced_transcript_parses() {
    let mut broken = Vec::new();
    for example in fenced_commands() {
        if !TRANSCRIPT.contains(&example.tag.as_str()) {
            continue;
        }
        let tokens = arguments(&example.text);
        if let Err(reason) = parses(&tokens) {
            broken.push(format!(
                "{}:{} `{}` — {reason}",
                example.file, example.line, example.text
            ));
        }
    }
    assert!(
        broken.is_empty(),
        "a reader pastes these lines into a shell, so they must parse against the binary. Either fix \
         the example or, if it is a syntax diagram, move it to a `{SYNTAX}` fence: {broken:#?}"
    );
}

#[test]
fn a_syntax_fence_only_holds_syntax() {
    let mut suspicious = Vec::new();
    for example in fenced_commands() {
        if example.tag != SYNTAX {
            continue;
        }
        // A metavariable is what makes a line syntax rather than a command: `<VALUE>` asks the reader
        // for something, `[--flag]` marks something optional.
        if !example.text.contains('<') && !example.text.contains('[') {
            suspicious.push(format!("{}:{} `{}`", example.file, example.line, example.text));
        }
    }
    assert!(
        suspicious.is_empty(),
        "a `{SYNTAX}` fence claims its contents are syntax, but these lines are runnable commands with \
         no metavariable in them. Put them in a `bash` fence, where they get checked: {suspicious:#?}"
    );
}
