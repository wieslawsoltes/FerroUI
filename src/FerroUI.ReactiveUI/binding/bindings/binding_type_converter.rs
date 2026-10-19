//! `BindingTypeConverter<TFrom, TTo>` (`ReactiveUI.Binding/Bindings/BindingTypeConverter.cs`).

use crate::{ConverterType, IBindingTypeConverter, IBindingTypeConverterOf};
use std::any::Any;
use std::rc::Rc;

/// The base of the typed converters (`BindingTypeConverter<TFrom, TTo>`): a converter implements
/// its typed conversion and its affinity, and the blanket implementations write
/// [`IBindingTypeConverter`] (the source and target types, the type-erased conversion) and
/// [`IBindingTypeConverterOf<From, To>`](IBindingTypeConverterOf) from them.
///
/// ```
/// use ferroui_reactiveui::{BindingTypeConverter, IBindingTypeConverter, IBindingTypeConverterOf};
/// use std::any::Any;
///
/// struct Doubler;
///
/// impl BindingTypeConverter for Doubler {
///     type From = i32;
///     type To = i64;
///
///     fn affinity(&self) -> i32 {
///         1
///     }
///
///     fn convert(&self, from: Option<&i32>, _conversion_hint: Option<&dyn Any>) -> Option<i64> {
///         Some(i64::from(*from?) * 2)
///     }
/// }
///
/// assert_eq!(Doubler.try_convert(Some(&21), None), Some(42));
/// let converted = Doubler.try_convert_typed(Some(&21), None).unwrap();
/// assert_eq!(converted.downcast_ref::<i64>(), Some(&42));
/// assert!(Doubler.try_convert_typed(Some(&"21"), None).is_none());
/// ```
// Deviation (DEVIATIONS.md, ReactiveUI): an abstract generic class becomes a trait with associated
// types, so that the blanket implementations can name them. Its two abstract members are named
// `affinity` and `convert`, so that calls on a converter are not ambiguous with
// `IBindingTypeConverter::get_affinity_for_objects` and `IBindingTypeConverterOf::try_convert`,
// which the blanket implementations forward to them.
pub trait BindingTypeConverter: 'static {
    /// The type converted from (`TFrom`).
    type From: 'static;

    /// The type converted to (`TTo`).
    type To: 'static;

    /// The affinity of the converter (`GetAffinityForObjects`).
    fn affinity(&self) -> i32;

    /// Converts `from` (`None` for upstream's `null`), with an optional `conversion_hint`; `None`
    /// when the conversion fails (`TryConvert`).
    fn convert(&self, from: Option<&Self::From>, conversion_hint: Option<&dyn Any>) -> Option<Self::To>;
}

impl<C: BindingTypeConverter> IBindingTypeConverter for C {
    fn from_type(&self) -> ConverterType {
        ConverterType::of::<C::From>()
    }

    fn to_type(&self) -> ConverterType {
        ConverterType::of::<C::To>()
    }

    fn get_affinity_for_objects(&self) -> i32 {
        self.affinity()
    }

    /// Converts a value of `From` with the typed conversion; a value of another type fails.
    ///
    /// Deviation (DEVIATIONS.md, ReactiveUI): upstream fails a `null` input before the conversion
    /// when `TFrom` is a value type. Without reflection the base cannot tell, so the conversion
    /// receives `None` and decides; the built-in converters of value types fail on it.
    fn try_convert_typed(&self, from: Option<&dyn Any>, conversion_hint: Option<&dyn Any>) -> Option<Rc<dyn Any>> {
        let from = match from {
            None => None,
            Some(value) => Some(value.downcast_ref::<C::From>()?),
        };
        let converted = self.convert(from, conversion_hint)?;
        Some(Rc::new(converted))
    }
}

impl<C: BindingTypeConverter> IBindingTypeConverterOf<C::From, C::To> for C {
    fn try_convert(&self, from: Option<&C::From>, conversion_hint: Option<&dyn Any>) -> Option<C::To> {
        self.convert(from, conversion_hint)
    }
}
