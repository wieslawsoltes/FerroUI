//! The bindings of properties, written once over property paths: the code upstream's source
//! generator writes at each `OneWayBind`, `Bind`, `BindOneWay`, `BindTwoWay` and `BindTo` call
//! site, and the triggered binding of its run-time fallback (`RuntimeBindingFallback.TriggerUpdate`
//! with `TriggeredBindingPlan`).
//!
//! Not an upstream file: the generated code and the fallback are what it replaces. The public
//! methods are those of [`PropertyBindingMixins`](crate::PropertyBindingMixins),
//! [`SourceBindingMixins`](crate::SourceBindingMixins) and [`BindToMixins`](crate::BindToMixins).
//!
//! The shape of every binding is the generated one:
//! - the registered [`IPropertyBindingHook`](crate::IPropertyBindingHook)s may refuse it first;
//! - the source side is observed through `when_any_value` (and so through the
//!   `ICreatesObservableForProperty` chosen for each object), converted, and delivered on the
//!   thread of the object written to ([`BindingSchedulers`]) or on the scheduler the call names;
//! - a write is skipped when the property already holds the value, and a write that fails follows
//!   the fault contract of [`BindingErrors`].
//!
//! A two-way binding seeds the target from the source, then mirrors each later change of either
//! side; the target's own value at the start is not written back (the fallback's `Skip(1)`), so a
//! pair of conversions that are not inverse of each other does not ping-pong.

use super::binding_conversion::Convert;
use super::{BindingChange, BindingErrors, BindingHooks, BindingSchedulers, ReactiveBinding, SettableProperty, TriggerUpdate};
use crate::binding::interfaces::{BindingDirection, BindingHookChanges, IReactiveBinding, ObservedChange};
use crate::binding::mixins::{schedule_latest_write, schedule_write, ReactiveUIBindingExtensions};
use crate::binding::observables::{AppliedChangeObservable, InitialBindingSignal};
use crate::{view_for_of, IViewFor};
use ferroui_reactive::concurrency::IScheduler;
use ferroui_reactive::disposables::{CompositeDisposable, Disposable};
use ferroui_reactive::linq::Observable;
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::ObservableExt;
use ferroui_system::component_model::{INotifyPropertyChanged, PropertyPath};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::any::Any;
use std::cell::Cell;
use std::rc::{Rc, Weak};

/// The view model of a view-first binding, as upstream's generated code and fallback find it.
enum ViewModelRoot<VM> {
    /// The view is typed for the view model (`view is IViewFor<TViewModel>`): the binding follows
    /// the view's `ViewModel`, so that replacing the view model rebinds, and a view without one
    /// waits for it.
    FollowView,
    /// The view names no type of view model: the binding uses the one handed in.
    Handed(Option<Rc<VM>>),
}

impl<VM: 'static> ViewModelRoot<VM> {
    /// The root of a binding of `view` handed `view_model`.
    fn of<V: IViewFor>(view: &Rc<V>, view_model: Option<Rc<VM>>) -> Self {
        if view_for_of::<VM>(view.clone() as Rc<dyn IViewFor>).is_some() {
            Self::FollowView
        } else {
            Self::Handed(view_model)
        }
    }

    /// The view model the binding reads and writes now.
    fn current<V: IViewFor>(&self, view: &Rc<V>) -> Option<Rc<VM>> {
        match self {
            Self::FollowView => view_model_of::<V, VM>(view),
            Self::Handed(view_model) => view_model.clone(),
        }
    }
}

/// The view model `view` holds, when it is a `VM`.
fn view_model_of<V: IViewFor, VM: 'static>(view: &Rc<V>) -> Option<Rc<VM>> {
    IViewFor::view_model(&**view).and_then(|view_model| view_model.downcast::<VM>().ok())
}

/// The view model of a view as the value of an observed path: two values are equal when they are
/// the same object.
struct ViewModelSlot<VM>(Option<Rc<VM>>);

impl<VM> Clone for ViewModelSlot<VM> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<VM> PartialEq for ViewModelSlot<VM> {
    fn eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (Some(first), Some(second)) => Rc::ptr_eq(first, second),
            (None, None) => true,
            _ => false,
        }
    }
}

