//! The render clock is a value, not a second reading of the wall clock.
//!
//! Pillar I.1: identical inputs must produce identical bytes. The date reaches the render core
//! through `ReleaseContext`; when the core re-derives `SystemTime::now()` the heading and the
//! template body can disagree as soon as the two reads straddle local midnight.

use cutver::changelog::{ReleaseContext, compute_update, render_template};

/// A clock injected as a value: one date, read as many times as the render needs.
///
/// A real clock read at 23:59:59 for the heading and at 00:00:01 for the body would produce two
/// different dates in one section. An injected date cannot: that is exactly the property under test.
fn midnight_straddling_clock() -> ReleaseContext {
    ReleaseContext {
        version: "v1.0.0".to_string(),
        tag: "v1.0.0".to_string(),
        date: "1999-12-31".to_string(),
        year: 1999,
        month: 12,
        day: 31,
        ..Default::default()
    }
}

fn render(ctx: &ReleaseContext) -> String {
    let body = render_template("{{date}}", ctx).expect("the body template renders");
    compute_update("# Changelog\n", "v1.0.0", &body, false, None, Some(ctx))
        .expect("the changelog update computes")
        .expect("a new version prepends a section")
}

#[test]
fn the_same_injected_clock_renders_identical_bytes_with_one_date() {
    let ctx = midnight_straddling_clock();

    let first = render(&ctx);
    let second = render(&ctx);

    assert_eq!(
        first, second,
        "the same inputs and the same clock must render identical bytes"
    );
    assert!(
        first.contains("## [v1.0.0] - 1999-12-31\n"),
        "the heading must use the injected date:\n{first}"
    );
    assert_eq!(
        first.matches("1999-12-31").count(),
        2,
        "the heading and the `{{{{date}}}}` body must carry the same injected date:\n{first}"
    );
}
