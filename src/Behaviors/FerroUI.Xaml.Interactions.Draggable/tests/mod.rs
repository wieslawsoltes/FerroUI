//! The unit tests of the crate.
//!
//! The reference has no test project of its own for these behaviors: the tests of
//! `tests/Xaml.Behaviors.Interactions.UnitTests` that test them are ported here, one module per
//! test file (`core/`), and the other modules test the logic of the drags that runs without a
//! drag session (hit testing, the computation of the reorder index, the drag state). The tests run
//! in the unit test application of the framework (`support.rs`).

pub(crate) mod support;

mod core;
mod draggable_tests;
mod styles_tests;
