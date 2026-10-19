//! `RoutingStateObservablesTests` (`ReactiveUI.Tests/Routing/RoutingStateObservablesTests.cs`).

use ferroui_reactive::concurrency::ImmediateScheduler;
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::*;
use ferroui_system::{IDisposable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;

const FIRST: &str = "first";
const SECOND: &str = "second";
const THIRD: &str = "third";

/// A routable view model named by its segment (`PageViewModel`).
struct PageViewModel {
    reactive: ReactiveObject,
    segment: String,
}
reactive_object!(PageViewModel);

impl IRoutableViewModel for PageViewModel {
    fn url_path_segment(&self) -> Option<String> {
        Some(self.segment.clone())
    }

    fn host_screen(&self) -> Option<Rc<dyn IScreen>> {
        None
    }
}

fn page(segment: &str) -> Rc<dyn IRoutableViewModel> {
    Rc::new(PageViewModel { reactive: ReactiveObject::new(), segment: segment.to_owned() })
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
}

impl Drop for ScriptedRoutingState {
    fn drop(&mut self) {
        self.changes.dispose();
    }
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

fn immediate_router() -> Rc<RoutingState> {
    RoutingState::with_scheduler(Some(ImmediateScheduler::instance()))
}

fn navigate(router: &RoutingState, view_model: &Rc<dyn IRoutableViewModel>) {
    let _ = router.navigate().execute(Some(view_model.clone())).subscribe_ignore();
}

fn navigate_back(router: &RoutingState) {
    let _ = router.navigate_back().execute(()).subscribe_ignore();
}

fn navigate_and_reset(router: &RoutingState, view_model: &Rc<dyn IRoutableViewModel>) {
    let _ = router.navigate_and_reset().execute(Some(view_model.clone())).subscribe_ignore();
}

fn collect<T: 'static>(source: Rc<dyn ferroui_system::IObservable<T>>) -> (Rc<RefCell<Vec<T>>>, Rc<dyn IDisposable>) {
    let items = Rc::new(RefCell::new(Vec::new()));
    let log = items.clone();
    let subscription = source.subscribe_next(move |value| log.borrow_mut().push(value));
    (items, subscription)
}

#[test]
fn navigation_stack_changed_on_subscribe_emits_nothing() {
    let router = immediate_router();
    router.navigation_stack().add(page(FIRST));

    let (stacks, subscription) = collect(router.navigation_stack_changed());

    assert!(stacks.borrow().is_empty());
    subscription.dispose();
}

#[test]
fn navigation_stack_changed_navigation_emits_the_whole_stack() {
    let router = immediate_router();
    let first = page(FIRST);
    let second = page(SECOND);
    let (stacks, subscription) = collect(router.navigation_stack_changed());

    navigate(&router, &first);
    navigate(&router, &second);
    navigate_back(&router);

    let stacks = stacks.borrow();
    assert_eq!(stacks.len(), 3);
    assert_eq!(stacks[0], vec![first.clone()]);
    assert_eq!(stacks[1], vec![first.clone(), second.clone()]);
    assert_eq!(stacks[2], vec![first.clone()]);
    subscription.dispose();
}

#[test]
fn navigation_stack_changed_navigate_and_reset_emits_empty_then_new_stack() {
    let router = immediate_router();
    let reset = page(SECOND);
    navigate(&router, &page(FIRST));
    let (stacks, subscription) = collect(router.navigation_stack_changed());

    navigate_and_reset(&router, &reset);

    let stacks = stacks.borrow();
    assert_eq!(stacks.len(), 2);
    assert!(stacks[0].is_empty());
    assert_eq!(stacks[1], vec![reset.clone()]);
    subscription.dispose();
}

#[test]
fn navigation_stack_changed_snapshot_is_not_changed_by_later_navigation() {
    let router = immediate_router();
    let first = page(FIRST);
    let first_snapshot: Rc<RefCell<Option<NavigationStackSnapshot>>> = Rc::new(RefCell::new(None));
    let kept = first_snapshot.clone();
    let subscription = router.navigation_stack_changed().subscribe_next(move |stack| {
        kept.borrow_mut().get_or_insert(stack);
    });

    navigate(&router, &first);
    navigate(&router, &page(SECOND));

    assert_eq!(*first_snapshot.borrow(), Some(vec![first.clone()]));
    subscription.dispose();
}

#[test]
fn navigation_stack_changed_disposed_stops_emitting() {
    let router = immediate_router();
    let (stacks, subscription) = collect(router.navigation_stack_changed());

    navigate(&router, &page(FIRST));
    subscription.dispose();
    navigate(&router, &page(SECOND));

    assert_eq!(stacks.borrow().len(), 1);
}

#[test]
fn navigation_stack_changed_stack_replaced_observes_the_stack_at_subscribe() {
    let router = immediate_router();
    let replacement = NavigationStack::new();
    router.set_navigation_stack(replacement.clone());
    let (stacks, subscription) = collect(router.navigation_stack_changed());

    let page = page(FIRST);
    replacement.add(page.clone());

    let stacks = stacks.borrow();
    assert_eq!(stacks.len(), 1);
    assert_eq!(stacks[0], vec![page.clone()]);
    subscription.dispose();
}

#[test]
fn current_view_model_emits_on_subscribe_and_after_every_change() {
    let router = immediate_router();
    let first = page(FIRST);
    let second = page(SECOND);
    let (current, subscription) = collect(router.current_view_model());

    navigate(&router, &first);
    navigate(&router, &second);
    navigate_and_reset(&router, &first);

    assert_eq!(*current.borrow(), vec![None, Some(first.clone()), Some(second.clone()), None, Some(first.clone())]);
    subscription.dispose();
}

#[test]
fn current_view_model_non_empty_stack_emits_top_on_subscribe() {
    let router = immediate_router();
    let second = page(SECOND);
    router.navigation_stack().add(page(FIRST));
    router.navigation_stack().add(second.clone());

    let (current, subscription) = collect(router.current_view_model());

    assert_eq!(current.borrow().last().cloned().flatten(), Some(second));
    subscription.dispose();
}

#[test]
fn can_navigate_back_emits_on_subscribe_then_only_changes() {
    let router = immediate_router();
    let (answers, subscription) = collect(router.can_navigate_back());

    navigate(&router, &page(FIRST));
    navigate(&router, &page(SECOND));
    navigate(&router, &page(THIRD));
    navigate_back(&router);
    navigate_back(&router);

    assert_eq!(*answers.borrow(), [false, true, false]);
    subscription.dispose();
}

#[test]
fn can_navigate_back_two_view_models_emits_true_on_subscribe() {
    let router = immediate_router();
    router.navigation_stack().add(page(FIRST));
    router.navigation_stack().add(page(SECOND));

    let (answers, subscription) = collect(router.can_navigate_back());

    assert_eq!(*answers.borrow(), [true]);
    subscription.dispose();
}

#[test]
fn navigate_back_can_execute_follows_can_navigate_back() {
    let router = immediate_router();
    let (can_execute, subscription) = collect(router.navigate_back().can_execute());

    navigate(&router, &page(FIRST));
    navigate(&router, &page(SECOND));
    navigate_back(&router);

    let can_execute = can_execute.borrow();
    assert_eq!(can_execute.last(), Some(&false));
    assert!(can_execute.contains(&true));
    subscription.dispose();
}

#[test]
fn derived_observables_forward_error_and_completion() {
    let failing = ScriptedRoutingState::new();
    let completing = ScriptedRoutingState::new();
    let error: ObservableError = Rc::new(StackFailed);
    let current_error: Rc<RefCell<Option<ObservableError>>> = Rc::new(RefCell::new(None));
    let can_go_back_error: Rc<RefCell<Option<ObservableError>>> = Rc::new(RefCell::new(None));
    let current_completed = Rc::new(Cell::new(false));
    let can_go_back_completed = Rc::new(Cell::new(false));

    let seen = current_error.clone();
    let current_failing = failing.router.current_view_model().subscribe_next_error(|_| {}, move |error| *seen.borrow_mut() = Some(error));
    let seen = can_go_back_error.clone();
    let can_go_back_failing = failing.router.can_navigate_back().subscribe_next_error(|_| {}, move |error| *seen.borrow_mut() = Some(error));
    let seen = current_completed.clone();
    let current_completing = completing.router.current_view_model().subscribe_all(|_| {}, |_| {}, move || seen.set(true));
    let seen = can_go_back_completed.clone();
    let can_go_back_completing = completing.router.can_navigate_back().subscribe_all(|_| {}, |_| {}, move || seen.set(true));

    failing.changes.on_error(error.clone());
    completing.changes.on_completed();

    assert!(current_error.borrow().as_ref().is_some_and(|seen| Rc::ptr_eq(seen, &error)));
    assert!(can_go_back_error.borrow().as_ref().is_some_and(|seen| Rc::ptr_eq(seen, &error)));
    assert!(current_completed.get());
    assert!(can_go_back_completed.get());
    for subscription in [current_failing, can_go_back_failing, current_completing, can_go_back_completing] {
        subscription.dispose();
    }
}

#[test]
fn derived_observables_follow_the_stack_stream() {
    let router = ScriptedRoutingState::new();
    let first = page(FIRST);
    let second = page(SECOND);
    let (current, current_subscription) = collect(router.router.current_view_model());
    let (answers, answer_subscription) = collect(router.router.can_navigate_back());

    router.changes.on_next(vec![first.clone()]);
    router.changes.on_next(vec![first.clone(), second.clone()]);
    router.changes.on_next(vec![first.clone(), second.clone()]);
    router.changes.on_next(Vec::new());

    assert_eq!(*current.borrow(), vec![None, Some(first.clone()), Some(second.clone()), Some(second.clone()), None]);
    assert_eq!(*answers.borrow(), [false, true, false]);
    current_subscription.dispose();
    answer_subscription.dispose();
}

// Not ported: Observables_NullObserver_Throw - argument validation
