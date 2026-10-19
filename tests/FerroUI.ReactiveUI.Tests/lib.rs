//! ferroui-reactiveui-tests
//!
//! The unit tests of `ferroui-reactiveui`, ported from ReactiveUI's `ReactiveUI.Tests` and the
//! tests of ReactiveUI.Binding: one module per upstream test file, under the folders of the
//! upstream projects in snake case (`tests/reactive_objects`, `tests/core`, ..., and
//! `tests/binding/...` for ReactiveUI.Binding), each test under the name of the upstream test
//! method in Rust case. Each file names the upstream tests it does not port and the reason.
//!
//! - [`mocks`]: the objects of the tests (the `Mocks` folders of `ReactiveUI.Tests` and the helpers
//!   of `ReactiveUI.Test.Utilities`): view models, services and the collecting observers.
//!
//! The state of ReactiveUI (the schedulers of `RxSchedulers`, the handler of `RxState`, the
//! locator of Splat) is per thread, and the test harness runs each test on a thread of its own, so
//! a test starts from a fresh state.

pub mod mocks;

#[cfg(test)]
mod tests;
