//! Native FerroUI control-layer port; incomplete surfaces are tracked in docs/ports.json.
mod callback_tokens;
pub mod icons;
pub mod input;
pub mod items_source;
mod keyboard_strings;
pub mod primitives;
pub mod services;
#[doc(hidden)]
pub use services::EmojiFontFallback;
pub use services::{EmojiFont, EmojiFontError};
pub mod themes;
pub use keyboard_strings::*;
pub(crate) mod views;

mod enum_bridge;
#[cfg(test)]
#[path = "tests/support/headless_session.rs"]
pub(crate) mod test_support;

pub use views::{SmartbarAction, SmartbarExecute, SmartbarIsActive};
mod smartbar_action_metadata;
#[doc(hidden)]
pub use smartbar_action_metadata::{
    SmartbarActionMetadata, SmartbarActionRef, SmartbarActionsMetadata,
};
mod on_screen_keyboard;
pub use on_screen_keyboard::*;
mod screen_keyboard_host;
pub use screen_keyboard_host::*;

#[cfg(test)]
#[allow(dead_code)]
#[path = "../FerroBoard.Core/tests/support/scheduler.rs"]
pub(crate) mod test_scheduler;
#[cfg(test)]
#[path = "tests/support/view_fixture.rs"]
pub(crate) mod view_fixture;

mod native_markup;
pub use native_markup::{ASSEMBLY, register_types};