/// The values of `path` on the view model of a view-first binding: on the view model handed in, or
/// on the view's `ViewModel`, switching to the new one each time the view raises `PropertyChanged`
/// for `ViewModel` (upstream's `SwitchMapSignal` over the view's `ViewModel`, then `UniqueSignal`).
fn observe_view_model<V, VM, T>(view: &Rc<V>, root: &ViewModelRoot<VM>, path: PropertyPath<Rc<VM>, T>) -> Rc<dyn IObservable<T>>
where
    V: IViewFor + INotifyPropertyChanged + 'static,
    VM: 'static,
    T: Clone + PartialEq + 'static,
{
    match root {
        ViewModelRoot::Handed(Some(view_model)) => view_model.when_any_value(path),
        ViewModelRoot::Handed(None) => Observable::empty(),
        ViewModelRoot::FollowView => {
            let slot = PropertyPath::<Rc<V>, ViewModelSlot<VM>>::property("ViewModel", |view: &Rc<V>| ViewModelSlot(view_model_of::<V, VM>(view)));
            view.when_any_value(slot)
                .select(move |slot: ViewModelSlot<VM>| match slot.0 {
                    Some(view_model) => view_model.when_any_value(path.clone()),
                    None => Observable::empty(),
                })
                .switch::<T>()
                .distinct_until_changed()
        }
    }
}

/// The current values along `path` from `root`, as a hook reads them: one change per step, until a
/// step has no object that owns it.
fn hook_changes<S: 'static, V: 'static>(root: Option<&Rc<S>>, path: &PropertyPath<Rc<S>, V>) -> BindingHookChanges {
    let mut changes = Vec::new();
    let Some(root) = root else {
        return changes;
    };
    let mut current: Rc<dyn Any> = Rc::new(root.clone());
    for step in path.steps() {
        let Some(owner) = (step.owner)(&*current).object else {
            break;
        };
        let value: Rc<dyn Any> = Rc::from((step.get)(&*current));
        changes.push(ObservedChange::new(owner, step.name.clone(), value.clone()));
        current = value;
    }
    changes
}

/// Whether the registered hooks let the binding of `source` (read along `source_path`) and
/// `target` (read along `target_path`) proceed; `true` without asking when none is registered.
fn hooks_allow<S: 'static, SV: 'static, T: 'static, TV: 'static>(
    source: Option<&Rc<S>>,
    source_path: &PropertyPath<Rc<S>, SV>,
    target: &Rc<T>,
    target_path: &PropertyPath<Rc<T>, TV>,
    direction: BindingDirection,
) -> bool {
    if !BindingHooks::any() {
        return true;
    }
    let source_object = source.map(|source| source.clone() as Rc<dyn Any>);
    let target_object: Rc<dyn Any> = target.clone();
    BindingHooks::should_bind(
        source_object.as_ref(),
        &target_object,
        &|| hook_changes(source, source_path),
        &|| hook_changes(Some(target), target_path),
        direction,
    )
}

/// Writes `value` to `property` of `owner` unless it holds that value already; returns whether it
/// wrote.
fn write_if_changed<S: 'static, T: PartialEq + 'static>(property: &SettableProperty<S, T>, owner: &S, value: T) -> bool {
    if property.get(owner) == value {
        return false;
    }
    property.set(owner, value);
    true
}

/// The values of `source` that `convert` converts; the values it drops are not delivered
/// (`LinqExtensions.Choose`).
pub(crate) fn choose<F: 'static, T: 'static>(source: Rc<dyn IObservable<F>>, convert: Convert<F, T>) -> Rc<dyn IObservable<T>> {
    Rc::new(ChooseObservable { source, convert })
}

struct ChooseObservable<F, T> {
    source: Rc<dyn IObservable<F>>,
    convert: Convert<F, T>,
}

impl<F: 'static, T: 'static> IObservable<T> for ChooseObservable<F, T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        self.source.subscribe(Rc::new(ChooseObserver { observer, convert: self.convert.clone() }))
    }
}

struct ChooseObserver<F, T> {
    observer: Rc<dyn IObserver<T>>,
    convert: Convert<F, T>,
}

