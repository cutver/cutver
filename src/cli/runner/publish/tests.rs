//! The `Resume` section rendered from the summary the closing phase returned (#326 D2).
//!
//! The runner used to answer "which plugins did this resume leave alone?" by loading a second
//! `PluginManager` and asking the same question the dispatch had already answered. The names travel
//! in `PublishSummary` now, so what these units pin is that the wording did not move with them.

use super::left_alone_line;

#[test]
fn a_resume_that_left_no_plugin_alone_names_the_absence_of_the_event() {
    assert_eq!(
        left_alone_line(&[]),
        "`on_post_release` not dispatched: this workspace declares no plugin for that event"
    );
}

#[test]
fn a_resume_names_the_plugins_it_left_alone_in_a_stable_order() {
    let line = left_alone_line(&["release".to_string(), "announce".to_string()]);

    assert!(
        line.starts_with("`on_post_release` not dispatched: `announce`, `release` left alone"),
        "the names are sorted and backticked: {line}"
    );
    assert!(
        line.ends_with("(`cutver publish` without `--resume` dispatches it)"),
        "the line still says how to dispatch them: {line}"
    );
}
