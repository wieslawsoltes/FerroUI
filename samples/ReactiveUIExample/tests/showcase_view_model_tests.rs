//! Port of `ShowcaseViewModelTests.cs`: the showcase's commands, routing, and computed state.

use super::support::{await_last, run_jobs, run_until, same_object, start_application, Disposing};
use crate::models::MachineSnapshot;
use crate::services::{ILocalMachineMetricsService, InvalidOperationError};
use crate::view_models::{MainViewModel, OverviewViewModel, FAILURE_MESSAGE};
use ferroui_base::utilities::DateTimeOffset;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_reactiveui::{ICommand, IInteractionContext, RoutableViewModel};
use ferroui_system::{IDisposable, IObservable, ObservableError};
use std::any::Any;
use std::rc::Rc;
use std::time::Duration;

/// The sample CPU utilisation (`ProcessorPercent`).
const PROCESSOR_PERCENT: f64 = 12.5;

/// The warning threshold below the sample utilisation (`WarningThreshold`).
const WARNING_THRESHOLD: f64 = 10.0;

/// A deterministic measurement source for the view-model tests (`FixedMetricsService`).
struct FixedMetricsService;

impl FixedMetricsService {
    /// The sample working set in megabytes (`WorkingSet`).
    const WORKING_SET: f64 = 128.0;

    /// The sample heap in megabytes (`ManagedHeap`).
    const MANAGED_HEAP: f64 = 32.0;

    /// The sample thread count (`ThreadCount`).
    const THREAD_COUNT: i32 = 8;

    /// The fixed sample (`Snapshot`).
    fn snapshot() -> MachineSnapshot {
        MachineSnapshot::new(DateTimeOffset::UNIX_EPOCH, PROCESSOR_PERCENT, Self::WORKING_SET, Self::MANAGED_HEAP, Self::THREAD_COUNT, "fixed")
    }
}

impl ILocalMachineMetricsService for FixedMetricsService {
    fn read_snapshot(&self) -> Result<MachineSnapshot, ObservableError> {
        Ok(Self::snapshot())
    }

    fn watch(&self, _interval: Duration) -> Rc<dyn IObservable<MachineSnapshot>> {
        Observable::return_(Self::snapshot())
    }
}

/// A measurement source that fails immediate reads (`ThrowingMetricsService`).
struct ThrowingMetricsService {
    message: String,
}

impl ILocalMachineMetricsService for ThrowingMetricsService {
    fn read_snapshot(&self) -> Result<MachineSnapshot, ObservableError> {
        Err(InvalidOperationError::new(self.message.clone()).into_error())
    }

    fn watch(&self, _interval: Duration) -> Rc<dyn IObservable<MachineSnapshot>> {
        Observable::return_(MachineSnapshot::empty())
    }
}

/// A shell over the fixed measurement source, disposed at the end of the test.
fn create_shell() -> Disposing<MainViewModel> {
    Disposing(MainViewModel::create(Rc::new(FixedMetricsService)))
}

/// Whether a routed view model is an overview (`IsTypeOf<OverviewViewModel>`).
fn is_overview(page: &RoutableViewModel) -> bool {
    let page: Rc<dyn Any> = page.clone();
    page.is::<OverviewViewModel>()
}

#[test]
fn shell_navigates_back_and_resets_with_router_commands() {
    let _app = start_application();
    let shell = create_shell();
    let view_model = &shell.0;
    run_jobs();
    assert_eq!(view_model.router().navigation_stack().count(), 1);
    assert!(is_overview(&view_model.router().navigation_stack().get(0)));
    assert!(!ICommand::can_execute(&view_model.go_back(), None));

    view_model.navigate(Some(view_model.metrics() as RoutableViewModel));
    run_jobs();
    assert!(ICommand::can_execute(&view_model.go_back(), None));
    await_last(view_model.go_back().execute(())).expect("the navigation goes back");
    assert!(is_overview(&view_model.router().navigation_stack().get(0)));

    view_model.navigate(Some(view_model.commands() as RoutableViewModel));
    view_model.reset_navigation();
    run_jobs();
    assert_eq!(view_model.router().navigation_stack().count(), 1);
    assert!(is_overview(&view_model.router().navigation_stack().get(0)));
    assert!(!ICommand::can_execute(&view_model.go_back(), None));
}

#[test]
fn page_commands_navigate_to_their_pages() {
    let _app = start_application();
    let shell = create_shell();
    let view_model = &shell.0;
    let top = || {
        let stack = view_model.router().navigation_stack();
        stack.get(stack.count() - 1)
    };

    ICommand::execute(&view_model.show_metrics(), None);
    run_jobs();
    assert!(same_object(&top(), &view_model.metrics()));

    ICommand::execute(&view_model.show_commands(), None);
    run_jobs();
    assert!(same_object(&top(), &view_model.commands()));

    ICommand::execute(&view_model.show_overview(), None);
    run_jobs();
    assert!(is_overview(&top()));
    assert!(view_model.navigation_status().starts_with("Navigated to"), "{}", view_model.navigation_status());
}

