//! `ReactiveUIBindingExtensions.Binding.cs`: the property bindings of a view and its view model
//! (`OneWayBind`, `Bind`) and of two objects (`BindOneWay`, `BindTwoWay`), with the scheduler forms
//! of `ReactiveSchedulerExtensions.cs` and the update-stream forms of the triggered bindings.
//!
//! Upstream's methods are stubs that a source generator replaces at each call site; here each is
//! the binding itself, written once over property paths (in `binding/bindings/property_binder.rs`).
//!
//! The side a binding writes is a [`SettableProperty`] (a property path with a setter, written by
//! [`settable_prop!`](crate::settable_prop)); the side it only reads is a [`PropertyPath`]
//! (`prop!`). Rust has no overloads, so upstream's overloads are methods of their own:
//!
//! | Upstream | Here |
//! |---|---|
//! | `OneWayBind(vm, vmProp, viewProp)` | [`one_way_bind`](PropertyBindingMixins::one_way_bind) |
//! | `OneWayBind(vm, vmProp, viewProp, selector)` | [`one_way_bind_select`](PropertyBindingMixins::one_way_bind_select) |
//! | `OneWayBind(vm, vmProp, viewProp, selector, scheduler)` | [`one_way_bind_select_on`](PropertyBindingMixins::one_way_bind_select_on) |
//! | `OneWayBind(vm, vmProp, viewProp, converter, hint, scheduler)` | [`one_way_bind_with`](PropertyBindingMixins::one_way_bind_with) |
//! | `Bind(vm, vmProp, viewProp)` | [`bind`](PropertyBindingMixins::bind) |
//! | `Bind(vm, vmProp, viewProp, toView, toViewModel)` | [`bind_select`](PropertyBindingMixins::bind_select) |
//! | `Bind(vm, vmProp, viewProp, toView, toViewModel, scheduler)` | [`bind_select_on`](PropertyBindingMixins::bind_select_on) |
//! | `Bind(vm, vmProp, viewProp, converter, converter, hint, scheduler)` | [`bind_with`](PropertyBindingMixins::bind_with) |
//! | `BindUnsafe(vm, vmProp, viewProp, signal, triggerUpdate)` | [`bind_with_trigger`](PropertyBindingMixins::bind_with_trigger) |
//! | `BindUnsafe(vm, vmProp, viewProp, toView, toViewModel, signal, triggerUpdate)` | [`bind_select_with_trigger`](PropertyBindingMixins::bind_select_with_trigger) |
//! | `BindOneWay(target, sourceProp, targetProp)` | [`bind_one_way`](SourceBindingMixins::bind_one_way) |
//! | `BindOneWay(target, sourceProp, targetProp, conversion)` | [`bind_one_way_select`](SourceBindingMixins::bind_one_way_select) |
//! | `BindOneWay(..., scheduler)` | [`bind_one_way_on`](SourceBindingMixins::bind_one_way_on), [`bind_one_way_select_on`](SourceBindingMixins::bind_one_way_select_on) |
//! | `BindOneWay(..., converter, hint, scheduler)` | [`bind_one_way_with`](SourceBindingMixins::bind_one_way_with) |
//! | `BindTwoWay(target, sourceProp, targetProp)` | [`bind_two_way`](SourceBindingMixins::bind_two_way) |
//! | `BindTwoWay(target, sourceProp, targetProp, toTarget, toSource)` | [`bind_two_way_select`](SourceBindingMixins::bind_two_way_select) |
//! | `BindTwoWay(..., scheduler)` | [`bind_two_way_on`](SourceBindingMixins::bind_two_way_on), [`bind_two_way_select_on`](SourceBindingMixins::bind_two_way_select_on) |
//! | `BindTwoWay(..., converter, converter, hint, scheduler)` | [`bind_two_way_with`](SourceBindingMixins::bind_two_way_with) |