impl<F, T> IObserver<F> for ChooseObserver<F, T> {
    fn on_next(&self, value: F) {
        if let Some(converted) = (self.convert)(value) {
            self.observer.on_next(converted);
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.observer.on_error(error);
    }

    fn on_completed(&self) {
        self.observer.on_completed();
    }
}

/// Wires the two directions of a two-way binding: `forward` (already converted and scheduled) is
/// written by `write_forward`, then `reverse` (already converted, without the target's own value
/// at the start) by `write_reverse`. Each write that happened is reported on the change stream,
/// tagged with its side.
fn join_two_way<TF: Clone + 'static, TR: Clone + 'static>(
    forward: Rc<dyn IObservable<TF>>,
    write_forward: impl Fn(TF) -> bool + 'static,
    reverse: Rc<dyn IObservable<TR>>,
    write_reverse: impl Fn(TR) -> bool + 'static,
    binding_expression: String,
) -> (Rc<dyn IDisposable>, Rc<AppliedChangeObservable>) {
    let changes = AppliedChangeObservable::new();
    let forward_changes = changes.clone();
    let to_target = BindingErrors::subscribe(
        forward,
        move |value: TF| {
            let reported = forward_changes.has_observers().then(|| value.clone());
            if write_forward(value) {
                if let Some(value) = reported {
                    forward_changes.on_next(BindingChange::new(value, true));
                }
            }
        },
        binding_expression.clone(),
    );
    let reverse_changes = changes.clone();
    let to_source = BindingErrors::subscribe(
        reverse,
        move |value: TR| {
            let reported = reverse_changes.has_observers().then(|| value.clone());
            if write_reverse(value) {
                if let Some(value) = reported {
                    reverse_changes.on_next(BindingChange::new(value, false));
                }
            }
        },
        binding_expression,
    );
    let subscription: Rc<dyn IDisposable> = CompositeDisposable::from_disposables([to_target, to_source]);
    (subscription, changes)
}

/// `OneWayBind`: the values of `view_model_property`, converted by `convert`, written to
/// `view_property` of `view`, on the view's thread or on `scheduler`; `None` when a hook refuses
/// the binding.
pub(crate) fn one_way_bind<V, VM, TVM, TV>(
    view: &Rc<V>,
    view_model: Option<Rc<VM>>,
    view_model_property: PropertyPath<Rc<VM>, TVM>,
    view_property: SettableProperty<Rc<V>, TV>,
    convert: Convert<TVM, TV>,
    scheduler: Option<Rc<dyn IScheduler>>,
) -> Option<Rc<dyn IReactiveBinding<V, TV>>>
where
    V: IViewFor + INotifyPropertyChanged + 'static,
    VM: 'static,
    TVM: Clone + PartialEq + 'static,
    TV: Clone + PartialEq + 'static,
{
    let root = ViewModelRoot::of(view, view_model);
    if !hooks_allow(root.current(view).as_ref(), &view_model_property, view, view_property.path(), BindingDirection::OneWay) {
        return None;
    }
    let values = choose(observe_view_model(view, &root, view_model_property), convert);
    let routed = schedule_write(values, scheduler, Some(view.clone() as Rc<dyn Any>));
    let target = Rc::downgrade(view);
    let binding_expression = view_property.path().path();
    let subscription = BindingErrors::subscribe(
        routed.clone(),
        move |value: TV| {
            if let Some(view) = target.upgrade() {
                write_if_changed(&view_property, &view, value);
            }
        },
        binding_expression,
    );
    let binding: Rc<dyn IReactiveBinding<V, TV>> = ReactiveBinding::new(view.clone(), routed, BindingDirection::OneWay, subscription);
    Some(binding)
}

/// `Bind`: `view_model_property` and `view_property` of `view` bound to each other, converted by
/// `forward` towards the view and `reverse` towards the view model; the writes to the view are
/// delivered on its thread or on `scheduler`. `None` when a hook refuses the binding.
#[allow(clippy::too_many_arguments)]
pub(crate) fn bind<V, VM, TVM, TV>(
    view: &Rc<V>,
    view_model: Option<Rc<VM>>,
    view_model_property: SettableProperty<Rc<VM>, TVM>,
    view_property: SettableProperty<Rc<V>, TV>,
    forward: Convert<TVM, TV>,
    reverse: Convert<TV, TVM>,
    scheduler: Option<Rc<dyn IScheduler>>,
) -> Option<Rc<dyn IReactiveBinding<V, BindingChange>>>
where
    V: IViewFor + INotifyPropertyChanged + 'static,
    VM: 'static,
    TVM: Clone + PartialEq + 'static,
    TV: Clone + PartialEq + 'static,
{
    let root = ViewModelRoot::of(view, view_model);
    if !hooks_allow(root.current(view).as_ref(), view_model_property.path(), view, view_property.path(), BindingDirection::TwoWay) {
        return None;
    }
    let binding_expression = format!("{} / {}", view_model_property.path().path(), view_property.path().path());
    let to_view = schedule_latest_write(
        choose(observe_view_model(view, &root, view_model_property.path().clone()), forward),
        scheduler,
        Some(view.clone() as Rc<dyn Any>),
    );
    let from_view = choose(view.when_any_value(view_property.path().clone()).skip(1), reverse);

    let view_target = Rc::downgrade(view);
    let write_view = move |value: TV| view_target.upgrade().is_some_and(|view| write_if_changed(&view_property, &view, value));
    let view_source = Rc::downgrade(view);
    let write_view_model = move |value: TVM| {
        let Some(view) = view_source.upgrade() else {
            return false;
        };
        let Some(view_model) = root.current(&view) else {
            return false;
        };
        write_if_changed(&view_model_property, &view_model, value)
    };

    let (subscription, changes) = join_two_way(to_view, write_view, from_view, write_view_model, binding_expression);
    let changed: Rc<dyn IObservable<BindingChange>> = changes;
    let binding: Rc<dyn IReactiveBinding<V, BindingChange>> = ReactiveBinding::new(view.clone(), changed, BindingDirection::TwoWay, subscription);
    Some(binding)
}

/// The direction signals of a triggered binding: `true` for the view model, `false` for the view,
/// and the update stream in place of one side (`CreateTriggerSource`).
fn trigger_source<D: 'static>(
    view_model_signals: Rc<dyn IObservable<bool>>,
    view_signals: Rc<dyn IObservable<bool>>,
    updates: Option<Rc<dyn IObservable<D>>>,
    direction: TriggerUpdate,
) -> Rc<dyn IObservable<bool>> {
    match (updates, direction) {
        (None, _) => view_model_signals.merge(view_signals),
        (Some(updates), TriggerUpdate::ViewToViewModel) => view_model_signals.merge(updates.select(|_: D| false)),
        (Some(updates), TriggerUpdate::ViewModelToView) => view_model_signals.take(1).merge(updates.select(|_: D| true)).merge(view_signals),
    }
}

