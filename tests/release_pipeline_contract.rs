//! Contract tests binding `cutver.toml` to `.github/workflows/release.yml`, the workflow that cuts
//! a release: the crate version, the git tag and the draft release.
//!
//! The release workflow cannot be run from a test, but every coupling that would silently break a
//! release is text-visible. Each assertion names the release failure it prevents, so a future edit
//! to either file fails here instead of during a dispatch.
//!
//! The OCI half of the same release is pinned in `container_workflow_contract.rs`.
//!
//! Guards PR #229 (risk item 5: delivery/CI).

use std::path::PathBuf;

use cutver::config;
use cutver::plugin::types::{Capability, RuntimeKind};
use cutver::plugin::{PluginConfig, PluginName};

const PLUGIN: &str = "github-publish";
/// Every job id in the file, in declaration order.
const JOBS: [&str; 6] = ["version", "build", "build-plugins", "smoke", "smoke-plugins", "publish"];
/// The jobs whose green result the publish job waits on: the bump, both builds and both smokes.
const VERIFIED_BEFORE_PUBLISHING: [&str; 5] = ["version", "build", "build-plugins", "smoke", "smoke-plugins"];

fn repo_text(relative: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative);
    let raw = std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
    // Git checks these files out with CRLF on Windows. Line endings are a checkout setting, not
    // content, so every assertion in this file compares normalised text.
    raw.replace("\r\n", "\n")
}

/// One job body, anchored on its two-space job key so a same-named value elsewhere in the file (a
/// `needs:` entry, a step name, an output) cannot stand in for the job. The last job runs to EOF.
fn job<'a>(workflow: &'a str, id: &str) -> &'a str {
    let marker = format!("\n  {id}:\n");
    let start = workflow
        .find(&marker)
        .unwrap_or_else(|| panic!("release.yml must declare the `{id}` job"))
        + marker.len();
    let rest = &workflow[start..];
    let end = JOBS
        .iter()
        .filter_map(|next| rest.find(&format!("\n  {next}:\n")))
        .min()
        .unwrap_or(rest.len());
    &rest[..end]
}

/// The lines of a workflow fragment that are not comments.
///
/// A comment can name a command — the version job's preamble explains why the bump step defers and
/// says `cutver bump` while doing it — so a pin has to read what the runner executes, not the prose
/// around it.
fn code(text: &str) -> String {
    text.lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The one step of a job body whose code contains `marker`, split on the six-space step marker.
///
/// Binding an `id:` to the step that runs a command needs the step's own text: `job()` returns the
/// whole body, where `id: cutver` could sit on any step and still satisfy a `contains`.
fn step_containing<'a>(job_body: &'a str, marker: &str) -> &'a str {
    let starts: Vec<usize> = job_body
        .match_indices("\n      - ")
        .map(|(index, _)| index + 1)
        .collect();
    assert!(!starts.is_empty(), "the job body declares no step");
    for (position, start) in starts.iter().enumerate() {
        let end = starts.get(position + 1).copied().unwrap_or(job_body.len());
        let block = &job_body[*start..end];
        if code(block).contains(marker) {
            return block;
        }
    }
    panic!("no step in this job body runs `{marker}`")
}

/// The `key: value` pairs a job declares under its top-level `outputs:` map, comments dropped.
///
/// Reading the map as pairs is what separates an output that is sourced from a step from one that
/// is a constant: a `contains` on the whole job body cannot tell `steps.cutver.outputs.version` from
/// `steps.release.outputs.version`, and both render a string that looks like an output name.
fn declared_outputs(job_body: &str) -> Vec<(String, String)> {
    const MARKER: &str = "\n    outputs:\n";
    let start = job_body
        .find(MARKER)
        .unwrap_or_else(|| panic!("the job must declare its outputs under `outputs:`"))
        + MARKER.len();
    let mut outputs = Vec::new();
    for line in job_body[start..].lines() {
        let Some(line) = line.strip_prefix("      ") else {
            break;
        };
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, value) = line
            .split_once(':')
            .unwrap_or_else(|| panic!("an output must be declared as `key: value`, got `{line}`"));
        outputs.push((key.trim().to_string(), value.trim().to_string()));
    }
    assert!(
        !outputs.is_empty(),
        "the version job must declare the outputs its dependants read"
    );
    outputs
}

/// The `else` and `fi` that belong to the line containing `exists`, as indices into its lines.
///
/// "The next `fi`" is the wrong question once the branch holds an `if` of its own: the block that
/// closes the test is the one at the same indentation, which is also how the shell reads it.
fn branch_of(text: &str, exists: &str) -> (usize, usize, usize) {
    let lines: Vec<&str> = text.lines().collect();
    let indent = |line: &str| line.len() - line.trim_start().len();
    let at = lines
        .iter()
        .position(|line| line.contains(exists))
        .unwrap_or_else(|| panic!("no line contains `{exists}`: {text}"));
    let column = indent(lines[at]);
    let else_at = (at..lines.len())
        .find(|index| lines[*index].trim() == "else" && indent(lines[*index]) == column)
        .unwrap_or_else(|| panic!("`{exists}` must have an `else` branch at its own indentation: {text}"));
    let fi_at = (else_at..lines.len())
        .find(|index| lines[*index].trim() == "fi" && indent(lines[*index]) == column)
        .unwrap_or_else(|| panic!("the `else` of `{exists}` must close with its own `fi`: {text}"));
    (at, else_at, fi_at)
}

/// One `workflow_dispatch` input, as declared: its block under `inputs:`, comments dropped.
///
/// Reading the block — not the whole file — is what separates an input that exists from a string
/// that happens to appear somewhere in the workflow: an `options:` list under another input or a
/// `default:` inside an unrelated job would satisfy a `contains` on `release.yml`.
fn dispatch_input(workflow: &str, name: &str) -> String {
    const INPUTS: &str = "\n    inputs:\n";
    let dispatch = workflow
        .find("\n  workflow_dispatch:\n")
        .expect("release.yml must be dispatchable by hand");
    let start = dispatch
        + workflow[dispatch..]
            .find(INPUTS)
            .unwrap_or_else(|| panic!("the dispatch must declare its inputs under `inputs:`"))
        + INPUTS.len();
    let marker = format!("\n      {name}:\n");
    let at = workflow[start..]
        .find(&marker)
        .unwrap_or_else(|| panic!("the dispatch must declare a `{name}` input"))
        + start
        + marker.len();

    let block = &workflow[at..];
    // An input's keys sit at six spaces and their children at eight, so the first non-blank line
    // indented by six or less is either the next input or the end of the map.
    let end = block
        .match_indices('\n')
        .find(|(index, _)| {
            let line = &block[index + 1..];
            !line.trim().is_empty() && line.len() - line.trim_start_matches(' ').len() <= 6
        })
        .map(|(index, _)| index)
        .unwrap_or(block.len());
    code(&block[..end])
}

/// The shell a step runs, read from its own `run: |` block rather than from the whole step: the
/// `env:` keys below it and the step's YAML header are not shell, and a reader that took them for
/// shell would have to guess what they add to the command line.
fn run_script(step: &str) -> String {
    const MARKER: &str = "        run: |\n";
    let start = step
        .find(MARKER)
        .unwrap_or_else(|| panic!("the step must carry a `run: |` block: {step}"))
        + MARKER.len();
    let body: Vec<&str> = step[start..]
        .lines()
        .take_while(|line| line.trim().is_empty() || line.starts_with("          "))
        .collect();
    code(&body.join("\n"))
}

/// The value one `${{ inputs.<name> }}` reference takes for the dispatch a test asks about.
///
/// The bump step reads three inputs and nothing else, so the reader models exactly that shape and
/// fails on anything new instead of guessing: a construct it skipped would be an argv nobody
/// checked, which is the same as no test at all.
fn expression(expr: &str, dispatch: &[(&str, &str)]) -> String {
    let reference = expr
        .trim()
        .strip_prefix("inputs.")
        .unwrap_or_else(|| panic!("the bump step reads `${{{{ {expr} }}}}`, which this reader does not model"));
    let (name, fallback) = match reference.split_once("||") {
        Some((name, fallback)) => (name.trim(), Some(fallback.trim().trim_matches('\''))),
        None => (reference.trim(), None),
    };
    let value = dispatch
        .iter()
        .find(|(key, _)| *key == name)
        .map(|(_, value)| *value)
        .unwrap_or_else(|| panic!("this dispatch sets no `{name}`, whose value decides the argv"));
    let value = if value.is_empty() {
        fallback.unwrap_or("")
    } else {
        value
    };
    assert!(
        !value.contains(char::is_whitespace),
        "a dispatch value with whitespace in it cannot be modelled here: `{name}` is `{value}`"
    );
    value.to_string()
}

/// `text` with every `${{ … }}` replaced by the value this dispatch gives it.
fn expand(text: &str, dispatch: &[(&str, &str)]) -> String {
    let mut expanded = String::new();
    let mut rest = text;
    while let Some(at) = rest.find("${{") {
        expanded.push_str(&rest[..at]);
        let close = rest[at..]
            .find("}}")
            .unwrap_or_else(|| panic!("unterminated `${{{{` in `{text}`"))
            + at;
        expanded.push_str(&expression(&rest[at + 3..close], dispatch));
        rest = &rest[close + 2..];
    }
    expanded.push_str(rest);
    expanded
}

/// The words of one shell array literal after expansion, quotes stripped: `--channel "alpha"` is two
/// arguments, and `""` is the one empty argument an unset input would contribute if it were passed.
fn shell_words(contents: &str, dispatch: &[(&str, &str)]) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    for character in expand(contents, dispatch).chars() {
        match character {
            '"' => quoted = !quoted,
            ' ' | '\t' if !quoted => {
                if !current.is_empty() {
                    words.push(std::mem::take(&mut current));
                }
            }
            other => current.push(other),
        }
    }
    assert!(
        !quoted,
        "unbalanced quote in `{contents}`: the reader cannot split it into arguments"
    );
    if !current.is_empty() {
        words.push(current);
    }
    words
}

