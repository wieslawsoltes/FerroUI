//! Not from the reference: the reference has no unit tests for this project. One module per
//! source file of the reference, testing its action or trigger on a router and an interaction of
//! ReactiveUI, and the support they share.
//!
//! The tests run in the unit test application of the framework (`support.rs`).

pub(crate) mod support;

mod clear_navigation_stack_action_tests;
mod interaction_trigger_behavior_tests;
mod navigate_action_tests;
mod navigate_and_reset_tests;
mod navigate_back_action_tests;
mod navigate_to_action_tests;
mod navigate_to_and_reset_action_tests;