/// Reads the current values of both sides when a signal is delivered, converts, compares and
/// writes to the side opposite the signal (`TriggeredBindingPlan<TVMProp, TVProp>`).
struct TriggeredPlan<V: 'static, VM: 'static, TVM: 'static, TV: 'static> {
    view: Weak<V>,
    root: ViewModelRoot<VM>,
    view_model_property: SettableProperty<Rc<VM>, TVM>,
    view_property: SettableProperty<Rc<V>, TV>,
    forward: Convert<TVM, TV>,
    reverse: Convert<TV, TVM>,
    /// Whether the first delivery has initialized the view from the view model.
    initialized: Cell<bool>,
}

impl<V, VM, TVM, TV> TriggeredPlan<V, VM, TVM, TV>
where
    V: IViewFor + 'static,
    VM: 'static,
    TVM: Clone + PartialEq + 'static,
    TV: Clone + PartialEq + 'static,
{
    /// Applies a signal from the view model (`true`) or the view (`false`): skips it when a side
    /// is unavailable, when the conversion fails or when the destination holds the converted value
    /// already; otherwise writes, and returns the change (`Project`, then `Write`). The first
    /// signal applied always initializes the view from the view model.
    fn apply(&self, from_view_model: bool) -> Option<BindingChange> {
        let view = self.view.upgrade()?;
        let view_model = self.root.current(&view)?;
        let model_value = self.view_model_property.get(&view_model);
        let view_value = self.view_property.get(&view);
        let from_view_model = if self.initialized.replace(true) { from_view_model } else { true };
        if from_view_model {
            let converted = (self.forward)(model_value)?;
            if converted == view_value {
                return None;
            }
            self.view_property.set(&view, converted.clone());
            Some(BindingChange::new(converted, true))
        } else {
            let converted = (self.reverse)(view_value)?;
            if converted == model_value {
                return None;
            }
            self.view_model_property.set(&view_model, converted.clone());
            Some(BindingChange::new(converted, false))
        }
    }
}

