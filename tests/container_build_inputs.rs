//! Contract tests binding the container image build to every file it compiles in.
//!
//! #258: `src/init/scaffold/templates.rs` embeds `.github/templates/cutver/RELEASE.md` with
//! `include_str!`, but the `Containerfile` never copied `.github/` into the builder stage and
//! `.dockerignore` excluded the directory outright. Every image build from v0.17.0 through v0.18.1
//! failed at compile time, and nothing noticed for three releases, because the image was only ever
//! built on `release: published` — a gate that runs after the tag exists and cannot be retracted.
//!
//! The embedded-file set is derived from the sources instead of listed here, so the next
//! `include_str!` that resolves outside the copied set fails this test instead of a release.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn repo_text(relative: &str) -> String {
    let path = repo_root().join(relative);
    let raw = std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
    // Git checks these files out with CRLF on Windows. Line endings are a checkout setting, not
    // content, so every assertion in this file compares normalised text.
    raw.replace("\r\n", "\n")
}

/// A repository-relative path with forward slashes, resolved through `..` and symlinks.
fn repo_relative(path: &Path) -> String {
    let root = repo_root().canonicalize().expect("the repository root must exist");
    let target = path
        .canonicalize()
        .unwrap_or_else(|err| panic!("cannot resolve {}: {err}", path.display()));
    let stripped = target
        .strip_prefix(&root)
        .unwrap_or_else(|_| panic!("{} is outside the repository", target.display()));
    stripped
        .components()
        .map(|part| part.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn walk(dir: &Path, extension: &str, found: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir).unwrap_or_else(|err| panic!("failed to list {}: {err}", dir.display()));
    for entry in entries {
        let path = entry.expect("a readable directory entry").path();
        if path.is_dir() {
            walk(&path, extension, found);
        } else if path.extension().is_some_and(|ext| ext == extension) {
            found.push(path);
        }
    }
}

/// `(declaring source, repository-relative embedded file)` for every compile-time inclusion in
/// `src/`.
///
/// Only the `include_str!("…")` and `include_bytes!("…")` spellings are understood. Any other
/// spelling panics, because a build input this test cannot read is a build input nothing checks —
/// and the whole point of #258 is that such an input breaks a release silently.
fn embedded_files() -> Vec<(String, String)> {
    let mut sources = Vec::new();
    walk(&repo_root().join("src"), "rs", &mut sources);
    assert!(
        !sources.is_empty(),
        "src/ must contain the sources this test derives its input set from"
    );

    let mut embedded = Vec::new();
    for source in sources {
        let text = std::fs::read_to_string(&source).expect("a readable source file");
        let declared_in = repo_relative(&source);
        for (number, line) in text.lines().enumerate() {
            // Comments quote macros to explain them; only code embeds a file.
            if line.trim_start().starts_with("//") {
                continue;
            }
            for marker in ["include_str!(", "include_bytes!("] {
                let Some(rest) = line.split_once(marker).map(|(_, rest)| rest) else {
                    continue;
                };
                let Some(literal) = rest.strip_prefix('"').and_then(|rest| rest.split_once('"')) else {
                    panic!(
                        "{declared_in}:{} uses {marker}… in a shape this test cannot resolve; extend \
                         `embedded_files` rather than leaving a build input unchecked",
                        number + 1
                    );
                };
                let declared = source
                    .parent()
                    .expect("a source file has a parent directory")
                    .join(literal.0);
                embedded.push((declared_in.clone(), repo_relative(&declared)));
            }
        }
    }
    assert!(
        !embedded.is_empty(),
        "no compile-time inclusion was found in src/: the derivation is broken and would pass on \
         any Containerfile"
    );
    embedded
}

/// One `COPY` in the `Containerfile`, reduced to the pair of paths it maps.
struct Copy {
    source: String,
    destination: String,
    /// A directory `COPY` moves a whole subtree, so its destination has to name the same path for
    /// the relative `include_str!` resolution to keep working inside the image.
    directory: bool,
}

fn containerfile_copies() -> Vec<Copy> {
    let mut copies = Vec::new();
    for line in repo_text("Containerfile").lines() {
        let Some(rest) = line.trim().strip_prefix("COPY ") else {
            continue;
        };
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        // `--from=<stage>` names another stage's filesystem, which this model does not cover.
        if tokens.iter().any(|token| token.starts_with("--from")) {
            continue;
        }
        let mut args: Vec<&str> = tokens.into_iter().filter(|token| !token.starts_with("--")).collect();
        let destination = if args.len() > 1 {
            args.pop().expect("a destination is present")
        } else {
            args[0]
        };
        for source in args {
            let directory = source.ends_with('/') || repo_root().join(source).is_dir();
            let mut landed = destination.trim_start_matches("./").trim_end_matches('/').to_string();
            if !directory && (destination.ends_with('/') || destination == ".") {
                let name = source.rsplit('/').next().expect("a file name");
                landed = format!("{landed}/{name}").trim_start_matches('/').to_string();
            }
            let source = source.trim_end_matches('/').trim_start_matches("./").to_string();
            if directory && landed != source {
                panic!(
                    "COPY {source} {destination} renames a directory: extend this test to follow the \
                     rename before any `include_str!` relies on it"
                );
            }
            copies.push(Copy {
                source,
                destination: landed,
                directory,
            });
        }
    }
    assert!(
        !copies.is_empty(),
        "the Containerfile must declare the inputs its build stage reads"
    );
    copies
}

fn copy_covers(copy: &Copy, target: &str) -> bool {
    if copy.directory {
        target
            .strip_prefix(format!("{}/", copy.source).as_str())
            .is_some_and(|rest| format!("{}/{rest}", copy.destination) == target)
    } else {
        copy.destination == target
    }
}

/// A `.dockerignore` entry this repository is allowed to use. Any other shape panics instead of
/// passing silently: a pattern nothing can interpret is a pattern nothing checks.
struct Entry {
    negated: bool,
    pattern: Pattern,
}

enum Pattern {
    /// `dir/` removes the directory, and with it everything below.
    Dir(String),
    /// `*.ext` is root-level: in Docker's matcher `*` does not cross a `/`.
    RootExtension(String),
    /// A literal file or directory path.
    Literal(String),
}

fn dockerignore_entries() -> Vec<Entry> {
    let mut entries = Vec::new();
    for line in repo_text(".dockerignore").lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (negated, body) = match line.strip_prefix('!') {
            Some(rest) => (true, rest),
            None => (false, line),
        };
        let pattern = if let Some(dir) = body.strip_suffix('/') {
            Pattern::Dir(dir.trim_start_matches("./").to_string())
        } else if body.starts_with("*.") && !body[2..].contains(['*', '?', '[', '/']) {
            Pattern::RootExtension(body[1..].to_string())
        } else if body.contains(['*', '?', '[']) {
            panic!(
                "unsupported .dockerignore pattern `{line}`: extend `Pattern` so the image inputs \
                 stay checked"
            )
        } else {
            Pattern::Literal(body.trim_start_matches("./").to_string())
        };
        entries.push(Entry { negated, pattern });
    }
    entries
}

