//! `ReactiveUI.Builder.Tests`: the tests of ReactiveUI's builder.
//!
//! `AssemblyHooks.cs` (the mode detector of a unit test runner) and `AssemblyHooks.Parallel.cs`
//! (the executor of every test, `BuilderTestExecutor`) are in [`executors`]: each test calls its
//! executor with its body. The tests run one per thread, and the state of ReactiveUI and Splat is
//! per thread, which `[NotInParallel]` ensures upstream.

mod executors;
mod mixins;
mod platforms;
mod reactive_ui_builder_converter_tests;
mod reactive_ui_builder_core_tests;
mod reactive_ui_builder_rebuild_tests;
mod reactive_ui_builder_rx_app_migration_tests;
mod reactive_ui_builder_view_locator_tests;
