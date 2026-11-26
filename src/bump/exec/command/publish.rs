//! The commands and the pushes a publication run reports, out of `command.rs` to stay under the
//! source ceiling.
//!
//! Running a command belongs to `command.rs`; the shape a run reports about the commands and the
//! pushes it made lives here, because the tree, the CI sink and the machine object read the same
//! facts (#404 S1, #326 4A S2).

/// One `[publish] commands` entry a closing phase ran, or planned in a `--dry-run`.
///
/// `position` is the entry's 1-based place in declaration order, which is also the identity a
/// `publish-<n>` attestation names (#326 D4): a command line cannot be a gate name.
#[derive(Debug)]
pub struct PublishCommand {
    pub position: usize,
    pub command: String,
    /// The code the child exited with, or `None` for a dry run, which runs none.
    pub exit_code: Option<i32>,
}

/// The pushes a publication run made, kept per caller: `planned` is what a dry run reports (the
/// pipeline's summary names a command only when it did not run it), `performed` is what was really
/// sent, which is what `cutver publish` reports after it closed a release.
#[derive(Default)]
pub struct PushLog {
    pub planned: Vec<String>,
    pub performed: Vec<String>,
    /// The refspecs those performed pushes named, in the same order: the list a machine reads where
    /// `publish_push_command` joins the same pushes for a human (#404 S1).
    pub performed_refs: Vec<String>,
}

impl PushLog {
    /// Records one push from the argv `git` received, so the display form, the refspec list and their
    /// order all come from the one builder that sent it.
    pub fn record(&mut self, argv: &[String]) {
        self.performed.push(push_command(argv));
        if let Some(refspec) = argv.last() {
            self.performed_refs.push(refspec.clone());
        }
    }
}

/// The pushes joined for a human: `None` when the run made none.
pub fn joined(push_commands: &[String]) -> Option<String> {
    if push_commands.is_empty() {
        None
    } else {
        Some(push_commands.join(" && "))
    }
}

/// The command line one push argv names, in the same display form `git::ops::run_push` uses: the
/// summary must name exactly what `git` received, so the argv comes from the same builders.
fn push_command(argv: &[String]) -> String {
    format!("git {}", argv.join(" "))
}
