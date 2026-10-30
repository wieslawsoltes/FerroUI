mod dock_command_bar_kind;
mod dock_command_bar_merge_mode;

pub use dock_command_bar_kind::*;
pub use dock_command_bar_merge_mode::*;

mod dock_command_bar_definition;
mod dock_command_bar_item;
mod i_dock_command_bar_provider;
pub use dock_command_bar_definition::DockCommandBarDefinition;
pub use dock_command_bar_item::DockCommandBarItem;
pub use i_dock_command_bar_provider::IDockCommandBarProvider;

#[cfg(test)]
mod model_tests;
