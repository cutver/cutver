//! Central facade managing plugin drivers and configuration orchestration.
//!
//! Follows the canonical Facade pattern of CONTRACT.md Pillar V.4:
//! `manager.rs` re-exports the public interface from focused sibling submodules
//! under `manager/` with zero legacy `mod.rs`.

mod core;
mod factory;
mod glob;
#[cfg(test)]
mod tests;

pub use core::PluginManager;
