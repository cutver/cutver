//! The two limits one invocation runs into, and the sequencing that keeps the diagnosis honest
//! (#369 S6, task T7).
//!
//! They live here rather than inside `process/tests.rs` for the reason `env_grants.rs` gives: that
//! file sits under the 400-line ratchet in `tests/source_line_budget.rs` and has three lines left.
//!
//! Every fixture is a child that reads stdin to EOF and then emits two byte streams the host built
//! in a scratch directory outside the worktree, so the sizes here are *exact*. That is the point: a
//! fixture that writes `"a".repeat(n)` cannot see an off-by-one at the limit, and an off-by-one at
//! the limit is the defect this file exists to catch.

use super::*;
use crate::plugin::driver::process::{MAX_PLUGIN_STDERR_BYTES, MAX_PLUGIN_STDOUT_BYTES};
use crate::plugin::dto::{PreBumpResponse, decode_answer};

/// A scratch directory outside the worktree, removed when the test that made it ends — including
/// when that test panics, which a `remove_dir_all` at the end of the body would not survive.
struct Scratch {
    dir: PathBuf,
}

impl Scratch {
    fn new(label: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("cutver-plugin-bounds-{}-{label}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("the scratch directory must be creatable");
        Self { dir }
    }

    fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.dir.join(name);
        std::fs::write(&path, bytes).expect("the fixture bytes must be writable");
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        // Test scratch only: the directory was created by `new` under the system temp directory.
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// What one fixture child emits, once it has read its envelope to EOF.
struct Fixture<'a> {
    answer: &'a [u8],
    diagnostics: &'a [u8],
    code: i32,
}

/// Builds the program that emits a [`Fixture`]: read stdin to EOF, write the bytes, exit `code`.
///
/// The bytes arrive as two files rather than as argv because a megabyte does not fit in an argument
/// list on either platform, and a shell loop that writes a megabyte one byte at a time is a slow
/// test. The program is only the sequencing, which is the part that must be real.
fn fixture(scratch: &Scratch, fixture: Fixture<'_>) -> (String, Vec<String>) {
    let answer = scratch.write("answer.bin", fixture.answer);
    let diagnostics = scratch.write("diagnostics.bin", fixture.diagnostics);
    let code = fixture.code;
    if cfg!(windows) {
        // `[Console]::Out.Write` bypasses the pipeline's encoder, which would rewrite newlines, and
        // `[IO.File]::ReadAllText` of ASCII bytes is byte-exact through any console code page.
        let script = format!(
            "$null = [Console]::In.ReadToEnd(); [Console]::Out.Write([IO.File]::ReadAllText('{}')); \
             [Console]::Error.Write([IO.File]::ReadAllText('{}')); exit {code}",
            answer.display(),
            diagnostics.display()
        );
        (
            "powershell".to_string(),
            vec![
                "-NoProfile".to_string(),
                "-NonInteractive".to_string(),
                "-Command".to_string(),
                script,
            ],
        )
    } else {
        let script = format!(
            "cat >/dev/null; cat '{}'; cat '{}' >&2; exit {code}",
            answer.display(),
            diagnostics.display()
        );
        ("sh".to_string(), vec!["-c".to_string(), script])
    }
}

/// A driver for one `lifecycle.v1` fixture, with the platform shell's own environment granted.
fn lifecycle_driver(program: (String, Vec<String>), timeout_seconds: u64) -> ProcessDriver {
    let (command, args) = program;
    ProcessDriver::new(
        PluginName::new("bounds-plugin").unwrap(),
        root(),
        command,
        args,
        shell_env(),
        timeout_seconds,
        HashSet::from([Capability::LifecycleV1]),
    )
}

fn pre_bump() -> PluginInvocation {
    PluginInvocation::new(PluginCall::PreBump, &serde_json::json!({})).unwrap()
}

/// A `lifecycle.v1` answer of exactly `total` bytes, so the limit can be hit on the byte.
///
/// `reason` is a real optional field of the answer, not a filler key, so the document is one the
/// host actually accepts at its declared size.
fn answer_of_size(total: usize) -> Vec<u8> {
    const PREFIX: &str = r#"{"allow":true,"reason":""#;
    const SUFFIX: &str = r#""}"#;
    let pad = total
        .checked_sub(PREFIX.len() + SUFFIX.len())
        .expect("the requested answer must be large enough to hold the required fields");
    let mut answer = Vec::with_capacity(total);
    answer.extend_from_slice(PREFIX.as_bytes());
    answer.extend(std::iter::repeat_n(b'a', pad));
    answer.extend_from_slice(SUFFIX.as_bytes());
    assert_eq!(answer.len(), total);
    answer
}

/// Diagnostics that open with `head` and then run past the limit in one uniform filler byte.
///
/// The head is the point: with uniform filler alone, keeping the first `MAX_PLUGIN_STDERR_BYTES`
/// bytes and keeping the last is the very same byte string, so a test built on it cannot tell the two
/// apart — which is the defect these fixtures exist to close.
fn diagnostics_over_the_limit(head: &[u8], filler: u8) -> Vec<u8> {
    let mut diagnostics = head.to_vec();
    diagnostics.extend(std::iter::repeat_n(filler, MAX_PLUGIN_STDERR_BYTES + 512 * 1024));
    diagnostics
}

/// The marker the drain appends after the kept head, byte for byte — numbers included, so a marker
/// naming the wrong limit, or a wrong dropped count, fails the test that reads it.
fn truncation_marker(diagnostics_len: usize) -> String {
    let dropped = diagnostics_len - MAX_PLUGIN_STDERR_BYTES;
    format!("\n[cutver] stderr truncated at {MAX_PLUGIN_STDERR_BYTES} bytes; {dropped} more were dropped.\n")
}

/// Runs one fixture through the `proc` seam, where the kept stderr is observable at all: the success
/// path drops stderr by decision (S7, `docs/guides/plugin-process-authoring.md`).
fn capture_via_proc(driver: &ProcessDriver, invocation: &PluginInvocation) -> proc::Capture {
    let mut command = driver
        .child_command()
        .expect("a driver with no declared grant has nothing to reject");
    let envelope = encode_invocation(driver.name(), invocation).expect("the request encodes");
    proc::run_capture(&mut command, &envelope, Duration::from_secs(30), capture_limits())
        .unwrap_or_else(|error| panic!("a loud stderr is not a failure: {}", error.into_io()))
}

/// S6: an answer past the limit is a **typed** error, and it arrives on the read rather than on the
/// deadline.
///
/// The second half is the sequencing rule, and it is the whole reason the drain keeps reading after
/// the limit: a drain that stopped there would leave the child blocked on a full pipe, `supervise`
/// would reach `timeout_seconds`, and the author would be told to raise a timeout that was never the
/// problem. `30` seconds of budget against `15` of wall clock separates the two diagnoses.
#[test]
fn an_answer_past_the_stdout_limit_is_a_typed_overrun_inside_the_budget() {
    let scratch = Scratch::new("stdout-overrun");
    // Twice the limit: past the pipe buffer by a wide margin, so a drain that truncated silently
    // instead of erroring would still be caught here.
    let answer = vec![b'a'; MAX_PLUGIN_STDOUT_BYTES * 2];
    let program = fixture(
        &scratch,
        Fixture {
            answer: &answer,
            diagnostics: b"",
            code: 0,
        },
    );
    let driver = lifecycle_driver(program, 30);

    let start = Instant::now();
    // Not `expect_err`: on the `Ok` side that would format two megabytes into the failure message.
    let error = match driver.invoke(&pre_bump()) {
        Ok(output) => panic!(
            "an answer past the limit must not be accepted; {} bytes came back",
            output.len()
        ),
        Err(error) => error,
    };
    let elapsed = start.elapsed();

    let message = match &error {
        PluginError::AnswerTooLarge {
            name,
            capability,
            limit_bytes,
        } => {
            assert_eq!(name, driver.name());
            assert_eq!(capability, "lifecycle.v1");
            assert_eq!(
                *limit_bytes, MAX_PLUGIN_STDOUT_BYTES,
                "the error names the limit that was crossed"
            );
            error.to_string()
        }
        other => panic!("a plugin that writes past the stdout limit is an overrun, not {other:?}"),
    };

    assert!(
        message.contains("Where:") && message.contains("Fix:"),
        "the house style carries a capitalized Where: and Fix:: {message}"
    );
    assert!(
        message.contains(&MAX_PLUGIN_STDOUT_BYTES.to_string()),
        "the message names the limit in bytes: {message}"
    );
    assert!(
        elapsed < Duration::from_secs(15),
        "the overrun must be diagnosed on the read, not on the 30s deadline; took {elapsed:?}"
    );
}

/// S6: the limit is a limit, not a fence one byte early. An answer of exactly the limit, and one of
/// the limit minus one, is read and decoded — an off-by-one here would refuse a legal answer.
#[test]
fn an_answer_at_and_just_under_the_stdout_limit_still_decodes() {
    for total in [MAX_PLUGIN_STDOUT_BYTES - 1, MAX_PLUGIN_STDOUT_BYTES] {
        let scratch = Scratch::new(&format!("stdout-at-limit-{total}"));
        let answer = answer_of_size(total);
        let program = fixture(
            &scratch,
            Fixture {
                answer: &answer,
                diagnostics: b"",
                code: 0,
            },
        );
        let driver = lifecycle_driver(program, 30);

        let output = driver
            .invoke(&pre_bump())
            .unwrap_or_else(|error| panic!("an answer of {total} bytes is inside the limit: {error}"));
        assert_eq!(output.len(), total, "the whole answer is read, not a prefix of it");
        let decoded: PreBumpResponse = decode_answer(driver.name(), "lifecycle.v1", &output)
            .unwrap_or_else(|error| panic!("an answer of {total} bytes must still decode: {error}"));
        assert!(decoded.allow, "the decoded answer is the one the plugin wrote");
    }
}

/// S6: diagnostics are a log, so a loud plugin must not fail a call that answered correctly — and
/// what the host keeps is the **first** bytes plus a marker, not the last.
///
/// The kept bytes are read through `run_capture` on purpose: the success path drops stderr by
/// decision (S7, `docs/guides/plugin-process-authoring.md`), so the capture seam is the only place
/// they can be observed at all. The fixture opens with `HEAD-` so that keeping the head and keeping
/// the tail are different byte strings; the marker is then read exactly, numbers included.
#[test]
fn loud_diagnostics_do_not_fail_a_successful_call_and_keep_the_head() {
    let scratch = Scratch::new("stderr-success");
    let diagnostics = diagnostics_over_the_limit(b"HEAD-\n", b'T');
    let answer = br#"{"allow":true}"#;
    let program = fixture(
        &scratch,
        Fixture {
            answer,
            diagnostics: &diagnostics,
            code: 0,
        },
    );
    let driver = lifecycle_driver(program, 30);
    let invocation = pre_bump();

    let output = driver
        .invoke(&invocation)
        .expect("a loud plugin that answers and exits 0 still succeeds");
    assert_eq!(output, answer);

    let capture = capture_via_proc(&driver, &invocation);
    assert!(capture.status.success(), "the fixture exits 0");

    let kept = String::from_utf8_lossy(&capture.stderr);
    assert!(
        capture.stderr.starts_with(b"HEAD-"),
        "the first {MAX_PLUGIN_STDERR_BYTES} bytes of stderr are the ones kept, so the opening line
         survives; the capture opens with {:?}",
        kept.chars().take(16).collect::<String>()
    );
    assert!(
        !kept.ends_with('T'),
        "the last bytes are the ones dropped, so the capture cannot end in the filler"
    );
    let marker = truncation_marker(diagnostics.len());
    assert!(
        capture.stderr.windows(2).any(|window| window == b"\n["),
        "the marker starts a line of its own, exactly where the head was cut; kept {} bytes",
        kept.len()
    );
    assert!(
        kept.ends_with(&marker),
        "the marker is appended after the kept head and names the limit and the dropped count;
         expected {marker:?}"
    );
}

/// S6, S7: on a non-zero exit the truncated diagnostics are what the error carries. Empty stderr
/// would send the author looking for a plugin that never wrote anything — and the part carried is
/// the head, which is where a traceback starts, not the tail where it stops.
#[test]
fn loud_diagnostics_on_a_failing_call_still_reach_the_error() {
    let scratch = Scratch::new("stderr-failure");
    let diagnostics = diagnostics_over_the_limit(b"HEAD-\n", b'T');
    let program = fixture(
        &scratch,
        Fixture {
            answer: b"",
            diagnostics: &diagnostics,
            code: 7,
        },
    );
    let driver = lifecycle_driver(program, 30);

    let error = driver.invoke(&pre_bump()).expect_err("a non-zero exit fails the call");
    let (exit_code, stderr) = match error {
        PluginError::ExecutionFailed { exit_code, stderr, .. } => (exit_code, stderr),
        other => panic!("expected ExecutionFailed, got {other:?}"),
    };

    assert_eq!(exit_code, 7);
    assert!(
        stderr.starts_with("HEAD-"),
        "the error carries the first {MAX_PLUGIN_STDERR_BYTES} bytes of stderr, so the opening line survives; it carried {:?} of {} bytes",
        stderr.chars().take(16).collect::<String>(),
        stderr.len()
    );
    assert!(
        stderr.len() > MAX_PLUGIN_STDERR_BYTES,
        "the marker follows the kept head, so the cut is visible to the reader"
    );
    // `ExecutionFailed` carries the stream trimmed, so the marker's closing newline is the one byte lost.
    let marker = truncation_marker(diagnostics.len());
    assert!(
        stderr.ends_with(marker.trim_end()),
        "the marker names the limit and the dropped count after the kept head; expected {marker:?}"
    );
    assert!(stderr.contains("truncated"), "the truncated text says it was truncated");
}

/// S6: the stderr limit is a limit, not a fence one byte early, and a stream that stops on it lost
/// nothing — a marker there would report a cut that never happened.
#[test]
fn diagnostics_of_exactly_the_limit_are_kept_whole_and_unmarked() {
    let scratch = Scratch::new("stderr-at-limit");
    let mut diagnostics = b"HEAD-\n".to_vec();
    let pad = MAX_PLUGIN_STDERR_BYTES - diagnostics.len();
    diagnostics.extend(std::iter::repeat_n(b'T', pad));
    assert_eq!(diagnostics.len(), MAX_PLUGIN_STDERR_BYTES);

    let answer = br#"{"allow":true}"#;
    let program = fixture(
        &scratch,
        Fixture {
            answer,
            diagnostics: &diagnostics,
            code: 0,
        },
    );
    let driver = lifecycle_driver(program, 30);
    let invocation = pre_bump();
    driver
        .invoke(&invocation)
        .expect("diagnostics at the limit are not a failure");

    let capture = capture_via_proc(&driver, &invocation);
    assert_eq!(
        capture.stderr.len(),
        MAX_PLUGIN_STDERR_BYTES,
        "the stream is kept whole: every byte of it is inside the limit"
    );
    assert_eq!(capture.stderr, diagnostics, "the bytes are the ones the plugin wrote");
    assert!(
        !String::from_utf8_lossy(&capture.stderr).contains("truncated"),
        "nothing was dropped, so no marker claims otherwise"
    );
}

/// PRESERVE: the ordinary exchange is unchanged — one small answer, exit 0, decoded by the host.
#[test]
fn a_small_answer_still_succeeds() {
    let scratch = Scratch::new("small-answer");
    let answer = br#"{"allow":true}"#;
    let program = fixture(
        &scratch,
        Fixture {
            answer,
            diagnostics: b"",
            code: 0,
        },
    );
    let driver = lifecycle_driver(program, 30);

    let output = driver.invoke(&pre_bump()).expect("a small answer is the ordinary case");
    assert_eq!(output, answer);
    let decoded: PreBumpResponse = decode_answer(driver.name(), "lifecycle.v1", &output).expect("it decodes");
    assert!(decoded.allow);
}