fn entry_matches(pattern: &Pattern, path: &str) -> bool {
    match pattern {
        Pattern::Dir(dir) => path == dir,
        Pattern::Literal(name) => path == name,
        Pattern::RootExtension(extension) => !path.contains('/') && path.ends_with(extension.as_str()),
    }
}

/// Every ancestor of `path`, closest to the root first, excluding `path` itself.
fn proper_ancestors(path: &str) -> Vec<String> {
    let parts: Vec<&str> = path.split('/').collect();
    (1..parts.len()).map(|end| parts[..end].join("/")).collect()
}

/// Whether the build context contains `path`.
///
/// Two rules, both Docker's, and the second is the trap #258 paid for:
///
/// 1. The last matching pattern wins, and `!` reverses it — for the path it names.
/// 2. A pattern matching an ancestor directory removes the whole subtree and cannot be reopened
///    from the inside. `.github/` plus `!.github/templates/cutver/RELEASE.md` looks like an
///    exception and is not one, which is why the fix deletes the directory entry instead of
///    leaning on the negation.
fn in_context(path: &str, entries: &[Entry]) -> bool {
    let ancestors = proper_ancestors(path);
    let excluded_directory = entries
        .iter()
        .filter(|entry| !entry.negated)
        .any(|entry| ancestors.iter().any(|ancestor| entry_matches(&entry.pattern, ancestor)));
    if excluded_directory {
        return false;
    }
    let mut excluded = false;
    for entry in entries {
        if entry_matches(&entry.pattern, path) {
            excluded = !entry.negated;
        }
    }
    !excluded
}