use crate::binding::bindings::binding_conversion::{converting, selecting};
use crate::binding::bindings::{property_binder, BindingChange, SettableProperty, TriggerUpdate};
use crate::binding::interfaces::IReactiveBinding;
use crate::{IBindingTypeConverter, IViewFor};
use ferroui_reactive::concurrency::IScheduler;
use ferroui_system::component_model::{INotifyPropertyChanged, PropertyPath};
use ferroui_system::{IDisposable, IObservable};
use std::any::Any;
use std::rc::Rc;

/// The bindings of a view to its view model (`ReactiveUIBindingExtensions.OneWayBind`, `Bind`, and
/// their forms of `ReactiveSchedulerExtensions`); implemented for every `Rc<V>` of a view `V`.
///
/// **The view model.** A view typed for the view model (one that implements
/// [`IViewForOf<VM>`](crate::IViewForOf), as [`view_for!`](crate::view_for) writes it) binds
/// through its own view model: the binding follows the view's `ViewModel`, so that replacing the
/// view model (with the view raising `PropertyChanged` for `ViewModel`) rebinds, and a view without
/// one waits for it; `view_model` only names its type. A view that names no type of view model
/// binds the `view_model` handed in. A binding with no view model binds nothing, and does not fail.
///
/// **The writes.** The view property is written on the thread that owns the view (through the
/// registered [`IViewThreadInvoker`](crate::IViewThreadInvoker)), or on the scheduler the call
/// names, and only when it does not hold the value already. A two-way binding seeds the view from
/// the view model, then mirrors each later change of either side; the view's own value at the start
/// is not written back.
///
/// **The conversions.** Without functions, a value is converted by the converter the call names,
/// else by the one registered for the pair of types in
/// [`BindingConverters`](crate::BindingConverters), else passed when it already is of the target
/// type; a value that cannot be converted is not written.
///
/// A registered [`IPropertyBindingHook`](crate::IPropertyBindingHook) may refuse a binding: the
/// method then returns `None`. The binding holds its view weakly in its writes; the
/// [`IReactiveBinding`] it returns holds it.
// Deviation (DEVIATIONS.md, ReactiveUI): the bindings are methods with the paths as values, where
// upstream's stubs are replaced by generated code at each call site; the view is
// `INotifyPropertyChanged`, as following its `ViewModel` needs.
pub trait PropertyBindingMixins<V: IViewFor + INotifyPropertyChanged + 'static> {
    /// The view.
    #[doc(hidden)]
    fn bound_view(&self) -> &Rc<V>;

    /// Binds `view_model_property` one way to `view_property`: the current value and each later
    /// change are written to the view (`OneWayBind(TViewModel?, Expression, Expression)`).
    fn one_way_bind<VM, TVM, TV>(
        &self,
        view_model: impl Into<Option<Rc<VM>>>,
        view_model_property: PropertyPath<Rc<VM>, TVM>,
        view_property: SettableProperty<Rc<V>, TV>,
    ) -> Option<Rc<dyn IReactiveBinding<V, TV>>>
    where
        VM: 'static,
        TVM: Clone + PartialEq + 'static,
        TV: Clone + PartialEq + 'static,
    {
        property_binder::one_way_bind(self.bound_view(), view_model.into(), view_model_property, view_property, converting::<TVM, TV>(None, None), None)
    }

    /// [`one_way_bind`](Self::one_way_bind) through `selector`
    /// (`OneWayBind(TViewModel?, Expression, Expression, Func<TProp, TOut>)`).
    fn one_way_bind_select<VM, TVM, TV>(
        &self,
        view_model: impl Into<Option<Rc<VM>>>,
        view_model_property: PropertyPath<Rc<VM>, TVM>,
        view_property: SettableProperty<Rc<V>, TV>,
        selector: impl Fn(TVM) -> TV + 'static,
    ) -> Option<Rc<dyn IReactiveBinding<V, TV>>>
    where
        VM: 'static,
        TVM: Clone + PartialEq + 'static,
        TV: Clone + PartialEq + 'static,
    {
        property_binder::one_way_bind(self.bound_view(), view_model.into(), view_model_property, view_property, selecting(selector), None)
    }

    /// [`one_way_bind_select`](Self::one_way_bind_select), writing on `scheduler`, or on the
    /// view's thread when it is `None` (`ReactiveSchedulerExtensions.OneWayBind(..., Func<TProp,
    /// TOut>, ISequencer?)`).
    fn one_way_bind_select_on<VM, TVM, TV>(
        &self,
        view_model: impl Into<Option<Rc<VM>>>,
        view_model_property: PropertyPath<Rc<VM>, TVM>,
        view_property: SettableProperty<Rc<V>, TV>,
        selector: impl Fn(TVM) -> TV + 'static,
        scheduler: Option<Rc<dyn IScheduler>>,
    ) -> Option<Rc<dyn IReactiveBinding<V, TV>>>
    where
        VM: 'static,
        TVM: Clone + PartialEq + 'static,
        TV: Clone + PartialEq + 'static,
    {
        property_binder::one_way_bind(self.bound_view(), view_model.into(), view_model_property, view_property, selecting(selector), scheduler)
    }

    /// [`one_way_bind`](Self::one_way_bind), converting with `converter` instead of a registered
    /// converter when it is given, with `conversion_hint`, and writing on `scheduler`, or on the
    /// view's thread when it is `None` (`ReactiveSchedulerExtensions.OneWayBind(...,
    /// IBindingTypeConverter, object?, ISequencer?)`).
    fn one_way_bind_with<VM, TVM, TV>(
        &self,
        view_model: impl Into<Option<Rc<VM>>>,
        view_model_property: PropertyPath<Rc<VM>, TVM>,
        view_property: SettableProperty<Rc<V>, TV>,
        converter: Option<Rc<dyn IBindingTypeConverter>>,
        conversion_hint: Option<Rc<dyn Any>>,
        scheduler: Option<Rc<dyn IScheduler>>,
    ) -> Option<Rc<dyn IReactiveBinding<V, TV>>>
    where
        VM: 'static,
        TVM: Clone + PartialEq + 'static,
        TV: Clone + PartialEq + 'static,
    {
        property_binder::one_way_bind(
            self.bound_view(),
            view_model.into(),
            view_model_property,
            view_property,
            converting::<TVM, TV>(converter, conversion_hint),
            scheduler,
        )
    }

    /// Binds `view_model_property` and `view_property` to each other: the view is seeded from the
    /// view model, then each change of either side is written to the other
    /// (`Bind(TViewModel?, Expression, Expression)`). The change stream of the binding reports each
    /// value written, with the side it came from.
    fn bind<VM, TVM, TV>(
        &self,
        view_model: impl Into<Option<Rc<VM>>>,
        view_model_property: SettableProperty<Rc<VM>, TVM>,
        view_property: SettableProperty<Rc<V>, TV>,
    ) -> Option<Rc<dyn IReactiveBinding<V, BindingChange>>>
    where
        VM: 'static,
        TVM: Clone + PartialEq + 'static,
        TV: Clone + PartialEq + 'static,
    {
        property_binder::bind(
            self.bound_view(),
            view_model.into(),
            view_model_property,
            view_property,
            converting::<TVM, TV>(None, None),
            converting::<TV, TVM>(None, None),
            None,
        )
    }

    /// [`bind`](Self::bind) through `view_model_to_view` and `view_to_view_model`
    /// (`Bind(TViewModel?, Expression, Expression, Func<TVMProp, TVProp>, Func<TVProp, TVMProp>)`).
    fn bind_select<VM, TVM, TV>(
        &self,
        view_model: impl Into<Option<Rc<VM>>>,
        view_model_property: SettableProperty<Rc<VM>, TVM>,
        view_property: SettableProperty<Rc<V>, TV>,
        view_model_to_view: impl Fn(TVM) -> TV + 'static,
        view_to_view_model: impl Fn(TV) -> TVM + 'static,
    ) -> Option<Rc<dyn IReactiveBinding<V, BindingChange>>>
    where
        VM: 'static,
        TVM: Clone + PartialEq + 'static,
        TV: Clone + PartialEq + 'static,
    {
        property_binder::bind(
            self.bound_view(),
            view_model.into(),
            view_model_property,
            view_property,
            selecting(view_model_to_view),
            selecting(view_to_view_model),
            None,
        )
    }

    /// [`bind_select`](Self::bind_select), writing to the view on `scheduler`, or on the view's
    /// thread when it is `None`; on a scheduler, only the latest value that waits is written
    /// (`ReactiveSchedulerExtensions.Bind(..., Func, Func, ISequencer?)`).
    fn bind_select_on<VM, TVM, TV>(
        &self,
        view_model: impl Into<Option<Rc<VM>>>,
        view_model_property: SettableProperty<Rc<VM>, TVM>,
        view_property: SettableProperty<Rc<V>, TV>,
        view_model_to_view: impl Fn(TVM) -> TV + 'static,
        view_to_view_model: impl Fn(TV) -> TVM + 'static,
        scheduler: Option<Rc<dyn IScheduler>>,
    ) -> Option<Rc<dyn IReactiveBinding<V, BindingChange>>>
    where
        VM: 'static,
        TVM: Clone + PartialEq + 'static,
        TV: Clone + PartialEq + 'static,
    {
        property_binder::bind(
            self.bound_view(),
            view_model.into(),
            view_model_property,
            view_property,
            selecting(view_model_to_view),
            selecting(view_to_view_model),
            scheduler,
        )
    }

    /// [`bind`](Self::bind), converting with `view_model_to_view_converter` and
    /// `view_to_view_model_converter` instead of the registered converters when they are given,
    /// with `conversion_hint`, and writing to the view on `scheduler`, or on the view's thread when
    /// it is `None` (`ReactiveSchedulerExtensions.Bind(..., IBindingTypeConverter,
    /// IBindingTypeConverter, object?, ISequencer?)`).
    #[allow(clippy::too_many_arguments)]
    fn bind_with<VM, TVM, TV>(
        &self,
        view_model: impl Into<Option<Rc<VM>>>,
        view_model_property: SettableProperty<Rc<VM>, TVM>,
        view_property: SettableProperty<Rc<V>, TV>,
        view_model_to_view_converter: Option<Rc<dyn IBindingTypeConverter>>,
        view_to_view_model_converter: Option<Rc<dyn IBindingTypeConverter>>,
        conversion_hint: Option<Rc<dyn Any>>,
        scheduler: Option<Rc<dyn IScheduler>>,
    ) -> Option<Rc<dyn IReactiveBinding<V, BindingChange>>>
    where
        VM: 'static,
        TVM: Clone + PartialEq + 'static,
        TV: Clone + PartialEq + 'static,
    {
        property_binder::bind(
            self.bound_view(),
            view_model.into(),
            view_model_property,
            view_property,
            converting::<TVM, TV>(view_model_to_view_converter, conversion_hint.clone()),
            converting::<TV, TVM>(view_to_view_model_converter, conversion_hint),
            scheduler,
        )
    }

    /// Binds `view_model_property` and `view_property` to each other, driven by direction
    /// signals (`BindUnsafe(..., IObservable<TDontCare>? signalViewUpdate, TriggerUpdate)`).
    ///
    /// Without `signal_view_update`, a change of either side is written to the other. With it,
    /// [`TriggerUpdate::ViewToViewModel`] writes the view to the view model only when the stream
    /// produces a value (the changes of the view are not observed), and
    /// [`TriggerUpdate::ViewModelToView`] writes the view model to the view only when the stream
    /// produces a value (the changes of the view are still written back). The current values are
    /// read on the view's thread when a signal is delivered, so a signal that waited writes the
    /// values of its delivery. The view is always initialized from the view model.
    fn bind_with_trigger<VM, TVM, TV, D>(
        &self,
        view_model: impl Into<Option<Rc<VM>>>,
        view_model_property: SettableProperty<Rc<VM>, TVM>,
        view_property: SettableProperty<Rc<V>, TV>,
        signal_view_update: Option<Rc<dyn IObservable<D>>>,
        trigger_update: TriggerUpdate,
    ) -> Option<Rc<dyn IReactiveBinding<V, BindingChange>>>
    where
        VM: 'static,
        TVM: Clone + PartialEq + 'static,
        TV: Clone + PartialEq + 'static,
        D: 'static,
    {
        property_binder::bind_triggered(
            self.bound_view(),
            view_model.into(),
            view_model_property,
            view_property,
            converting::<TVM, TV>(None, None),
            converting::<TV, TVM>(None, None),
            signal_view_update,
            trigger_update,
        )
    }

    /// [`bind_with_trigger`](Self::bind_with_trigger) through `view_model_to_view` and
    /// `view_to_view_model` (`BindUnsafe(..., Func, Func, IObservable<TDontCare>?, TriggerUpdate)`).
    #[allow(clippy::too_many_arguments)]
    fn bind_select_with_trigger<VM, TVM, TV, D>(
        &self,
        view_model: impl Into<Option<Rc<VM>>>,
        view_model_property: SettableProperty<Rc<VM>, TVM>,
        view_property: SettableProperty<Rc<V>, TV>,
        view_model_to_view: impl Fn(TVM) -> TV + 'static,
        view_to_view_model: impl Fn(TV) -> TVM + 'static,
        signal_view_update: Option<Rc<dyn IObservable<D>>>,
        trigger_update: TriggerUpdate,
    ) -> Option<Rc<dyn IReactiveBinding<V, BindingChange>>>
    where
        VM: 'static,
        TVM: Clone + PartialEq + 'static,
        TV: Clone + PartialEq + 'static,
        D: 'static,
    {
        property_binder::bind_triggered(
            self.bound_view(),
            view_model.into(),
            view_model_property,
            view_property,
            selecting(view_model_to_view),
            selecting(view_to_view_model),
            signal_view_update,
            trigger_update,
        )
    }
}

