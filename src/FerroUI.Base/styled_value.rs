//! The value types of styled properties of this crate: the form the property store keeps them in
//! ([`StyledValue`], declared in `ferroui_system` and re-exported here).
//!
//! The property store (`EffectiveValue`, the typed paths of `ValueStore`, the entries of the
//! value frames) is compiled once for each type it keeps values as, its storage type, and not
//! once for each value type of a property: the members of every enumeration are kept as one
//! [`EnumValue`](crate::EnumValue), and an optional member as an `Option` of it
//! (docs/binary-size.md, section 4.27).

use crate::{styled_value, ElementRef, ObjectType, Ref, StyledValue, WeakRef};

/// A handle to an object is its own storage type. Kept as a handle to a [`FerroObject`], a read
/// would check the class of the object again (the downcast of a handle, +45 % instructions for
/// the read of a handle in the gate of #986) and a change notification would convert both values
/// back through the routes of the property.
impl<T: ObjectType> StyledValue for Ref<T> {
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

// The value types of this crate that are their own storage type.
styled_value!(
    crate::Point, crate::Rect, crate::Size, crate::Vector, crate::Thickness, crate::CornerRadius,
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
    fn a_handle_is_its_own_storage_type() {
        assert!(!<Ref<crate::FerroObject> as StyledValue>::SHARED_STORAGE);
        assert!(!<Option<Ref<crate::Visual>> as StyledValue>::SHARED_STORAGE);
    }

    #[test]
    fn a_markup_enumeration_is_kept_as_its_numeric_value() {
        let stored = HorizontalAlignment::Right.into_storage();
        assert_eq!(HorizontalAlignment::Right as i64, stored.value());
        assert_eq!(HorizontalAlignment::Right, HorizontalAlignment::from_storage(stored));
        let member = stored.member_of(HorizontalAlignment::MEMBERS).expect("a member");
        assert_eq!(Some(&HorizontalAlignment::Right), member.downcast_ref::<HorizontalAlignment>());
    }
}