/// Whether one shell test passes for this dispatch: `-n` on a value, and `=` against a literal.
fn holds(test: &str, dispatch: &[(&str, &str)]) -> bool {
    if let Some(reference) = test.strip_prefix("[ -n ").and_then(|rest| rest.strip_suffix(" ]")) {
        let words = shell_words(reference, dispatch);
        return words.len() == 1 && !words[0].is_empty();
    }
    if let Some(rest) = test.strip_prefix("[ ").and_then(|rest| rest.strip_suffix(" ]"))
        && let Some((reference, expected)) = rest.split_once(" = ")
    {
        let words = shell_words(reference, dispatch);
        return words.len() == 1 && words[0] == expected.trim().trim_matches('"');
    }
    panic!("the bump step tests `{test}`, a condition this reader does not model")
}

/// The two promotion commands, one per channel, exactly as the pipeline runs them.
///
/// Written out in full rather than assembled from flags: the two lines ask for opposite states, and
/// a `--prerelease` that landed in the stable arm, or a `--latest=true` in the prerelease one, would
/// still satisfy any substring check on the step.
const PROMOTE_STABLE: &str = "gh release edit \"$TAG\" --draft=false --prerelease=false --latest=true";
const PROMOTE_PRERELEASE: &str = "gh release edit \"$TAG\" --draft=false --prerelease --latest=false";

/// The `cutver bump` invocation exactly as the pipeline runs it today.
///
/// Byte-identical is the requirement for a dispatch that sets no channel: the channel is appended to
/// the argument array, so this line never learns about it and an untouched dispatch cannot change.
const BUMP_CALL: &str = "cutver bump \"${{ inputs.bump || 'auto' }}\" \"${args[@]}\"";

/// The argv the bump step produces for one dispatch, read from the shell it runs.
///
/// The channel flag only means anything if it reaches this command line, and `contains("--channel")`
/// cannot tell a guarded append from one spliced into the call: the flag printed unconditionally, or
/// appended after the call, reads the same to a substring check while `release.yml` runs a different
/// command. So the array is built here the way the shell builds it, from the step's own lines, and any
/// construct this reader does not model stops the test rather than being skipped.
fn bump_arguments(step: &str, dispatch: &[(&str, &str)]) -> Vec<String> {
    let script = run_script(step);
    let call_at = script
        .find("cutver bump \"")
        .unwrap_or_else(|| panic!("the version job must run `cutver bump`: {script}"));
    let (head, rest) = script.split_at(call_at);
    let call = rest.lines().next().unwrap_or_default();
    assert_eq!(
        call, BUMP_CALL,
        "the bump invocation must stay byte-identical: the channel is appended to `args`, never \
         spliced into the call line, or a dispatch that sets no channel stops running today's argv"
    );

    let mut arguments = vec!["bump".to_string(), expand("${{ inputs.bump || 'auto' }}", dispatch)];
    // `None` while no branch is open, `Some(false)` while skipping the branch that did not apply.
    let mut branch: Option<bool> = None;
    for line in head.lines().map(str::trim).filter(|line| !line.is_empty()) {
        if let Some(test) = line.strip_prefix("if ").and_then(|rest| rest.strip_suffix("; then")) {
            assert!(
                branch.is_none(),
                "the reader models one conditional at a time, and the bump step nests `{line}`"
            );
            branch = Some(holds(test, dispatch));
            continue;
        }
        if line == "fi" {
            assert!(branch.take().is_some(), "`fi` without an `if` in the bump step: {head}");
            continue;
        }
        if branch == Some(false) {
            continue;
        }
        if let Some(contents) = line
            .strip_prefix("args=(")
            .or_else(|| line.strip_prefix("args+=("))
            .and_then(|rest| rest.strip_suffix(')'))
        {
            arguments.extend(shell_words(contents, dispatch));
            continue;
        }
        panic!(
            "the bump step runs `{line}`, a construct this reader does not model: extend it with the \
             arguments that line adds, or the argv this test reads is not the one the step builds"
        );
    }
    assert!(
        branch.is_none(),
        "the bump step leaves a conditional open before its call: {head}"
    );
    arguments
}

fn declared_plugin() -> PluginConfig {
    let config_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cutver.toml");
    let cfg = config::load(config_path).expect("the repository cutver.toml must load");
    let name = PluginName::new(PLUGIN).unwrap();
    cfg.plugins
        .get(&name)
        .cloned()
        .unwrap_or_else(|| panic!("cutver.toml must declare [plugins.{PLUGIN}]"))
}

/// The workflow must not repeat what `cutver.toml` already declares. A second copy of the
/// artifact name drifted the moment the pin moved, and the release then looked for a file nobody
/// wrote. `cutver plugin restore` reads `source` and `hash` from the configuration, writes into
/// the cache root the host resolves, and verifies the digest as it downloads.
#[test]
fn workflow_restores_the_declared_plugins_instead_of_pinning_them_again() {
    let workflow = repo_text(".github/workflows/release.yml");
    assert!(
        workflow.contains("cutver plugin restore"),
        "release.yml must hydrate the plugin cache with `cutver plugin restore`"
    );
    assert!(
        !workflow.contains(&format!("{PLUGIN}-v")),
        "release.yml must not name a plugin version of its own: cutver.toml owns the pin"
    );
}

/// The release must be cut by the CLI built from the commit being released.
///
/// `cutver/setup` resolves `version: latest`, i.e. the previously published binary, so a defect in
/// the sandbox or in the release path could only ship in a release it was unable to cut. #251 did
/// exactly that: 0.17.0 could not read `cutver.toml` inside the plugin sandbox, `github-publish`
/// 0.2.x failed closed, and 0.18.0 was blocked until the pin was rolled back to 0.1.2 by hand.
/// These assertions fail if the workflow goes back to depending on an external binary.
#[test]
fn the_release_is_cut_by_the_cli_built_from_the_released_commit() {
    let workflow = repo_text(".github/workflows/release.yml");
    assert!(
        !workflow.contains("uses: cutver/release@v1"),
        "the release must not run the published action: `cutver/release` resolves `version: latest`, \
         so a fix in the release path could only ship in a release it was unable to cut"
    );

    // Scoped to the job that cuts the release: a literal elsewhere in the file keeps the test green
    // while the mechanism is dead.
    let version = job(&workflow, "version");
    let built = version.find("Build the CLI from this commit").expect("build step");
    let on_path = version.find("Put the built CLI first on PATH").expect("PATH step");
    let restored = version.find("run: cutver plugin restore").expect("plugin restore step");
    let bump = version
        .find("cutver bump \"${{ inputs.bump || 'auto' }}\"")
        .expect("bump step");
    assert!(
        built < on_path && on_path < restored && restored < bump,
        "the plugin-enabled CLI must be built, put on PATH and hydrated with the declared plugins \
         before the bump step runs: every step in between shells out to whatever `cutver` resolves"
    );
    // Later jobs may install a published CLI on purpose: the smoke test proves the artifact a user
    // downloads works. What must never happen is an installed binary *cutting* the release.
    assert!(
        !version[..bump].contains("uses: cutver/setup"),
        "no published CLI may be installed before the release step: the binary that cuts release N must be N's code"
    );
}

/// The plugin only owns the release if it is a draft: its `draft` field defaults to false, and a
/// non-draft release is published the moment `on_post_release` fires, before its assets exist.
#[test]
fn plugin_creates_the_release_as_a_draft() {
    let plugin = declared_plugin();
    let plugin_config = plugin.config.expect("the publish plugin needs a config table");
    assert_eq!(
        plugin_config.get("draft").and_then(|value| value.as_bool()),
        Some(true),
        "`draft = true` is required: the default publishes a public, asset-less release before build"
    );
    assert!(
        plugin_config
            .get("repository")
            .and_then(|value| value.as_str())
            .is_some(),
        "the plugin needs `config.repository` to address the API"
    );
}

/// S6: `[plugins.github-publish.config]` must not declare `prerelease`.
///
/// GitHub's own rule is the protection: "The latest release is the most recent non-prerelease,
/// non-draft release" and "Drafts and prereleases cannot be set as latest". The plugin leans on it by
/// deriving the mark from the version it is handed
/// (`plugins/github-publish/src/post_release.rs:78-80`, `config.prerelease.unwrap_or_else(||
/// is_prerelease(&payload.version))`), so this key is the one thing that cancels the derivation while
/// the repository never has to set anything. Unset is not "false": unset is "let the version decide".
/// `prerelease = false` would make the draft for `0.22.0-alpha.1` a non-prerelease release, and a
/// non-prerelease release is exactly what can take the repository's Latest slot — the slot
/// `uses: cutver/setup@v0` resolves with its `version: latest` default. `prerelease = true` would go
/// the other way and mark every stable release as a prerelease, which no `--latest=true` can undo.
#[test]
fn the_publish_plugin_config_declares_no_prerelease_key() {
    let plugin = declared_plugin();
    let plugin_config = plugin.config.expect("the publish plugin needs a config table");
    assert!(
        !plugin_config.contains_key("prerelease"),
        "`[plugins.github-publish.config]` must not declare `prerelease`: leaving it unset is what \
         makes the plugin derive the mark from the released version, so `0.22.0-alpha.1` is created \
         as a prerelease. Setting it false would let an alpha take the repository's Latest slot, \
         because only a release *marked* prerelease is protected from it, and `version: latest` is \
         what `uses: cutver/setup@v0` installs by default"
    );
}