/// `Bind` with an update stream (`BindUnsafe(..., IObservable<TDontCare>? signalViewUpdate,
/// TriggerUpdate)`): the view model and the view, or the update stream in place of one of them,
/// signal which side moved; the current values are read on the view's thread when the signal is
/// delivered, converted, compared and written to the other side. `None` when a hook refuses the
/// binding.
#[allow(clippy::too_many_arguments)]
pub(crate) fn bind_triggered<V, VM, TVM, TV, D>(
    view: &Rc<V>,
    view_model: Option<Rc<VM>>,
    view_model_property: SettableProperty<Rc<VM>, TVM>,
    view_property: SettableProperty<Rc<V>, TV>,
    forward: Convert<TVM, TV>,
    reverse: Convert<TV, TVM>,
    signal_view_update: Option<Rc<dyn IObservable<D>>>,
    trigger_update: TriggerUpdate,
) -> Option<Rc<dyn IReactiveBinding<V, BindingChange>>>
where
    V: IViewFor + INotifyPropertyChanged + 'static,
    VM: 'static,
    TVM: Clone + PartialEq + 'static,
    TV: Clone + PartialEq + 'static,
    D: 'static,
{
    let root = ViewModelRoot::of(view, view_model);
    if !hooks_allow(root.current(view).as_ref(), view_model_property.path(), view, view_property.path(), BindingDirection::TwoWay) {
        return None;
    }
    let view_model_signals = observe_view_model(view, &root, view_model_property.path().clone()).select(|_: TVM| true);
    let view_signals = view.when_any_value(view_property.path().clone()).select(|_: TV| false);
    let source = trigger_source(view_model_signals, view_signals, signal_view_update, trigger_update);
    let signals: Rc<dyn IObservable<bool>> = Rc::new(InitialBindingSignal::new(source));
    let routed = BindingSchedulers::observe_on_view_thread(signals, Some(view.clone() as Rc<dyn Any>));

    let binding_expression = view_model_property.path().path();
    let plan = TriggeredPlan {
        view: Rc::downgrade(view),
        root,
        view_model_property,
        view_property,
        forward,
        reverse,
        initialized: Cell::new(false),
    };
    let changes: Rc<Subject<BindingChange>> = Subject::new();
    let reported = changes.clone();
    let subscription = BindingErrors::subscribe(
        routed,
        move |from_view_model: bool| {
            if let Some(change) = plan.apply(from_view_model) {
                reported.on_next(change);
            }
        },
        binding_expression,
    );
    let completed = changes.clone();
    let completion = Disposable::create(move || completed.on_completed());
    let subscription: Rc<dyn IDisposable> = CompositeDisposable::from_disposables([subscription, completion]);
    let binding: Rc<dyn IReactiveBinding<V, BindingChange>> = ReactiveBinding::new(view.clone(), changes.as_observable(), BindingDirection::TwoWay, subscription);
    Some(binding)
}

