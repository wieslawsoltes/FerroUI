//! `ReactiveUI.Tests/Suspension`: the suspension hosts, their extensions and the dummy driver.

mod dummy_app_state;
mod dummy_suspension_driver_tests;
mod suspension_host_extensions_aot_tests;
mod suspension_host_extensions_aot_tests_state_lifecycle;
mod suspension_host_generic_tests;
mod suspension_host_tests;

pub(crate) use dummy_app_state::DummyAppState;
