//! The unit tests of the crate.
//!
//! The reference has no test project of its own for these behaviors: the tests of
//! `tests/Xaml.Behaviors.Interactions.UnitTests/DragAndDrop` that test them are ported here, one
//! module per test file (`drag_and_drop/`), and the other modules test what runs without a drag
//! session of the system: the drag state of the drag sources, the drag events raised on the drop
//! targets of a headless window, the context store, the drop handler helpers and the managed drag.
//! The tests run in the unit test application of the framework (`support.rs`).

pub(crate) mod support;

mod drag_and_drop;
mod drag_and_drop_tests;
mod managed_drag_drop_tests;
mod styles_tests;
