//! The unit tests of the reference (`tests/Xaml.Behaviors.Animations.UnitTests`): one module per
//! test file, on the unit test application of the framework. `TestAppBuilder.cs`, the test
//! application of the reference, is replaced by [`support`].

pub(crate) mod support;

mod animation_factory_tests;
mod animation_runner_tests;
mod composition_animation_tests;
mod composition_catalog_smoke_tests;
mod selecting_items_control_animation_tests;
mod transition_operations_tests;
