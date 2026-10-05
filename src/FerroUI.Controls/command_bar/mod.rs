//! A bar of primary commands shown inline and secondary commands shown in
//! an overflow menu, with the elements it is made of.

#[allow(clippy::module_inception)]
mod command_bar;
mod command_bar_button;
mod command_bar_default_label_position;
mod command_bar_overflow_button_visibility;
mod command_bar_separator;
mod command_bar_toggle_button;
mod i_command_bar_element;

pub use command_bar::{CommandBar, CommandBarElementCollection, CommandBarElementList};
pub use command_bar_button::CommandBarButton;
pub use command_bar_default_label_position::CommandBarDefaultLabelPosition;
pub use command_bar_overflow_button_visibility::CommandBarOverflowButtonVisibility;
pub use command_bar_separator::CommandBarSeparator;
pub use command_bar_toggle_button::CommandBarToggleButton;
pub use i_command_bar_element::ICommandBarElement;

#[cfg(test)]
mod command_bar_tests;
#[cfg(test)]
mod command_bar_tests_keyboard;
#[cfg(test)]
mod command_bar_tests_overflow;
