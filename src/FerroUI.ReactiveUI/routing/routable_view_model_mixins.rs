//! `RoutableViewModelMixins` (`ReactiveUI.Shared/Routing/RoutableViewModelMixins.cs`).

use super::routing_state::{NavigationStackSnapshot, RoutableViewModel};
use crate::{IRoutableViewModel, RoutingState};
use ferroui_reactive::disposables::{Disposable, SerialDisposable, SingleAssignmentDisposable};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::Cell;
use std::rc::{Rc, Weak};

/// The navigation focus of a routable view model: what to do while it is the current view model
/// of its screen's router (`RoutableViewModelMixins`), on its shared handle.
///
/// The router is the one of [`host_screen`](IRoutableViewModel::host_screen); a view model
/// without a screen panics (upstream's `NullReferenceException`).
// Deviation (DEVIATIONS.md, ReactiveUI): the observables and the scope know the view model by its
// address and the router weakly, so that a view model that keeps them does not keep itself or
// its router alive.
pub trait RoutableViewModelMixins {
    /// Runs `on_navigated_to` each time the view model becomes the topmost of the stack, and
    /// disposes what it returned when the view model loses the top (`WhenNavigatedTo`).
    ///
    /// The returned disposable stops watching and disposes the scope in force.
    fn when_navigated_to(&self, on_navigated_to: impl Fn() -> Rc<dyn IDisposable> + 'static) -> Rc<dyn IDisposable>;

    /// Ticks each time the view model becomes the topmost of the stack, at once when it already
    /// is; completes when the view model is removed from the stack (`WhenNavigatedToObservable`).
    /// A view model removed and reused later is watched again with a new call.
    fn when_navigated_to_observable(&self) -> Rc<dyn IObservable<()>>;

    /// Ticks each time the view model stops being the topmost of the stack; completes when the
    /// view model is removed from the stack (`WhenNavigatingFromObservable`).
    fn when_navigating_from_observable(&self) -> Rc<dyn IObservable<()>>;
}

impl<V: IRoutableViewModel + ?Sized> RoutableViewModelMixins for Rc<V> {
    fn when_navigated_to(&self, on_navigated_to: impl Fn() -> Rc<dyn IDisposable> + 'static) -> Rc<dyn IDisposable> {
        let router = router_of(&**self);
        let scope = Rc::new(NavigationFocusScope {
            item: address_of(self),
            on_navigated_to: Box::new(on_navigated_to),
            scope: SerialDisposable::new(),
            subscription: SingleAssignmentDisposable::new(),
            previous_count: Cell::new(0),
        });
        scope.clone().run(&router);
        scope
    }

    fn when_navigated_to_observable(&self) -> Rc<dyn IObservable<()>> {
        let router = router_of(&**self);
        Rc::new(NavigationFocusObservable { router: Rc::downgrade(&router), item: address_of(self), transition: NavigationFocusTransition::Arrival })
    }

    fn when_navigating_from_observable(&self) -> Rc<dyn IObservable<()>> {
        let router = router_of(&**self);
        Rc::new(NavigationFocusObservable { router: Rc::downgrade(&router), item: address_of(self), transition: NavigationFocusTransition::Departure })
    }
}

/// The router of the screen of the view model (`item.HostScreen.Router`).
fn router_of<V: IRoutableViewModel + ?Sized>(item: &V) -> Rc<RoutingState> {
    match item.host_screen() {
        Some(screen) => screen.router(),
        None => panic!("The routable view model has no host screen, and therefore no router to watch"),
    }
}

/// The address of a view model, which identifies it (`ReferenceEquals`).
fn address_of<V: ?Sized>(item: &Rc<V>) -> *const () {
    Rc::as_ptr(item) as *const ()
}

/// The address of a view model of the stack.
fn address_of_entry(entry: &RoutableViewModel) -> *const () {
    Rc::as_ptr(entry) as *const ()
}

/// The view model on top of a copy of the stack, by address.
fn top_of(stack: &[RoutableViewModel]) -> Option<*const ()> {
    RoutingState::top(stack).as_ref().map(address_of_entry)
}

/// Which side of a change of the focus an observable reports (`NavigationFocusTransition`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NavigationFocusTransition {
    /// The watched view model became the topmost one (`Arrival`).
    Arrival,
    /// The watched view model stopped being the topmost one (`Departure`).
    Departure,
}

/// Ticks on one side of a change of the focus of the watched view model, and completes when the
/// view model is removed from the stack (`NavigationFocusObservable`).
struct NavigationFocusObservable {
    router: Weak<RoutingState>,
    item: *const (),
    transition: NavigationFocusTransition,
}

impl IObservable<()> for NavigationFocusObservable {
    fn subscribe(&self, observer: Rc<dyn IObserver<()>>) -> Rc<dyn IDisposable> {
        let Some(router) = self.router.upgrade() else {
            return Disposable::empty();
        };
        let sink = Rc::new(FocusSink {
            downstream: observer,
            item: self.item,
            transition: self.transition,
            subscription: SingleAssignmentDisposable::new(),
            previous_current: Cell::new(None),
            previous_count: Cell::new(0),
            was_in_stack: Cell::new(false),
            stopped: Cell::new(false),
        });
        sink.clone().run(&router);
        sink
    }
}

