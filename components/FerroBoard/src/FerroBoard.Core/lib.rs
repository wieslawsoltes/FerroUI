//! UI-independent typing engine port. See docs/ports.json for current coverage.
pub mod clipboard;
pub mod editor;
pub mod emoji;
pub mod engine;
pub mod events;
pub mod glide;
pub mod input;
pub mod keys;
pub mod layouts;
pub mod resources;
pub mod serialization;
pub mod settings;
pub mod text;

#[cfg(test)]
#[path = "../../tests/support/fixture_clock.rs"]
mod fixture_clock;
