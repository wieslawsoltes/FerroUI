//! The unit tests of the reference (`tests/Xaml.Behaviors.Interactions.UnitTests` of
//! Xaml.Behaviors): one module per test file of the reference that tests the types of this crate,
//! and its support files.
//!
//! The reference runs its tests on the headless platform of its framework (`TestAppBuilder.cs`,
//! the application document with the Fluent theme, `HeadlessWindowExtensions.cs`); here they run in the unit test
//! application of the framework (`support.rs`). The test files of the reference for the types of
//! its other projects are listed in `not_ported.rs`.

pub(crate) mod support;

mod core;
mod not_ported;
