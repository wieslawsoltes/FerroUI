//! Tests of the ABI, called as a binding calls it. UI objects belong to one thread per process,
//! and the test harness runs each test on a thread of its own, so every UI call of a test runs on
//! one thread that the tests share ([`support::ui`]).

mod abi_tests;
mod events_tests;
mod header_tests;
mod support;
mod tree_tests;
mod values_tests;
