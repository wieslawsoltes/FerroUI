//! The form a property store keeps a value in ([`StyledValue`]).
//!
//! The property store of FerroUI (`ferroui_base`) is compiled once for each type it keeps values
//! as, the storage type of a value type, and not once for each value type of a property: the
//! members of every enumeration are kept as one [`EnumValue`], every handle to an object as a
//! handle to the root class, and an `Option` of either as an `Option` of the same form. Every
//! other type is its own storage type. The trait is here, beside the other items without a user
//! interface, so that a library that has no user interface (a model of the application) states
//! it for its own types; `ferroui_base` re-exports it with the impls of its own types.
//!
//! What a property hands out is always a value of its value type: a read converts the stored
//! value back ([`StyledValue::from_storage`]), and a boxed value, a binding value or a change
//! notification holds the value type.

use std::any::Any;
use std::rc::Rc;
use std::sync::Arc;

/// A value type of a styled or attached property: the type its values are kept as by the
/// property store, and the conversions to and from it.
///
/// Implemented for the standard types, `Option<T>`, `Rc<T>`, `Arc<T>`, `Vec<T>`, and in
/// `ferroui_base` for object handles (`Ref<T>`, `ElementRef<T>`) and the enumerations declared
/// with its `ferro_markup_enum!`. A type of an application that is the value type of a styled
/// property implements it with [`styled_value!`](crate::styled_value), which keeps the values as
/// they are:
///
/// ```ignore
/// #[derive(Clone, PartialEq)]
/// pub struct Margins { pub inner: f64, pub outer: f64 }
///
/// ferroui_base::styled_value!(Margins);
/// ```
///
/// An enumeration without markup metadata is kept with the members of every other enumeration
/// with [`styled_enum!`](crate::styled_enum).
pub trait StyledValue: Clone + PartialEq + 'static {
    /// The type the property store keeps values of this type as.
    type Storage: StyledValue;

    /// Whether values of other types are kept as values of this type: `true` for the shared
    /// storage types ([`EnumValue`], a handle to the root class and an `Option` of either), whose values
    /// the store converts back to the value type of the property before it hands them to a
    /// change notification. A type that is only its own storage type keeps `false`.
    #[doc(hidden)]
    const SHARED_STORAGE: bool = false;

    /// The value as the property store keeps it.
    fn into_storage(self) -> Self::Storage;

    /// The value a stored value stands for. `storage` was made by
    /// [`into_storage`](Self::into_storage) of a value of this type.
    fn from_storage(storage: Self::Storage) -> Self;

    /// Calls `read` with the value a stored value stands for: borrowed where the type is its
    /// own storage type, a converted copy otherwise.
    #[inline]
    fn with_storage<R>(storage: &Self::Storage, read: impl FnOnce(&Self) -> R) -> R {
        read(&Self::from_storage(storage.clone()))
    }
}

