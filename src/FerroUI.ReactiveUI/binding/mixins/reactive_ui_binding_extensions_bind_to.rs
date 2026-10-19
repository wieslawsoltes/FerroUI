//! `ReactiveUIBindingExtensions.BindTo.cs`: `BindTo`, the write of each value of an observable to
//! a property.

use crate::binding::bindings::binding_conversion::converting;
use crate::binding::bindings::{property_binder, SettableProperty};
use crate::IBindingTypeConverter;
use ferroui_system::{IDisposable, IObservable};
use std::any::Any;
use std::rc::Rc;

/// The write of the values of an observable to a property (`ReactiveUIBindingExtensions.BindTo`);
/// implemented for every `Rc<dyn IObservable<V>>`.
///
/// Each value is converted to the type of the property (by the converter the call names, else by
/// the one registered for the pair of types, else passed when it already is of that type; a value
/// that cannot be converted is not written), and written on the thread that owns the target when
/// its platform has one. A value the property already holds is not written again. A registered
/// [`IPropertyBindingHook`](crate::IPropertyBindingHook) may refuse the binding, which binds
/// nothing.
// Deviation (DEVIATIONS.md, ReactiveUI): the target is held weakly; once it is dropped, the values
// are not written.
pub trait BindToMixins<V: 'static> {
    /// The observable.
    #[doc(hidden)]
    fn bound_observable(&self) -> &Rc<dyn IObservable<V>>;

    /// Writes each value to `property` of `target`; nothing is bound when `target` is `None`
    /// (`BindTo(TTarget?, Expression<Func<TTarget, TTargetValue?>>)`).
    fn bind_to<T, TT>(&self, target: impl Into<Option<Rc<T>>>, property: SettableProperty<Rc<T>, TT>) -> Rc<dyn IDisposable>
    where
        T: 'static,
        TT: Clone + PartialEq + 'static,
    {
        property_binder::bind_to(self.bound_observable().clone(), target.into(), property, converting::<V, TT>(None, None))
    }

    /// [`bind_to`](Self::bind_to), converting with `converter_override` instead of a registered
    /// converter when it is given, and handing `conversion_hint` to the converter
    /// (`BindTo(..., object? conversionHint)`, `BindTo(..., IBindingTypeConverter? converterOverride)`
    /// and `BindTo(..., object? conversionHint, IBindingTypeConverter? converterOverride)`).
    fn bind_to_with<T, TT>(
        &self,
        target: impl Into<Option<Rc<T>>>,
        property: SettableProperty<Rc<T>, TT>,
        conversion_hint: Option<Rc<dyn Any>>,
        converter_override: Option<Rc<dyn IBindingTypeConverter>>,
    ) -> Rc<dyn IDisposable>
    where
        T: 'static,
        TT: Clone + PartialEq + 'static,
    {
        property_binder::bind_to(self.bound_observable().clone(), target.into(), property, converting::<V, TT>(converter_override, conversion_hint))
    }
}

impl<V: 'static> BindToMixins<V> for Rc<dyn IObservable<V>> {
    fn bound_observable(&self) -> &Rc<dyn IObservable<V>> {
        self
    }
}
