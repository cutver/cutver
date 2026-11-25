//! S2, S3, S4 (#377): a `runtime = "process"` plugin's stderr has a switch, not a workaround.
//!
//! The driver drains the child's stderr into an already-capped buffer and, on a call that exits 0,
//! threw it away: the natural way to debug a plugin is to log to stderr, so on a *working* call that
//! advice produced nothing and read as "my logging is broken". `--verbose` is the switch that turns
//! the drop into a forward, and its absence is the previous behaviour byte for byte.
//!
//! Why this suite spawns the binary instead of calling the driver in-process: there is no way to
//! capture *this* process's own stderr on stable Rust without a new dependency or an `unsafe`,
//! process-global `dup2` that would also redirect every concurrently scheduled test. The observable
//! claim here is "the child's bytes reach cutver's stderr", so the test reads the stderr of a real
//! `cutver` process — the same door a user looks through.
//!
//! The fixture plugin is written for both platforms rather than gated to one: a `.cmd` batch file is
//! executable on Windows without a shell prefix (Rust's `Command` runs it), and a `chmod +x` POSIX
//! script is executable on the rest, so the assertion runs wherever the suite runs.

mod common;

use common::{FixtureGuard, cutver_command};
use std::path::Path;
use std::process::Output;

use cutver::plugin::driver::process::MAX_PLUGIN_STDERR_BYTES;

/// The line the fixture plugin writes to stderr when it succeeds.
const MARKER: &str = "cutver-verbose-probe: a working plugin logging to stderr";

/// The line it writes when it fails, so the failure pin is not vacuous.
const FAILURE_MARKER: &str = "cutver-verbose-probe: a broken plugin logging to stderr";

/// A plugin that drains its envelope, counts the call, logs to stderr and answers on stdout, exiting
/// 0. The counter is what makes "the host forwarded each invocation's stderr" provable, and what
/// proves the fixture really reaches the plugin at all, rather than pinning how many reads the
/// command happens to make.
const PLUGIN_UNIX: &str = r#"#!/bin/sh
cat >/dev/null
printf 'run\n' >> calls.log
printf 'cutver-verbose-probe: a working plugin logging to stderr\n' >&2
printf '{"version": "1.2.3"}'
"#;

/// The same plugin for Windows, where stderr reaches the host through the file handle and the batch
/// interpreter's own CRLF: the bytes are compared against exactly what the child wrote.
const PLUGIN_WINDOWS: &str = "@echo off\r\nfindstr . >nul\r\necho run>>calls.log\r\necho cutver-verbose-probe: a working plugin logging to stderr>&2\r\necho {\"version\": \"1.2.3\"}\r\n";

/// A plugin that fails after logging, which is the path S3 leaves untouched.
const FAILING_UNIX: &str = r#"#!/bin/sh
cat >/dev/null
printf 'cutver-verbose-probe: a broken plugin logging to stderr\n' >&2
exit 1
"#;

const FAILING_WINDOWS: &str =
    "@echo off\r\nfindstr . >nul\r\necho cutver-verbose-probe: a broken plugin logging to stderr>&2\r\nexit /b 1\r\n";

/// The exact bytes the successful child wrote to stderr on this platform.
fn expected_stderr() -> String {
    let terminator = if cfg!(windows) { "\r\n" } else { "\n" };
    format!("{MARKER}{terminator}")
}

fn script_name() -> &'static str {
    if cfg!(windows) { "probe.cmd" } else { "probe.sh" }
}

/// A workspace whose only manifest is read through a `runtime = "process"` plugin, so
/// `cutver doctor` performs one real invocation whose outcome the flag can change.
fn workspace(name: &str, script: &str) -> FixtureGuard {
    let guard = FixtureGuard::new(name);
    let fixture = guard.fixture();

    let path = fixture.dir.join(script_name());
    fixture.write(script_name(), script);
    make_executable(&path);

    let command = path.to_string_lossy().replace('\\', "/");
    fixture.write("Chart.yaml", "apiVersion: v2\nname: probe\nversion: 0.0.0\n");
    fixture.write(
        "cutver.toml",
        &format!(
            r#"[version]
current_source = "Chart.yaml"

[[manifest]]
path = "Chart.yaml"
kind = "plugin"

[plugins.probe]
runtime = "process"
command = "{command}"
capabilities = ["manifest.v1"]
manifest_match = ["Chart.yaml"]

[plugins.probe.permissions]
env = ["PATH"]
"#
        ),
    );
    guard
}

/// Marks the POSIX fixture executable. A batch file needs no mode bit.
fn make_executable(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(path)
            .expect("the fixture script exists")
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(path, perms).expect("the fixture script is executable");
    }
    #[cfg(not(unix))]
    let _ = path;
}

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

/// How many times the fixture plugin really ran: the child appends one line per invocation, in the
/// workspace root it is started in. A command is free to read a manifest more than once, and this
/// test is about what the host does with each invocation's stderr, not about how many there are.
fn invocations(guard: &FixtureGuard) -> usize {
    let log = guard.fixture().dir.join("calls.log");
    std::fs::read_to_string(&log)
        .unwrap_or_else(|err| {
            panic!(
                "the fixture plugin never ran ({} was not written): {err}",
                log.display()
            )
        })
        .lines()
        .count()
}

