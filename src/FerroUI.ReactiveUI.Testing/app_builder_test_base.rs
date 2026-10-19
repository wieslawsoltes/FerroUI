//! `AppBuilderTestBase` (`ReactiveUI.Testing/AppBuilderTestBase.cs`).

use crate::RxTest;
use ferroui_system::threading::tasks::Task;

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
    fn run_app_builder_test_async(test_body: impl FnOnce() -> Task<()> + 'static) -> Task<()> {
        RxTest::app_builder_test_async(test_body)
    }

    /// Runs a synchronous test of the application builder (`RunAppBuilderTestAsync(Action)`): see
    /// [`RxTest::app_builder_test_async`]. A panic of `test_body` ends the returned task with the
    /// error of the panic.
    fn run_app_builder_test_action(test_body: impl FnOnce() + 'static) -> Task<()> {
        RxTest::app_builder_test_async(move || {
            test_body();
            Task::from_result(())
        })
    }
}
