//! Where a deprecation becomes visible (#272).
//!
//! One sentence, two readers: a `warning: …` line on stderr for whoever reads the log, and the same
//! sentence as a `::warning::` annotation for the runner that reads the command stream. stdout is
//! never the channel, in `--json` and without it, because stdout carries the payload.
//!
//! A deprecation is not a failure. The command keeps working and the exit code does not move; what
//! changes is that a spelling the contract has deprecated stops being silent, which is the whole
//! obligation `CONTRACT.md` Pillar IV.2 and Pillar VII.2 put on a rename.

use std::sync::Mutex;

use crate::cli::{ci, style};
use crate::config::Deprecation;

/// The sentences this process already printed, so a command that loads the configuration twice does
/// not warn twice about the same spelling.
static EMITTED: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Reports every deprecated spelling a document carried, in table order.
pub fn deprecations(found: &[&'static Deprecation]) {
    for deprecation in found {
        emit(&deprecation.message());
    }
}

/// Prints one deprecation on both channels, once per process.
fn emit(message: &str) {
    // A poisoned lock means another thread panicked while reporting. Losing the dedupe is better
    // than losing the warning, so a poisoned lock lets the sentence through.
    if let Ok(mut emitted) = EMITTED.lock() {
        if emitted.iter().any(|seen| seen == message) {
            return;
        }
        emitted.push(message.to_string());
    }
    style::warn_degradation(message);
    ci::annotate_warning(message);
}
