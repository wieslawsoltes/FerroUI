//! The tests of part B of the crate: one module per test file of
//! `tests/Xaml.Behaviors.Interactions.UnitTests/Custom` of the reference that tests a type of the
//! part, and the test application (`support.rs`).

#[allow(dead_code)]
pub(crate) mod support;

mod click_event_trigger_tests;
mod focus_trap_behavior_tests;
mod inline_edit_behavior_tests;
mod validation_rule_tests;