/// Events are compared by exact string, and lifecycle events only reach `lifecycle.v1` plugins.
#[test]
fn declared_capability_events_and_permissions_are_the_ones_the_host_uses() {
    let plugin = declared_plugin();
    assert_eq!(plugin.runtime, RuntimeKind::Wasm);
    assert!(plugin.capabilities.contains(&Capability::LifecycleV1));
    assert_eq!(
        plugin.events,
        vec!["on_pre_bump".to_string(), "on_post_release".to_string()]
    );
    for key in ["GITHUB_TOKEN", "GITHUB_REPOSITORY"] {
        assert!(
            plugin.permissions.env.iter().any(|declared| declared == key),
            "`{key}` must be declared in permissions.env to reach the guest"
        );
    }
    assert!(plugin.permissions.network.iter().any(|host| host == "api.github.com"));
}

/// A lean binary cannot create the wasm driver, and a boolean input only forwards if it is wired.
#[test]
fn workflow_requests_the_plugin_build_and_forwards_dry_run() {
    let workflow = repo_text(".github/workflows/release.yml");
    let version = job(&workflow, "version");
    assert!(
        version.contains("cargo build --locked --release --features plugins"),
        "the version job must build the plugin-enabled CLI: a lean build fails closed on the \
         [plugins.github-publish] declaration"
    );
    assert!(
        version.contains("if [ \"${{ inputs.dry_run }}\" = \"true\" ]"),
        "the `workflow_dispatch` boolean must drive the flag, or `dry_run: true` still releases"
    );
    assert!(
        version.contains("args+=(--dry-run)"),
        "`--dry-run` must be the branch `inputs.dry_run` controls"
    );
    for key in ["GITHUB_TOKEN:", "GITHUB_REPOSITORY:"] {
        assert!(
            version.contains(key),
            "the bump step must export `{key}`, or the declared permission arrives empty"
        );
    }
}

/// S1: the channel is the dispatch's own closed choice, and its empty default is load-bearing.
///
/// The option set is `cutver bump --channel`'s ladder and nothing else: a free-text input accepts
/// `alhpa`, and the bump then fails after the token mint, the checkout and the release-profile build
/// instead of at the dispatch form. The empty option is what keeps a dispatch that never heard of the
/// ladder on today's argv, so it has to be the default rather than the first channel in the list.
#[test]
fn the_dispatch_declares_the_channel_as_a_choice_input() {
    let workflow = repo_text(".github/workflows/release.yml");
    let input = dispatch_input(&workflow, "channel");

    assert!(
        input.contains("        type: choice"),
        "the `channel` input must be a `choice`, so the dispatch form can only submit a channel the \
         ladder knows: {input}"
    );
    assert!(
        input.contains("        required: false"),
        "the `channel` input must stay optional: a dispatch without it is the stable release this \
         workflow already cuts: {input}"
    );
    assert!(
        input.contains("        default: \"\""),
        "the `channel` input must default to empty: any other default silently turns every existing \
         dispatch into a prerelease, and `inputs.channel` would never be empty for the guard to \
         skip: {input}"
    );
    let options: Vec<&str> = input
        .lines()
        .map(str::trim)
        .filter_map(|line| line.strip_prefix("- "))
        .collect();
    assert_eq!(
        options,
        ["\"\"", "alpha", "beta", "rc", "stable"],
        "the `channel` options must be the empty choice followed by the ladder `cutver bump --channel` \
         accepts, in that order: a free-text or differently-spelled option set fails at the bump, \
         after the build, instead of at the dispatch form: {input}"
    );
}

/// #259: the version job must leave no trace a failed smoke would have to undo. A tag cannot be
/// unpublished, a crate version cannot be reused, and a public release without its assets is
/// exactly what `draft = true` exists to prevent.
#[test]
fn the_version_job_publishes_nothing_irreversible() {
    let workflow = repo_text(".github/workflows/release.yml");
    let version = job(&workflow, "version");
    let version_code = code(version);
    let bump_step = step_containing(version, "cutver bump");
    let bump_code = code(bump_step);
    for forbidden in ["cutver publish", "cargo publish", "git tag"] {
        assert!(
            !version_code.contains(forbidden),
            "the version job must not run `{forbidden}`: an irreversible step there means a run whose \
             smoke fails has already published a tag, a crate version or a public release"
        );
    }
    assert!(
        bump_code.contains("--defer-publish"),
        "the bump step must pass `--defer-publish`, or `cutver bump` runs the tag, the registry and \
         `on_post_release` inside the version job again"
    );
    assert!(
        version_code.contains("id: cutver"),
        "the job outputs must come from a step that carries `id: cutver`, which is what \
         `steps.cutver.outputs.*` reads"
    );
    // Phase 4B of #326: the bump step owns the job outputs. `cutver bump` writes them to
    // `$GITHUB_OUTPUT` itself, so a `--report` or a `jq` back in this job is not a style regression:
    // it is a second authority over the release result, and two authorities drift.
    assert!(
        bump_code.contains("id: cutver"),
        "the job outputs read `steps.cutver.outputs.*`, so the step that runs `cutver bump` must \
         carry `id: cutver` — and it must be that step, not a second one that parses its output"
    );
    assert!(
        !version_code.contains("--report"),
        "no step in the version job may pass `--report`: the report file is a local and script \
         surface, and the workflow reads the tool's step outputs instead — a step that writes it and \
         another that reads it back is a second authority over the release result next to the tool"
    );
    assert!(
        !version_code.contains(".cutver-release.json"),
        "no step in the version job may name `.cutver-release.json`: reading the report back — with \
         `jq`, `python3` or any other tool — makes the workflow parse the release result instead of \
         reading the step outputs the bump step wrote"
    );
    assert!(
        !workflow.contains(".cutver-release.json"),
        "release.yml must not name `.cutver-release.json` anywhere: the release result has one owner, \
         the bump step's outputs, and a reader anywhere in the file is the coupling phase 4B removed"
    );
    assert!(
        !version_code.contains("jq"),
        "the version job must not parse a release report with `jq`: `cutver bump` writes the outputs \
         the downstream jobs read, so a parser here would silently go stale"
    );
}

/// S2: the bump step fails closed when the tool wrote no usable release status.
///
/// The report-file existence check was the old guard against a run that published nothing and left
/// every downstream job skipped. Its replacement must accept exactly the two values the gated `if:`
/// conditions compare against: presence alone is not enough, because a `released=` line with an
/// empty value renders `steps.cutver.outputs.released` empty and skips the same jobs on the same
/// green run.
#[test]
fn the_bump_step_fails_closed_when_the_tool_wrote_no_outputs() {
    const GUARD: &str = "grep -qE '^released=(true|false)$' \"$GITHUB_OUTPUT\"";
    let workflow = repo_text(".github/workflows/release.yml");
    let version = job(&workflow, "version");
    let bump_code = code(step_containing(version, "cutver bump"));

    let call_at = bump_code
        .find("cutver bump \"")
        .expect("the version job must run `cutver bump`");
    let guard_at = bump_code.find(GUARD).unwrap_or_else(|| {
        panic!(
            "the bump step must read exactly `released=true` or `released=false` from \
             $GITHUB_OUTPUT: a presence-only match lets an empty value through, and every job gated \
             on `released == 'true'` is then skipped on a green run: {bump_code}"
        )
    });
    assert!(
        call_at < guard_at,
        "the guard must read $GITHUB_OUTPUT after `cutver bump` wrote it: checked before the call it \
         reads a stale or absent file and fails a healthy release: {bump_code}"
    );
    assert!(
        bump_code.contains("exit 1"),
        "the guard must fail the job, not warn: a warning leaves the downstream jobs skipped and the \
         run green, which is the silent no-release the report-file check used to catch: {bump_code}"
    );
    assert!(
        bump_code.contains("::error::cutver bump wrote no usable 'released' step output; nothing was published."),
        "the guard must fail with the actionable `::error::` line that says nothing was published: \
         an exit without it surfaces as a bare step failure with no cause: {bump_code}"
    );
    assert!(
        !bump_code.contains("grep -q '^released='"),
        "the guard must not regress to matching the bare `released=` prefix: an empty value passes it \
         and every downstream job is skipped on a green run: {bump_code}"
    );
}

/// #259/S1: an argument array the call never expands publishes the release from the version job.
///
/// `--defer-publish` is what keeps this job harmless, and it only defers if it reaches the command:
/// the flag sitting in `args` while the invocation runs `cutver bump <level>` creates the tag,
/// publishes the crate and fires `on_post_release` before a single smoke has run.
#[test]
fn the_bump_call_expands_the_defer_flag_it_declares() {
    let workflow = repo_text(".github/workflows/release.yml");
    let version = job(&workflow, "version");
    let bump_code = code(step_containing(version, "cutver bump"));

    assert!(
        bump_code.contains("args=(--defer-publish)"),
        "the bump step must declare `--defer-publish`: without it `cutver bump` runs the tag, the \
         registry and `on_post_release` inside the version job"
    );
    let call = bump_code
        .lines()
        .find(|line| line.contains("cutver bump \"${{ inputs.bump || 'auto' }}\""))
        .unwrap_or_else(|| {
            panic!("the version job must invoke `cutver bump` with the requested bump level: {bump_code}")
        });
    assert!(
        call.contains("\"${args[@]}\""),
        "the `cutver bump` invocation must expand the argument array: a flag left in `args` and \
         never passed to the command is the same as no flag, and the bump publishes irreversibly \
         before smoke runs: {call}"
    );
}