/// `BindOneWay`: the values of `source_property` of `source`, converted by `convert`, written to
/// `target_property` of `target`, on the target's thread or on `scheduler`. Nothing is bound when a
/// hook refuses the binding.
pub(crate) fn bind_one_way<S, T, TS, TT>(
    source: &Rc<S>,
    target: &Rc<T>,
    source_property: PropertyPath<Rc<S>, TS>,
    target_property: SettableProperty<Rc<T>, TT>,
    convert: Convert<TS, TT>,
    scheduler: Option<Rc<dyn IScheduler>>,
) -> Rc<dyn IDisposable>
where
    S: 'static,
    T: 'static,
    TS: Clone + PartialEq + 'static,
    TT: Clone + PartialEq + 'static,
{
    if !hooks_allow(Some(source), &source_property, target, target_property.path(), BindingDirection::OneWay) {
        return Disposable::empty();
    }
    let binding_expression = format!("{} / {}", source_property.path(), target_property.path().path());
    let values = schedule_write(choose(source.when_any_value(source_property), convert), scheduler, Some(target.clone() as Rc<dyn Any>));
    let written = Rc::downgrade(target);
    BindingErrors::subscribe(
        values,
        move |value: TT| {
            if let Some(target) = written.upgrade() {
                write_if_changed(&target_property, &target, value);
            }
        },
        binding_expression,
    )
}

/// `BindTwoWay`: `source_property` of `source` and `target_property` of `target` bound to each
/// other, converted by `forward` towards the target and `reverse` towards the source; the writes to
/// the target are delivered on its thread or on `scheduler`. Nothing is bound when a hook refuses
/// the binding.
pub(crate) fn bind_two_way<S, T, TS, TT>(
    source: &Rc<S>,
    target: &Rc<T>,
    source_property: SettableProperty<Rc<S>, TS>,
    target_property: SettableProperty<Rc<T>, TT>,
    forward: Convert<TS, TT>,
    reverse: Convert<TT, TS>,
    scheduler: Option<Rc<dyn IScheduler>>,
) -> Rc<dyn IDisposable>
where
    S: 'static,
    T: 'static,
    TS: Clone + PartialEq + 'static,
    TT: Clone + PartialEq + 'static,
{
    if !hooks_allow(Some(source), source_property.path(), target, target_property.path(), BindingDirection::TwoWay) {
        return Disposable::empty();
    }
    let binding_expression = format!("{} / {}", source_property.path().path(), target_property.path().path());
    let to_target = schedule_latest_write(
        choose(source.when_any_value(source_property.path().clone()), forward),
        scheduler,
        Some(target.clone() as Rc<dyn Any>),
    );
    let to_source = choose(target.when_any_value(target_property.path().clone()).skip(1), reverse);

    let written_target = Rc::downgrade(target);
    let write_target = move |value: TT| written_target.upgrade().is_some_and(|target| write_if_changed(&target_property, &target, value));
    let written_source = Rc::downgrade(source);
    let write_source = move |value: TS| written_source.upgrade().is_some_and(|source| write_if_changed(&source_property, &source, value));

    join_two_way(to_target, write_target, to_source, write_source, binding_expression).0
}

/// `BindTo`: each value of `source`, converted by `convert`, written to `property` of `target`, on
/// the target's thread. Nothing is bound when `target` is `None` or a hook refuses the binding.
pub(crate) fn bind_to<V, T, TT>(source: Rc<dyn IObservable<V>>, target: Option<Rc<T>>, property: SettableProperty<Rc<T>, TT>, convert: Convert<V, TT>) -> Rc<dyn IDisposable>
where
    V: 'static,
    T: 'static,
    TT: Clone + PartialEq + 'static,
{
    let Some(target) = target else {
        return Disposable::empty();
    };
    if BindingHooks::any() {
        let source_object: Rc<dyn Any> = Rc::new(source.clone());
        let target_object: Rc<dyn Any> = target.clone();
        if !BindingHooks::should_bind(Some(&source_object), &target_object, &Vec::new, &|| hook_changes(Some(&target), property.path()), BindingDirection::OneWay) {
            return Disposable::empty();
        }
    }
    let routed = BindingSchedulers::observe_on_view_thread(choose(source, convert), Some(target.clone() as Rc<dyn Any>));
    let written = Rc::downgrade(&target);
    let binding_expression = property.path().path();
    BindingErrors::subscribe(
        routed,
        move |value: TT| {
            if let Some(target) = written.upgrade() {
                write_if_changed(&property, &target, value);
            }
        },
        binding_expression,
    )
}
