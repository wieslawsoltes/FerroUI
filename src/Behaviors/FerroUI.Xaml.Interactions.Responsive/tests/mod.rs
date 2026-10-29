//! Not from the reference: the reference has no unit tests for
//! `Xaml.Behaviors.Interactions.Responsive`. These tests change the bounds of a control in a
//! window and check that the behaviors add and remove the classes of their setters as the
//! conditions of the setters say, on the control, a target control or as pseudo classes, until
//! the control leaves the visual tree.
//!
//! They run in the unit test application of the framework (`support.rs`), as the tests of the
//! interactivity crate do.

pub(crate) mod support;

mod adaptive_behavior_tests;
mod aspect_ratio_behavior_tests;
mod register_types_tests;
