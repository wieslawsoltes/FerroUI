//! `ReactiveUIBindingExtensions.BindInteraction.cs`: `BindInteraction`, with the runtime of
//! `RuntimeInteractionFallback` (`Fallback/RuntimeInteractionFallback.cs`) and of the code the
//! source generator writes for it.

use super::ReactiveUIBindingExtensions;
use crate::binding::observable_for_property::{find_observer_for_property, ValueComparer};
use crate::binding::observables::PluginPropertyObservable;
use crate::BindingErrors;
use crate::binding::interactions::{IInteraction, IInteractionContext, Interaction};
use crate::binding::interfaces::IViewForOf;
use ferroui_reactive::disposables::{CompositeDisposable, Disposable, SerialDisposable};
use ferroui_reactive::linq::Observable;
use ferroui_reactive::ObservableExt;
use ferroui_system::component_model::{PropertyOwner, PropertyPath};
use ferroui_system::threading::tasks::Task;
use ferroui_system::{IDisposable, IObservable, ObservableError};
use std::any::Any;
use std::future::Future;
use std::rc::Rc;

/// The value of a property that holds an interaction: an [`Interaction`], or an `Option` of one
/// (a property not assigned yet).
pub trait InteractionValue: Clone + PartialEq + 'static {
    /// The input of the interaction (`TInput`).
    type Input: 'static;
    /// The output of the interaction (`TOutput`).
    type Output: 'static;

    /// The interaction the property holds, if any.
    fn interaction(&self) -> Option<Rc<dyn IInteraction<Self::Input, Self::Output>>>;
}

impl<I: Clone + 'static, O: Clone + 'static> InteractionValue for Interaction<I, O> {
    type Input = I;
    type Output = O;

    fn interaction(&self) -> Option<Rc<dyn IInteraction<I, O>>> {
        Some(Rc::new(self.clone()))
    }
}

impl<I: Clone + 'static, O: Clone + 'static> InteractionValue for Option<Interaction<I, O>> {
    type Input = I;
    type Output = O;

    fn interaction(&self) -> Option<Rc<dyn IInteraction<I, O>>> {
        self.as_ref().map(|interaction| Rc::new(interaction.clone()) as Rc<dyn IInteraction<I, O>>)
    }
}

/// The context a handler of the interaction a property value `X` holds receives.
pub type InteractionContextOf<X> = Rc<dyn IInteractionContext<<X as InteractionValue>::Input, <X as InteractionValue>::Output>>;

/// Whether two view models of a view are the same object.
fn same_view_model<VM>(first: &Option<Rc<VM>>, second: &Option<Rc<VM>>) -> bool {
    match (first, second) {
        (Some(first), Some(second)) => Rc::ptr_eq(first, second),
        (None, None) => true,
        _ => false,
    }
}

/// The view models of `view`: its `ViewModel` on subscription, then after each change, through the
/// registered property observer with the highest affinity for the view and its property
/// `ViewModel` (the first step of the path that upstream's generated code roots at the view). A
/// view that does not notify the changes of its view model
/// ([`IActivatableView::as_notify_property_changed`]) reports its current view model once. The
/// view models compare by identity.
pub(crate) fn view_models_of<V, VM>(view: &Rc<V>) -> Rc<dyn IObservable<Option<Rc<VM>>>>
where
    V: IViewForOf<VM> + 'static,
    VM: 'static,
{
    let object: Rc<dyn Any> = view.clone();
    let owner = PropertyOwner { object: Some(object), notify: view.clone().as_notify_property_changed() };
    let weak = Rc::downgrade(view);
    let read = move || weak.upgrade().map(|view| <V as IViewForOf<VM>>::view_model(&*view));
    match find_observer_for_property(&owner, "ViewModel", false) {
        Some((plugin, _)) => {
            let comparer: ValueComparer<Option<Rc<VM>>> = Rc::new(same_view_model::<VM>);
            Rc::new(PluginPropertyObservable::with_comparer(plugin, &owner, "ViewModel", read, false, Some(comparer))) as Rc<dyn IObservable<Option<Rc<VM>>>>
        }
        None => Observable::return_(read().flatten()),
    }
}

/// The values of `path` on the latest of `view_models`, `None` while there is no view model.
pub(crate) fn follow_view_models<VM, X>(view_models: Rc<dyn IObservable<Option<Rc<VM>>>>, path: PropertyPath<Rc<VM>, X>) -> Rc<dyn IObservable<Option<X>>>
where
    VM: 'static,
    X: Clone + PartialEq + 'static,
{
    view_models
        .select(move |view_model: Option<Rc<VM>>| -> Rc<dyn IObservable<Option<X>>> {
            match view_model {
                Some(view_model) => view_model.when_any_value(path.clone()).select(Some),
                None => Observable::return_(None),
            }
        })
        .switch::<Option<X>>()
}