/// Ticks on each matching change of the focus, and completes once the watched view model is
/// removed (`NavigationFocusObservable.Sink`).
struct FocusSink {
    downstream: Rc<dyn IObserver<()>>,
    item: *const (),
    transition: NavigationFocusTransition,
    /// The subscription to the copies of the stack (`_subscription`).
    subscription: Rc<SingleAssignmentDisposable>,
    /// The current view model at the previous change; for departures only (`_previousCurrent`).
    previous_current: Cell<Option<*const ()>>,
    /// The count of the stack at the previous change (`_previousCount`).
    previous_count: Cell<usize>,
    /// Whether the watched view model was in the stack at the previous change (`_wasInStack`).
    was_in_stack: Cell<bool>,
    /// Whether the downstream has ended (`_stopped`).
    stopped: Cell<bool>,
}

impl FocusSink {
    /// Judges the stack as it is, then subscribes to its later changes (`Run`).
    fn run(self: Rc<Self>, router: &Rc<RoutingState>) {
        self.on_stack(&RoutingState::snapshot(&router.navigation_stack()));
        let subscription = router.navigation_stack_changed().subscribe(self.clone());
        self.subscription.set_disposable(Some(subscription));
    }

    fn on_stack(&self, value: &[RoutableViewModel]) {
        if self.stopped.get() {
            return;
        }
        // Every piece of bookkeeping advances before anything is handed downstream, so a handler
        // that navigates synchronously sees state that is already current.
        let is_in_stack = value.iter().any(|entry| address_of_entry(entry) == self.item);
        let removed = self.was_in_stack.get() && !is_in_stack;
        self.was_in_stack.set(is_in_stack);
        let count_changed = value.len() != self.previous_count.get();
        self.previous_count.set(value.len());
        if removed {
            self.complete();
            return;
        }
        if !count_changed || !self.has_transitioned(top_of(value)) {
            return;
        }
        self.downstream.on_next(());
    }

    /// Whether this change is the transition watched for (`HasTransitioned`).
    fn has_transitioned(&self, current: Option<*const ()>) -> bool {
        match self.transition {
            NavigationFocusTransition::Arrival => current == Some(self.item),
            NavigationFocusTransition::Departure => {
                let departed = self.previous_current.get() == Some(self.item);
                self.previous_current.set(current);
                departed
            }
        }
    }

    /// Completes the downstream and disposes the subscription, once (`Complete`).
    fn complete(&self) {
        if self.stopped.replace(true) {
            return;
        }
        self.downstream.on_completed();
        self.subscription.dispose();
    }
}

impl IObserver<NavigationStackSnapshot> for FocusSink {
    fn on_next(&self, value: NavigationStackSnapshot) {
        self.on_stack(&value);
    }

    fn on_error(&self, error: ObservableError) {
        if self.stopped.replace(true) {
            return;
        }
        self.downstream.on_error(error);
        self.subscription.dispose();
    }

    fn on_completed(&self) {
        self.complete();
    }
}

impl IDisposable for FocusSink {
    fn dispose(&self) {
        self.subscription.dispose();
    }
}

/// Runs the scope of the caller for exactly as long as the watched view model is the topmost
/// one, replacing it on every change of the size of the stack (`NavigationFocusScope`).
struct NavigationFocusScope {
    item: *const (),
    on_navigated_to: Box<dyn Fn() -> Rc<dyn IDisposable>>,
    /// The scope in force (`_scope`).
    scope: Rc<SerialDisposable>,
    /// The subscription to the copies of the stack (`_subscription`).
    subscription: Rc<SingleAssignmentDisposable>,
    /// The count of the stack at the previous change (`_previousCount`).
    previous_count: Cell<usize>,
}

impl NavigationFocusScope {
    /// Judges the stack as it is, then subscribes to its later changes (`Run`).
    fn run(self: Rc<Self>, router: &Rc<RoutingState>) {
        self.on_stack(&RoutingState::snapshot(&router.navigation_stack()));
        let subscription = router.navigation_stack_changed().subscribe(self.clone());
        self.subscription.set_disposable(Some(subscription));
    }

    fn on_stack(&self, value: &[RoutableViewModel]) {
        if value.len() == self.previous_count.get() {
            return;
        }
        self.previous_count.set(value.len());
        // The old scope is torn down before the new one is built, so that the two never overlap.
        self.scope.set_disposable(None);
        if top_of(value) != Some(self.item) {
            return;
        }
        self.scope.set_disposable(Some((self.on_navigated_to)()));
    }
}

impl IObserver<NavigationStackSnapshot> for NavigationFocusScope {
    fn on_next(&self, value: NavigationStackSnapshot) {
        self.on_stack(&value);
    }

    /// The stream of the stack does not fail in practice; a failure leaves the scope in force,
    /// since the view model has not lost the focus.
    fn on_error(&self, _error: ObservableError) {}

    /// The end of the stream does not mean that the focus was lost: the scope stays in force until
    /// the caller disposes the handle.
    fn on_completed(&self) {}
}

impl IDisposable for NavigationFocusScope {
    fn dispose(&self) {
        self.subscription.dispose();
        self.scope.dispose();
    }
}
