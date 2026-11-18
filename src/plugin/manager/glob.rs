//! Glob matching utilities for manifest pattern resolution.
//!
//! The matcher is one authority shared with `[[artifact]]` verification and `cutver plan`: it lives
//! in `crate::config::glob`, outside the `plugins` feature, so the lean build ships it too (#326 S3).

pub(crate) use crate::config::glob::glob_matches;
