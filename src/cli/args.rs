mod commands;
mod normalize;
#[cfg(test)]
mod tests;

pub use commands::{BumpLevel, ChangelogCommands, Cli, Commands};
pub use normalize::normalize_args;
pub use plugin::{PluginAddArgs, PluginCommands};

mod plugin;
