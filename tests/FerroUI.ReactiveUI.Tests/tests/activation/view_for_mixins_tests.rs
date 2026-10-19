//! `ViewForMixinsTests` (`ReactiveUI.Tests/Activation/ViewForMixinsTests.cs`).
//!
//! Upstream builds an application with `CreateReactiveUIBuilder().WithCoreServices()` and a
//! custom registration of the fetcher; here the fetcher is registered in the locator of the
//! thread ([`register_activating_view_fetcher`]), the observation of properties using its
//! defaults.

use super::{register_activating_view_fetcher, ActivatingView, ActivatingViewModel};
use ferroui_reactive::disposables::{CompositeDisposable, Disposable, DisposableExt};
use ferroui_reactive::subjects::BehaviorSubject;
use ferroui_reactiveui::*;
use ferroui_system::{IDisposable, IObserver};
use std::any::Any;
use std::cell::Cell;
use std::rc::Rc;

/// A view model with an activator and nothing else (`ActivatableViewModelMock`).
struct ActivatableViewModelMock {
    reactive: ReactiveObject,
    activator: ViewModelActivator,
}
reactive_object!(ActivatableViewModelMock);

impl IActivatableViewModel for ActivatableViewModelMock {
    fn activator(&self) -> &ViewModelActivator {
        &self.activator
    }
}

impl ActivatableViewModelMock {
    fn new() -> Rc<Self> {
        Rc::new(Self { reactive: ReactiveObject::new(), activator: ViewModelActivator::new() })
    }
}

/// A disposable that counts its disposals in `counter` (`Scope.Create(deactivations, d => d.Value++)`).
fn counting(counter: &Rc<Cell<i32>>) -> Rc<dyn IDisposable> {
    let counter = counter.clone();
    Disposable::create(move || counter.set(counter.get() + 1))
}

/// The view model as the observable of a view's view model gives it.
fn as_view_model(view_model: &Rc<ActivatingViewModel>) -> Option<Rc<dyn Any>> {
    Some(view_model.clone() as Rc<dyn Any>)
}

#[test]
fn view_model_when_activated_func_overload_activates_and_deactivates() {
    let activations = Rc::new(Cell::new(0));
    let deactivations = Rc::new(Cell::new(0));
    let view_model = ActivatableViewModelMock::new();
    let (counted, disposed) = (activations.clone(), deactivations.clone());
    ViewModelActivationMixins::when_activated(&*view_model, move || {
        counted.set(counted.get() + 1);
        vec![counting(&disposed)]
    });

    assert_eq!(activations.get(), 0);
    assert_eq!(deactivations.get(), 0);

    let _ = view_model.activator().activate();
    assert_eq!(activations.get(), 1);

    view_model.activator().deactivate();
    assert_eq!(deactivations.get(), 1);
}

#[test]
fn view_when_activated_no_arg_overload_activates_view_model() {
    register_activating_view_fetcher();
    let view_model = ActivatingViewModel::new();
    let fixture = ActivatingView::new();
    fixture.set_view_model(Some(view_model.clone()));

    let activation = fixture.when_activated_empty();

    fixture.loaded().on_next(());
    assert_eq!(view_model.is_active_count(), 1);

    fixture.unloaded().on_next(());
    assert_eq!(view_model.is_active_count(), 0);
    activation.dispose();
}

#[test]
fn view_when_activated_func_overload_activates_and_deactivates() {
    register_activating_view_fetcher();
    let view = ActivatingView::new();
    let activations = Rc::new(Cell::new(0));
    let deactivations = Rc::new(Cell::new(0));
    let (counted, disposed) = (activations.clone(), deactivations.clone());

    let activation = view.when_activated(move || {
        counted.set(counted.get() + 1);
        vec![counting(&disposed)]
    });

    view.loaded().on_next(());
    assert_eq!(activations.get(), 1);

    view.unloaded().on_next(());
    assert_eq!(deactivations.get(), 1);
    activation.dispose();
}

#[test]
fn view_model_when_activated_activation_disposables_overload_activates_and_deactivates() {
    let activations = Rc::new(Cell::new(0));
    let deactivations = Rc::new(Cell::new(0));
    let view_model = ActivatableViewModelMock::new();
    let (counted, disposed) = (activations.clone(), deactivations.clone());
    ViewModelActivationMixins::when_activated_disposables(&*view_model, move |d| {
        counted.set(counted.get() + 1);
        d.add(counting(&disposed));
    });

    let _ = view_model.activator().activate();
    assert_eq!(activations.get(), 1);

    view_model.activator().deactivate();
    assert_eq!(deactivations.get(), 1);
}

#[test]
fn view_when_activated_with_view_model_changed_forwards_view_model_activation() {
    register_activating_view_fetcher();
    let view_model = ActivatingViewModel::new();
    let view_model_changed = BehaviorSubject::new(as_view_model(&view_model));
    let view = ActivatingView::new();
    let activations = Rc::new(Cell::new(0));
    let deactivations = Rc::new(Cell::new(0));
    let (counted, disposed) = (activations.clone(), deactivations.clone());

    let activation = view.when_activated_with_view_model_changed(
        move || {
            counted.set(counted.get() + 1);
            vec![counting(&disposed)]
        },
        view_model_changed.as_observable(),
    );

    assert_eq!(activations.get(), 0);
    assert_eq!(view_model.is_active_count(), 0);

    view.loaded().on_next(());
    assert_eq!(activations.get(), 1);
    assert_eq!(view_model.is_active_count(), 1);

    view.unloaded().on_next(());
    assert_eq!(deactivations.get(), 1);
    assert_eq!(view_model.is_active_count(), 0);
    activation.dispose();
    view_model_changed.dispose();
}

