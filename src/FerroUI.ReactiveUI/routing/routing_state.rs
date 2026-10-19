//! `RoutingState` (`ReactiveUI.Shared/Routing/RoutingState.cs`).

use crate::{IRoutableViewModel, ReactiveCommand, ReactiveObject, RxSchedulers};
use ferroui_reactive::concurrency::IScheduler;
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::collections::object_model::ObservableCollection;
use ferroui_system::collections::specialized::NotifyCollectionChangedEventArgs;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::{Rc, Weak};

/// A view model shared on the navigation stack.
pub type RoutableViewModel = Rc<dyn IRoutableViewModel>;

/// The navigation stack: the view models, oldest first, the current one last.
pub type NavigationStack = ObservableCollection<RoutableViewModel>;

/// A copy of the navigation stack, oldest first (`IReadOnlyList<IRoutableViewModel>`); later
/// navigation does not change it.
pub type NavigationStackSnapshot = Vec<RoutableViewModel>;

/// The command that navigates back, emitting the new current view model.
pub type NavigateBackCommand = ReactiveCommand<(), Option<RoutableViewModel>>;

/// The command that navigates to a view model, emitting it.
pub type NavigateCommand = ReactiveCommand<Option<RoutableViewModel>, RoutableViewModel>;

/// The stack of view models of a screen, and the commands that navigate it (`RoutingState`).
///
/// An [`IScreen`](crate::IScreen) holds one to coordinate the navigation of an application of
/// several pages. The stack is last in, first out: [`navigate`](Self::navigate) pushes a view
/// model, [`navigate_back`](Self::navigate_back) pops one, and
/// [`navigate_and_reset`](Self::navigate_and_reset) replaces the whole stack.
///
/// Every notification of the navigation is an observable of values:
/// [`current_view_model`](Self::current_view_model) gives the view model on top of the stack,
/// [`navigation_stack_changed`](Self::navigation_stack_changed) gives the whole stack after each
/// change, and [`can_navigate_back`](Self::can_navigate_back) whether there is a view model to go
/// back to.
///
/// The router is shared (`Rc<RoutingState>`); its observables and commands refer to it weakly, so
/// that they do not keep it alive.
// Deviation (DEVIATIONS.md, ReactiveUI): the parameter of `Navigate` and `NavigateAndReset` is an
// `Option`, upstream's nullable reference (a command parameter has a default value, and `None` is
// the error upstream throws for `null`); the result of `NavigateBack` is an `Option`, `None` when
// the stack is left empty (upstream emits `null!`). The serialization attributes (`[DataContract]`,
// `[DataMember]`, `[OnDeserialized]`) are not ported: a router is not serialized by reflection.
pub struct RoutingState {
    reactive: ReactiveObject,
    /// The scheduler of the navigation notifications (`_scheduler`).
    scheduler: Rc<dyn IScheduler>,
    navigation_stack: RefCell<NavigationStack>,
    navigate_back: RefCell<Option<NavigateBackCommand>>,
    navigate: RefCell<Option<NavigateCommand>>,
    navigate_and_reset: RefCell<Option<NavigateCommand>>,
    current_view_model: RefCell<Rc<dyn IObservable<Option<RoutableViewModel>>>>,
    navigation_stack_changed: RefCell<Rc<dyn IObservable<NavigationStackSnapshot>>>,
    can_navigate_back: RefCell<Rc<dyn IObservable<bool>>>,
}

crate::reactive_object!(RoutingState);

impl fmt::Debug for RoutingState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RoutingState {{ navigation_stack_count: {} }}", self.navigation_stack.borrow().count())
    }
}

impl RoutingState {
    /// A router with an empty stack, whose navigation notifies on
    /// [`RxSchedulers::main_thread_scheduler`] (`RoutingState()`).
    pub fn new() -> Rc<Self> {
        Self::with_scheduler(None)
    }

