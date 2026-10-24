//! Not from the reference: the reference has no unit tests for `Xaml.Behaviors.Interactions.Events`.
//! These tests raise each routed event on a host in a window and check that the trigger runs its
//! actions (with the args of the event as their parameter) and the behavior calls its hook, until
//! the host leaves the visual tree; and the default routing strategies of each class.
//!
//! They run in the unit test application of the framework (`support.rs`), as the tests of the
//! interactivity crate do.

pub(crate) mod support;

mod event_behavior_tests;
mod event_trigger_tests;
mod register_types_tests;
