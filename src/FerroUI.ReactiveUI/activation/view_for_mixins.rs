//! `ViewForMixins` (`ReactiveUI.Shared/Activation/ViewForMixins.cs`): the activation of views and
//! view models.

use super::view_model_activator::ActivationBlock;
use super::IActivationForViewFetcher;
use crate::{find_observer_for_property, DeferredValueObservable, IActivatableView, IActivatableViewModel, IViewFor, PluginPropertyObservable, ValueComparer};
use ferroui_reactive::disposables::{CompositeDisposable, Disposable, SerialDisposable};
use ferroui_reactive::Observer;
use ferroui_splat::AppLocator;
use ferroui_system::component_model::PropertyOwner;
use ferroui_system::{IDisposable, IObservable};
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// The disposables of one activation, which a block adds to and which are disposed on
/// deactivation (`ActivationDisposables`, a `CompositeDisposable`).
pub type ActivationDisposables = CompositeDisposable;

/// The view model as a view gives it, type-erased (`object?`).
pub type ViewModelValue = Option<Rc<dyn Any>>;

/// Gives the activatable view model of an object of one type.
type ActivatableCast = Rc<dyn Fn(Rc<dyn Any>) -> Option<Rc<dyn IActivatableViewModel>>>;

ferroui_system::ferro_thread_local! {
    /// The types of view models known to be activatable, by type.
    static ACTIVATABLE_VIEW_MODELS: RefCell<HashMap<TypeId, ActivatableCast>> = RefCell::new(HashMap::new());
}

/// Makes the view models of the type `VM` known as activatable, so that a view whose view model
/// is one activates it (the test `value is IActivatableViewModel` of upstream).
///
/// [`ViewModelActivationMixins`] calls it when a view model registers what to do on activation,
/// which is what every activatable view model does; a view model that only uses its activator's
/// observables calls it once.
// Deviation (DEVIATIONS.md, ReactiveUI): a view gives its view model as `Rc<dyn Any>`, and a trait
// object cannot be tested for another trait; the activatable types are registered here, per
// thread, by type.
pub fn register_activatable_view_model<VM: IActivatableViewModel>() {
    ACTIVATABLE_VIEW_MODELS.with(|types| {
        types.borrow_mut().entry(TypeId::of::<VM>()).or_insert_with(|| {
            Rc::new(|object: Rc<dyn Any>| object.downcast::<VM>().ok().map(|view_model| view_model as Rc<dyn IActivatableViewModel>))
        });
    });
}

/// The view model as an activatable view model, when its type is registered.
pub(crate) fn as_activatable_view_model(object: &Rc<dyn Any>) -> Option<Rc<dyn IActivatableViewModel>> {
    let type_id = (**object).type_id();
    let cast = ACTIVATABLE_VIEW_MODELS.with(|types| types.borrow().get(&type_id).cloned())?;
    cast(object.clone())
}