/// S1: a dispatch that names a channel runs the bump on that channel.
///
/// The argv is the contract, not the presence of a flag: `--channel alpha` has to be two arguments
/// after `--defer-publish`, exactly as `cutver bump --channel` parses them. Without it the workflow
/// cuts a stable version from a dispatch that asked for an alpha, which is the whole ladder lost
/// silently — the release is green, published, and not the prerelease the maintainer requested.
#[test]
fn a_dispatch_channel_reaches_the_bump_arguments() {
    let workflow = repo_text(".github/workflows/release.yml");
    let version = job(&workflow, "version");
    let step = step_containing(version, "cutver bump");

    // The dry run is the first dispatch of the ladder: it has to read the plan for the channel it
    // was asked about, so the flag cannot be skipped when `--dry-run` is set.
    for (dry_run, expected) in [
        ("false", vec!["bump", "minor", "--defer-publish", "--channel", "alpha"]),
        (
            "true",
            vec!["bump", "minor", "--defer-publish", "--dry-run", "--channel", "alpha"],
        ),
    ] {
        let arguments = bump_arguments(step, &[("bump", "minor"), ("channel", "alpha"), ("dry_run", dry_run)]);
        assert_eq!(
            arguments, expected,
            "a dispatch with `channel: alpha` and `dry_run: {dry_run}` must run `cutver bump` with \
             `--channel alpha`: the bump step is the only place the channel can enter the release, and \
             a run that drops it publishes a stable version instead of the requested prerelease"
        );
    }
}

/// S1: an untouched dispatch runs today's argv, byte for byte.
///
/// This is the other half of the channel pin, and the half a `contains("--channel")` cannot see: a
/// flag appended outside its guard, or a call line that grew a suffix, still satisfies every string
/// the rest of this file looks for while every dispatch that sets no channel changes what it
/// publishes. The empty channel must contribute no argument at all — not an empty one.
#[test]
fn an_empty_channel_keeps_todays_bump_arguments() {
    let workflow = repo_text(".github/workflows/release.yml");
    let version = job(&workflow, "version");
    let step = step_containing(version, "cutver bump");

    for (bump, dry_run, expected) in [
        ("auto", "false", vec!["bump", "auto", "--defer-publish"]),
        ("minor", "true", vec!["bump", "minor", "--defer-publish", "--dry-run"]),
    ] {
        let arguments = bump_arguments(step, &[("bump", bump), ("channel", ""), ("dry_run", dry_run)]);
        assert_eq!(
            arguments, expected,
            "a dispatch with no channel must run exactly the argv it runs today, `{expected:?}`: the \
             channel is appended only when it is set, and an empty value that still contributed an \
             argument would hand `cutver bump` a channel it never meant to publish on"
        );
    }
}

/// S1: every job output must read a key `cutver bump` writes, from the step that ran it.
///
/// The bump step appends `released`, `deferred`, `version`, `channel`, `tag` and `commit` to
/// `$GITHUB_OUTPUT` (phase 4A of #326). An output pointing at another step, or naming a key nobody writes, resolves
/// to an empty string, and the `needs.version.outputs.*` gate it feeds then skips the release it was
/// supposed to run.
#[test]
fn every_version_job_output_reads_a_key_the_tool_writes() {
    /// The keys `cutver bump` writes to `$GITHUB_OUTPUT`.
    const WRITTEN: [&str; 6] = ["released", "deferred", "version", "channel", "tag", "commit"];
    let workflow = repo_text(".github/workflows/release.yml");
    let version = job(&workflow, "version");
    let outputs = declared_outputs(version);

    for key in ["tag", "released", "version", "commit"] {
        let expected = format!("${{{{ steps.cutver.outputs.{key} }}}}");
        let value = outputs
            .iter()
            .find(|(declared, _)| declared == key)
            .map(|(_, value)| value.as_str())
            .unwrap_or_else(|| {
                panic!(
                    "the version job must keep its `{key}` output: every downstream job reads \
                     `needs.version.outputs.{key}`, and a missing output is an empty string"
                )
            });
        assert_eq!(
            value, expected,
            "`{key}` must read `{expected}`: the bump step owns the release result, so a value taken \
             from any other step is empty and the release it gates never runs"
        );
    }

    for (key, value) in &outputs {
        let Some(reference) = value.strip_prefix("${{ steps.") else {
            // A constant output (`notes`) reads no step at all.
            continue;
        };
        let (step, output) = reference
            .split_once(".outputs.")
            .unwrap_or_else(|| panic!("`{key}` must read `steps.<id>.outputs.<name>`, not `{value}`"));
        assert_eq!(
            step, "cutver",
            "`{key}` must read the step that carries `id: cutver`, not `{step}`: no other step wrote \
             a release result, so the output is empty"
        );
        let output = output.trim_end_matches(" }}");
        assert!(
            WRITTEN.contains(&output),
            "`{key}` must read a key `cutver bump` writes ({WRITTEN:?}), not `{output}`: an unwritten \
             key resolves to an empty string and skips the jobs gated on it"
        );
    }
}

/// The only job that publishes must run after every verification job, on one exact `needs:` line:
/// a leg added here but left out there runs red while the release goes public anyway.
#[test]
fn the_publish_job_waits_for_every_verification_job() {
    let workflow = repo_text(".github/workflows/release.yml");
    let publish = job(&workflow, "publish");
    let needs = format!("    needs: [{}]", VERIFIED_BEFORE_PUBLISHING.join(", "));
    assert!(
        publish.contains(&format!("\n{needs}\n")),
        "the publish job must declare exactly `{needs}`: a smoke that fails has to leave the tag, \
         the crate and the draft unpublished"
    );
    for id in VERIFIED_BEFORE_PUBLISHING {
        assert!(
            workflow.contains(&format!("\n  {id}:\n")),
            "the publish job needs `{id}`, which must still exist as a job"
        );
    }
    assert!(
        publish.contains("if: needs.version.outputs.released == 'true'"),
        "a dry run must publish nothing: the job stays gated on the reported `released`"
    );
}

/// The tag no longer exists while the build and smoke jobs run, so they must check out the release
/// commit. A `ref: needs.version.outputs.tag` there resolves to nothing and the run dies before the
/// verification it exists to perform.
#[test]
fn the_built_artifacts_come_from_the_release_commit() {
    let workflow = repo_text(".github/workflows/release.yml");
    for id in ["build", "build-plugins", "smoke", "smoke-plugins"] {
        let slice = job(&workflow, id);
        let checkout = slice
            .find("uses: actions/checkout@v7")
            .unwrap_or_else(|| panic!("the `{id}` job must check out"));
        let rest = &slice[checkout..];
        let step = &rest[..rest.find("\n      - ").unwrap_or(rest.len())];
        assert!(
            step.contains("ref: ${{ needs.version.outputs.commit }}"),
            "the `{id}` job must check out the release commit: {step}"
        );
        assert!(
            !step.contains("needs.version.outputs.tag"),
            "the `{id}` job must not check out by tag: that ref does not exist until the publish \
             job creates it: {step}"
        );
    }
}

/// The irreversibility order #259 is about: the tag and the crate first, the assets into the draft,
/// and only then the draft made public. A public release without its assets cannot be undone.
#[test]
fn the_tag_the_crate_and_the_draft_are_published_after_verification() {
    let workflow = repo_text(".github/workflows/release.yml");
    let publish = job(&workflow, "publish");
    let closing = publish.find("cutver publish --tag").expect("the closing phase");
    let assets = publish
        .find("Download all asset artifacts")
        .expect("asset download step");
    let upload = publish
        .find("gh release upload \"$TAG\" dist/* --clobber")
        .expect("upload step");
    let promote = publish.find(PROMOTE_STABLE).expect("promotion step");
    assert!(
        closing < assets && assets < upload && upload < promote,
        "`cutver publish` creates the tag and the crate, the assets land in its draft, and only then \
         is the draft promoted"
    );
    assert!(
        publish.contains("--commit \"${{ needs.version.outputs.commit }}\""),
        "`cutver publish` must name the commit the verification jobs built, or it can tag a commit \
         nobody tested"
    );
    assert!(
        publish.contains("name: release-notes"),
        "the publish job must download the rendered notes: they are the body `on_post_release` \
         releases, not a second copy of the changelog"
    );
    assert!(
        publish.contains("path: ${{ runner.temp }}")
            && publish.contains("cat \"${{ runner.temp }}/RELEASE_NOTES.md\" | cutver publish")
            && publish.contains("--notes-file -"),
        "the notes must be downloaded outside the checkout and streamed into `cutver publish`: \
         handing the guard a notes path makes it the one input it has to excuse, and `cargo publish` \
         in `[publish].commands` runs its own clean-tree check that knows nothing about that \
         exemption (#302, #306)"
    );
}

/// #259/S1: the publish job runs on its own runner, so it prepares its own copy of the CLI.
///
/// `cutver publish` shells out to the built binary through PATH and fails closed on the
/// `[plugins.github-publish]` declaration without the restored plugin artifacts. Moving the
/// closing step above the PATH export or above `cutver plugin restore` used to leave every other
/// assertion in this file green, because they are all scoped to the `version` job.
#[test]
fn the_closing_job_prepares_the_cli_it_runs() {
    let workflow = repo_text(".github/workflows/release.yml");
    let publish = job(&workflow, "publish");
    let built = publish
        .find("Build the CLI from the release commit")
        .expect("the publish job must build the CLI it runs");
    let on_path = publish
        .find("Put the built CLI first on PATH")
        .expect("the publish job must put the built CLI on PATH");
    let restored = publish
        .find("run: cutver plugin restore")
        .expect("the publish job must hydrate the declared plugins");
    let notes = publish
        .find("name: release-notes")
        .expect("the publish job must download the rendered notes");
    let closing = publish.find("cutver publish --tag").expect("the closing phase");
    assert!(
        built < on_path && on_path < restored && restored < notes && notes < closing,
        "the publish job must build the CLI, put it on PATH and restore the declared plugins before \
         `cutver publish` runs: every step in between shells out to whatever `cutver` resolves, and \
         `cutver publish` needs RELEASE_NOTES.md from the `release-notes` artifact to stream as its \
         notes"
    );
}

