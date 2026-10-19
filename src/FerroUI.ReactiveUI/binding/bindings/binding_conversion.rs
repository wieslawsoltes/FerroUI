//! The conversion of the values a binding moves between two types: the converter the call names,
//! else the one registered for the pair of types, else the value itself when it already is of the
//! target type (the conversion code upstream's source generator writes at each call site, and
//! `RuntimeBindingConverter`).

use crate::{BindingConverters, ConverterType, IBindingTypeConverter};
use std::any::{Any, TypeId};
use std::rc::Rc;

/// A conversion of the values of a binding: `None` drops the value, which is not written.
pub(crate) type Convert<F, T> = Rc<dyn Fn(F) -> Option<T>>;

/// The affinity of the built-in converters ([`BindingAffinity::DEFAULT_INTERNAL_TYPE_CONVERTER`](crate::BindingAffinity::DEFAULT_INTERNAL_TYPE_CONVERTER)).
/// A registered converter of this affinity or less is ignored between two values of one type,
/// which pass as they are.
const BUILT_IN_AFFINITY: i32 = 2;

/// The converter registered for `F` to `T` in the current service
/// (`BindingConverters.Current.TypedConverters.TryGetConverter(typeof(F), typeof(T))`); between two
/// values of one type, only a converter of an affinity above the built-in ones.
// Deviation (DEVIATIONS.md, ReactiveUI): upstream's generated code ignores every registered
// converter of affinity 2 or less, the built-in ones, because it writes the same conversions inline
// (`ToString`, `int.TryParse`, ...). Without a generator, the port uses the built-in converters for
// two different types, and ignores them only between two values of one type.
pub(crate) fn registered_converter<F: 'static, T: 'static>() -> Option<Rc<dyn IBindingTypeConverter>> {
    let converter = BindingConverters::current().typed_converters().try_get_converter(ConverterType::of::<F>(), ConverterType::of::<T>())?;
    let same_type = TypeId::of::<F>() == TypeId::of::<T>();
    (!same_type || converter.get_affinity_for_objects() > BUILT_IN_AFFINITY).then_some(converter)
}

/// The conversion from `F` to `T` of a binding: through `converter_override` when the call names
/// one, else through the converter registered for the pair (looked up once, when the binding is
/// made), with `conversion_hint`; a value the converter refuses, or a value with no converter,
/// passes when it already is a `T`, and is dropped otherwise.
// Deviation (DEVIATIONS.md, ReactiveUI): with no converter, upstream's generator also writes the
// conversions it knows at compile time (`ToString` with a format hint, for a string target); here
// only a value of the target's own type passes.
pub(crate) fn converting<F: 'static, T: Clone + 'static>(converter_override: Option<Rc<dyn IBindingTypeConverter>>, conversion_hint: Option<Rc<dyn Any>>) -> Convert<F, T> {
    let converter = converter_override.or_else(registered_converter::<F, T>);
    Rc::new(move |value: F| {
        if let Some(converter) = &converter {
            if let Some(converted) = converter.try_convert_typed(Some(&value as &dyn Any), conversion_hint.as_deref()) {
                return unshare(converted);
            }
        }
        pass_through(value)
    })
}

/// The conversion that applies `selector` to every value (a `Func<TFrom, TTo>` of the call).
pub(crate) fn selecting<F: 'static, T: 'static>(selector: impl Fn(F) -> T + 'static) -> Convert<F, T> {
    Rc::new(move |value: F| Some(selector(value)))
}

/// `value` as a `T`, when it is one.
fn pass_through<F: 'static, T: 'static>(value: F) -> Option<T> {
    let boxed: Box<dyn Any> = Box::new(value);
    boxed.downcast::<T>().ok().map(|value| *value)
}

/// The converted value, when it is a `T`.
fn unshare<T: Clone + 'static>(converted: Rc<dyn Any>) -> Option<T> {
    let converted = converted.downcast::<T>().ok()?;
    Some(Rc::try_unwrap(converted).unwrap_or_else(|shared| (*shared).clone()))
}
