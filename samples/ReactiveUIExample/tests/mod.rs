//! The tests of the example: the ported tests of the reference's test project
//! (`ReactiveUI.<framework>.Example.Tests`), one module per test file, the tests of the DynamicData
//! page (an addition of the port), and the generated per-document tests.
//!
//! The reference runs its tests on one headless session of the production application
//! (`ExampleTestSession`, `ExampleTestExecutor`, `CallerTestExecutor`); here each test starts the
//! production application on the unit test application of the framework, or on the headless shell
//! of the samples when it clicks a control (`support`).
//!
//! `documents` holds the generated per-document tests (`sample-build`): `document_<name>` loads the
//! document through the run-time loader and `class_<name>` constructs its class and shows it in a
//! window. `compiled_markup` compares each class populated by its compiled markup with the same
//! class populated by the run-time loader.

mod app_tests;
mod browser_machine_metrics_service_tests;
mod compiled_markup;
mod dynamic_data_view_model_tests;
mod example_test_executor_tests;
mod local_machine_metrics_service_tests;
mod showcase_headless_tests;
mod showcase_session_tests;
mod showcase_view_model_tests;
mod support;

mod documents {
    include!(concat!(env!("OUT_DIR"), "/document_tests.rs"));
}