/// Implements [`StyledValue`] for types whose values the property store keeps as they are (the
/// storage type of each is the type itself).
///
/// ```ignore
/// ferroui_base::styled_value!(Thickness, CornerRadius);
/// // A generic type:
/// ferroui_base::styled_value!(impl<T> for Selector<T>);
/// ```
#[macro_export]
macro_rules! styled_value {
    (impl<$($param:ident),* $(,)?> for $type_:ty) => {
        impl<$($param: 'static),*> $crate::StyledValue for $type_
        where
            $type_: ::core::clone::Clone + ::core::cmp::PartialEq,
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
    };
    ($($type_:ty),* $(,)?) => {
        $(
            impl $crate::StyledValue for $type_ {
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
        )*
    };
}

/// Implements [`StyledValue`] for a fieldless enumeration, kept as an [`EnumValue`] with the
/// members of every other enumeration. Every variant is listed: the conversions are a `match`
/// each way, and one that misses a variant does not compile.
///
/// ```ignore
/// ferroui_base::styled_enum!(Placement { Top, Bottom, Left, Right });
/// ```
///
/// An enumeration declared with `ferro_markup_enum!` (`ferroui_base`) has the implementation
/// already.
#[macro_export]
macro_rules! styled_enum {
    ($type_:ident { $($member:ident $(= $variant:ident)?),* $(,)? }) => {
        $crate::__styled_enum!($type_ { $($member $(= $variant)?),* });
    };
}

/// The implementation of [`StyledValue`] for a fieldless enumeration, with the members in the
/// syntax of `ferro_markup_enum!` (`Member` or `Member = Variant`).
#[doc(hidden)]
#[macro_export]
macro_rules! __styled_enum {
    ($type_:ident { $($member:ident $(= $variant:ident)?),* $(,)? }) => {
        impl $crate::StyledValue for $type_ {
            type Storage = $crate::EnumValue;

            #[inline]
            #[allow(unreachable_patterns)]
            fn into_storage(self) -> $crate::EnumValue {
                // The match names every variant: an enumeration with one more does not compile
                // here, so `from_storage` below knows every value it can be given.
                $crate::EnumValue::new(match self {
                    $(
                        $crate::__styled_enum_variant!($type_, $member $(, $variant)?) => {
                            $crate::__styled_enum_variant!($type_, $member $(, $variant)?) as i64
                        }
                    )*
                })
            }

            #[inline]
            fn from_storage(storage: $crate::EnumValue) -> Self {
                let value = storage.value();
                $(
                    if value == $crate::__styled_enum_variant!($type_, $member $(, $variant)?) as i64 {
                        return $crate::__styled_enum_variant!($type_, $member $(, $variant)?);
                    }
                )*
                $crate::EnumValue::foreign()
            }
        }
    };
}

/// The variant of an enumeration member written `Member` or `Member = Variant`.
#[doc(hidden)]
#[macro_export]
macro_rules! __styled_enum_variant {
    ($type_:ident, $member:ident) => {
        $type_::$member
    };
    ($type_:ident, $member:ident, $variant:ident) => {
        $type_::$variant
    };
}

/// The form the property store keeps a member of an enumeration in: its numeric value. One
/// storage type for every enumeration, so that the store is compiled once for all of them; what
/// a property hands out is the member itself ([`StyledValue::from_storage`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EnumValue(i64);

impl EnumValue {
    /// The stored form of the member whose numeric value is `value`.
    #[inline]
    pub const fn new(value: i64) -> Self {
        Self(value)
    }

    /// The numeric value of the member.
    #[inline]
    pub const fn value(self) -> i64 {
        self.0
    }

    /// A stored value that is no member of the enumeration it is read as: a value of another
    /// enumeration given to the store of a property. The store never does that.
    #[cold]
    #[inline(never)]
    #[track_caller]
    pub fn foreign<T>() -> T {
        panic!("a stored enumeration value is read as another enumeration")
    }
}

impl StyledValue for EnumValue {
    type Storage = Self;
    const SHARED_STORAGE: bool = true;

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

/// An `Option` is kept as an `Option` of the storage type of its value: `Option<Ref<T>>` as
/// `Option<Ref<FerroObject>>`, an optional member of an enumeration as `Option<EnumValue>`, and
/// an optional value of any other type as it is.
impl<T: StyledValue> StyledValue for Option<T> {
    type Storage = Option<T::Storage>;
    const SHARED_STORAGE: bool = T::SHARED_STORAGE;

    #[inline]
    fn into_storage(self) -> Option<T::Storage> {
        self.map(T::into_storage)
    }

    #[inline]
    fn from_storage(storage: Option<T::Storage>) -> Self {
        storage.map(T::from_storage)
    }

    #[inline]
    fn with_storage<R>(storage: &Option<T::Storage>, read: impl FnOnce(&Self) -> R) -> R {
        // An `Option` of a type that is its own storage type is its own storage type too: the
        // stored value is borrowed (the check is decided when the function is compiled).
        if let Some(value) = (storage as &dyn Any).downcast_ref::<Self>() {
            return read(value);
        }
        read(&Self::from_storage(storage.clone()))
    }
}

styled_value!(
    bool, char, f32, f64, i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, (), String,
    std::time::Duration,
);

/// Two or four numbers: the form the property store keeps the small plain structures of
/// numbers in (a point, a size, a vector; a thickness, a rectangle, the radii of corners), one
/// storage type for each count. Equal as the structures are equal: number by number.
macro_rules! shared_number_storage {
    ($($type_:ty),*) => {
        $(
            impl StyledValue for $type_ {
                type Storage = Self;
                const SHARED_STORAGE: bool = true;

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
        )*
    };
}

shared_number_storage!([f64; 2], [f64; 4]);

// The value types of other crates that properties of FerroUI hold.
#[cfg(feature = "locale")]
styled_value!(icu_locale_core::Locale);
#[cfg(feature = "url")]
styled_value!(url::Url);

/// The generic types the property store keeps values of as they are.
macro_rules! generic_styled_values {
    ($(impl<$($param:ident $(: ?$sized:ident)?),*> for $type_:ty;)*) => {
        $(
            impl<$($param: $(?$sized +)? 'static),*> StyledValue for $type_
            where
                $type_: Clone + PartialEq,
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
        )*
    };
}

generic_styled_values! {
    impl<T: ?Sized> for Rc<T>;
    impl<T: ?Sized> for Arc<T>;
    impl<T: ?Sized> for &'static T;
    impl<T> for Vec<T>;
    impl<T> for Box<T>;
    impl<A, B> for (A, B);
    impl<A, B, C> for (A, B, C);
    impl<A, B, C, D> for (A, B, C, D);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq)]
    enum Side {
        Left,
        Right = 7,
        Top = -3,
    }

    styled_enum!(Side { Left, Right, Top });

    #[derive(Clone, Copy, Debug, PartialEq)]
    enum Renamed {
        First,
        Self_,
    }

    styled_enum!(Renamed { First, Self = Self_ });

    #[test]
    fn an_enumeration_is_kept_as_its_numeric_value_and_read_back() {
        for side in [Side::Left, Side::Right, Side::Top] {
            let stored = side.into_storage();
            assert_eq!(side as i64, stored.value());
            assert_eq!(side, Side::from_storage(stored));
        }
        assert_eq!(Some(EnumValue::new(7)), Some(Side::Right).into_storage());
        assert_eq!(None, Option::<Side>::from_storage(None));
        assert_eq!(Renamed::Self_, Renamed::from_storage(Renamed::Self_.into_storage()));
    }

    #[test]
    #[should_panic(expected = "read as another enumeration")]
    fn a_value_of_another_enumeration_is_not_read() {
        let _ = Side::from_storage(EnumValue::new(1));
    }

    #[test]
    fn the_shared_storage_types_say_so() {
        assert!(EnumValue::SHARED_STORAGE);
        assert!(<Option<EnumValue> as StyledValue>::SHARED_STORAGE);
        assert!(!f64::SHARED_STORAGE);
        assert!(!<Option<String> as StyledValue>::SHARED_STORAGE);
    }

    #[test]
    fn an_option_of_a_type_that_is_its_own_storage_type_is_read_borrowed() {
        let stored = Some(String::from("text"));
        let address = Option::<String>::with_storage(&stored, |value| value as *const Option<String>);
        assert_eq!(&stored as *const Option<String>, address);
    }
}
