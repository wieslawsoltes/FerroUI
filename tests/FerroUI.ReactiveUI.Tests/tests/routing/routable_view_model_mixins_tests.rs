//! `RoutableViewModelMixinsTests` (`ReactiveUI.Tests/Routing/RoutableViewModelMixinsTests.cs`).

use ferroui_reactive::concurrency::ImmediateScheduler;
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::*;
use ferroui_system::{IDisposable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::{Rc, Weak};

const CONCURRENT_SUBSCRIPTION_COUNT: usize = 200;

/// Counts the notifications of a focus observable, and runs a handler once on the next value
/// (`FocusRecorder`).
#[derive(Default)]
struct FocusRecorder {
    emissions: Cell<i32>,
    completed: Cell<i32>,
    errors: Cell<i32>,
    on_next_handler: RefCell<Option<Box<dyn Fn()>>>,
}

impl FocusRecorder {
    fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// Sets the handler run, once, on the next value (`OnNextHandler`, which the tests clear in
    /// the handler).
    fn set_on_next_handler(&self, handler: impl Fn() + 'static) {
        *self.on_next_handler.borrow_mut() = Some(Box::new(handler));
    }
}

impl IObserver<()> for FocusRecorder {
    fn on_next(&self, _value: ()) {
        self.emissions.set(self.emissions.get() + 1);
        let handler = self.on_next_handler.borrow_mut().take();
        if let Some(handler) = handler {
            handler();
        }
    }

    fn on_error(&self, _error: ObservableError) {
        self.errors.set(self.errors.get() + 1);
    }

    fn on_completed(&self) {
        self.completed.set(self.completed.get() + 1);
    }
}

/// A screen with its router (`TestScreen`).
struct TestScreen {
    router: Rc<RoutingState>,
}

impl IScreen for TestScreen {
    fn router(&self) -> Rc<RoutingState> {
        self.router.clone()
    }
}

impl TestScreen {
    /// A screen whose router navigates on the immediate scheduler.
    fn new() -> Rc<Self> {
        Self::with_router(RoutingState::with_scheduler(Some(ImmediateScheduler::instance())))
    }

    fn with_router(router: Rc<RoutingState>) -> Rc<Self> {
        Rc::new(Self { router })
    }
}

/// A router whose stack stream is a subject the test drives (`ScriptedRoutingState`).
struct ScriptedRoutingState {
    router: Rc<RoutingState>,
    changes: Rc<Subject<NavigationStackSnapshot>>,
}

impl ScriptedRoutingState {
    fn new() -> Self {
        let router = RoutingState::with_scheduler(Some(ImmediateScheduler::instance()));
        let changes = Subject::new();
        router.set_navigation_stack_changed(changes.as_observable());
        Self { router, changes }
    }

    fn push(&self, view_model: &Rc<RoutableViewModel>) {
        let view_model: Rc<dyn IRoutableViewModel> = view_model.clone();
        self.router.navigation_stack().add(view_model);
        self.changes.on_next(self.router.navigation_stack().to_vec());
    }

    fn remove(&self, view_model: &Rc<RoutableViewModel>) {
        let view_model: Rc<dyn IRoutableViewModel> = view_model.clone();
        let _ = self.router.navigation_stack().remove(&view_model);
        self.changes.on_next(self.router.navigation_stack().to_vec());
    }
}

impl Drop for ScriptedRoutingState {
    fn drop(&mut self) {
        self.changes.dispose();
    }
}

/// A routable view model of a screen, which it holds weakly (`RoutableViewModel`).
struct RoutableViewModel {
    reactive: ReactiveObject,
    host_screen: Weak<dyn IScreen>,
}
reactive_object!(RoutableViewModel);

impl IRoutableViewModel for RoutableViewModel {
    fn url_path_segment(&self) -> Option<String> {
        Some("test".to_owned())
    }

    fn host_screen(&self) -> Option<Rc<dyn IScreen>> {
        self.host_screen.upgrade()
    }
}

fn routable(screen: &Rc<TestScreen>) -> Rc<RoutableViewModel> {
    let screen: Rc<dyn IScreen> = screen.clone();
    Rc::new(RoutableViewModel { reactive: ReactiveObject::new(), host_screen: Rc::downgrade(&screen) })
}

fn navigate(screen: &TestScreen, view_model: &Rc<RoutableViewModel>) {
    let view_model: Rc<dyn IRoutableViewModel> = view_model.clone();
    let _ = screen.router.navigate().execute(Some(view_model)).subscribe_ignore();
}

fn navigate_back(screen: &TestScreen) {
    let _ = screen.router.navigate_back().execute(()).subscribe_ignore();
}

/// The error of a stack stream that fails.
#[derive(Debug)]
struct StackFailed;

impl fmt::Display for StackFailed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("stack failed")
    }
}

impl std::error::Error for StackFailed {}