#[test]
fn shell_reports_a_rejected_navigation() {
    let _app = start_application();
    let shell = create_shell();
    let view_model = &shell.0;
    run_jobs();
    let initial_status = view_model.navigation_status();

    view_model.navigate(None);

    // The router reports the error on the UI thread; drain it while the shell still observes it.
    run_jobs();

    assert_ne!(view_model.navigation_status(), initial_status);
    assert!(!view_model.navigation_status().starts_with("Navigated to"));
    assert_eq!(view_model.router().navigation_stack().count(), 1);
}

#[test]
fn command_lab_uses_can_execute_async_and_interaction_acknowledgement() {
    let _app = start_application();
    let shell = create_shell();
    let view_model = shell.0.commands();
    run_jobs();
    assert!(!ICommand::can_execute(&view_model.run_work(), None));

    view_model.set_work_item_text("sample".to_owned());
    run_jobs();
    assert!(ICommand::can_execute(&view_model.run_work(), None));
    let operation = view_model.run_work().execute(()).to_task();
    view_model.set_work_item_text("changed during execution".to_owned());
    let result = super::support::await_task(&operation).expect("the work runs");
    run_jobs();
    assert!(result.contains("'sample'"), "{result}");
    assert!(run_until(|| view_model.last_result() == result));

    let acknowledging = view_model.clone();
    let handler = view_model.report_error().register_handler(move |context| {
        acknowledging.acknowledge_error(context.input());
        context.set_output(());
    });
    let error = await_last(view_model.fail_work().execute(())).err().expect("the work fails");
    assert!(error.downcast_ref::<InvalidOperationError>().is_some(), "{error}");
    assert!(run_until(|| view_model.acknowledged_errors() == 1));
    assert_eq!(view_model.last_error(), FAILURE_MESSAGE);
    assert_eq!(view_model.acknowledged_errors(), 1);
    assert!(view_model.interaction_status().contains("acknowledged"), "{}", view_model.interaction_status());
    handler.dispose();
}

#[test]
fn command_lab_reports_unhandled_error_interaction() {
    let _app = start_application();
    let shell = create_shell();
    let view_model = shell.0.commands();

    let error = await_last(view_model.fail_work().execute(())).err().expect("the work fails");
    assert!(error.downcast_ref::<InvalidOperationError>().is_some(), "{error}");
    assert!(
        run_until(|| view_model.interaction_status().starts_with("No active view handled")),
        "the status reports the unhandled interaction: {}",
        view_model.interaction_status()
    );
    assert!(view_model.interaction_status().contains("No active view handled the interaction"));
}

#[test]
fn metrics_page_applies_samples_and_computes_load_label() {
    let _app = start_application();
    let shell = create_shell();
    let view_model = shell.0.metrics();
    view_model.set_cpu_warning_threshold(WARNING_THRESHOLD);
    view_model.apply_snapshot(FixedMetricsService::snapshot());

    assert_eq!(view_model.processor_card().value(), "12.5%");
    assert_eq!(view_model.memory_card().value(), "128.0 MB");
    assert_eq!(view_model.managed_card().value(), "32.0 MB");
    assert_eq!(view_model.sample_count(), 1);
    assert!(view_model.load_label().contains("at or above"), "{}", view_model.load_label());
    view_model.set_cpu_warning_threshold(PROCESSOR_PERCENT + 1.0);
    assert!(view_model.load_label().contains("below"), "{}", view_model.load_label());
}

#[test]
fn metrics_sample_once_reads_and_applies_snapshot() {
    let _app = start_application();
    let shell = create_shell();
    let result = await_last(shell.0.metrics().sample_once().execute(())).expect("the sample is read");

    assert!(result == FixedMetricsService::snapshot());
    assert!(shell.0.metrics().latest() == result);
    assert_eq!(shell.0.metrics().sample_count(), 1);
}

#[test]
fn metrics_sample_once_failure_updates_latest_status() {
    const EXPECTED_ERROR: &str = "sample failed";
    let _app = start_application();
    let shell = Disposing(MainViewModel::create(Rc::new(ThrowingMetricsService { message: EXPECTED_ERROR.to_owned() })));

    let error = await_last(shell.0.metrics().sample_once().execute(())).err().expect("the sample fails");
    assert!(error.downcast_ref::<InvalidOperationError>().is_some(), "{error}");
    assert!(run_until(|| shell.0.metrics().latest().status() == EXPECTED_ERROR), "the status of the latest sample: {}", shell.0.metrics().latest().status());
    assert_eq!(shell.0.metrics().latest().status(), EXPECTED_ERROR);
}