    /// A router whose navigation notifies on `scheduler`, or on
    /// [`RxSchedulers::main_thread_scheduler`] when `None` (`RoutingState(IScheduler?)`).
    pub fn with_scheduler(scheduler: Option<Rc<dyn IScheduler>>) -> Rc<Self> {
        let this = Rc::new_cyclic(|this: &Weak<RoutingState>| {
            let navigation_stack_changed: Rc<dyn IObservable<NavigationStackSnapshot>> = Rc::new(NavigationStackChangedObservable { owner: this.clone() });
            let current_view_model: Rc<dyn IObservable<Option<RoutableViewModel>>> = Rc::new(CurrentViewModelObservable { owner: this.clone() });
            let can_navigate_back: Rc<dyn IObservable<bool>> = Rc::new(CanNavigateBackObservable { owner: this.clone() });
            RoutingState {
                reactive: ReactiveObject::new(),
                scheduler: scheduler.unwrap_or_else(RxSchedulers::main_thread_scheduler),
                navigation_stack: RefCell::new(NavigationStack::new()),
                navigate_back: RefCell::new(None),
                navigate: RefCell::new(None),
                navigate_and_reset: RefCell::new(None),
                current_view_model: RefCell::new(current_view_model),
                navigation_stack_changed: RefCell::new(navigation_stack_changed),
                can_navigate_back: RefCell::new(can_navigate_back),
            }
        });
        this.setup_rx();
        this
    }

    /// The navigation stack, the last view model being the current one (`NavigationStack`).
    pub fn navigation_stack(&self) -> NavigationStack {
        self.navigation_stack.borrow().clone()
    }

    /// Replaces the navigation stack (`NavigationStack { set; }`). The observables observe the
    /// stack the router has when they are subscribed.
    pub fn set_navigation_stack(&self, value: NavigationStack) {
        *self.navigation_stack.borrow_mut() = value;
    }

    /// Navigates back to the previous view model of the stack, and emits the new current one
    /// (`NavigateBack`). It can execute while the stack holds at least two view models.
    pub fn navigate_back(&self) -> NavigateBackCommand {
        self.navigate_back.borrow().clone().expect("the commands of the router are set up when it is created")
    }

    /// Replaces [`navigate_back`](Self::navigate_back) (its protected setter).
    pub fn set_navigate_back(&self, value: NavigateBackCommand) {
        *self.navigate_back.borrow_mut() = Some(value);
    }

    /// Pushes a view model on the stack, and emits it once the navigation scheduler delivers it
    /// (`Navigate`). Executing it with `None` fails, as upstream fails for `null`.
    pub fn navigate(&self) -> NavigateCommand {
        self.navigate.borrow().clone().expect("the commands of the router are set up when it is created")
    }

    /// Replaces [`navigate`](Self::navigate) (its protected setter).
    pub fn set_navigate(&self, value: NavigateCommand) {
        *self.navigate.borrow_mut() = Some(value);
    }

    /// Replaces the whole stack with a view model, clearing the history of the navigation
    /// (`NavigateAndReset`).
    pub fn navigate_and_reset(&self) -> NavigateCommand {
        self.navigate_and_reset.borrow().clone().expect("the commands of the router are set up when it is created")
    }

    /// Replaces [`navigate_and_reset`](Self::navigate_and_reset) (its protected setter).
    pub fn set_navigate_and_reset(&self, value: NavigateCommand) {
        *self.navigate_and_reset.borrow_mut() = Some(value);
    }

    /// The view model on top of the stack: the current one on subscription, then again after
    /// every change of the stack; `None` while the stack is empty (`CurrentViewModel`).
    pub fn current_view_model(&self) -> Rc<dyn IObservable<Option<RoutableViewModel>>> {
        self.current_view_model.borrow().clone()
    }

    /// Replaces [`current_view_model`](Self::current_view_model) (its protected setter).
    pub fn set_current_view_model(&self, value: Rc<dyn IObservable<Option<RoutableViewModel>>>) {
        *self.current_view_model.borrow_mut() = value;
    }

    /// A copy of the whole stack after each change of [`navigation_stack`](Self::navigation_stack),
    /// oldest first; nothing on subscription (`NavigationStackChanged`).
    pub fn navigation_stack_changed(&self) -> Rc<dyn IObservable<NavigationStackSnapshot>> {
        self.navigation_stack_changed.borrow().clone()
    }

