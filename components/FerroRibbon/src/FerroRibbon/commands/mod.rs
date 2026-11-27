//! Framework culture bridge for the UI-independent keyboard gesture parser.
pub use ferroribbon_core::commands::{RibbonKeyGesture, RibbonModifierKeys};

mod culture_bridge;
pub(crate) use culture_bridge::install_culture_provider;

mod native_command;
pub(crate) use native_command::register_conversions;
pub use native_command::{adapt_core_command, command_parameter, source_core_command};

mod async_executor;
pub use async_executor::dispatcher_command_executor;