/// The values of `path` on `view_model`, or `None` without a view model.
pub(crate) fn values_on<VM, X>(view_model: Option<Rc<VM>>, path: PropertyPath<Rc<VM>, X>) -> Option<Rc<dyn IObservable<Option<X>>>>
where
    VM: 'static,
    X: Clone + PartialEq + 'static,
{
    view_model.map(|view_model| view_model.when_any_value(path).select(Some))
}

/// Keeps a handler registered on whichever interaction `values` holds: when it holds another, the
/// handler is unregistered from the one it replaced and registered on the new one, and `None`
/// leaves nothing registered (`RuntimeInteractionFallback.BindInteraction`). Disposing the result
/// unregisters the handler and ends the observation.
fn bind_to_interactions<X: InteractionValue>(
    values: Rc<dyn IObservable<Option<X>>>,
    register: impl Fn(Rc<dyn IInteraction<X::Input, X::Output>>) -> Rc<dyn IDisposable> + 'static,
    binding_expression: String,
) -> Rc<dyn IDisposable> {
    let registration = SerialDisposable::new();
    let current = registration.clone();
    let observation = BindingErrors::subscribe(
        values,
        move |value: Option<X>| {
            let next = match value.and_then(|value| value.interaction()) {
                Some(interaction) => register(interaction),
                None => Disposable::empty(),
            };
            current.set_disposable(Some(next));
        },
        binding_expression,
    );
    CompositeDisposable::from_disposables([observation, registration as Rc<dyn IDisposable>])
}