/// The activation of views (the extension members of `IActivatableView` in `ViewForMixins`),
/// for the shared handle of a view.
///
/// The activation of a view comes from the registered
/// [`IActivationForViewFetcher`] with the highest affinity for it; a view that no fetcher
/// handles panics (`ArgumentException`). While the view is active, the view model it shows (when
/// the view is an [`IViewFor`]) is activated too, when it is an [`IActivatableViewModel`]:
/// setting another view model deactivates the old one and activates the new one.
///
/// Upstream overloads `WhenActivated` on the shape of the block and on the extra arguments; here
/// each shape has a name:
///
/// | Upstream | Here |
/// |---|---|
/// | `WhenActivated(Func<IEnumerable<IDisposable>>)` | [`when_activated`](Self::when_activated) |
/// | `WhenActivated(Action<Action<IDisposable>>)` | [`when_activated_register`](Self::when_activated_register) |
/// | `WhenActivated(Action<ActivationDisposables>)` | [`when_activated_disposables`](Self::when_activated_disposables) |
/// | `WhenActivated()` | [`when_activated_empty`](Self::when_activated_empty) |
/// | the same with `IViewFor? view` | `..._for_view` |
/// | the same with `IObservable<object?> viewModelChanged` | `..._with_view_model_changed` |
///
/// Each returns a disposable that ends the activation of the view and disposes what the block
/// set up.
// Deviation (DEVIATIONS.md, ReactiveUI): the overloads of `WhenActivated` have one name per shape of
// block and per extra argument (Rust has no overloading); the `IObservable<object?>` of the view
// model is an `IObservable<Option<Rc<dyn Any>>>`, and `ActivationDisposables` is a
// `CompositeDisposable`.
pub trait ViewForMixins {
    /// Runs `block` each time the view is activated; what it returns is disposed when the view
    /// is deactivated (`WhenActivated(Func<IEnumerable<IDisposable>>)`).
    fn when_activated(&self, block: impl Fn() -> Vec<Rc<dyn IDisposable>> + 'static) -> Rc<dyn IDisposable>;

    /// [`when_activated`](Self::when_activated) where the view model activated with the view is
    /// the one of `view` rather than of this view (`WhenActivated(Func<IEnumerable<IDisposable>>,
    /// IViewFor?)`), for a view hosted apart from its view model.
    fn when_activated_for_view(
        &self,
        block: impl Fn() -> Vec<Rc<dyn IDisposable>> + 'static,
        view: Option<Rc<dyn IViewFor>>,
    ) -> Rc<dyn IDisposable>;

    /// Runs `block` each time the view is activated; the block registers what to dispose on
    /// deactivation with the function it is given (`WhenActivated(Action<Action<IDisposable>>)`).
    fn when_activated_register(&self, block: impl Fn(&dyn Fn(Rc<dyn IDisposable>)) + 'static) -> Rc<dyn IDisposable>;

    /// [`when_activated_register`](Self::when_activated_register) with the view whose view model
    /// is activated (`WhenActivated(Action<Action<IDisposable>>, IViewFor)`).
    fn when_activated_register_for_view(
        &self,
        block: impl Fn(&dyn Fn(Rc<dyn IDisposable>)) + 'static,
        view: Option<Rc<dyn IViewFor>>,
    ) -> Rc<dyn IDisposable>;

    /// Runs `block` each time the view is activated with the [`ActivationDisposables`] that are
    /// disposed on deactivation (`WhenActivated(Action<ActivationDisposables>)`).
    fn when_activated_disposables(&self, block: impl Fn(&Rc<ActivationDisposables>) + 'static) -> Rc<dyn IDisposable>;

    /// [`when_activated_disposables`](Self::when_activated_disposables) with the view whose view
    /// model is activated (`WhenActivated(Action<ActivationDisposables>, IViewFor?)`).
    fn when_activated_disposables_for_view(
        &self,
        block: impl Fn(&Rc<ActivationDisposables>) + 'static,
        view: Option<Rc<dyn IViewFor>>,
    ) -> Rc<dyn IDisposable>;

    /// Activates the view, and with it its view model, without a block of its own
    /// (`WhenActivated()`).
    fn when_activated_empty(&self) -> Rc<dyn IDisposable>;

    /// [`when_activated`](Self::when_activated), where the view model to activate is the latest
    /// value of `view_model_changed`, which emits the current view model and every later one
    /// (`WhenActivated(Func<IEnumerable<IDisposable>>, IObservable<object?>)`).
    fn when_activated_with_view_model_changed(
        &self,
        block: impl Fn() -> Vec<Rc<dyn IDisposable>> + 'static,
        view_model_changed: Rc<dyn IObservable<ViewModelValue>>,
    ) -> Rc<dyn IDisposable>;

    /// [`when_activated_register`](Self::when_activated_register) with the view model's changes
    /// (`WhenActivated(Action<Action<IDisposable>>, IObservable<object?>)`).
    fn when_activated_register_with_view_model_changed(
        &self,
        block: impl Fn(&dyn Fn(Rc<dyn IDisposable>)) + 'static,
        view_model_changed: Rc<dyn IObservable<ViewModelValue>>,
    ) -> Rc<dyn IDisposable>;

    /// [`when_activated_disposables`](Self::when_activated_disposables) with the view model's
    /// changes (`WhenActivated(Action<ActivationDisposables>, IObservable<object?>)`).
    fn when_activated_disposables_with_view_model_changed(
        &self,
        block: impl Fn(&Rc<ActivationDisposables>) + 'static,
        view_model_changed: Rc<dyn IObservable<ViewModelValue>>,
    ) -> Rc<dyn IDisposable>;

    /// [`when_activated_empty`](Self::when_activated_empty) with the view model's changes
    /// (`WhenActivated(IObservable<object?>)`).
    fn when_activated_empty_with_view_model_changed(&self, view_model_changed: Rc<dyn IObservable<ViewModelValue>>) -> Rc<dyn IDisposable>;

    /// Whether the view is being loaded by a designer (`GetIsDesignMode()`): false; the
    /// integration of a toolkit can tell otherwise for its views.
    fn get_is_design_mode(&self) -> bool;
}

impl<V: IActivatableView> ViewForMixins for Rc<V> {
    fn when_activated(&self, block: impl Fn() -> Vec<Rc<dyn IDisposable>> + 'static) -> Rc<dyn IDisposable> {
        self.when_activated_for_view(block, None)
    }

    fn when_activated_for_view(
        &self,
        block: impl Fn() -> Vec<Rc<dyn IDisposable>> + 'static,
        view: Option<Rc<dyn IViewFor>>,
    ) -> Rc<dyn IDisposable> {
        let item: Rc<dyn IActivatableView> = self.clone();
        let target: Option<Rc<dyn IActivatableView>> = view.map(|view| view as Rc<dyn IActivatableView>);
        when_activated_on_view(&item, std::any::type_name::<V>(), Rc::new(block), target)
    }

    fn when_activated_register(&self, block: impl Fn(&dyn Fn(Rc<dyn IDisposable>)) + 'static) -> Rc<dyn IDisposable> {
        self.when_activated_register_for_view(block, None)
    }

    fn when_activated_register_for_view(
        &self,
        block: impl Fn(&dyn Fn(Rc<dyn IDisposable>)) + 'static,
        view: Option<Rc<dyn IViewFor>>,
    ) -> Rc<dyn IDisposable> {
        self.when_activated_for_view(register_block(block), view)
    }

    fn when_activated_disposables(&self, block: impl Fn(&Rc<ActivationDisposables>) + 'static) -> Rc<dyn IDisposable> {
        self.when_activated_disposables_for_view(block, None)
    }

    fn when_activated_disposables_for_view(
        &self,
        block: impl Fn(&Rc<ActivationDisposables>) + 'static,
        view: Option<Rc<dyn IViewFor>>,
    ) -> Rc<dyn IDisposable> {
        self.when_activated_for_view(disposables_block(block), view)
    }

    fn when_activated_empty(&self) -> Rc<dyn IDisposable> {
        self.when_activated_for_view(Vec::new, None)
    }

    fn when_activated_with_view_model_changed(
        &self,
        block: impl Fn() -> Vec<Rc<dyn IDisposable>> + 'static,
        view_model_changed: Rc<dyn IObservable<ViewModelValue>>,
    ) -> Rc<dyn IDisposable> {
        let item: Rc<dyn IActivatableView> = self.clone();
        let activation_events = resolve_activation_events(&item, std::any::type_name::<V>());
        let view_model_disposable = handle_view_model_activation(view_model_changed, activation_events.clone());
        let view_disposable = handle_view_activation(Rc::new(block), activation_events);
        CompositeDisposable::from_disposables([view_model_disposable, view_disposable])
    }

    fn when_activated_register_with_view_model_changed(
        &self,
        block: impl Fn(&dyn Fn(Rc<dyn IDisposable>)) + 'static,
        view_model_changed: Rc<dyn IObservable<ViewModelValue>>,
    ) -> Rc<dyn IDisposable> {
        self.when_activated_with_view_model_changed(register_block(block), view_model_changed)
    }

    fn when_activated_disposables_with_view_model_changed(
        &self,
        block: impl Fn(&Rc<ActivationDisposables>) + 'static,
        view_model_changed: Rc<dyn IObservable<ViewModelValue>>,
    ) -> Rc<dyn IDisposable> {
        self.when_activated_with_view_model_changed(disposables_block(block), view_model_changed)
    }

    fn when_activated_empty_with_view_model_changed(&self, view_model_changed: Rc<dyn IObservable<ViewModelValue>>) -> Rc<dyn IDisposable> {
        self.when_activated_with_view_model_changed(Vec::new, view_model_changed)
    }

    fn get_is_design_mode(&self) -> bool {
        false
    }
}

/// The activation of view models (the extension members of `IActivatableViewModel` in
/// `ViewForMixins`): what to do while the view model is active, registered with its
/// [`ViewModelActivator`](crate::ViewModelActivator).
///
/// The blocks run when the activator is activated, and what they set up is disposed when it is
/// deactivated. Registering a block also makes the type of the view model known as activatable
/// ([`register_activatable_view_model`]), so that the views that show it activate it.
pub trait ViewModelActivationMixins: IActivatableViewModel + Sized {
    /// Runs `block` on each activation; what it returns is disposed on deactivation
    /// (`WhenActivated(Func<IEnumerable<IDisposable>>)`).
    fn when_activated(&self, block: impl Fn() -> Vec<Rc<dyn IDisposable>> + 'static) {
        register_activatable_view_model::<Self>();
        self.activator().add_activation_block(Rc::new(block));
    }

    /// Runs `block` on each activation; the block registers what to dispose on deactivation with
    /// the function it is given (`WhenActivated(Action<Action<IDisposable>>)`).
    fn when_activated_register(&self, block: impl Fn(&dyn Fn(Rc<dyn IDisposable>)) + 'static) {
        self.when_activated(register_block(block));
    }

    /// Runs `block` on each activation with the [`ActivationDisposables`] disposed on
    /// deactivation (`WhenActivated(Action<ActivationDisposables>)`).
    fn when_activated_disposables(&self, block: impl Fn(&Rc<ActivationDisposables>) + 'static) {
        self.when_activated(disposables_block(block));
    }
}

impl<T: IActivatableViewModel> ViewModelActivationMixins for T {}

/// A block that registers its disposables through a function, as one that returns them.
fn register_block(block: impl Fn(&dyn Fn(Rc<dyn IDisposable>)) + 'static) -> impl Fn() -> Vec<Rc<dyn IDisposable>> + 'static {
    move || {
        let registered: RefCell<Vec<Rc<dyn IDisposable>>> = RefCell::new(Vec::new());
        block(&|disposable| registered.borrow_mut().push(disposable));
        registered.into_inner()
    }
}

/// A block that adds its disposables to an [`ActivationDisposables`], as one that returns them.
fn disposables_block(block: impl Fn(&Rc<ActivationDisposables>) + 'static) -> impl Fn() -> Vec<Rc<dyn IDisposable>> + 'static {
    move || {
        let disposables = ActivationDisposables::new();
        block(&disposables);
        vec![disposables as Rc<dyn IDisposable>]
    }
}

/// `WhenActivated(Func<IEnumerable<IDisposable>>, IViewFor?)`.
fn when_activated_on_view(
    item: &Rc<dyn IActivatableView>,
    type_name: &str,
    block: ActivationBlock,
    view: Option<Rc<dyn IActivatableView>>,
) -> Rc<dyn IDisposable> {
    let activation_events = resolve_activation_events(item, type_name);
    let target = view.unwrap_or_else(|| item.clone());
    let view_model_disposable: Rc<dyn IDisposable> = if target.as_view_for().is_some() {
        handle_view_model_activation(observe_view_model(&target), activation_events.clone())
    } else {
        Disposable::empty()
    };
    let view_disposable = handle_view_activation(block, activation_events);
    CompositeDisposable::from_disposables([view_model_disposable, view_disposable])
}

/// The registered fetcher with the highest positive affinity for the view
/// (`ResolveActivationFetcher`).
// Deviation (DEVIATIONS.md, ReactiveUI): upstream memoizes the fetcher per type of view
// (`MemoizingMRUCache`, reset by `ResetActivationFetcherCacheForTesting`); here the affinity is
// computed on the view each time a view sets up its activation.
fn resolve_activation_fetcher(view: &dyn IActivatableView) -> Option<Rc<dyn IActivationForViewFetcher>> {
    let mut best_score = 0;
    let mut best: Option<Rc<dyn IActivationForViewFetcher>> = None;
    for fetcher in AppLocator::get_services::<dyn IActivationForViewFetcher>() {
        let score = fetcher.get_affinity_for_view(view);
        if score <= best_score {
            continue;
        }
        best_score = score;
        best = Some(fetcher);
    }
    best
}

/// The activation of the view, from its fetcher (`ResolveActivationEvents`).
///
/// Panics when no fetcher handles the view (`ArgumentException`).
fn resolve_activation_events(item: &Rc<dyn IActivatableView>, type_name: &str) -> Rc<dyn IObservable<bool>> {
    match resolve_activation_fetcher(&**item) {
        Some(fetcher) => fetcher.get_activation_for_view(item),
        None => panic!("Don't know how to detect when {type_name} is activated/deactivated, you may need to implement IActivationForViewFetcher"),
    }
}

/// Runs the block on each activation and disposes what it returned on deactivation, or before
/// the next activation (`HandleViewActivation`).
fn handle_view_activation(block: ActivationBlock, activation: Rc<dyn IObservable<bool>>) -> Rc<dyn IDisposable> {
    let view_disposable = SerialDisposable::new();
    let current = view_disposable.clone();
    let subscription = activation.subscribe(Observer::create_all(
        move |activated: bool| {
            current.set_disposable(Some(Disposable::empty()));
            if !activated {
                return;
            }
            let disposables = block();
            current.set_disposable(Some(CompositeDisposable::from_disposables(disposables)));
        },
        |_| {},
        || {},
    ));
    CompositeDisposable::from_disposables([subscription, view_disposable as Rc<dyn IDisposable>])
}

/// While the view is active, activates the latest view model of `view_model_changed` when it is
/// activatable (`HandleViewModelActivation`).
fn handle_view_model_activation(view_model_changed: Rc<dyn IObservable<ViewModelValue>>, activation: Rc<dyn IObservable<bool>>) -> Rc<dyn IDisposable> {
    let view_model_disposable = SerialDisposable::new();
    let view_view_model_disposable = SerialDisposable::new();
    let view_model = view_model_disposable.clone();
    let view_view_model = view_view_model_disposable.clone();
    let subscription = activation.subscribe(Observer::create_all(
        move |activated: bool| {
            if activated {
                let view_model = view_model.clone();
                let subscription = view_model_changed.subscribe(Observer::create_all(
                    move |value: ViewModelValue| {
                        view_model.set_disposable(Some(Disposable::empty()));
                        let Some(activatable) = value.as_ref().and_then(as_activatable_view_model) else {
                            return;
                        };
                        view_model.set_disposable(Some(activatable.activator().activate()));
                    },
                    |_| {},
                    || {},
                ));
                view_view_model.set_disposable(Some(subscription));
            } else {
                view_view_model.set_disposable(Some(Disposable::empty()));
                view_model.set_disposable(Some(Disposable::empty()));
            }
        },
        |_| {},
        || {},
    ));
    CompositeDisposable::from_disposables([
        subscription,
        view_model_disposable as Rc<dyn IDisposable>,
        view_view_model_disposable as Rc<dyn IDisposable>,
    ])
}

/// Whether two values of the view model are the same object.
fn same_view_model(first: &ViewModelValue, second: &ViewModelValue) -> bool {
    match (first, second) {
        (Some(first), Some(second)) => Rc::ptr_eq(first, second),
        (None, None) => true,
        _ => false,
    }
}

/// The view model of the view: its current value, then each later one (`ObserveViewModel`).
///
/// The registered property observer with the highest affinity for the view and its property
/// `ViewModel` watches it; when none can, the current value is reported once.
fn observe_view_model(view: &Rc<dyn IActivatableView>) -> Rc<dyn IObservable<ViewModelValue>> {
    let object: Rc<dyn Any> = view.clone();
    let owner = PropertyOwner { object: Some(object), notify: view.clone().as_notify_property_changed() };
    let weak = Rc::downgrade(view);
    match find_observer_for_property(&owner, "ViewModel", false) {
        None => Rc::new(DeferredValueObservable::new(move || {
            Ok(weak.upgrade().and_then(|view| view.as_view_for().and_then(|view_for| view_for.view_model())))
        })),
        Some((plugin, _)) => {
            let comparer: ValueComparer<ViewModelValue> = Rc::new(same_view_model);
            Rc::new(PluginPropertyObservable::with_comparer(
                plugin,
                &owner,
                "ViewModel",
                move || weak.upgrade().map(|view| view.as_view_for().and_then(|view_for| view_for.view_model())),
                false,
                Some(comparer),
            ))
        }
    }
}