fn is_job_key(line: &str) -> bool {
    line.strip_prefix("  ")
        .is_some_and(|rest| !rest.starts_with(' ') && !rest.trim().is_empty() && !rest.starts_with('#'))
}

/// One job body as lines: from its two-space key to the next job key.
fn workflow_job<'a>(workflow: &'a str, id: &str) -> Vec<&'a str> {
    const MARKER: &str = "\njobs:\n";
    let jobs_at = workflow
        .find(MARKER)
        .unwrap_or_else(|| panic!("the container workflow must declare a `jobs:` section"))
        + MARKER.len();
    let body: Vec<&str> = workflow[jobs_at..].lines().collect();
    let key = format!("  {id}:");
    let start = body
        .iter()
        .position(|line| *line == key)
        .unwrap_or_else(|| panic!("the container workflow must declare the `{id}` job"));
    body[start + 1..]
        .iter()
        .copied()
        .take_while(|line| !is_job_key(line))
        .collect()
}

fn workflow_triggers(workflow: &str) -> Vec<&str> {
    const MARKER: &str = "\njobs:\n";
    let jobs_at = workflow
        .find(MARKER)
        .unwrap_or_else(|| panic!("the container workflow must declare a `jobs:` section"));
    workflow[..jobs_at].lines().collect()
}

/// #258 (S1, S7): every file the crate embeds at compile time has to enter the builder stage.
///
/// This is the invariant, not the one instance: the derivation reads `src/`, so an `include_str!`
/// pointing outside the `COPY` set fails here on the pull request that adds it. The same class had
/// already broken the build once before, with the `crates/cutver-pdk` manifest.
#[test]
fn every_file_the_crate_embeds_reaches_the_builder_stage() {
    let copies = containerfile_copies();
    let entries = dockerignore_entries();
    for (declared_in, embedded) in embedded_files() {
        assert!(
            copies.iter().any(|copy| copy_covers(copy, &embedded)),
            "{declared_in} embeds {embedded} with `include_str!`, but no `COPY` in the Containerfile \
             puts it in the builder stage: the image build fails at compile time. copies = {:?}",
            copies
                .iter()
                .map(|copy| format!("{} -> {}", copy.source, copy.destination))
                .collect::<Vec<_>>()
        );
        assert!(
            in_context(&embedded, &entries),
            "{embedded} is embedded by {declared_in} but `.dockerignore` keeps it out of the build \
             context, so the `COPY` that needs it fails"
        );
    }
}

/// #258 (S2): nothing the `Containerfile` copies may be excluded from the build context.
///
/// The `COPY` and the `.dockerignore` are two files that have to agree, and the failure is a
/// release-time one. This also pins that `.github/` stays out of the ignore file: bringing the
/// directory entry back breaks the entry above, one negation line does not fix it, and this test
/// says so at the level of the directories a `COPY` reads.
#[test]
fn the_build_context_carries_every_copy_source() {
    let entries = dockerignore_entries();
    for copy in containerfile_copies() {
        assert!(
            in_context(&copy.source, &entries),
            "`.dockerignore` excludes {}, which the Containerfile copies into the build",
            copy.source
        );
    }
}

/// #258 (S2): the negation Docker users reach for first does not work, and the Containerfile's fix
/// depends on knowing it.
///
/// `.github/` plus `!.github/templates/cutver/RELEASE.md` reads like an exception. It is not: the
/// excluded directory is never walked, so the file never reaches the context and the `COPY` fails.
/// The fix removes the directory entry instead. If this test ever fails, the model above has
/// drifted from Docker and the Containerfile comment is a lie.
#[test]
fn a_negation_inside_an_excluded_directory_does_not_rescue_the_file() {
    let entries = vec![
        Entry {
            negated: false,
            pattern: Pattern::Dir(".github".to_string()),
        },
        Entry {
            negated: true,
            pattern: Pattern::Literal(".github/templates/cutver/RELEASE.md".to_string()),
        },
    ];
    assert!(
        !in_context(".github/templates/cutver/RELEASE.md", &entries),
        "an excluded parent directory must win: `!.github/templates/cutver/RELEASE.md` under a \
         `.github/` entry cannot put the file back in the build context"
    );

    let carved_out = vec![
        Entry {
            negated: false,
            pattern: Pattern::RootExtension(".md".to_string()),
        },
        Entry {
            negated: true,
            pattern: Pattern::Literal(".github/templates/cutver/RELEASE.md".to_string()),
        },
    ];
    assert!(
        in_context(".github/templates/cutver/RELEASE.md", &carved_out),
        "`*.md` is a root-level pattern and must not reach a nested file: only `.github/` ever \
         excluded the template"
    );
}

