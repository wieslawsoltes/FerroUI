//! UI-independent ribbon algorithms.
mod enums;
pub use enums::*;
pub mod commands;
pub mod layout;
pub mod model;
pub mod text;
pub use text::RibbonString;
mod observable_object;
pub use observable_object::{ObservableObject, ObservableObjectImpl};
pub mod theming;

pub mod collections;

pub mod keytips;

pub mod search;

pub mod state;

pub mod localization;

pub mod customization_editor;

// Track package content changes even when restored files have old timestamps.
const _: Option<&str> = option_env!("FERRORIBBON_BUILD_FINGERPRINT");