/// The registration of an asynchronous handler.
fn task_registration<X, Fut>(handler: impl Fn(InteractionContextOf<X>) -> Fut + 'static) -> impl Fn(Rc<dyn IInteraction<X::Input, X::Output>>) -> Rc<dyn IDisposable> + 'static
where
    X: InteractionValue,
    Fut: Future<Output = Result<(), ObservableError>> + 'static,
{
    let handler = Rc::new(handler);
    move |interaction: Rc<dyn IInteraction<X::Input, X::Output>>| {
        let handler = handler.clone();
        interaction.register_handler_task(Rc::new(move |context: InteractionContextOf<X>| Task::spawn_local(handler(context))))
    }
}

/// The registration of an observable-based handler.
fn observable_registration<X, D>(
    handler: impl Fn(InteractionContextOf<X>) -> Rc<dyn IObservable<D>> + 'static,
) -> impl Fn(Rc<dyn IInteraction<X::Input, X::Output>>) -> Rc<dyn IDisposable> + 'static
where
    X: InteractionValue,
    D: 'static,
{
    let handler = Rc::new(handler);
    move |interaction: Rc<dyn IInteraction<X::Input, X::Output>>| {
        let handler = handler.clone();
        interaction.register_handler_observable(Rc::new(move |context: InteractionContextOf<X>| handler(context).select(|_: D| ())))
    }
}

/// `BindInteraction`: registers a handler of a view on the interaction a property of its view model
/// holds, and moves it to the new interaction when the property changes. Implemented for every
/// `Rc` of a view.
///
/// Upstream's source generator roots the property at the view's `ViewModel` when the view declares
/// one: the handler follows the view model the view displays, a new view model included, and a
/// view without a view model has nothing registered. [`bind_interaction`](Self::bind_interaction)
/// and [`bind_interaction_observable`](Self::bind_interaction_observable) do that for a view that
/// declares its view model (`IViewForOf<VM>`), observed as the property `ViewModel` of the view (a view
/// that does not notify its changes is read once);
/// [`bind_interaction_on`](Self::bind_interaction_on) and
/// [`bind_interaction_observable_on`](Self::bind_interaction_observable_on) root the property at a
/// given view model, as for a view that does not declare one.
///
/// Deviation (DEVIATIONS.md, ReactiveUI): upstream overloads `BindInteraction` by the kind of
/// handler (a `Task` or an observable) and resolves the root at compile time; here each is a
/// method of its own. The view-rooted forms leave out upstream's `viewModel` argument, which the
/// generated code does not read either. The expression is a [`PropertyPath`].
pub trait BindInteractionMixins<V: 'static> {
    /// Registers the asynchronous `handler` on the interaction `path` reads from the view's view
    /// model (`BindInteraction(..., Func<IInteractionContext<TInput, TOutput>, Task>)`). The
    /// handler gives a future (a `Task<()>` is one), which ends when it has handled the question.
    fn bind_interaction<VM, X, Fut>(&self, path: PropertyPath<Rc<VM>, X>, handler: impl Fn(InteractionContextOf<X>) -> Fut + 'static) -> Rc<dyn IDisposable>
    where
        V: IViewForOf<VM>,
        VM: 'static,
        X: InteractionValue,
        Fut: Future<Output = Result<(), ObservableError>> + 'static;

    /// Registers the observable-based `handler` on the interaction `path` reads from the view's
    /// view model; the handler has finished when its observable ends
    /// (`BindInteraction(..., Func<IInteractionContext<TInput, TOutput>, IObservable<TDontCare>>)`).
    fn bind_interaction_observable<VM, X, D>(
        &self,
        path: PropertyPath<Rc<VM>, X>,
        handler: impl Fn(InteractionContextOf<X>) -> Rc<dyn IObservable<D>> + 'static,
    ) -> Rc<dyn IDisposable>
    where
        V: IViewForOf<VM>,
        VM: 'static,
        X: InteractionValue,
        D: 'static;

    /// [`bind_interaction`](Self::bind_interaction) rooted at `view_model`; no view model
    /// registers nothing.
    fn bind_interaction_on<VM, X, Fut>(
        &self,
        view_model: Option<Rc<VM>>,
        path: PropertyPath<Rc<VM>, X>,
        handler: impl Fn(InteractionContextOf<X>) -> Fut + 'static,
    ) -> Rc<dyn IDisposable>
    where
        VM: 'static,
        X: InteractionValue,
        Fut: Future<Output = Result<(), ObservableError>> + 'static;

    /// [`bind_interaction_observable`](Self::bind_interaction_observable) rooted at `view_model`;
    /// no view model registers nothing.
    fn bind_interaction_observable_on<VM, X, D>(
        &self,
        view_model: Option<Rc<VM>>,
        path: PropertyPath<Rc<VM>, X>,
        handler: impl Fn(InteractionContextOf<X>) -> Rc<dyn IObservable<D>> + 'static,
    ) -> Rc<dyn IDisposable>
    where
        VM: 'static,
        X: InteractionValue,
        D: 'static;
}

impl<V: 'static> BindInteractionMixins<V> for Rc<V> {
    fn bind_interaction<VM, X, Fut>(&self, path: PropertyPath<Rc<VM>, X>, handler: impl Fn(InteractionContextOf<X>) -> Fut + 'static) -> Rc<dyn IDisposable>
    where
        V: IViewForOf<VM>,
        VM: 'static,
        X: InteractionValue,
        Fut: Future<Output = Result<(), ObservableError>> + 'static,
    {
        let expression = format!("ViewModel.{}", path.path());
        let values = follow_view_models(view_models_of::<V, VM>(self), path);
        bind_to_interactions(values, task_registration::<X, Fut>(handler), expression)
    }

    fn bind_interaction_observable<VM, X, D>(
        &self,
        path: PropertyPath<Rc<VM>, X>,
        handler: impl Fn(InteractionContextOf<X>) -> Rc<dyn IObservable<D>> + 'static,
    ) -> Rc<dyn IDisposable>
    where
        V: IViewForOf<VM>,
        VM: 'static,
        X: InteractionValue,
        D: 'static,
    {
        let expression = format!("ViewModel.{}", path.path());
        let values = follow_view_models(view_models_of::<V, VM>(self), path);
        bind_to_interactions(values, observable_registration::<X, D>(handler), expression)
    }

    fn bind_interaction_on<VM, X, Fut>(
        &self,
        view_model: Option<Rc<VM>>,
        path: PropertyPath<Rc<VM>, X>,
        handler: impl Fn(InteractionContextOf<X>) -> Fut + 'static,
    ) -> Rc<dyn IDisposable>
    where
        VM: 'static,
        X: InteractionValue,
        Fut: Future<Output = Result<(), ObservableError>> + 'static,
    {
        let expression = path.path();
        match values_on(view_model, path) {
            Some(values) => bind_to_interactions(values, task_registration::<X, Fut>(handler), expression),
            None => Disposable::empty(),
        }
    }

    fn bind_interaction_observable_on<VM, X, D>(
        &self,
        view_model: Option<Rc<VM>>,
        path: PropertyPath<Rc<VM>, X>,
        handler: impl Fn(InteractionContextOf<X>) -> Rc<dyn IObservable<D>> + 'static,
    ) -> Rc<dyn IDisposable>
    where
        VM: 'static,
        X: InteractionValue,
        D: 'static,
    {
        let expression = path.path();
        match values_on(view_model, path) {
            Some(values) => bind_to_interactions(values, observable_registration::<X, D>(handler), expression),
            None => Disposable::empty(),
        }
    }
}