#[test]
fn when_navigating_from_observable_handler_navigates_synchronously_reports_one_departure() {
    let router = Rc::new(ScriptedRoutingState::new());
    let screen = TestScreen::with_router(router.router.clone());
    let watched = routable(&screen);
    let second = routable(&screen);
    let third = routable(&screen);
    let recorder = FocusRecorder::new();
    let subscription = watched.when_navigating_from_observable().subscribe(recorder.clone());

    // Navigating from inside the departure handler re-enters the sink underneath its own
    // notification.
    let scripted = router.clone();
    recorder.set_on_next_handler(move || scripted.push(&third));
    router.push(&watched);
    router.push(&second);

    assert_eq!(recorder.emissions.get(), 1);
    assert_eq!(recorder.completed.get(), 0);
    subscription.dispose();
}

#[test]
fn when_navigated_to_observable_handler_navigates_synchronously_reports_one_arrival() {
    let router = Rc::new(ScriptedRoutingState::new());
    let screen = TestScreen::with_router(router.router.clone());
    let watched = routable(&screen);
    let other = routable(&screen);
    let recorder = FocusRecorder::new();
    let subscription = watched.when_navigated_to_observable().subscribe(recorder.clone());

    let scripted = router.clone();
    recorder.set_on_next_handler(move || scripted.push(&other));
    router.push(&watched);

    assert_eq!(recorder.emissions.get(), 1);
    assert_eq!(recorder.completed.get(), 0);
    subscription.dispose();
}

#[test]
fn when_navigated_to_observable_handler_removes_watched_view_model_completes_once() {
    let router = Rc::new(ScriptedRoutingState::new());
    let screen = TestScreen::with_router(router.router.clone());
    let watched = routable(&screen);
    let other = routable(&screen);
    let recorder = FocusRecorder::new();
    let subscription = watched.when_navigated_to_observable().subscribe(recorder.clone());

    let scripted = router.clone();
    let removed = watched.clone();
    recorder.set_on_next_handler(move || scripted.remove(&removed));
    router.push(&watched);
    router.push(&other);
    router.remove(&other);

    assert_eq!(recorder.emissions.get(), 1);
    assert_eq!(recorder.completed.get(), 1);
    subscription.dispose();
}

#[test]
fn when_navigated_to_observable_removed_then_navigated_again_completes_once() {
    let screen = TestScreen::new();
    let watched = routable(&screen);
    let other = routable(&screen);
    let recorder = FocusRecorder::new();
    let subscription = watched.when_navigated_to_observable().subscribe(recorder.clone());

    navigate(&screen, &watched);
    navigate_back(&screen);
    assert_eq!(recorder.completed.get(), 1);
    let emissions_at_removal = recorder.emissions.get();

    navigate(&screen, &other);
    navigate(&screen, &watched);

    assert_eq!(recorder.completed.get(), 1);
    assert_eq!(recorder.emissions.get(), emissions_at_removal);
    subscription.dispose();
}

#[test]
fn when_navigating_from_observable_removed_then_navigated_again_completes_once() {
    let screen = TestScreen::new();
    let watched = routable(&screen);
    let other = routable(&screen);
    let recorder = FocusRecorder::new();
    let subscription = watched.when_navigating_from_observable().subscribe(recorder.clone());

    navigate(&screen, &watched);
    navigate_back(&screen);
    assert_eq!(recorder.completed.get(), 1);
    let emissions_at_removal = recorder.emissions.get();

    navigate(&screen, &other);
    navigate(&screen, &watched);

    assert_eq!(recorder.completed.get(), 1);
    assert_eq!(recorder.emissions.get(), emissions_at_removal);
    subscription.dispose();
}

#[test]
fn when_navigated_to_handle_disposed_disposes_the_active_scope() {
    let screen = TestScreen::new();
    let watched = routable(&screen);
    let scope_disposals = Rc::new(Cell::new(0));
    let disposals = scope_disposals.clone();
    let handle = watched.when_navigated_to(move || {
        let disposals = disposals.clone();
        Disposable::create(move || disposals.set(disposals.get() + 1))
    });

    navigate(&screen, &watched);
    assert_eq!(scope_disposals.get(), 0);

    handle.dispose();
    assert_eq!(scope_disposals.get(), 1);
}

#[test]
fn when_navigated_to_handle_disposed_stops_building_scopes() {
    let screen = TestScreen::new();
    let watched = routable(&screen);
    let other = routable(&screen);
    let scopes_built = Rc::new(Cell::new(0));
    let built = scopes_built.clone();
    let handle = watched.when_navigated_to(move || {
        built.set(built.get() + 1);
        Disposable::empty()
    });

    navigate(&screen, &watched);
    handle.dispose();
    navigate(&screen, &other);
    navigate(&screen, &watched);

    assert_eq!(scopes_built.get(), 1);
}

