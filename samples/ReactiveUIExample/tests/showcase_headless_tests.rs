//! Port of `ShowcaseHeadlessTests.cs`: the production application setup and the lifetimes of the
//! rendered views.
//!
//! The reference renders on its framework's headless platform; here a test that clicks a control
//! runs on the headless shell of the samples (`Shell`), and the others on the unit test application
//! of the framework, with the services of a styled window.

use super::support::{await_task, run_jobs, run_until, start_application, start_shell, Disposing};
use crate::models::MachineSnapshot;
use crate::services::ILocalMachineMetricsService;
use crate::view_models::{MainViewModel, MetricCardViewModel, MetricsViewModel, FAILURE_MESSAGE};
use crate::views::{CommandLabView, FeatureView, MainWindow, MemoryMetricCardView, MetricsView, OverviewView, PerformanceMetricCardView};
use ferroui_base::input::ICommand as _;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::{Application, Control, Window};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_reactiveui::{IActivatableView, IReactiveNotifyPropertyChanged, IViewForOf, RoutableViewModel};
use ferroui_reactiveui_controls::{view_control, FerroScheduler};
use ferroui_splat::{AppLocator, ReadonlyDependencyResolverExt};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use sample_testing::Shell;
use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

/// The width of a test window (`WindowWidth`).
const WINDOW_WIDTH: f64 = 1000.0;

/// The height of a test window (`WindowHeight`).
const WINDOW_HEIGHT: f64 = 700.0;

/// A controllable measurement source that records the ownership of its subscriptions
/// (`CountingMetricsService`).
struct CountingMetricsService {
    /// The source of the later measurements (`_samples`).
    samples: Rc<Subject<MachineSnapshot>>,
    /// The number of active subscriptions (`ActiveSubscriptions`).
    active_subscriptions: Rc<Cell<i32>>,
}

impl CountingMetricsService {
    fn new() -> Rc<Self> {
        Rc::new(Self { samples: Subject::new(), active_subscriptions: Rc::new(Cell::new(0)) })
    }

    /// The number of active subscriptions (`ActiveSubscriptions`).
    fn active_subscriptions(&self) -> i32 {
        self.active_subscriptions.get()
    }

    /// Pushes another measurement on the calling thread (`Publish()`).
    fn publish(&self) {
        self.samples.on_next(MachineSnapshot::empty());
    }
}

impl ILocalMachineMetricsService for CountingMetricsService {
    fn read_snapshot(&self) -> Result<MachineSnapshot, ObservableError> {
        Ok(MachineSnapshot::empty())
    }

    fn watch(&self, _interval: Duration) -> Rc<dyn IObservable<MachineSnapshot>> {
        let samples = self.samples.clone();
        let active = self.active_subscriptions.clone();
        Observable::create(move |observer: Rc<dyn IObserver<MachineSnapshot>>| {
            active.set(active.get() + 1);
            observer.on_next(MachineSnapshot::empty());
            let subscription = samples.subscribe(observer);
            let active = active.clone();
            Disposable::create(move || {
                subscription.dispose();
                active.set(active.get() - 1);
            })
        })
    }
}

impl IDisposable for CountingMetricsService {
    /// `Dispose()`.
    fn dispose(&self) {
        self.samples.dispose();
    }
}

/// A window of the test size that shows `content` (`new Window { Content = content, Width = ..,
/// Height = .. }`).
fn window_with(content: Ref<Control>) -> Ref<Window> {
    let window = Window::new();
    window.set_content(Some(Control::boxed(content)));
    window.set_width(WINDOW_WIDTH);
    window.set_height(WINDOW_HEIGHT);
    window
}

/// The control a content holds, if it holds one.
fn content_control(content: Option<BoxedValue>) -> Option<Ref<Control>> {
    content.as_ref().and_then(Control::from_boxed)
}

#[test]
fn showcase_uses_explicit_light_theme() {
    let _app = start_application();
    let application = Application::current().expect("the application of the test");
    assert!(application.requested_theme_variant() == Some(ThemeVariant::light()));
}

