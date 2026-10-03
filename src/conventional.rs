mod deduce;
mod parse;
mod types;

#[cfg(test)]
mod tests;

pub use deduce::{deduce_bump, deduce_rationale, parse_and_deduce_bump, parse_and_deduce_with_rationale};
pub use types::{BumpRationale, ConventionalCommit};
