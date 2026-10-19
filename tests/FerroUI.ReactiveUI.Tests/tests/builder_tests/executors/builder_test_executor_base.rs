//! `BuilderTestExecutorBase` (`ReactiveUI.Builder.Tests/Executors/BuilderTestExecutorBase.cs`),
//! with the set-up of the assembly (`AssemblyHooks.cs`).

use ferroui_reactiveui::RxAppBuilder;
use ferroui_splat::{AppBuilder, IModeDetector, ModeDetector};
use std::rc::Rc;

/// The mode detector that says the code runs in a unit test runner (`TestModeDetector`).
struct TestModeDetector;

impl IModeDetector for TestModeDetector {
    fn in_unit_test_runner(&self) -> Option<bool> {
        Some(true)
    }
}

/// `AssemblyHooks.AssemblySetup`: the framework detects a unit test runner. The mode detector is
/// per thread, and each test runs on its own thread, so each test sets it.
pub fn assembly_setup() {
    ModeDetector::override_mode_detector(Rc::new(TestModeDetector));
}

/// `ResetState`: the state of ReactiveUI and of Splat's builder.
pub fn reset_state() {
    RxAppBuilder::reset_for_testing();
    AppBuilder::reset_builder_state_for_tests();
}

/// `ConfigureBuilder`: the core services, built.
pub fn configure_builder() {
    let _ = RxAppBuilder::create_reactive_ui_builder().with_core_services().build_app();
}

/// `ExecuteTest`: resets the state with `reset`, configures with `configure`, runs `action`, and
/// resets the state again. A test that panics ends its thread, and with it the state.
pub fn execute_test<R>(reset: impl Fn(), configure: impl FnOnce(), action: impl FnOnce() -> R) -> R {
    assembly_setup();
    reset();
    configure();
    let result = action();
    reset();
    result
}

/// An executor that resets the state and leaves the configuration to the test (the
/// `ResetOnlyExecutor` of several test classes).
pub fn reset_only<R>(action: impl FnOnce() -> R) -> R {
    execute_test(reset_state, || {}, action)
}