/// #259/S1: `cutver publish` creates the tag with `git tag -a`, which needs an author identity.
///
/// The only `git config --global user.*` step lives in the `version` job, a different runner: the
/// publish job inherits no global config, so an annotated tag there fails with `unable to
/// auto-detect email address` — after the bump push, before the release exists. The identity is
/// also the #166 attribution, which is why it must be the release bot's and not the runner's.
#[test]
fn the_tag_creating_job_configures_the_git_identity() {
    let workflow = repo_text(".github/workflows/release.yml");
    let publish = job(&workflow, "publish");
    let closing = publish.find("cutver publish --tag").expect("the closing phase");
    for needle in ["git config --global user.name", "git config --global user.email"] {
        let at = publish
            .find(needle)
            .unwrap_or_else(|| panic!("the publish job must configure the tag author with `{needle}`"));
        assert!(
            at < closing,
            "`{needle}` must run before `cutver publish`: the tag it creates is annotated, and this \
             job inherits no global git identity"
        );
    }
}

/// The plugin owns release creation, so the workflow must fill that draft and publish it — not
/// create a second release. A leftover `softprops/action-gh-release` would race the plugin for the
/// same tag and fail with 422.
#[test]
fn the_plugin_owns_release_creation_and_the_workflow_publishes_its_draft() {
    let workflow = repo_text(".github/workflows/release.yml");
    assert!(
        !workflow.contains("softprops/action-gh-release"),
        "release.yml must not create the release: the github-publish plugin does, as a draft"
    );
    for needle in [
        "gh release view \"$TAG\" --json isDraft",
        "gh release upload \"$TAG\" dist/* --clobber",
        "gh release edit \"$TAG\"",
        "--draft=false",
    ] {
        assert!(workflow.contains(needle), "release.yml must keep `{needle}`");
    }
    // Assets must be uploaded before the draft becomes visible.
    let upload = workflow.find("gh release upload").expect("upload step");
    let publish = workflow.find("--draft=false").expect("publish step");
    assert!(
        upload < publish,
        "assets must be uploaded before the release is published"
    );
}

/// The guard must accept a draft, accept an already-published release as a retry, and fail only
/// when no release exists — while tolerating unexpected output rather than blocking a release on a
/// stray byte from `gh`.
#[test]
fn publish_guard_distinguishes_draft_published_and_missing() {
    let workflow = repo_text(".github/workflows/release.yml");
    for needle in [
        "if .isDraft then \"draft\" else \"published\" end",
        "draft|published) echo",
        "published)",
        "draft) ;;",
        "Could not read the release for $TAG",
        "Unexpected release state for $TAG",
        "Refusing to promote $TAG",
    ] {
        assert!(workflow.contains(needle), "release.yml must keep `{needle}`");
    }
    // An auth failure must surface gh's own message, never a fabricated "no release" verdict.
    assert!(
        workflow.contains("--json isDraft --jq 'if .isDraft then \"draft\" else \"published\" end' 2>&1"),
        "the state read must capture stderr so a 401 is diagnosable"
    );
    assert!(
        !workflow.contains("grep -qx true"),
        "a byte-exact match on `.isDraft` blocks the release on trailing whitespace"
    );
}

/// Both gh steps must authenticate and receive the tag; the publish job infers the repository from
/// the checkout, so a missing GH_TOKEN fails at the API rather than at the guard.
#[test]
fn publish_steps_pass_gh_token_and_tag() {
    let workflow = repo_text(".github/workflows/release.yml");
    assert_eq!(
        workflow
            .matches("GH_TOKEN: ${{ steps.release-token.outputs.token || secrets.GITHUB_TOKEN }}")
            .count(),
        2,
        "the upload and the publish step must both authenticate, falling back to GITHUB_TOKEN"
    );
    assert_eq!(
        workflow.matches("TAG: ${{ needs.version.outputs.tag }}").count(),
        2,
        "both gh steps must receive the tag as TAG"
    );
}

/// The version job renders the entry once and saves it as an artifact. The publish job downloads it
/// and streams it into `cutver publish`, whose `on_post_release` plugin creates the draft from it;
/// the promotion that follows only flips the draft state. Repeating the body there gave the release
/// a second owner — that is how `title_template` once came out dead.
#[test]
fn release_notes_artifact_is_kept_but_does_not_feed_the_promotion() {
    let workflow = repo_text(".github/workflows/release.yml");
    let version = job(&workflow, "version");
    assert!(
        version.contains("name: release-notes\n          path: RELEASE_NOTES.md"),
        "the version job must save RELEASE_NOTES.md as the `release-notes` artifact"
    );
    assert!(
        version.contains("cutver changelog latest --template .github/templates/cutver/RELEASE.md > RELEASE_NOTES.md"),
        "the version job must render the notes from the template cutver.toml names"
    );
    // The promotion is exactly the two channel commands, and nothing else: a `--title` or a
    // `--notes-file` here gives the release a second author, and any third `gh release edit` in this
    // job is a promotion nobody reviewed.
    let publish_job = job(&workflow, "publish");
    let promotions: Vec<&str> = publish_job
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("gh release edit \"$TAG\""))
        .collect();
    assert_eq!(
        promotions,
        vec![PROMOTE_STABLE, PROMOTE_PRERELEASE],
        "the promotion must be exactly `{PROMOTE_STABLE}` for a stable cut and \
         `{PROMOTE_PRERELEASE}` for a prerelease: a `--title`, a body or a fourth flag there \
         overrides the plugin that authored the release"
    );
}

/// The arms of the promotion's `case "$CHANNEL" in … esac`, as `(patterns, commands)`.
///
/// The promotion does not build an argv array, so the branch that picks the command *is* the
/// contract: both command literals sit in the same step, and a `contains` on either cannot tell the
/// stable arm from the prerelease one. Reading the arms is what a swapped or duplicated pattern
/// fails. A construct this reader does not model stops the test rather than being skipped.
fn channel_arms(step: &str) -> Vec<(String, Vec<String>)> {
    const HEAD: &str = "case \"$CHANNEL\" in";
    let script = run_script(step);
    let start = script
        .find(HEAD)
        .unwrap_or_else(|| panic!("the promotion must pick its command from the reported channel: {script}"))
        + HEAD.len();
    let mut arms: Vec<(String, Vec<String>)> = Vec::new();
    for line in script[start..].lines().map(str::trim) {
        if line == "esac" {
            return arms;
        }
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(patterns) = line.strip_suffix(')') {
            arms.push((patterns.to_string(), Vec::new()));
            continue;
        }
        if line == ";;" {
            continue;
        }
        let arm = arms
            .last_mut()
            .unwrap_or_else(|| panic!("`{line}` runs outside any channel arm of the promotion: {script}"));
        arm.1.push(line.to_string());
    }
    panic!("the channel `case` must close with `esac`: {script}")
}

/// S6: the promotion states the release state the channel asks for, and the publish job can only do
/// that because the bump step reports the channel.
///
/// GitHub cannot make a prerelease Latest — "the latest release is the most recent non-prerelease,
/// non-draft release" — but the draft arrives already marked prerelease only because the plugin
/// derives that flag from the version it is handed
/// (`plugins/github-publish/src/post_release.rs:78-80`). Two things nothing in this repository pins
/// then decide whether an alpha takes the Latest slot `uses: cutver/setup@v0` resolves:
/// `version: latest` defaults to it, and the mark is the only thing GitHub refuses to override.
/// Passing both flags here removes the first dependency; `the_publish_plugin_config_declares_no_prerelease_key`
/// removes the second. A stable cut asks for Latest explicitly and clears a prerelease mark an
/// earlier run could have left; a prerelease asks to stay marked and to never be Latest.
#[test]
fn the_promotion_publishes_by_channel() {
    let workflow = repo_text(".github/workflows/release.yml");
    let version = job(&workflow, "version");
    let publish = job(&workflow, "publish");

    // The channel travels exactly like `released`, `tag` and `commit`: an output of the step that
    // ran `cutver bump`, declared on the `version` job and read as `needs.version.outputs.*`.
    let channel = declared_outputs(version)
        .into_iter()
        .find(|(key, _)| key == "channel")
        .map(|(_, value)| value)
        .unwrap_or_else(|| {
            panic!(
                "the version job must declare a `channel` output: `cutver bump` reports the channel \
                 the run produced, and without the output the publish job cannot know whether the \
                 release it promotes is Latest or a prerelease — `needs.version.outputs.channel` \
                 then renders empty for every run"
            )
        });
    assert_eq!(
        channel, "${{ steps.cutver.outputs.channel }}",
        "the `channel` output must read the step that ran `cutver bump`: that step owns the release \
         result, so a value taken from anywhere else is empty and the promotion fails closed on \
         every stable release"
    );

    let step = step_containing(publish, "gh release edit \"$TAG\"");
    assert!(
        step.contains("CHANNEL: ${{ needs.version.outputs.channel }}"),
        "the promotion step must receive the reported channel as CHANNEL, the same way it receives \
         TAG: the command it runs is decided by the channel, and a step that has to re-derive it from \
         the tag string is the coupling this pin exists to prevent: {step}"
    );

    let arms = channel_arms(step);
    assert_eq!(
        arms.len(),
        3,
        "the promotion must declare exactly three channel arms — stable, the prerelease ladder, and \
         a fail-closed default: {arms:?}"
    );
    assert_eq!(
        arms[0],
        ("stable".to_string(), vec![PROMOTE_STABLE.to_string()]),
        "a stable release must be published as a non-prerelease Latest: `--prerelease=false` clears a \
         mark an earlier run could have left and `--latest=true` sets the slot explicitly, instead of \
         leaving both to whichever release GitHub happened to consider most recent"
    );
    assert_eq!(
        arms[1],
        ("alpha|beta|rc".to_string(), vec![PROMOTE_PRERELEASE.to_string()]),
        "every channel `cutver bump --channel` accepts must publish as a marked prerelease that is \
         never Latest, or `0.22.0-alpha.1` takes the slot `uses: cutver/setup@v0` with `version: \
         latest` installs by default"
    );
    assert_eq!(arms[2].0, "*", "the last arm must be the catch-all: {arms:?}");
    let default_arm = &arms[2].1;
    assert!(
        default_arm.iter().any(|line| line.contains("exit 1")),
        "an unknown channel must fail the job, not fall through to a published release: a step that \
         continues here promotes a release whose state nobody decided: {default_arm:?}"
    );
    assert!(
        default_arm
            .iter()
            .any(|line| line.contains("::error::Refusing to promote")),
        "the catch-all must fail with an `::error::` line naming the release, the way the draft guard \
         above does: a bare exit surfaces as a step failure with no cause: {default_arm:?}"
    );
}

