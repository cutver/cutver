//! The diagnostic switch one run selected, and the one thing it changes (#377).
//!
//! A `runtime = "process"` plugin's stderr is drained into memory and, on a call that exits 0, was
//! discarded: the natural way to debug a plugin is to log to stderr, so on a *working* call that
//! advice produced nothing and read as "my logging is broken". `--verbose` is the switch that turns
//! the drop into a forward, and its absence is the previous behaviour byte for byte.
//!
//! The state lives here rather than in `crate::cli` because the layer that reads it is the plugin
//! driver, and the plugin layer must not learn a command-line preference by importing the command
//! line (S1). The seam is the one `--ci` uses: `src/cli/runner/dispatch.rs` resolves the flag once,
//! before dispatch picks a command, and writes it here through [`select`]. Nothing is threaded
//! through `PluginManager::from_config` or `ProcessDriver::new`, which are public and wide-fanout,
//! and nothing here knows which command ran.

use std::io::Write;
use std::sync::OnceLock;

/// Whether this run asked for a process plugin's diagnostics. Written once by [`select`], read by
/// every invocation; one value is enough because one process runs one command.
static VERBOSE: OnceLock<bool> = OnceLock::new();

/// Records the diagnostic preference for this run, once, before dispatch picks a command.
pub fn select(verbose: bool) {
    let _ = VERBOSE.set(verbose);
}

/// Whether a successful invocation's child stderr reaches cutver's own stderr.
///
/// A run that never selected anything has the diagnostic switched off, which is the behaviour every
/// caller had before the flag existed.
fn enabled() -> bool {
    *VERBOSE.get_or_init(|| false)
}

/// Forwards a successful invocation's child stderr to cutver's own stderr, unchanged (S2).
///
/// The bytes arrive already drained and capped by [`crate::proc`] at
/// [`crate::plugin::driver::process::MAX_PLUGIN_STDERR_BYTES`], with the truncation marker that cut
/// already wrote into them (S4): this forwards that buffer. It does not read the child, cap it again
/// or drain a second time, and it invents nothing — no prefix, no trim, no rewrap — because the
/// bytes are the plugin author's diagnosis and the host is only carrying them.
///
/// A write that fails (a closed stderr, say) is dropped rather than raised: diagnostics are a log,
/// and a plugin that logs must never fail an otherwise valid call.
pub fn forward_child_stderr(stderr: &[u8]) {
    if stderr.is_empty() || !enabled() {
        return;
    }
    let mut out = std::io::stderr();
    let _ = out.write_all(stderr);
    let _ = out.flush();
}
