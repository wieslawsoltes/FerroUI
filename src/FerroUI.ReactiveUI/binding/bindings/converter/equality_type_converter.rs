//! `EqualityTypeConverter` (`ReactiveUI.Binding/Bindings/Converter/EqualityTypeConverter.cs`).

use crate::{ConverterType, IBindingTypeConverter};
use ferroui_splat::Guid;
use std::any::Any;
use std::rc::Rc;
use std::time::Duration;

/// Converts any value to whether it equals the conversion hint (`EqualityTypeConverter`): the
/// converter of last resort, from `dyn Any` (upstream's `object`) to `bool`, with an affinity of 1.
/// It always succeeds; no value (`None`) equals only no hint.
///
/// Two values are equal when they are the same object, or when they have the same type and that
/// type is one whose equality is known here: the integers, the floating-point numbers (`NaN` equals
/// itself, as `Double.Equals` has it), `bool`, `char`, `()`, [`Guid`], [`Duration`], an `Option`
/// of those, and the strings (`String` and `&'static str`, which compare with each other).
// Deviation (DEVIATIONS.md, ReactiveUI): upstream calls the virtual `Object.Equals`; without
// reflection the equality of type-erased values is known for the types above only, and values of
// other types are equal only when they are the same object.
#[derive(Clone, Copy, Debug, Default)]
pub struct EqualityTypeConverter;

impl IBindingTypeConverter for EqualityTypeConverter {
    fn from_type(&self) -> ConverterType {
        ConverterType::of::<dyn Any>()
    }

    fn to_type(&self) -> ConverterType {
        ConverterType::of::<bool>()
    }

    fn get_affinity_for_objects(&self) -> i32 {
        1
    }

    fn try_convert_typed(&self, from: Option<&dyn Any>, conversion_hint: Option<&dyn Any>) -> Option<Rc<dyn Any>> {
        Some(Rc::new(values_equal(from, conversion_hint)))
    }
}

/// The value as a string slice, when it is a `String` or a `&'static str`.
fn as_str(value: &dyn Any) -> Option<&str> {
    value.downcast_ref::<String>().map(String::as_str).or_else(|| value.downcast_ref::<&'static str>().copied())
}

/// `Object.Equals(left, right)` for the values whose equality is known (see
/// [`EqualityTypeConverter`]).
fn values_equal(left: Option<&dyn Any>, right: Option<&dyn Any>) -> bool {
    let (left, right) = match (left, right) {
        (None, None) => return true,
        (Some(left), Some(right)) => (left, right),
        _ => return false,
    };
    if Any::type_id(left) == Any::type_id(right) && std::ptr::addr_eq(left as *const dyn Any, right as *const dyn Any) {
        return true;
    }
    if let (Some(left), Some(right)) = (as_str(left), as_str(right)) {
        return left == right;
    }

    macro_rules! equal_as {
        ($($ty:ty),* $(,)?) => {
            $(
                if let (Some(left), Some(right)) = (left.downcast_ref::<$ty>(), right.downcast_ref::<$ty>()) {
                    return left == right;
                }
                if let (Some(left), Some(right)) = (left.downcast_ref::<Option<$ty>>(), right.downcast_ref::<Option<$ty>>()) {
                    return left == right;
                }
            )*
        };
    }
    macro_rules! equal_as_float {
        ($($ty:ty),* $(,)?) => {
            $(
                if let (Some(left), Some(right)) = (left.downcast_ref::<$ty>(), right.downcast_ref::<$ty>()) {
                    return left == right || (left.is_nan() && right.is_nan());
                }
                if let (Some(left), Some(right)) = (left.downcast_ref::<Option<$ty>>(), right.downcast_ref::<Option<$ty>>()) {
                    return match (left, right) {
                        (Some(left), Some(right)) => left == right || (left.is_nan() && right.is_nan()),
                        (None, None) => true,
                        _ => false,
                    };
                }
            )*
        };
    }

    equal_as!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, bool, char, (), Guid, Duration);
    equal_as_float!(f32, f64);
    false
}