/// The guard every package-manager step carries: the channel `cutver bump` reported decides, and a
/// missing token still skips the step instead of failing it.
const STABLE_ONLY_PACKAGING: &str = "if: needs.version.outputs.channel == 'stable' && (steps.bot-token.outputs.token != '' || env.HOMEBREW_TAP_TOKEN != '')";

/// The four steps that write to a tap or a bucket, each with the `git clone` that proves it is one.
const PACKAGE_MANAGER_STEPS: [(&str, &str); 4] = [
    ("Update Homebrew tap formula", "homebrew-tap.git\" tap\n"),
    ("Update Homebrew tap plugin formula", "homebrew-tap.git\" tap-plugins\n"),
    ("Update Scoop bucket manifest", "scoop-bucket.git\" scoop\n"),
    (
        "Generate Scoop bucket plugin manifest",
        "scoop-bucket.git\" scoop-plugins\n",
    ),
];

/// D3/S8: the tap and the bucket are stable-only, and a prerelease says it skipped them. A
/// prerelease published as the tap's `version` hands `brew install cutver` an alpha nobody asked for.
/// The skip is announced because a step GitHub skips shows as "skipped" with no cause, which reads
/// like a missing token.
#[test]
fn the_package_manager_steps_publish_only_on_a_stable_channel() {
    let workflow = repo_text(".github/workflows/release.yml");
    let publish = job(&workflow, "publish");

    // A fifth `git clone` added without a gate is the regression no per-step assertion can see.
    let clones = publish.matches("git clone \"https://x-access-token:").count();
    assert_eq!(
        clones, 4,
        "the publish job pushes to exactly four tap/bucket clones, found {clones}: a new surface has \
         to be gated like the other four"
    );

    for (name, clone) in PACKAGE_MANAGER_STEPS {
        let step = step_containing(publish, name);
        assert!(
            step.contains(clone),
            "`{name}` must still be the step that clones `{clone}`: {step}"
        );
        let guard = step
            .lines()
            .find(|line| line.trim_start().starts_with("if: "))
            .unwrap_or_else(|| panic!("`{name}` must gate itself on the reported channel: {step}"));
        assert_eq!(
            guard.trim(),
            STABLE_ONLY_PACKAGING,
            "`{name}` must run only on the channel `cutver bump` reported, and still skip with no token"
        );
    }

    let notice = step_containing(publish, "Say the package-manager update was skipped");
    assert!(
        notice.contains("if: needs.version.outputs.channel != 'stable'"),
        "the skip is announced on every channel but stable, from the same reported value: {notice}"
    );
    let message = run_script(notice);
    for expected in ["::notice::", "Homebrew", "Scoop"] {
        assert!(
            message.contains(expected),
            "the skip must surface as an `::notice::` naming Homebrew and Scoop and the file each \
             left alone, so a prerelease reads as a decision: {message}"
        );
    }
}

/// S8: one build per package manager, and it is the plugin-enabled one.
///
/// `cutver` installed the lean archive, which asked a user to know what a WASM plugin is before
/// choosing a formula. The plugin build is a superset and its sums are already computed in the same
/// step, so nothing new is measured; the binary name, the conflict and the assertion stay.
#[test]
fn the_plain_package_installs_the_plugin_enabled_build() {
    let workflow = repo_text(".github/workflows/release.yml");
    let publish = job(&workflow, "publish");

    let brew = run_script(step_containing(publish, "git add Formula/cutver.rb"));
    let (mut urls, mut sums) = (Vec::new(), Vec::new());
    for line in brew.lines().map(str::trim) {
        if line.starts_with("url \"") {
            urls.push(line);
        } else if line.starts_with("sha256 \"") {
            sums.push(line);
        }
    }
    let targets = [
        "cutver-plugins-${VERSION}-macos-arm64.tar.gz",
        "cutver-plugins-${VERSION}-macos-x86_64.tar.gz",
        "cutver-plugins-${VERSION}-linux-musl-x86_64.tar.gz",
        "cutver-plugins-${VERSION}-linux-musl-arm64.tar.gz",
    ];
    assert_eq!(urls.len(), targets.len(), "one url per target: {urls:?}");
    for (url, target) in urls.iter().zip(targets) {
        assert!(
            url.contains(target),
            "`cutver` must install the plugin archive `{target}`, or a package-manager user gets \
             a binary that cannot run `cutver plugin`: {url}"
        );
    }
    assert_eq!(sums.len(), targets.len(), "one digest per target: {sums:?}");
    for sum in &sums {
        assert!(
            sum.contains("${SHA_PLUGIN_"),
            "the digest must be the plugin archive's own sum; a lean one beside a plugin url fails \
             `brew install`: {sum}"
        );
    }
    assert!(
        brew.matches("grep \"cutver-plugins-${VERSION}-").count() == 4,
        "the sums come from the `cutver-plugins-` lines of SHA256SUMS: {brew}"
    );
    for kept in [
        "bin.install \"cutver\"",
        "conflicts_with \"cutver-plugins\"",
        "assert_match \"cutver #{version}\"",
    ] {
        assert!(
            brew.contains(kept),
            "the formula must keep `{kept}`: the asset changed, not the package"
        );
    }

    let scoop = run_script(step_containing(publish, "git add bucket/cutver.json"));
    let urls: Vec<&str> = scoop
        .lines()
        .filter(|line| line.contains("'url': ") && line.contains("download/${TAG}/"))
        .collect();
    assert_eq!(urls.len(), 2, "one url per architecture: {urls:?}");
    for (url, arch) in urls.iter().zip(["x86_64", "arm64"]) {
        assert!(
            url.contains(&format!("cutver-plugins-${{VERSION}}-windows-{arch}.zip")),
            "`scoop install cutver` must install the plugin-enabled archive: {url}"
        );
    }
    // `autoupdate` names the archive for a version it has not seen (`\$version`), so the release's
    // own directories are the ones with `${VERSION}` in them.
    let extracts: Vec<&str> = scoop
        .lines()
        .filter(|line| line.contains("'extract_dir': ") && line.contains("${VERSION}"))
        .collect();
    assert_eq!(extracts.len(), 2, "one extract_dir per architecture: {extracts:?}");
    for extract in extracts {
        assert!(
            extract.contains("cutver-plugins-${VERSION}-windows-"),
            "`extract_dir` is what points at the unpacked plugin archive: {extract}"
        );
    }
}

/// `cutver-plugins` is a shim for one release: same archive, same conflict, and it says what
/// replaces it in each manager's own vocabulary — Homebrew `deprecate!` with a date and a reason
/// naming `cutver`, Scoop a `notes` entry.
#[test]
fn the_deprecated_shim_still_resolves_and_names_what_replaces_it() {
    let workflow = repo_text(".github/workflows/release.yml");
    let publish = job(&workflow, "publish");

    let brew = run_script(step_containing(publish, "git add Formula/cutver-plugins.rb"));
    assert!(
        brew.contains("class CutverPlugins < Formula"),
        "the shim keeps its own class so `brew install cutver-plugins` still resolves: {brew}"
    );
    assert!(
        brew.matches("cutver-plugins-${VERSION}-").count() >= 4,
        "the shim must still install the same asset it always did: {brew}"
    );
    let deprecation = brew
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("deprecate!"))
        .unwrap_or_else(|| {
            panic!(
                "the shim must say it is going away in Homebrew's own vocabulary — `deprecate!` — \
                 or a formula removed without it reads as a broken tap: {brew}"
            )
        });
    let date = deprecation
        .split("date: \"")
        .nth(1)
        .and_then(|rest| rest.split('"').next());
    assert!(
        date.is_some_and(|date| date.len() == 10 && date.as_bytes()[4] == b'-' && date.as_bytes()[7] == b'-'),
        "`deprecate!` must carry an ISO date, the only form `brew` reads: {deprecation}"
    );
    assert!(
        deprecation
            .split("because: ")
            .nth(1)
            .is_some_and(|reason| reason.contains("cutver")),
        "the reason must point at `cutver`, which installs the same build: {deprecation}"
    );
    assert!(
        brew.contains("conflicts_with \"cutver\""),
        "both packages install the `cutver` binary, so the shim keeps the conflict: {brew}"
    );

    let scoop = run_script(step_containing(publish, "git add bucket/cutver-plugins.json"));
    let notes = scoop
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("'notes':"))
        .unwrap_or_else(|| panic!("the Scoop shim must carry a `notes` entry: {scoop}"));
    assert!(
        notes.contains("cutver") && notes.to_lowercase().contains("deprecat"),
        "the Scoop notes are the only place Scoop prints, so they must say deprecated and name \
         `cutver`: {notes}"
    );
    assert!(
        scoop.contains("'bin': 'cutver.exe'"),
        "the shim keeps installing the `cutver` binary, which is why it conflicts: {scoop}"
    );
}