// Not ported: WhenNavigatedToObservable_DisposalRacesRemoval_CompletesAtMostOnce - real threads

#[test]
fn when_navigated_to_observable_many_subscriptions_each_completes_exactly_once() {
    let screen = TestScreen::new();
    let watched = routable(&screen);
    navigate(&screen, &watched);
    let recorders: Vec<Rc<FocusRecorder>> = (0..CONCURRENT_SUBSCRIPTION_COUNT).map(|_| FocusRecorder::new()).collect();
    let handles: Vec<Rc<dyn IDisposable>> = recorders.iter().map(|recorder| watched.when_navigated_to_observable().subscribe(recorder.clone())).collect();

    navigate_back(&screen);

    let completed_exactly_once = recorders.iter().filter(|recorder| recorder.completed.get() == 1).count();
    for handle in handles {
        handle.dispose();
    }
    assert_eq!(completed_exactly_once, CONCURRENT_SUBSCRIPTION_COUNT);
}

#[test]
fn when_navigated_to_observable_already_on_top_emits_on_subscribe() {
    let screen = TestScreen::new();
    let watched = routable(&screen);
    navigate(&screen, &watched);
    let recorder = FocusRecorder::new();

    let subscription = watched.when_navigated_to_observable().subscribe(recorder.clone());

    assert_eq!(recorder.emissions.get(), 1);
    subscription.dispose();
}

#[test]
fn when_navigated_to_observable_replace_keeps_count_does_not_emit() {
    let screen = TestScreen::new();
    let watched = routable(&screen);
    let other = routable(&screen);
    navigate(&screen, &other);
    let recorder = FocusRecorder::new();
    let subscription = watched.when_navigated_to_observable().subscribe(recorder.clone());

    screen.router.navigation_stack().set(0, watched.clone());

    assert_eq!(recorder.emissions.get(), 0);
    assert_eq!(recorder.completed.get(), 0);
    subscription.dispose();
}

#[test]
fn focus_observables_stack_stream_fails_forward_error_once() {
    let router = ScriptedRoutingState::new();
    let screen = TestScreen::with_router(router.router.clone());
    let watched = routable(&screen);
    let arrivals = FocusRecorder::new();
    let departures = FocusRecorder::new();
    let arrival_subscription = watched.when_navigated_to_observable().subscribe(arrivals.clone());
    let departure_subscription = watched.when_navigating_from_observable().subscribe(departures.clone());

    router.changes.on_error(Rc::new(StackFailed));

    assert_eq!(arrivals.errors.get(), 1);
    assert_eq!(departures.errors.get(), 1);
    arrival_subscription.dispose();
    departure_subscription.dispose();
}

#[test]
fn focus_observables_stack_stream_completes_complete() {
    let router = ScriptedRoutingState::new();
    let screen = TestScreen::with_router(router.router.clone());
    let watched = routable(&screen);
    let arrivals = FocusRecorder::new();
    let subscription = watched.when_navigated_to_observable().subscribe(arrivals.clone());

    router.changes.on_completed();

    assert_eq!(arrivals.completed.get(), 1);
    subscription.dispose();
}

#[test]
fn when_navigated_to_already_on_top_builds_scope_immediately() {
    let screen = TestScreen::new();
    let watched = routable(&screen);
    navigate(&screen, &watched);
    let scopes_built = Rc::new(Cell::new(0));
    let built = scopes_built.clone();

    let handle = watched.when_navigated_to(move || {
        built.set(built.get() + 1);
        Disposable::empty()
    });

    assert_eq!(scopes_built.get(), 1);
    handle.dispose();
}

#[test]
fn when_navigated_to_same_size_change_error_and_completion_keep_scope() {
    let failing = ScriptedRoutingState::new();
    let completing = ScriptedRoutingState::new();
    let failing_screen = TestScreen::with_router(failing.router.clone());
    let completing_screen = TestScreen::with_router(completing.router.clone());
    let failing_watched = routable(&failing_screen);
    let completing_watched = routable(&completing_screen);
    let scope_disposals = Rc::new(Cell::new(0));
    let (failing_disposals, completing_disposals) = (scope_disposals.clone(), scope_disposals.clone());
    let failing_handle = failing_watched.when_navigated_to(move || {
        let disposals = failing_disposals.clone();
        Disposable::create(move || disposals.set(disposals.get() + 1))
    });
    let completing_handle = completing_watched.when_navigated_to(move || {
        let disposals = completing_disposals.clone();
        Disposable::create(move || disposals.set(disposals.get() + 1))
    });

    failing.push(&failing_watched);
    completing.push(&completing_watched);
    failing.changes.on_next(vec![failing_watched.clone() as Rc<dyn IRoutableViewModel>]);
    failing.changes.on_error(Rc::new(StackFailed));
    completing.changes.on_completed();

    assert_eq!(scope_disposals.get(), 0);
    failing_handle.dispose();
    completing_handle.dispose();
}
