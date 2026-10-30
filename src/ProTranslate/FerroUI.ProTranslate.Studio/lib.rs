//! Portable catalog editing, persistence and translation assistance.
//!
//! Hosts own storage locations, time, credentials, scheduling and network transport.
//! This engine never changes global culture or starts background work implicitly.

mod ai;
mod document;
mod error;
mod format_helper;
mod gateway;
mod persistence;
mod settings;
mod suggestions;

pub use ai::*;
pub use document::*;
pub use error::*;
pub use format_helper::*;
pub use gateway::*;
pub use persistence::*;
pub use settings::*;
pub use suggestions::*;

pub use ferroui_protranslate_formats as formats;

#[cfg(test)]
mod tests;
