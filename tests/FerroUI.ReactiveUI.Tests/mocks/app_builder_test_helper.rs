//! `AppBuilderTestHelper`, `BaseAppBuilderTestExecutor` and `AppBuilderTestExecutor`
//! (`ReactiveUI.Test.Utilities/AppBuilder`): the set-up and clean-up of a ReactiveUI application
//! around a test.

use ferroui_reactive::concurrency::IScheduler;
use ferroui_reactiveui::{ReactiveUIBuilder, RxAppBuilder, RxSchedulers};
use ferroui_splat::AppBuilder;
use std::rc::Rc;

/// Builds a ReactiveUI application for a test and restores the state after it
/// (`AppBuilderTestHelper`).
#[derive(Default)]
pub struct AppBuilderTestHelper {
    /// `_originalMainThreadScheduler`.
    original_main_thread_scheduler: Option<Rc<dyn IScheduler>>,
    /// `_originalTaskpoolScheduler`.
    original_taskpool_scheduler: Option<Rc<dyn IScheduler>>,
}

impl AppBuilderTestHelper {
    /// Resets the state of ReactiveUI, configures a new builder with `configure_builder` and
    /// builds the application (`Initialize(Action<IReactiveUIBuilder>)`).
    pub fn initialize(&mut self, configure_builder: impl FnOnce(&ReactiveUIBuilder)) {
        // The schedulers, restored after the test.
        self.original_main_thread_scheduler = Some(RxSchedulers::main_thread_scheduler());
        self.original_taskpool_scheduler = Some(RxSchedulers::task_pool_scheduler());

        RxAppBuilder::reset_for_testing();
        AppBuilder::reset_builder_state_for_tests();

        let builder = RxAppBuilder::create_reactive_ui_builder();
        configure_builder(&builder);

        let _ = builder.build_app();
    }

    /// Restores the schedulers and builds a clean application for the next test (`CleanUp`).
    pub fn clean_up(&mut self) {
        if let Some(scheduler) = self.original_main_thread_scheduler.take() {
            RxSchedulers::set_main_thread_scheduler(scheduler);
        }

        if let Some(scheduler) = self.original_taskpool_scheduler.take() {
            RxSchedulers::set_task_pool_scheduler(scheduler);
        }

        RxAppBuilder::reset_for_testing();

        let _ = RxAppBuilder::create_reactive_ui_builder().with_core_services().build_app();
    }
}

/// Runs `test` in a ReactiveUI application with the core services, as the executor that
/// upstream's `ReactiveUI.Tests` gives every test does (`AppBuilderTestExecutor`,
/// `BaseAppBuilderTestExecutor.ExecuteTest`).
pub fn with_app_builder<R>(test: impl FnOnce() -> R) -> R {
    with_configured_app_builder(
        |builder| {
            let _ = builder.with_core_services();
        },
        test,
    )
}

/// Runs `test` in a ReactiveUI application that `configure_builder` configured
/// (`BaseAppBuilderTestExecutor` with its `ConfigureAppBuilder` overridden).
pub fn with_configured_app_builder<R>(configure_builder: impl FnOnce(&ReactiveUIBuilder), test: impl FnOnce() -> R) -> R {
    let mut helper = AppBuilderTestHelper::default();
    helper.initialize(configure_builder);
    let result = test();
    helper.clean_up();
    result
}