#[test]
fn failure_button_executes_and_reports_error() {
    let shell = start_shell(WINDOW_WIDTH, WINDOW_HEIGHT);
    assert!(Arc::ptr_eq(FerroScheduler::instance().dispatcher(), &Dispatcher::ui_thread()));
    let service = CountingMetricsService::new();
    let _service = Disposing(service.clone());
    let view_model = MainViewModel::create(service);
    let _shell = Disposing(view_model.clone());
    let window = MainWindow::new();
    MainWindow::typed(&window).set_view_model(Some(view_model.clone()));

    window.show();
    shell.settle();
    view_model.navigate(Some(view_model.commands() as RoutableViewModel));
    shell.settle();
    let view = content_control(window.main().router_host().content())
        .and_then(|content| content.cast::<CommandLabView>())
        .expect("the router host shows the command lab");
    let button = view.fail_work_button();
    // Deviation (DEVIATIONS.md, samples): the command of the button is an adapter of FailWork
    // (`ReactiveCommandAdapter`), so the test checks that the button has a command that can execute,
    // and that clicking it runs FailWork, where the reference compares the references.
    let command = button.command().expect("the command of the button runs FailWork");
    assert!(button.is_effectively_enabled());
    assert!(command.can_execute(None));

    let center = Shell::bounds_of(&window, &button).center();
    shell.click(0, center);
    assert!(run_until(|| view_model.commands().acknowledged_errors() > 0), "the view acknowledged the error");
    assert_eq!(view_model.commands().last_error(), FAILURE_MESSAGE);

    window.close();
}

#[test]
fn command_view_binds_input_and_handles_interactions_only_while_active() {
    const INITIAL_INPUT: &str = "Review measurements";
    const EXPECTED_ACKNOWLEDGEMENTS: i32 = 2;
    let _app = start_application();
    let service = CountingMetricsService::new();
    let _service = Disposing(service.clone());
    let shell = MainViewModel::create(service);
    let _shell = Disposing(shell.clone());
    let view = CommandLabView::new();
    CommandLabView::typed(&view).set_view_model(Some(shell.commands()));
    let window = window_with(view.clone().upcast());

    window.show();
    run_jobs();
    let input = view.work_item_text_box();
    let button = view.run_work_button();
    assert!(!button.is_effectively_enabled());
    input.set_text(Some(INITIAL_INPUT));
    run_jobs();
    assert_eq!(shell.commands().work_item_text(), INITIAL_INPUT);
    assert!(button.is_effectively_enabled());
    assert!(button.command().is_some(), "the command of the button runs RunWork");
    await_task(&shell.commands().report_error().handle("Expected test interaction".to_owned())).expect("the view handles the interaction");
    assert_eq!(shell.commands().acknowledged_errors(), 1);

    window.set_content(None);
    run_jobs();
    assert!(button.command().is_none());
    shell.commands().set_work_item_text("Changed while inactive".to_owned());
    assert_eq!(input.text().as_deref(), Some(INITIAL_INPUT));

    window.set_content(Some(Control::boxed(view.clone())));
    run_jobs();
    assert_eq!(input.text().as_deref(), Some("Changed while inactive"));
    await_task(&shell.commands().report_error().handle("Reactivated interaction".to_owned())).expect("the view handles the interaction again");
    assert_eq!(shell.commands().acknowledged_errors(), EXPECTED_ACKNOWLEDGEMENTS);

    window.close();
}

#[test]
fn metrics_view_activation_stops_and_resumes_live_subscription() {
    let _app = start_application();
    let service = CountingMetricsService::new();
    let _service = Disposing(service.clone());
    let shell = MainViewModel::create(service.clone());
    let _shell = Disposing(shell.clone());
    let view = MetricsView::new();
    MetricsView::typed(&view).set_view_model(Some(shell.metrics()));
    let window = window_with(view.clone().upcast());

    window.show();
    run_jobs();
    assert_eq!(service.active_subscriptions(), 1);

    window.set_content(None);
    run_jobs();
    assert_eq!(service.active_subscriptions(), 0);
    let sample_count = shell.metrics().sample_count();
    service.publish();
    run_jobs();
    assert_eq!(shell.metrics().sample_count(), sample_count);

    window.set_content(Some(Control::boxed(view.clone())));
    run_jobs();
    assert_eq!(service.active_subscriptions(), 1);

    window.close();
    run_jobs();
    assert_eq!(service.active_subscriptions(), 0);
}

