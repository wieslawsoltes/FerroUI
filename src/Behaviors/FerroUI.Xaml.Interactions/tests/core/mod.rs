//! The tests of `Core/` of the reference, and its support files (`Command.cs`,
//! `ObservableCommand.cs`, `TestValueConverter.cs`).

pub(crate) mod command;
pub(crate) mod observable_command;
pub(crate) mod test_value_converter;

mod action_logical_tree_lifetime_tests;
mod call_method_action_tests;
mod change_property_action_tests;
mod data_trigger_behavior_tests;
mod event_trigger_behavior_tests;
mod invoke_command_action_tests;
mod multi_data_trigger_behavior_tests;
mod picker_action_base_tests;
mod picker_behavior_base_tests;