    /// Replaces [`navigation_stack_changed`](Self::navigation_stack_changed) (its protected
    /// setter); [`current_view_model`](Self::current_view_model) and
    /// [`can_navigate_back`](Self::can_navigate_back) follow the replacement.
    pub fn set_navigation_stack_changed(&self, value: Rc<dyn IObservable<NavigationStackSnapshot>>) {
        *self.navigation_stack_changed.borrow_mut() = value;
    }

    /// Whether there is a view model to go back to, which holds while the stack has at least two:
    /// the current answer on subscription, then each change of it (`CanNavigateBack`).
    pub fn can_navigate_back(&self) -> Rc<dyn IObservable<bool>> {
        self.can_navigate_back.borrow().clone()
    }

    /// Replaces [`can_navigate_back`](Self::can_navigate_back) (its protected setter).
    pub fn set_can_navigate_back(&self, value: Rc<dyn IObservable<bool>>) {
        *self.can_navigate_back.borrow_mut() = value;
    }

    /// A copy of `stack`, oldest first (`Snapshot`).
    pub(crate) fn snapshot(stack: &NavigationStack) -> NavigationStackSnapshot {
        stack.to_vec()
    }

    /// The view model on top of `stack` (`Top`).
    pub(crate) fn top(stack: &[RoutableViewModel]) -> Option<RoutableViewModel> {
        stack.last().cloned()
    }

    /// Creates the commands (`SetupRx`).
    fn setup_rx(self: &Rc<Self>) {
        let navigate_scheduler = self.scheduler.clone();
        let owner = Rc::downgrade(self);
        let back_scheduler = navigate_scheduler.clone();
        let navigate_back = ReactiveCommand::try_create_from_observable_with(
            move || {
                let owner = owner.upgrade().ok_or_else(|| routing_error("The routing state was dropped"))?;
                let stack = owner.navigation_stack();
                let count = stack.count();
                if count == 0 {
                    return Err(routing_error("The navigation stack is empty: there is no view model to remove"));
                }
                stack.remove_at(count - 1);
                let current = if stack.count() > 0 { Some(stack.get(stack.count() - 1)) } else { None };
                Ok(scheduled_value(current, back_scheduler.clone()))
            },
            Some(self.can_navigate_back()),
            None,
        );
        let owner = Rc::downgrade(self);
        let navigate = ReactiveCommand::try_create_from_observable_param(move |view_model: Option<RoutableViewModel>| {
            let Some(view_model) = view_model else {
                return Err(routing_error("Navigate must be called on an IRoutableViewModel"));
            };
            let owner = owner.upgrade().ok_or_else(|| routing_error("The routing state was dropped"))?;
            owner.navigation_stack().add(view_model.clone());
            Ok(scheduled_value(view_model, navigate_scheduler.clone()))
        });
        let owner = Rc::downgrade(self);
        let navigate_and_reset = ReactiveCommand::try_create_from_observable_param(move |view_model: Option<RoutableViewModel>| {
            let owner = owner.upgrade().ok_or_else(|| routing_error("The routing state was dropped"))?;
            owner.navigation_stack().clear();
            Ok(owner.navigate().execute(view_model))
        });
        self.set_navigate_back(navigate_back);
        self.set_navigate(navigate);
        self.set_navigate_and_reset(navigate_and_reset);
    }
}

/// One value, delivered on `scheduler` (`ScheduledValueObservable<T>`, upstream's
/// `Observable.Return(value).ObserveOn(scheduler)`).
fn scheduled_value<T: Clone + 'static>(value: T, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<T>> {
    Observable::return_(value).observe_on(scheduler)
}

/// The error of a navigation that cannot happen (upstream's `InvalidOperationException` and
/// `ArgumentOutOfRangeException`).
#[derive(Debug)]
struct RoutingError {
    message: &'static str,
}

impl fmt::Display for RoutingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message)
    }
}

impl std::error::Error for RoutingError {}

fn routing_error(message: &'static str) -> ObservableError {
    Rc::new(RoutingError { message })
}