/// S1/S3: the render step prefers a hand-written notes file for the tag being released and keeps the
/// template render as its fallback, in the same step.
///
/// The notes live at `.github/releases/<tag>.md`, so the path comes from the tag the bump step reported
/// and no convention has to be remembered. The template stays because the convention is optional: a
/// release with no notes file has to publish exactly the body it publishes today. A pin that only
/// checked the preferred path would let the fallback be deleted and would turn "optional with
/// fallback" into "required" — the next dispatch with no notes file would then copy an absent path
/// and publish nothing. Both halves are read from one step's code, because they are one decision.
#[test]
fn the_render_step_prefers_the_per_tag_notes_file_and_keeps_the_template_fallback() {
    const NOTES_NAME: &str = "notes=\".github/releases/${{ steps.cutver.outputs.tag }}.md\"";
    const PREFERRED: &str = "cp \"$notes\" RELEASE_NOTES.md";
    const TEMPLATE: &str = "cutver changelog latest --template .github/templates/cutver/RELEASE.md";
    let workflow = repo_text(".github/workflows/release.yml");
    let version = job(&workflow, "version");
    let step = code(step_containing(version, "name: Render the release notes"));

    assert!(
        step.contains(NOTES_NAME),
        "the render step must look for `.github/releases/<tag>.md` using the tag the bump step \
         reported: a fixed file name renders one release's notes for every release, and the file the \
         author committed is then read for one tag only: {step}"
    );
    assert!(
        step.contains("if [ -f \"$notes\" ]"),
        "the render step must test the notes file for existence before using it: copying an absent \
         path fails the step, so a dispatch with no notes file publishes no body at all: {step}"
    );
    let preferred = step.find(PREFERRED).unwrap_or_else(|| {
        panic!(
            "the render step must prefer the notes file when it exists: without it the notes are in \
             the tree, reviewed and never used, and the release ships the commit-level render anyway: {step}"
        )
    });
    let Some(template_line) = step.lines().find(|line| line.contains(TEMPLATE)) else {
        panic!(
            "the render step must keep `{TEMPLATE}` as its fallback, in this step: deleting it \
             turns an optional convention into a mandatory one, and a dispatch with no notes file \
             publishes no body: {step}"
        )
    };
    assert!(
        template_line.contains("> RELEASE_NOTES.md"),
        "the fallback must still write RELEASE_NOTES.md, which is the file the `release-notes` \
         artifact and `cutver publish` consume: {template_line}"
    );
    let template = step
        .find(TEMPLATE)
        .expect("the fallback was found on a line of this step");
    assert!(
        preferred < template,
        "the preference must come before the template render, as the branch the existence test falls \
         through to: a render first makes every hand-written file dead: {step}"
    );
}

/// S3a: the template render must be the `else` branch of the existence test, not a line beside it.
///
/// Moving the render to a line after the `fi` keeps every string
/// `the_render_step_prefers_the_per_tag_notes_file_and_keeps_the_template_fallback` looks for, passes
/// this suite, and makes the preference dead: the render then runs on every dispatch, overwrites the
/// notes the author committed and every release ships the commit-level body the convention exists to
/// replace. The branch is what S2 promises, so this pin reads the lines in the order the shell takes
/// them instead of asking whether each string exists somewhere in the step.
#[test]
fn the_fallback_render_is_the_else_branch_of_the_existence_test() {
    const EXISTS: &str = "if [ -f \"$notes\" ]";
    const TEMPLATE: &str = "cutver changelog latest --template .github/templates/cutver/RELEASE.md";
    let workflow = repo_text(".github/workflows/release.yml");
    let version = job(&workflow, "version");
    let step = code(step_containing(version, "name: Render the release notes"));

    let (_, branch, end) = branch_of(&step, EXISTS);
    let render = step
        .lines()
        .position(|line| line.contains(TEMPLATE))
        .unwrap_or_else(|| panic!("the render step must keep `{TEMPLATE}` as its fallback: {step}"));
    assert!(
        branch < render && render < end,
        "the template render must sit between the `else` (line {}) and the `fi` (line {}) of \
         `{EXISTS}`, and it is on line {}: as a neighbour of that block it renders on every dispatch, \
         overwrites the notes the author committed and makes the preference dead, so every release \
         ships the commit-level body while the notes file sits unread in the tree: {step}",
        branch + 1,
        end + 1,
        render + 1
    );
}

/// S3a: `RELEASE_NOTES.md` has exactly one writer in the whole workflow, and it is the render step.
///
/// The `release-notes` artifact carries that one file to `cutver publish`, which streams it as the
/// release body. A second writer — another step that redirects into the same path, or copies its own
/// source over it — means the body comes from two places and which one ships depends on the step
/// order, not on the file the author committed.
#[test]
fn only_the_render_step_writes_the_notes_file_the_artifact_carries() {
    const NOTES: &str = "RELEASE_NOTES.md";
    const REDIRECT: &str = "> RELEASE_NOTES.md";
    const COPY: &str = "cp \"$notes\" RELEASE_NOTES.md";

    /// The lines that write the notes file: a shell redirect into it, a copy that names it as the
    /// destination, or a `tee`. Reading it is not a write, and the artifact's `path:` and the `cat`
    /// in the publish job both legitimately name it.
    fn writers(text: &str) -> Vec<String> {
        text.lines()
            .map(str::trim)
            .filter(|line| {
                line.contains(NOTES)
                    && (line.contains(REDIRECT)
                        || line.contains(">RELEASE_NOTES.md")
                        || (line.starts_with("cp ") && line.ends_with(NOTES))
                        || line.contains("tee "))
            })
            .map(str::to_string)
            .collect()
    }

    let workflow = repo_text(".github/workflows/release.yml");
    let version = job(&workflow, "version");
    let step = code(step_containing(version, "name: Render the release notes"));
    let everywhere = writers(&code(&workflow));

    assert_eq!(
        everywhere.len(),
        2,
        "RELEASE_NOTES.md must have exactly two writes in the whole workflow — the redirect and the \
         copy that are the two halves of the preference: a third means another step writes the file \
         the `release-notes` artifact carries, so the release body comes from two places and the last \
         step to run decides which one ships: {everywhere:?}"
    );
    assert_eq!(
        code(&workflow).matches(REDIRECT).count(),
        1,
        "exactly one `{REDIRECT}` redirect must exist in release.yml: a second one renders some other \
         body over the notes the artifact carries"
    );
    assert_eq!(
        code(&workflow).matches(COPY).count(),
        1,
        "exactly one `{COPY}` must exist in release.yml: a second copy makes another source the \
         release body"
    );
    assert_eq!(
        writers(&step),
        everywhere,
        "both writes must live in the render step, not in a step beside it: the `release-notes` \
         artifact carries whatever that step wrote, so a writer elsewhere is a second author of the \
         release body: {step}"
    );
}

/// S2a: a notes file that exists but carries no text fails at the render step, not at the publish.
///
/// `cutver publish --notes-file -` refuses an empty or whitespace-only stream (#306), and that
/// refusal lands after the bump push and every build and smoke leg: nineteen minutes into a dispatch
/// that then fails at its last step with the branch already moved. Failing here costs one dispatch.
/// The guard is `grep -q '[^[:space:]]'` and not `-s`, because a file holding a single newline passes
/// `-s` and is exactly the file the closing phase refuses.
///
/// It fails rather than falling through to the render because an empty file is an authoring mistake —
/// someone created the file and meant to fill it — so quietly shipping the template body would hide
/// the mistake behind a body nobody chose.
#[test]
fn an_empty_notes_file_fails_at_the_render_step() {
    const EXISTS: &str = "if [ -f \"$notes\" ]";
    const GUARD: &str = "if ! grep -q '[^[:space:]]' \"$notes\"; then";
    const ERROR: &str = "::error::$notes is empty; write the notes or delete the file so the template renders.";
    const COPY: &str = "cp \"$notes\" RELEASE_NOTES.md";
    let workflow = repo_text(".github/workflows/release.yml");
    let version = job(&workflow, "version");
    let step = code(step_containing(version, "name: Render the release notes"));

    assert!(
        !step.contains("[ -s \"$notes\" ]"),
        "the emptiness check must reject whitespace, not just zero bytes: `-s` accepts a file holding \
         only a newline, the artifact carries it, and `cutver publish` refuses the stream at the end \
         of the release: {step}"
    );
    assert!(
        step.contains(GUARD),
        "the render step must test the notes file for content with `{GUARD}`: without it a file that \
         exists and carries nothing is copied to RELEASE_NOTES.md, and the release fails nineteen \
         minutes later inside `cutver publish` with the bump already pushed: {step}"
    );
    let exists = step.find(EXISTS).expect("the existence test");
    let guard = step.find(GUARD).expect("the guard was found on a line of this step");
    let copy = step.find(COPY).expect("the preference copy");
    assert!(
        exists < guard && guard < copy,
        "the emptiness guard must run after the file is found and before it is copied: after the copy \
         the artifact already carries the empty body, and outside the `if` it rejects the absent file \
         the fallback exists for: {step}"
    );
    assert!(
        step.contains(ERROR),
        "the guard must fail with `{ERROR}`: an exit without a line naming the file surfaces as a bare \
         step failure, and nothing says which notes file is empty: {step}"
    );
    assert!(
        step.contains("exit 1"),
        "the guard must fail the step, not warn: an `::error::` annotation on a step that goes on to \
         render would leave the run green with no body: {step}"
    );
}