// Deviation (DEVIATIONS.md, samples): the reference publishes the measurement from a worker thread
// (`Task.Run(service.Publish)`) and checks that the change of `SampleCount` arrives on the UI
// thread. The observables of the port belong to the thread that creates them (docs/porting/
// reactive.md, section 5.1), so the test publishes on the thread of the test and checks what the
// marshalling does there: the change arrives through the scheduler of the user-interface thread
// (`ObserveOn(MainThreadScheduler)`), in a job of the dispatcher, on the UI thread.
#[test]
fn metrics_view_marshals_background_samples_to_ui_thread() {
    let _app = start_application();
    let service = CountingMetricsService::new();
    let _service = Disposing(service.clone());
    let shell = MainViewModel::create(service.clone());
    let _shell = Disposing(shell.clone());
    let view = MetricsView::new();
    MetricsView::typed(&view).set_view_model(Some(shell.metrics()));
    let window = window_with(view.clone().upcast());

    window.show();
    run_jobs();
    let updated: Rc<Cell<Option<bool>>> = Rc::new(Cell::new(None));
    let seen = updated.clone();
    let metrics: Rc<MetricsViewModel> = shell.metrics();
    let _changes = metrics
        .changed()
        .where_(|change| change.property_name == "SampleCount")
        .subscribe_next(move |_| seen.set(Some(Dispatcher::ui_thread().check_access())));

    service.publish();
    assert!(updated.get().is_none(), "the sample is delivered through the scheduler of the user-interface thread");
    assert!(run_until(|| updated.get().is_some()), "the sample is delivered");
    assert_eq!(updated.get(), Some(true));

    window.close();
}

#[test]
fn production_startup_resolves_initial_route_and_contracted_cards() {
    let shell = start_shell(WINDOW_WIDTH, WINDOW_HEIGHT);
    let service = CountingMetricsService::new();
    let _service = Disposing(service.clone());
    let view_model = MainViewModel::create(service);
    let _shell = Disposing(view_model.clone());
    let window = MainWindow::new();
    MainWindow::typed(&window).set_view_model(Some(view_model.clone()));

    window.show();
    shell.settle();
    let resolver = AppLocator::current();
    let performance = resolver
        .get_service_with_contract::<dyn IViewForOf<MetricCardViewModel>>(Some("performance"))
        .expect("a view of the contract performance");
    let performance: &dyn IActivatableView = &*performance;
    assert!(view_control(performance).and_then(|control| control.cast::<PerformanceMetricCardView>()).is_some());
    let memory = resolver.get_service_with_contract::<dyn IViewForOf<MetricCardViewModel>>(Some("memory")).expect("a view of the contract memory");
    let memory: &dyn IActivatableView = &*memory;
    assert!(view_control(memory).and_then(|control| control.cast::<MemoryMetricCardView>()).is_some());

    let host = window.main().router_host();
    let overview = content_control(host.content()).and_then(|content| content.cast::<OverviewView>()).expect("the initial route is the overview");
    let items = overview.feature_items();
    assert!(items.item_template().is_some());
    let rendered_features = items.get_visual_descendants().filter(|descendant| descendant.cast::<FeatureView>().is_some()).count();
    let features = OverviewView::view_model(&overview).expect("the overview has its view model").features();
    assert_eq!(rendered_features, features.len());

    view_model.navigate(Some(view_model.metrics() as RoutableViewModel));
    shell.settle();
    assert!(content_control(host.content()).and_then(|content| content.cast::<MetricsView>()).is_some());

    window.close();
}