/// S3, the previous-behaviour pin: without the flag a successful call stays silent on stderr, which
/// is exactly what the driver did before the switch existed.
#[test]
fn a_successful_call_writes_nothing_to_stderr_without_the_flag() {
    let guard = workspace(
        "stderr-verbose-off",
        if cfg!(windows) { PLUGIN_WINDOWS } else { PLUGIN_UNIX },
    );

    let output = run(&guard, &["doctor"]);

    assert!(
        output.status.success(),
        "--verbose is what is missing here, nothing else: stdout={} stderr={}",
        stdout(&output),
        stderr(&output)
    );
    assert!(
        invocations(&guard) > 0,
        "the fixture must really invoke the plugin, or the silence below proves nothing"
    );
    assert_eq!(
        stderr(&output),
        "",
        "a successful call keeps the drained stderr to itself unless the flag asks for it"
    );
}

/// S2, S4: the bytes the child already wrote reach cutver's stderr, unchanged and in order, with no
/// read, no second cap and no prefix invented.
#[test]
fn the_flag_forwards_a_successful_calls_stderr_unchanged() {
    let guard = workspace(
        "stderr-verbose-on",
        if cfg!(windows) { PLUGIN_WINDOWS } else { PLUGIN_UNIX },
    );

    let silent = run(&guard, &["doctor"]);
    let silent_calls = invocations(&guard);
    let verbose = run(&guard, &["doctor", "--verbose"]);
    let verbose_calls = invocations(&guard) - silent_calls;

    assert!(
        verbose.status.success(),
        "the call itself is unchanged by the flag: stdout={} stderr={}",
        stdout(&verbose),
        stderr(&verbose)
    );
    assert_eq!(
        stdout(&verbose),
        stdout(&silent),
        "the switch moves diagnostics only: stdout is byte for byte what it was without it"
    );
    assert!(silent_calls > 0, "the fixture must really invoke the plugin");
    assert!(verbose_calls > 0, "and it must do so in the run that set the flag");
    assert_eq!(
        stderr(&verbose),
        expected_stderr().repeat(verbose_calls),
        "every successful invocation's stderr reaches cutver's own stderr unchanged — no prefix, no \
         trim, no rewrap — and the run with the flag is the one that put those bytes there"
    );
    assert_eq!(
        stderr(&silent),
        "",
        "and it is the flag, not the call, that put those bytes on stderr"
    );
}

/// S3: the failure path is out of scope and stays byte identical with the switch set. The marker is
/// already in the error message, so the equality is not two empty strings agreeing.
#[test]
fn the_flag_leaves_the_failure_path_byte_identical() {
    let guard = workspace(
        "stderr-verbose-failure",
        if cfg!(windows) { FAILING_WINDOWS } else { FAILING_UNIX },
    );

    let silent = run(&guard, &["doctor"]);
    let verbose = run(&guard, &["doctor", "--verbose"]);

    assert!(!silent.status.success(), "the fixture plugin fails on purpose");
    assert_eq!(
        verbose.status.code(),
        silent.status.code(),
        "the flag does not move the exit code of a failed call"
    );
    assert_eq!(
        stdout(&verbose),
        stdout(&silent),
        "the flag does not touch stdout on the failure path either"
    );
    assert!(
        stderr(&silent).contains(FAILURE_MARKER),
        "the child's diagnosis is already quoted by the failure, so this pin is not vacuous: {}",
        stderr(&silent)
    );
    assert_eq!(
        stderr(&verbose),
        stderr(&silent),
        "on a failed call the child's stderr is the error message, never a second copy of it"
    );
}

/// S4: the forward is of the buffer the drain already capped, so a plugin past the 64 KiB cap
/// forwards the head plus the truncation marker, and still never fails an otherwise valid call.
#[test]
fn the_forward_is_the_capped_buffer_with_its_truncation_marker() {
    let loud = if cfg!(windows) {
        "@echo off\r\nfindstr . >nul\r\necho run>>calls.log\r\nfor /L %%i in (1,1,1400) do echo cutver-verbose-probe: a working plugin logging to stderr>&2\r\necho {\"version\": \"1.2.3\"}\r\n"
    } else {
        "#!/bin/sh\ncat >/dev/null\nprintf 'run\\n' >> calls.log\ni=0\nwhile [ $i -lt 1400 ]; do printf 'cutver-verbose-probe: a working plugin logging to stderr\\n' >&2; i=$((i+1)); done\nprintf '{\"version\": \"1.2.3\"}'\n"
    };
    let guard = workspace("stderr-verbose-loud", loud);

    let verbose = run(&guard, &["doctor", "--verbose"]);

    assert!(
        verbose.status.success(),
        "a loud plugin must never fail an otherwise valid call: stdout={} stderr={}",
        stdout(&verbose),
        stderr(&verbose)
    );
    let forwarded = stderr(&verbose);
    let calls = invocations(&guard);
    assert!(
        forwarded.starts_with(MARKER),
        "the beginning of the message is the half that is kept: {forwarded:?}"
    );
    let marker = format!("[cutver] stderr truncated at {MAX_PLUGIN_STDERR_BYTES} bytes");
    assert_eq!(
        forwarded.matches(&marker).count(),
        calls,
        "each forward carries the marker the drain already wrote, so the buffer is the capped one: \
         {forwarded:?}"
    );
    assert!(
        forwarded.len() < calls * (MAX_PLUGIN_STDERR_BYTES + 200),
        "the forward is the already-capped buffer, not a second drain or a re-read: {} bytes over \
         {calls} calls",
        forwarded.len()
    );
}