/// S2a: a notes file written for another tag is named, so a mis-named one is visible.
///
/// `.github/releases/0.21.0.md` for `v0.21.0` is the mistake this watches for: the tag's file is
/// absent, the fallback renders, and without a word the release ships the commit-level body while
/// the notes the author wrote sit in the tree. Naming the files that are there is what makes the
/// mismatch readable in the log.
///
/// A repository that never uses notes files must stay silent: the scan has to tolerate an absent or
/// empty `.github/releases` with no error and no output, or every release of every project without
/// the convention carries a warning about a directory that is not meant to exist.
#[test]
fn a_notes_file_for_another_tag_is_named_instead_of_silently_ignored() {
    const EXISTS: &str = "if [ -f \"$notes\" ]";
    const SCAN: &str = "for other in .github/releases/*.md; do";
    const SKIP: &str = "[ -f \"$other\" ] || continue";
    const GUARDED: &str = "if [ -n \"$existing\" ]; then";
    const WARNING: &str = "::warning::no notes file for ${{ steps.cutver.outputs.tag }}; .github/releases has: ";
    const TRAILER: &str = ". Rendering the template instead.";
    let workflow = repo_text(".github/workflows/release.yml");
    let version = job(&workflow, "version");
    let step = code(step_containing(version, "name: Render the release notes"));

    assert!(
        step.contains(SCAN),
        "the fallback branch must scan `.github/releases/*.md` for notes files of other tags: \
         without the scan a mis-named file is silently replaced by the commit-level render: {step}"
    );
    assert!(
        step.contains(SKIP),
        "the scan must skip the unmatched glob with `{SKIP}`: an absent directory leaves the pattern \
         literal, and a step that then names `.github/releases/*.md` as a notes file warns about a \
         directory no project without the convention has: {step}"
    );
    assert!(
        step.contains(GUARDED),
        "the warning must be emitted only when the scan found a file, guarded by `{GUARDED}`: an \
         unguarded echo warns on every release of a repository that keeps no notes files, and a \
         warning that always fires is a warning nobody reads: {step}"
    );
    assert!(
        step.contains(WARNING) && step.contains(TRAILER),
        "the warning must read `{WARNING}<the files>{TRAILER}`: it has to name the tag that has no \
         notes and the files that exist, and say which body renders instead, or the reader cannot \
         tell a mis-named file from a release that has none: {step}"
    );
    let (_, branch, end) = branch_of(&step, EXISTS);
    let lines: Vec<&str> = step.lines().collect();
    let warning = lines
        .iter()
        .position(|line| line.contains(WARNING))
        .expect("the warning was found on a line of this step");
    assert!(
        branch < warning && warning < end,
        "the warning belongs to the fallback branch (lines {}..{}): emitted in the preference branch \
         it describes a release that is using its notes file, and emitted after the `fi` it fires on \
         runs that took no fallback: {step}",
        branch + 1,
        end + 1
    );
    assert!(
        lines.iter().position(|line| line.contains(SCAN)).expect("the scan")
            < lines
                .iter()
                .position(|line| line.contains(GUARDED))
                .expect("the list guard"),
        "the guard has to read a list the scan already filtered, or the pattern itself becomes a \
         `has:` entry: {step}"
    );
}

/// The plugin authenticates with whatever token reaches it, so that token decides who the release
/// is attributed to. `secrets.GITHUB_TOKEN` publishes as github-actions[bot], which is the
/// attribution regression issue #166 removed. Both jobs that hand the plugin a token mint it
/// themselves and fail closed without it: an optional token is how 0.16.0 shipped under the wrong
/// identity, and how the tag push once reached the ruleset unauthenticated.
#[test]
fn the_plugin_authenticates_with_the_app_token() {
    let workflow = repo_text(".github/workflows/release.yml");
    const MANDATORY: &str = "GITHUB_TOKEN: ${{ steps.release-token.outputs.token }}";
    for id in ["version", "publish"] {
        let slice = job(&workflow, id);
        assert!(
            slice.contains("id: release-token"),
            "the `{id}` job must mint the app token it hands the plugin"
        );
        assert!(
            slice.contains(MANDATORY),
            "the `{id}` job must hand the plugin `{MANDATORY}`, with no fallback"
        );
    }
    assert!(
        !workflow.contains("GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}"),
        "a bare secrets.GITHUB_TOKEN attributes the release to github-actions[bot]"
    );
}

/// The plugin sets the release title from `title_template` when it creates the draft. The
/// promotion used to pass its own `--title`, which is what made that option dead: the release
/// shipped as `Release v0.17.0` while the configuration asked for `cutver 0.17.0`.
#[test]
fn the_promotion_does_not_override_the_title() {
    let workflow = repo_text(".github/workflows/release.yml");
    assert!(
        !workflow.contains("--title \"Release"),
        "the promotion must not pass its own title: cutver.toml owns it through title_template"
    );
    assert!(
        repo_text("cutver.toml").contains("title_template"),
        "the title must be declared where the plugin reads it"
    );
}

/// Delivery (S1/S2): the bump push must authenticate as the app, inside the version job.
///
/// The ruleset requires `CI gate` on `main` and the bump commit is created in this same job, so the
/// push can never carry a check run: it has to authenticate as the one bypass actor, the
/// `cutver-release` app. The token is mandatory, not opportunistic: an empty token or a fallback to
/// `secrets.GITHUB_TOKEN` only moves the failure to the push, where the ruleset rejects it after the
/// tag has already landed. Scoped to the job, because a literal elsewhere in the file keeps the test
/// green while the mechanism is dead.
#[test]
fn the_version_job_checkout_persists_the_app_token_in_that_job() {
    const LITERAL: &str = "token: ${{ steps.release-token.outputs.token }}";
    let workflow = repo_text(".github/workflows/release.yml");
    let start = workflow.find("\n  version:\n").expect("the version job must exist");
    let end = workflow[start + 1..]
        .find("\n  build:\n")
        .map(|i| start + 1 + i)
        .expect("the build job marks the end of the version job");
    let job = &workflow[start..end];

    let token_at = job
        .find("id: release-token")
        .expect("the version job must mint the app token");
    let checkout_at = job
        .find("uses: actions/checkout@v7")
        .expect("the version job must check out");
    assert!(
        token_at < checkout_at,
        "the app token must be minted before the version job's checkout"
    );

    // The minting step must not be skippable or tolerated when it fails: a soft failure leaves the
    // token empty, which is how the push used to arrive at the ruleset unauthenticated.
    let token_step_start = job[..token_at]
        .rfind("\n      - ")
        .map(|i| i + 1)
        .expect("the token minting step must start with a step marker");
    let token_step_rest = &job[token_step_start..];
    let token_step = &token_step_rest[..token_step_rest.find("\n      - ").unwrap_or(token_step_rest.len())];
    assert!(
        token_step.contains("uses: actions/create-github-app-token"),
        "`id: release-token` must belong to the app-token step: on any other step the output is empty and the checkout silently falls back: {token_step}"
    );
    assert!(
        !token_step.contains("continue-on-error"),
        "the token step must fail the job when the app credentials are missing, not continue: {token_step}"
    );

    let after = &job[checkout_at..];
    let block = &after[..after.find("\n      - ").unwrap_or(after.len())];
    assert!(
        block.contains(LITERAL),
        "the version job's own checkout must pass `{LITERAL}`: {block}"
    );
    assert!(
        !block.contains("secrets.GITHUB_TOKEN"),
        "the version job's checkout must not fall back to secrets.GITHUB_TOKEN: the ruleset only lets the app push, so the fallback hides the failure until the push is rejected: {block}"
    );
    assert!(
        !block.contains("persist-credentials: false"),
        "`persist-credentials: false` removes the credential `git push origin main --tags` needs"
    );

    // The guard is what turns a late, confusing rejection into an early actionable error: with a
    // guard-less step an empty token reaches the checkout and the run fails only at the push, after
    // the commit exists and the tag has landed.
    let guard_at = job
        .find("name: Require the release bot token")
        .expect("the version job must guard the app token before it builds");
    let guard_rest = &job[guard_at..];
    let guard = &guard_rest[..guard_rest.find("\n      - ").unwrap_or(guard_rest.len())];
    assert!(
        guard.contains("exit 1"),
        "the guard must fail the job when the token is empty: {guard}"
    );
}

/// The install smoke runs the binary a user would download, so the action that installs it is pinned to
/// a commit: a floating major tag can be moved to code nobody reviewed, and that code runs with the
/// release token in this job (#322).
#[test]
fn the_install_smoke_pins_its_action_by_commit() {
    let workflow = repo_text(".github/workflows/release.yml");
    let line = workflow
        .lines()
        .find(|line| line.trim_start().starts_with("uses: cutver/setup@"))
        .expect("the workflow must install the CLI to smoke it");

    let reference = line
        .trim_start()
        .trim_start_matches("uses: cutver/setup@")
        .trim()
        .to_string();
    assert!(
        reference.len() == 40 && reference.chars().all(|c| c.is_ascii_hexdigit()),
        "`cutver/setup` must be pinned to a 40-character commit sha, not `{reference}`: a tag can move, \
         and this step installs the binary the rest of the job runs"
    );
}

/// #300 S4: the `Restore` and `Doctor` steps never invoke a plugin, so the `GITHUB_TOKEN` block on
/// them is a redundant, removable line. The `run:` line is what makes that true — neither command
/// interpolates a variable — while the bump step, which does invoke the plugin, keeps exporting the
/// declared grants.
#[test]
fn the_diagnostic_steps_name_no_token() {
    let workflow = repo_text(".github/workflows/release.yml");
    let version = job(&workflow, "version");
    for command in ["run: cutver plugin restore", "run: cutver doctor --check-changelog"] {
        let at = version
            .find(command)
            .unwrap_or_else(|| panic!("the version job must run `{command}`"));
        let end = version[at..].find('\n').map_or(version.len(), |offset| at + offset);
        assert!(
            !version[at..end].contains('$'),
            "`{command}` must not interpolate the token it is handed: a removable line is one the \
             command never reads"
        );
    }
}
