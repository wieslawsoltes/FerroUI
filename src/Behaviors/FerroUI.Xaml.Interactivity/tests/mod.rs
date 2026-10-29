//! The unit tests of the reference (`tests/Xaml.Behaviors.Interactivity.UnitTests` of
//! Xaml.Behaviors): one module per test file of the reference, and its support files.
//!
//! The reference runs its tests on the headless platform of its framework (`TestAppBuilder.cs`,
//! `App.xaml` with the Fluent theme); here they run in the unit test application of the
//! framework (`support.rs`).

pub(crate) mod stub_action;
pub(crate) mod stub_behavior;
pub(crate) mod support;
pub(crate) mod test_behaviors;
pub(crate) mod test_uitilties;

mod action_tests;
mod add_event_handler_registry_tests;
mod behavior_of_t_tests;
mod behavior_tests;
mod collections;
mod command_can_execute_observer_tests;
mod comparison_condition_type_helper_tests;
mod condition_tests;
mod interaction_test;
mod interaction_tests;
mod invoke_command_action_base_tests;
mod invoke_command_behavior_base_tests;
mod styled_element_action_tests;
mod styled_element_behavior_tests;
mod trigger_of_t_tests;
mod trigger_tests;