impl<V: IViewFor + INotifyPropertyChanged + 'static> PropertyBindingMixins<V> for Rc<V> {
    fn bound_view(&self) -> &Rc<V> {
        self
    }
}

/// The bindings of a property of an object to a property of another
/// (`ReactiveUIBindingExtensions.BindOneWay`, `BindTwoWay`, and their forms of
/// `ReactiveSchedulerExtensions`); implemented for every `Rc<S>`.
///
/// The target property is written on the thread that owns the target (through the registered
/// [`IViewThreadInvoker`](crate::IViewThreadInvoker)), or on the scheduler the call names, and
/// only when it does not hold the value already. A two-way binding seeds the target from the
/// source, then mirrors each later change of either side; the target's own value at the start is
/// not written back. The conversions are those of [`PropertyBindingMixins`]. A registered
/// [`IPropertyBindingHook`](crate::IPropertyBindingHook) may refuse a binding, which then binds
/// nothing. Disposing the result disconnects the binding.
// Deviation (DEVIATIONS.md, ReactiveUI): the binding holds both objects weakly; once one is
// dropped, its side is not written.
pub trait SourceBindingMixins<S: 'static> {
    /// The source object.
    #[doc(hidden)]
    fn bound_source(&self) -> &Rc<S>;

    /// Binds `source_property` one way to `target_property` of `target`
    /// (`BindOneWay(TTarget, Expression, Expression)`).
    fn bind_one_way<T, TS, TT>(&self, target: &Rc<T>, source_property: PropertyPath<Rc<S>, TS>, target_property: SettableProperty<Rc<T>, TT>) -> Rc<dyn IDisposable>
    where
        T: 'static,
        TS: Clone + PartialEq + 'static,
        TT: Clone + PartialEq + 'static,
    {
        property_binder::bind_one_way(self.bound_source(), target, source_property, target_property, converting::<TS, TT>(None, None), None)
    }

    /// [`bind_one_way`](Self::bind_one_way) through `conversion`
    /// (`BindOneWay(TTarget, Expression, Expression, Func<TSourceProp, TTargetProp>)`).
    fn bind_one_way_select<T, TS, TT>(
        &self,
        target: &Rc<T>,
        source_property: PropertyPath<Rc<S>, TS>,
        target_property: SettableProperty<Rc<T>, TT>,
        conversion: impl Fn(TS) -> TT + 'static,
    ) -> Rc<dyn IDisposable>
    where
        T: 'static,
        TS: Clone + PartialEq + 'static,
        TT: Clone + PartialEq + 'static,
    {
        property_binder::bind_one_way(self.bound_source(), target, source_property, target_property, selecting(conversion), None)
    }

    /// [`bind_one_way`](Self::bind_one_way), writing on `scheduler`, or on the target's thread
    /// when it is `None` (`ReactiveSchedulerExtensions.BindOneWay(..., ISequencer?)`).
    fn bind_one_way_on<T, TS, TT>(
        &self,
        target: &Rc<T>,
        source_property: PropertyPath<Rc<S>, TS>,
        target_property: SettableProperty<Rc<T>, TT>,
        scheduler: Option<Rc<dyn IScheduler>>,
    ) -> Rc<dyn IDisposable>
    where
        T: 'static,
        TS: Clone + PartialEq + 'static,
        TT: Clone + PartialEq + 'static,
    {
        property_binder::bind_one_way(self.bound_source(), target, source_property, target_property, converting::<TS, TT>(None, None), scheduler)
    }

    /// [`bind_one_way_select`](Self::bind_one_way_select), writing on `scheduler`, or on the
    /// target's thread when it is `None` (`ReactiveSchedulerExtensions.BindOneWay(..., Func,
    /// ISequencer?)`).
    fn bind_one_way_select_on<T, TS, TT>(
        &self,
        target: &Rc<T>,
        source_property: PropertyPath<Rc<S>, TS>,
        target_property: SettableProperty<Rc<T>, TT>,
        conversion: impl Fn(TS) -> TT + 'static,
        scheduler: Option<Rc<dyn IScheduler>>,
    ) -> Rc<dyn IDisposable>
    where
        T: 'static,
        TS: Clone + PartialEq + 'static,
        TT: Clone + PartialEq + 'static,
    {
        property_binder::bind_one_way(self.bound_source(), target, source_property, target_property, selecting(conversion), scheduler)
    }

    /// [`bind_one_way`](Self::bind_one_way), converting with `converter` instead of a registered
    /// converter when it is given, with `conversion_hint`, and writing on `scheduler`, or on the
    /// target's thread when it is `None` (`ReactiveSchedulerExtensions.BindOneWay(...,
    /// IBindingTypeConverter, object?, ISequencer?)`).
    fn bind_one_way_with<T, TS, TT>(
        &self,
        target: &Rc<T>,
        source_property: PropertyPath<Rc<S>, TS>,
        target_property: SettableProperty<Rc<T>, TT>,
        converter: Option<Rc<dyn IBindingTypeConverter>>,
        conversion_hint: Option<Rc<dyn Any>>,
        scheduler: Option<Rc<dyn IScheduler>>,
    ) -> Rc<dyn IDisposable>
    where
        T: 'static,
        TS: Clone + PartialEq + 'static,
        TT: Clone + PartialEq + 'static,
    {
        property_binder::bind_one_way(
            self.bound_source(),
            target,
            source_property,
            target_property,
            converting::<TS, TT>(converter, conversion_hint),
            scheduler,
        )
    }

    /// Binds `source_property` and `target_property` of `target` to each other
    /// (`BindTwoWay(TTarget, Expression, Expression)`).
    fn bind_two_way<T, TS, TT>(&self, target: &Rc<T>, source_property: SettableProperty<Rc<S>, TS>, target_property: SettableProperty<Rc<T>, TT>) -> Rc<dyn IDisposable>
    where
        T: 'static,
        TS: Clone + PartialEq + 'static,
        TT: Clone + PartialEq + 'static,
    {
        property_binder::bind_two_way(
            self.bound_source(),
            target,
            source_property,
            target_property,
            converting::<TS, TT>(None, None),
            converting::<TT, TS>(None, None),
            None,
        )
    }

    /// [`bind_two_way`](Self::bind_two_way) through `source_to_target` and `target_to_source`
    /// (`BindTwoWay(TTarget, Expression, Expression, Func, Func)`).
    fn bind_two_way_select<T, TS, TT>(
        &self,
        target: &Rc<T>,
        source_property: SettableProperty<Rc<S>, TS>,
        target_property: SettableProperty<Rc<T>, TT>,
        source_to_target: impl Fn(TS) -> TT + 'static,
        target_to_source: impl Fn(TT) -> TS + 'static,
    ) -> Rc<dyn IDisposable>
    where
        T: 'static,
        TS: Clone + PartialEq + 'static,
        TT: Clone + PartialEq + 'static,
    {
        property_binder::bind_two_way(
            self.bound_source(),
            target,
            source_property,
            target_property,
            selecting(source_to_target),
            selecting(target_to_source),
            None,
        )
    }

    /// [`bind_two_way`](Self::bind_two_way), writing to the target on `scheduler`, or on the
    /// target's thread when it is `None`; on a scheduler, only the latest value that waits is
    /// written (`ReactiveSchedulerExtensions.BindTwoWay(..., ISequencer?)`).
    fn bind_two_way_on<T, TS, TT>(
        &self,
        target: &Rc<T>,
        source_property: SettableProperty<Rc<S>, TS>,
        target_property: SettableProperty<Rc<T>, TT>,
        scheduler: Option<Rc<dyn IScheduler>>,
    ) -> Rc<dyn IDisposable>
    where
        T: 'static,
        TS: Clone + PartialEq + 'static,
        TT: Clone + PartialEq + 'static,
    {
        property_binder::bind_two_way(
            self.bound_source(),
            target,
            source_property,
            target_property,
            converting::<TS, TT>(None, None),
            converting::<TT, TS>(None, None),
            scheduler,
        )
    }

    /// [`bind_two_way_select`](Self::bind_two_way_select), writing to the target on `scheduler`,
    /// or on the target's thread when it is `None` (`ReactiveSchedulerExtensions.BindTwoWay(...,
    /// Func, Func, ISequencer?)`).
    fn bind_two_way_select_on<T, TS, TT>(
        &self,
        target: &Rc<T>,
        source_property: SettableProperty<Rc<S>, TS>,
        target_property: SettableProperty<Rc<T>, TT>,
        source_to_target: impl Fn(TS) -> TT + 'static,
        target_to_source: impl Fn(TT) -> TS + 'static,
        scheduler: Option<Rc<dyn IScheduler>>,
    ) -> Rc<dyn IDisposable>
    where
        T: 'static,
        TS: Clone + PartialEq + 'static,
        TT: Clone + PartialEq + 'static,
    {
        property_binder::bind_two_way(
            self.bound_source(),
            target,
            source_property,
            target_property,
            selecting(source_to_target),
            selecting(target_to_source),
            scheduler,
        )
    }

    /// [`bind_two_way`](Self::bind_two_way), converting with `source_to_target_converter` and
    /// `target_to_source_converter` instead of the registered converters when they are given, with
    /// `conversion_hint`, and writing to the target on `scheduler`, or on the target's thread when
    /// it is `None` (`ReactiveSchedulerExtensions.BindTwoWay(..., IBindingTypeConverter,
    /// IBindingTypeConverter, object?, ISequencer?)`).
    #[allow(clippy::too_many_arguments)]
    fn bind_two_way_with<T, TS, TT>(
        &self,
        target: &Rc<T>,
        source_property: SettableProperty<Rc<S>, TS>,
        target_property: SettableProperty<Rc<T>, TT>,
        source_to_target_converter: Option<Rc<dyn IBindingTypeConverter>>,
        target_to_source_converter: Option<Rc<dyn IBindingTypeConverter>>,
        conversion_hint: Option<Rc<dyn Any>>,
        scheduler: Option<Rc<dyn IScheduler>>,
    ) -> Rc<dyn IDisposable>
    where
        T: 'static,
        TS: Clone + PartialEq + 'static,
        TT: Clone + PartialEq + 'static,
    {
        property_binder::bind_two_way(
            self.bound_source(),
            target,
            source_property,
            target_property,
            converting::<TS, TT>(source_to_target_converter, conversion_hint.clone()),
            converting::<TT, TS>(target_to_source_converter, conversion_hint),
            scheduler,
        )
    }
}

impl<S: 'static> SourceBindingMixins<S> for Rc<S> {
    fn bound_source(&self) -> &Rc<S> {
        self
    }
}
