//! The value types of styled properties of this crate: the form the property store keeps them in
//! ([`StyledValue`], declared in `ferroui_system` and re-exported here).
//!
//! The property store (`EffectiveValue`, the typed paths of `ValueStore`, the entries of the
//! value frames) is compiled once for each type it keeps values as, its storage type, and not
//! once for each value type of a property: the members of every enumeration are kept as one
//! [`EnumValue`](crate::EnumValue), every handle to an object as a `Ref<FerroObject>`, and an
//! `Option` of either as an `Option` of the same form (docs/binary-size.md, section 4.25).

use crate::{styled_value, ElementRef, FerroObject, ObjectType, Ref, StyledValue, WeakRef};
use std::any::Any;

/// A handle to an object is kept as a handle to a [`FerroObject`]: an upcast in, and on the way
/// out the checked downcast of [`Ref::downcast`], which the store only ever asks of a handle a
/// value of the property's type was made from.
impl<T: ObjectType> StyledValue for Ref<T> {
    type Storage = Ref<FerroObject>;
    const SHARED_STORAGE: bool = true;

    #[inline]
    fn into_storage(self) -> Ref<FerroObject> {
        self.upcast()
    }

    #[inline]
    fn from_storage(storage: Ref<FerroObject>) -> Self {
        // A property of handles to any object: the stored handle is the value, with no check.
        let mut slot = Some(storage);
        if let Some(same) = (&mut slot as &mut dyn Any).downcast_mut::<Option<Ref<T>>>() {
            return same.take().expect("the stored handle");
        }
        match slot.expect("the stored handle").downcast::<T>() {
            Ok(handle) => handle,
            Err(_) => foreign_handle(),
        }
    }
}

#[cold]
#[inline(never)]
#[track_caller]
fn foreign_handle<T>() -> T {
    panic!("a stored handle is read as a handle to a class its object is not")
}

/// The small plain structures of numbers are kept as their numbers: two for a point, a size and
/// a vector, four for a thickness, a rectangle and the radii of corners, each count one storage
/// type (`[f64; 2]`, `[f64; 4]`), as the members of the enumerations share one.
macro_rules! kept_as_numbers {
    ($($type_:ty => [$($field:ident),*];)*) => {
        $(
            impl StyledValue for $type_ {
                type Storage = [f64; kept_as_numbers!(@count $($field)*)];

                #[inline(always)]
                fn into_storage(self) -> Self::Storage {
                    [$(self.$field),*]
                }

                #[inline(always)]
                fn from_storage(storage: Self::Storage) -> Self {
                    let [$($field),*] = storage;
                    Self { $($field),* }
                }
            }
        )*
    };
    (@count $($field:ident)*) => { 0 $(+ kept_as_numbers!(@one $field))* };
    (@one $field:ident) => { 1 };
}

kept_as_numbers! {
    crate::Point => [x, y];
    crate::Size => [width, height];
    crate::Vector => [x, y];
    crate::Thickness => [left, top, right, bottom];
    crate::Rect => [x, y, width, height];
    crate::CornerRadius => [top_left, top_right, bottom_right, bottom_left];
}

// The value types of this crate that are their own storage type.
styled_value!(
    crate::Matrix, crate::PixelRect, crate::RelativePoint, crate::RelativeRect, crate::RelativeScalar,
    crate::media::Color, crate::media::BoxShadow, crate::media::BoxShadows, crate::input::KeyGesture,
    crate::animation::Transitions, crate::controls::NameScopeRef, crate::styling::ThemeVariant,
    crate::Vector3D, crate::media::FontFamily, crate::media::FontWeight, crate::media::HsvColor,
    crate::media::text_formatting::TextTabStops, crate::animation::SignedDuration,
    crate::animation::RepeatBehavior, crate::animation::TimelineDuration, crate::utilities::CultureInfo,
    crate::utilities::DateTime, crate::utilities::DateTimeOffset, crate::utilities::Decimal,
    crate::utilities::Uri, crate::utilities::UriRef,
    crate::interactivity::RoutedEvent, crate::data::core::ValueType, crate::numerics::Quaternion,
);

styled_value!(impl<T> for crate::collections::FerroList<T>);
styled_value!(impl<K, V> for crate::collections::FerroDictionary<K, V>);


impl<T: ObjectType> StyledValue for ElementRef<T> {
    type Storage = Self;

    #[inline(always)]
    fn into_storage(self) -> Self {
        self
    }

    #[inline(always)]
    fn from_storage(storage: Self) -> Self {
        storage
    }

    #[inline(always)]
    fn with_storage<R>(storage: &Self, read: impl FnOnce(&Self) -> R) -> R {
        read(storage)
    }
}

impl<T: ObjectType> StyledValue for WeakRef<T>
where
    WeakRef<T>: Clone + PartialEq,
{
    type Storage = Self;

    #[inline(always)]
    fn into_storage(self) -> Self {
        self
    }

    #[inline(always)]
    fn from_storage(storage: Self) -> Self {
        storage
    }

    #[inline(always)]
    fn with_storage<R>(storage: &Self, read: impl FnOnce(&Self) -> R) -> R {
        read(storage)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::HorizontalAlignment;

    #[test]
    fn a_handle_is_kept_as_a_handle_to_an_object_and_read_back() {
        assert!(<Ref<FerroObject> as StyledValue>::SHARED_STORAGE);
        assert!(<Option<Ref<crate::Visual>> as StyledValue>::SHARED_STORAGE);
        let object = FerroObject::new();
        let stored = object.clone().into_storage();
        assert!(Ref::<FerroObject>::from_storage(stored).ptr_eq(&object));
    }

    #[test]
    fn a_markup_enumeration_is_kept_as_its_numeric_value() {
        let stored = HorizontalAlignment::Right.into_storage();
        assert_eq!(HorizontalAlignment::Right as i64, stored.value());
        assert_eq!(HorizontalAlignment::Right, HorizontalAlignment::from_storage(stored));
    }
}
