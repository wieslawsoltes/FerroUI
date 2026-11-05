//! `AppBuilderTestBase` (`ReactiveUI.Testing/AppBuilderTestBase.cs`).

use crate::RxTest;
use ferroui_system::task::Shared;
use ferroui_system::ObservableError;
use std::future::Future;

/// The base of the fixtures of tests of the application builder, which gives them
/// [`RxTest::app_builder_test_async`] (`AppBuilderTestBase`).
///
/// Deviation (DEVIATIONS.md, ReactiveUI): upstream's abstract base class with protected static
/// helpers is a trait with provided functions; a fixture implements it (`impl AppBuilderTestBase
/// for MyTests {}`) and calls `MyTests::run_app_builder_test_async(..)`. The overload that takes an
/// `Action` is [`run_app_builder_test_action`](Self::run_app_builder_test_action).
pub trait AppBuilderTestBase {
    /// Runs an asynchronous test of the application builder
    /// (`RunAppBuilderTestAsync(Func<Task>)`): see [`RxTest::app_builder_test_async`].
    fn run_app_builder_test_async<Fut: Future<Output = Result<(), ObservableError>> + 'static>(
        test_body: impl FnOnce() -> Fut + 'static,
    ) -> Shared<Result<(), ObservableError>> {
        RxTest::app_builder_test_async(test_body)
    }

    /// Runs a synchronous test of the application builder (`RunAppBuilderTestAsync(Action)`): see
    /// [`RxTest::app_builder_test_async`]. A panic of `test_body` ends the returned task with the
    /// error of the panic.
    fn run_app_builder_test_action(test_body: impl FnOnce() + 'static) -> Shared<Result<(), ObservableError>> {
        RxTest::app_builder_test_async(move || {
            test_body();
            std::future::ready(Ok::<(), ObservableError>(()))
        })
    }
}
