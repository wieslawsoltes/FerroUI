//! WinUI's commands that carry their user experience: `XamlUICommand`, `StandardUICommand` and
//! the `KeyboardAccelerator`s they describe (docs/porting/winui-controls.md, section 10.16).

mod commanding_helpers;
mod keyboard_accelerator;
mod standard_ui_command;
mod xaml_ui_command;

pub(crate) use commanding_helpers::{command_changed, UICommandSource};
pub use keyboard_accelerator::KeyboardAccelerator;
pub use standard_ui_command::{StandardUICommand, StandardUICommandKind};
pub use xaml_ui_command::{CanExecuteRequestedEventArgs, ExecuteRequestedEventArgs, XamlUICommand};

#[cfg(test)]
mod ui_command_tests;