/// #258 (S4, S7): the image is built on pull requests, without pushing anywhere.
///
/// Three releases shipped broken because the only build ran on `release: published`. A build that
/// only runs once the tag exists cannot protect the tag; a build on the pull request can.
#[test]
fn the_image_is_built_on_pull_requests_without_pushing() {
    const CONTAINER: &str = ".github/workflows/container.yml";
    let workflow = repo_text(CONTAINER);
    let triggers = workflow_triggers(&workflow);
    assert!(
        triggers.contains(&"  pull_request:"),
        "{CONTAINER} must build on `pull_request`, or a missing build input stays invisible until a \
         release has already happened"
    );
    for path in [
        "      - 'src/**'",
        "      - 'crates/**'",
        "      - 'Cargo.toml'",
        "      - 'Cargo.lock'",
        "      - 'Containerfile'",
        "      - '.dockerignore'",
        "      - '.github/templates/**'",
    ] {
        assert!(
            triggers.contains(&path),
            "{CONTAINER} must run for `{}`: it is a build input, and a change to it is exactly the \
             change that breaks the image",
            path.trim().trim_start_matches("- '").trim_end_matches('\'')
        );
    }
    // The release path is untouched by this: a pull request must not push or promote.
    for required in ["  release:", "  workflow_dispatch:"] {
        assert!(
            triggers.contains(&required),
            "{CONTAINER} must keep `{}`",
            required.trim()
        );
    }

    let job = workflow_job(&workflow, "build-pull-request");
    for required in [
        "    if: github.event_name == 'pull_request'",
        "          platforms: linux/amd64",
        "          push: false",
        "          cache-from: type=gha",
    ] {
        assert!(
            job.contains(&required),
            "the pull-request build job must declare `{}`: without it the leg either pushes to the \
             registry from a pull request or rebuilds both architectures for nothing",
            required.trim()
        );
    }
    for forbidden in ["          push: true", "      packages: write"] {
        assert!(
            !job.contains(&forbidden),
            "the pull-request build job must not declare `{}`: a pull request has no business \
             writing to the registry",
            forbidden.trim()
        );
    }

    // The pull-request trigger reaches every job in this file, and the publishing path writes to
    // the registry and resolves release tags. Each of those jobs has to say it skips a pull
    // request, or the first pull request pushes a rehearsal tag nobody asked for.
    for id in [
        "build-and-push",
        "build-and-push-plugins",
        "smoke",
        "smoke-plugins",
        "promote",
    ] {
        let publishing = workflow_job(&workflow, id);
        assert!(
            publishing.contains(&"    if: github.event_name != 'pull_request'"),
            "the `{id}` job must skip pull requests: the new trigger reaches it, and it pushes to \
             the registry or resolves a release tag"
        );
    }
}

/// #258 (S3, S8): one artifact, one definition.
///
/// The root `Dockerfile` was a second, unreferenced image definition that had already drifted —
/// no `crates/cutver-pdk` manifest, no `CUTVER_FEATURES`. A reader who builds it gets a broken
/// image, and nothing in the repository would have said so.
#[test]
fn the_root_dockerfile_is_gone_and_nothing_points_at_it() {
    let root = repo_root();
    assert!(
        !root.join("Dockerfile").exists(),
        "the root `Dockerfile` must stay deleted: `container.yml` builds `./Containerfile`, and a \
         second definition is a decoy that reads as the real one"
    );

    let mut scanned = vec![root.join("README.md")];
    walk(&root.join("docs"), "md", &mut scanned);
    walk(&root.join(".github").join("workflows"), "yml", &mut scanned);
    for file in scanned {
        let raw =
            std::fs::read_to_string(&file).unwrap_or_else(|err| panic!("failed to read {}: {err}", file.display()));
        for (number, line) in raw.replace("\r\n", "\n").lines().enumerate() {
            assert!(
                !line.contains("Dockerfile"),
                "{}:{} still points at the deleted `Dockerfile`: {}",
                repo_relative(&file),
                number + 1,
                line.trim()
            );
        }
    }
}
