//! `CanActivateViewFetcherTests` (`ReactiveUI.Tests/Activation/CanActivateViewFetcherTests.cs`).

use crate::mocks::collect_items;
use ferroui_reactive::concurrency::ImmediateScheduler;
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::binding::interfaces::{IActivatableView, IViewFor};
use ferroui_reactiveui::*;
use ferroui_system::component_model::INotifyPropertyChanged;
use ferroui_system::{IObservable, IObserver};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A view model the views display (`TestViewModel`).
struct TestViewModel;

/// A view that says itself when it is activated (`TestCanActivateView`).
struct TestCanActivateView {
    reactive: ReactiveObject,
    activated: Rc<Subject<()>>,
    deactivated: Rc<Subject<()>>,
    view_model: RefCell<Option<Rc<TestViewModel>>>,
}
reactive_object!(TestCanActivateView);

impl TestCanActivateView {
    fn new() -> Rc<Self> {
        Rc::new(Self { reactive: ReactiveObject::new(), activated: Subject::new(), deactivated: Subject::new(), view_model: RefCell::new(None) })
    }

    fn activate(&self) {
        self.activated.on_next(());
    }

    fn deactivate(&self) {
        self.deactivated.on_next(());
    }
}

impl IActivatableView for TestCanActivateView {
    fn as_view_for(&self) -> Option<&dyn IViewFor> {
        Some(self)
    }

    fn as_notify_property_changed(self: Rc<Self>) -> Option<Rc<dyn INotifyPropertyChanged>> {
        Some(self)
    }

    fn as_can_activate(&self) -> Option<&dyn ICanActivate> {
        Some(self)
    }
}

impl IViewFor for TestCanActivateView {
    fn view_model(&self) -> Option<Rc<dyn Any>> {
        self.view_model.borrow().clone().map(|view_model| view_model as Rc<dyn Any>)
    }

    fn set_view_model(&self, view_model: Option<Rc<dyn Any>>) {
        *self.view_model.borrow_mut() = view_model.map(|view_model| view_model.downcast::<TestViewModel>().unwrap_or_else(|_| panic!("InvalidCastException")));
    }
}

impl ICanActivate for TestCanActivateView {
    fn activated(&self) -> Rc<dyn IObservable<()>> {
        self.activated.as_observable()
    }

    fn deactivated(&self) -> Rc<dyn IObservable<()>> {
        self.deactivated.as_observable()
    }
}

/// A view that does not say when it is activated (`TestNonActivatableView`).
struct TestNonActivatableView {
    view_model: RefCell<Option<Rc<TestViewModel>>>,
}

impl IActivatableView for TestNonActivatableView {
    fn as_view_for(&self) -> Option<&dyn IViewFor> {
        Some(self)
    }
}

impl IViewFor for TestNonActivatableView {
    fn view_model(&self) -> Option<Rc<dyn Any>> {
        self.view_model.borrow().clone().map(|view_model| view_model as Rc<dyn Any>)
    }

    fn set_view_model(&self, view_model: Option<Rc<dyn Any>>) {
        *self.view_model.borrow_mut() = view_model.map(|view_model| view_model.downcast::<TestViewModel>().unwrap_or_else(|_| panic!("InvalidCastException")));
    }
}

/// The activation of `view` from the fetcher, delivered on the immediate scheduler.
fn activation_of(view: Rc<dyn IActivatableView>) -> Rc<dyn IObservable<bool>> {
    CanActivateViewFetcher::new().get_activation_for_view(&view).observe_on(ImmediateScheduler::instance())
}

#[test]
fn get_activation_for_view_activate_deactivate_cycle_emits_correct_sequence() {
    let view = TestCanActivateView::new();
    let results = collect_items(&activation_of(view.clone()));

    view.activate();
    view.deactivate();
    view.activate();

    assert_eq!(*results.borrow(), [true, false, true]);
}

#[test]
fn get_activation_for_view_multiple_activations_emits_each_time() {
    let view = TestCanActivateView::new();
    let results = collect_items(&activation_of(view.clone()));

    view.activate();
    view.activate();
    view.activate();

    assert_eq!(*results.borrow(), [true, true, true]);
}

#[test]
fn get_activation_for_view_multiple_deactivations_emits_each_time() {
    let view = TestCanActivateView::new();
    let results = collect_items(&activation_of(view.clone()));

    view.deactivate();
    view.deactivate();
    view.deactivate();

    assert_eq!(*results.borrow(), [false, false, false]);
}

#[test]
fn get_activation_for_view_with_activated_view_returns_true() {
    let view = TestCanActivateView::new();
    let result = Rc::new(Cell::new(None));
    let seen = result.clone();
    let _subscription = activation_of(view.clone()).subscribe_next(move |value| seen.set(Some(value)));

    view.activate();

    assert_eq!(result.get(), Some(true));
}

#[test]
fn get_activation_for_view_with_deactivated_view_returns_false() {
    let view = TestCanActivateView::new();
    let result = Rc::new(Cell::new(None));
    let seen = result.clone();
    let _subscription = activation_of(view.clone()).subscribe_next(move |value| seen.set(Some(value)));

    view.deactivate();

    assert_eq!(result.get(), Some(false));
}

#[test]
fn get_activation_for_view_with_non_i_can_activate_returns_false() {
    let view = Rc::new(TestNonActivatableView { view_model: RefCell::new(None) });
    let result = Rc::new(Cell::new(None));
    let seen = result.clone();
    let _subscription = activation_of(view).subscribe_next(move |value| seen.set(Some(value)));

    assert_eq!(result.get(), Some(false));
}

// Not ported: GetAffinityForView_WithICanActivate_Returns10 - reflection (the affinity of the type `ICanActivate` itself; the port computes it on a view)

#[test]
fn get_affinity_for_view_with_i_can_activate_derivative_returns10() {
    let view = TestCanActivateView::new();

    let affinity = CanActivateViewFetcher::new().get_affinity_for_view(&*view);

    assert_eq!(affinity, BindingAffinity::EXACT_TYPE);
}

#[test]
fn get_affinity_for_view_with_non_activatable_view_returns0() {
    let view = TestNonActivatableView { view_model: RefCell::new(None) };

    let affinity = CanActivateViewFetcher::new().get_affinity_for_view(&view);

    assert_eq!(affinity, 0);
}

// Not ported: GetAffinityForView_WithNonICanActivate_Returns0 - reflection (the affinity of the type `string`, which is not a view)
