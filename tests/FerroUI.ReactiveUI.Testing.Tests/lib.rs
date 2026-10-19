//! ferroui-reactiveui-testing-tests
//!
//! The unit tests of `ferroui-reactiveui-testing`, ported from ReactiveUI's
//! `ReactiveUI.Testing.Tests`: one module per upstream test file under `tests`, each test under the
//! name of the upstream test method in Rust case. Each file names the upstream tests it does not
//! port and the reason.
//!
//! - [`mocks`]: the objects of the tests (`TestFixture`, `TestFixtureBuilder`) and the helpers that
//!   stand in for the asynchronous parts of TUnit and the base class library (`Task.Delay`,
//!   `Task.Yield`, an `InvalidOperationException`).
//!
//! The state that the helpers under test set (the schedulers of `RxSchedulers`, the current message
//! bus, the state of the application builder of Splat) is per thread, and the test harness runs
//! each test on a thread of its own, so a test starts from a fresh state. That is what upstream's
//! test executors (`AppBuilderTestExecutor`, `WithMessageBusExecutor`) and assembly hooks
//! (`AssemblyHooks`, which marks the run as a unit test run) set up for each test, so they are not
//! ported.

pub mod mocks;

#[cfg(test)]
mod tests;