#[test]
fn view_when_activated_with_view_model_changed_action_overload_activates_view() {
    register_activating_view_fetcher();
    let view_model = ActivatingViewModel::new();
    let view_model_changed = BehaviorSubject::new(as_view_model(&view_model));
    let view = ActivatingView::new();
    let activations = Rc::new(Cell::new(0));
    let deactivations = Rc::new(Cell::new(0));
    let (counted, disposed) = (activations.clone(), deactivations.clone());

    let activation = view.when_activated_register_with_view_model_changed(
        move |d| {
            counted.set(counted.get() + 1);
            d(counting(&disposed));
        },
        view_model_changed.as_observable(),
    );

    view.loaded().on_next(());
    assert_eq!(activations.get(), 1);
    assert_eq!(view_model.is_active_count(), 1);

    view.unloaded().on_next(());
    assert_eq!(deactivations.get(), 1);
    assert_eq!(view_model.is_active_count(), 0);
    activation.dispose();
    view_model_changed.dispose();
}

#[test]
fn view_when_activated_with_view_model_changed_activation_disposables_overload_activates_view() {
    register_activating_view_fetcher();
    let view_model = ActivatingViewModel::new();
    let view_model_changed = BehaviorSubject::new(as_view_model(&view_model));
    let view = ActivatingView::new();
    let activations = Rc::new(Cell::new(0));
    let deactivations = Rc::new(Cell::new(0));
    let (counted, disposed) = (activations.clone(), deactivations.clone());

    let activation = view.when_activated_disposables_with_view_model_changed(
        move |d| {
            counted.set(counted.get() + 1);
            d.add(counting(&disposed));
        },
        view_model_changed.as_observable(),
    );

    view.loaded().on_next(());
    assert_eq!(activations.get(), 1);
    assert_eq!(view_model.is_active_count(), 1);

    view.unloaded().on_next(());
    assert_eq!(deactivations.get(), 1);
    assert_eq!(view_model.is_active_count(), 0);
    activation.dispose();
    view_model_changed.dispose();
}

#[test]
fn view_when_activated_with_view_model_changed_no_block_overload_forwards_view_model_activation() {
    register_activating_view_fetcher();
    let view_model = ActivatingViewModel::new();
    let view_model_changed = BehaviorSubject::new(as_view_model(&view_model));
    let view = ActivatingView::new();

    let activation = view.when_activated_empty_with_view_model_changed(view_model_changed.as_observable());

    assert_eq!(view_model.is_active_count(), 0);

    view.loaded().on_next(());
    assert_eq!(view_model.is_active_count(), 1);

    // A non-activatable ViewModel emitted while active deactivates the previous one.
    view_model_changed.on_next(None);
    assert_eq!(view_model.is_active_count(), 0);

    // Re-emitting the activatable ViewModel while active re-activates it.
    view_model_changed.on_next(as_view_model(&view_model));
    assert_eq!(view_model.is_active_count(), 1);

    view.unloaded().on_next(());
    assert_eq!(view_model.is_active_count(), 0);
    activation.dispose();
    view_model_changed.dispose();
}

#[test]
fn when_activated_skips_null_activation_fetchers() {
    ferroui_splat::AppLocator::register::<dyn IActivationForViewFetcher>(|| None);
    register_activating_view_fetcher();
    let view_model = ActivatingViewModel::new();
    let view = ActivatingView::new();
    view.set_view_model(Some(view_model.clone()));

    view.loaded().on_next(());
    assert_eq!(view_model.is_active_count(), 1);

    view.unloaded().on_next(());
    assert_eq!(view_model.is_active_count(), 0);
}

#[test]
fn view_model_when_activated_container_is_usable_as_composite_disposable() {
    let deactivations = Rc::new(Cell::new(0));
    let view_model = ActivatableViewModelMock::new();
    let disposed = deactivations.clone();
    ViewModelActivationMixins::when_activated_disposables(&*view_model, move |d| {
        let composite: &CompositeDisposable = d;
        composite.add(counting(&disposed));
    });

    let _ = view_model.activator().activate();
    assert_eq!(deactivations.get(), 0);

    view_model.activator().deactivate();
    assert_eq!(deactivations.get(), 1);
}

#[test]
fn view_when_activated_container_accepts_system_reactive_dispose_with() {
    register_activating_view_fetcher();
    let view_model = ActivatingViewModel::new();
    let view = ActivatingView::new();
    view.set_view_model(Some(view_model));
    let deactivations = Rc::new(Cell::new(0));
    let disposed = deactivations.clone();

    let activation = view.when_activated_disposables(move |d| {
        let _ = counting(&disposed).dispose_with(d);
    });

    view.loaded().on_next(());
    assert_eq!(deactivations.get(), 0);

    view.unloaded().on_next(());
    assert_eq!(deactivations.get(), 1);
    activation.dispose();
}