/// A copy of the stack of the router after each change of it; the stack is the one the router
/// has when an observer subscribes (`NavigationStackChangedObservable`).
struct NavigationStackChangedObservable {
    owner: Weak<RoutingState>,
}

impl IObservable<NavigationStackSnapshot> for NavigationStackChangedObservable {
    fn subscribe(&self, observer: Rc<dyn IObserver<NavigationStackSnapshot>>) -> Rc<dyn IDisposable> {
        let Some(owner) = self.owner.upgrade() else {
            return Disposable::empty();
        };
        let stack = owner.navigation_stack();
        let observed = stack.clone();
        let token = stack.add_collection_changed(Rc::new(move |_: &NotifyCollectionChangedEventArgs<'_, RoutableViewModel>| {
            observer.on_next(RoutingState::snapshot(&observed));
        }));
        Disposable::create(move || {
            stack.remove_collection_changed(token);
        })
    }
}

/// The top of the stack on subscription, then the top of each copy of
/// [`navigation_stack_changed`](RoutingState::navigation_stack_changed)
/// (`CurrentViewModelObservable`).
struct CurrentViewModelObservable {
    owner: Weak<RoutingState>,
}

impl IObservable<Option<RoutableViewModel>> for CurrentViewModelObservable {
    fn subscribe(&self, observer: Rc<dyn IObserver<Option<RoutableViewModel>>>) -> Rc<dyn IDisposable> {
        let Some(owner) = self.owner.upgrade() else {
            return Disposable::empty();
        };
        observer.on_next(RoutingState::top(&RoutingState::snapshot(&owner.navigation_stack())));
        owner.navigation_stack_changed().subscribe(Rc::new(CurrentViewModelSink { downstream: observer }))
    }
}

/// Maps each copy of the stack to its top (`CurrentViewModelObservable.Sink`).
struct CurrentViewModelSink {
    downstream: Rc<dyn IObserver<Option<RoutableViewModel>>>,
}

impl IObserver<NavigationStackSnapshot> for CurrentViewModelSink {
    fn on_next(&self, value: NavigationStackSnapshot) {
        self.downstream.on_next(RoutingState::top(&value));
    }

    fn on_error(&self, error: ObservableError) {
        self.downstream.on_error(error);
    }

    fn on_completed(&self) {
        self.downstream.on_completed();
    }
}

/// The fewest view models the stack holds for there to be one to go back to.
const MINIMUM_COUNT_TO_GO_BACK: usize = 2;

/// Whether the stack holds at least two view models on subscription, then each time a copy of
/// [`navigation_stack_changed`](RoutingState::navigation_stack_changed) changes the answer
/// (`CanNavigateBackObservable`).
struct CanNavigateBackObservable {
    owner: Weak<RoutingState>,
}

impl IObservable<bool> for CanNavigateBackObservable {
    fn subscribe(&self, observer: Rc<dyn IObserver<bool>>) -> Rc<dyn IDisposable> {
        let Some(owner) = self.owner.upgrade() else {
            return Disposable::empty();
        };
        let can_go_back = owner.navigation_stack().count() >= MINIMUM_COUNT_TO_GO_BACK;
        observer.on_next(can_go_back);
        owner.navigation_stack_changed().subscribe(Rc::new(CanNavigateBackSink { downstream: observer, last: Cell::new(can_go_back) }))
    }
}

/// Forwards the answer when it differs from the last one (`CanNavigateBackObservable.Sink`).
struct CanNavigateBackSink {
    downstream: Rc<dyn IObserver<bool>>,
    last: Cell<bool>,
}

impl IObserver<NavigationStackSnapshot> for CanNavigateBackSink {
    fn on_next(&self, value: NavigationStackSnapshot) {
        let can_go_back = value.len() >= MINIMUM_COUNT_TO_GO_BACK;
        if can_go_back == self.last.get() {
            return;
        }
        self.last.set(can_go_back);
        self.downstream.on_next(can_go_back);
    }

    fn on_error(&self, error: ObservableError) {
        self.downstream.on_error(error);
    }

    fn on_completed(&self) {
        self.downstream.on_completed();
    }
}
