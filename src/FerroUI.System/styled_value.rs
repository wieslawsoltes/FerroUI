//! The form a property store keeps a value in ([`StyledValue`]).
//!
//! The property store of FerroUI (`ferroui_base`) is compiled once for each type it keeps values
//! as, the storage type of a value type, and not once for each value type of a property: the
//! members of every enumeration are kept as one [`EnumValue`] and an optional member as an
//! `Option<EnumValue>`. Every other type is its own storage type. The trait is here, beside the
//! other items without a user interface, so that a library that has no user interface (a model
//! of the application) states it for its own types; `ferroui_base` re-exports it with the
//! impls of its own types.
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
    /// storage types ([`EnumValue`], `Option<EnumValue>`), whose values
    /// the store converts back to the value type of the property before it hands them to a
    /// change notification. A type that is only its own storage type keeps `false`.
    #[doc(hidden)]
    const SHARED_STORAGE: bool = false;

    /// The value as the property store keeps it.
    fn into_storage(self) -> Self::Storage;

    /// The value a stored value stands for. `storage` was made by
    /// [`into_storage`](Self::into_storage) of a value of this type.
    fn from_storage(storage: Self::Storage) -> Self;

    /// The members of an enumeration kept as an [`EnumValue`]: the numeric value of each and
    /// the member itself, a static value. A change notification of such a property hands the
    /// member out as it is, without converting the stored value back. Empty for other types.
    #[doc(hidden)]
    const MEMBERS: &'static [(i64, &'static dyn Any)] = &[];

    /// The static value a stored value of a storage type stands for, found in `members` (the
    /// [`MEMBERS`](Self::MEMBERS) of the value type): the member of an enumeration. `None` for
    /// every other storage type.
    #[doc(hidden)]
    #[inline(always)]
    fn member_of(&self, members: &'static [(i64, &'static dyn Any)]) -> Option<&'static dyn Any> {
        let _ = members;
        None
    }

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

            const MEMBERS: &'static [(i64, &'static dyn ::core::any::Any)] = &[
                $((
                    $crate::__styled_enum_variant!($type_, $member $(, $variant)?) as i64,
                    &$crate::__styled_enum_variant!($type_, $member $(, $variant)?) as &dyn ::core::any::Any,
                ),)*
            ];

            #[inline(always)]
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

            #[inline(always)]
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
    fn member_of(&self, members: &'static [(i64, &'static dyn Any)]) -> Option<&'static dyn Any> {
        // The members are usually numbered from zero in order: the place of the value first.
        match members.get(self.0 as usize) {
            Some((value, member)) if *value == self.0 => Some(*member),
            _ => members.iter().find(|(value, _)| *value == self.0).map(|(_, member)| *member),
        }
    }

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

/// An `Option` is kept as an `Option` of the storage type of its value: an optional member of an
/// enumeration as `Option<EnumValue>`, and an optional value of any other type as it is.
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

styled_value!(char, i8, i16, i128, isize, u8, u16, u128, usize, (), String, std::time::Duration);

/// The common scalar types are kept inline in a [`SmallValue`].
macro_rules! small_scalars {
    ($($type_:ty => $variant:ident),* $(,)?) => {
        $(
            impl StyledValue for $type_ {
                type Storage = SmallValue;

                #[inline(always)]
                fn into_storage(self) -> SmallValue {
                    SmallValue::$variant(self)
                }

                #[inline(always)]
                fn from_storage(storage: SmallValue) -> Self {
                    match storage {
                        SmallValue::$variant(value) => value,
                        _ => SmallValue::foreign(),
                    }
                }
            }
        )*
    };
}

small_scalars!(bool => Bool, i32 => I32, u32 => U32, i64 => I64, u64 => U64, f32 => F32, f64 => F64);

/// A value kept by [`SmallValue::Boxed`]: its comparison, for one compiled form of the store.
#[doc(hidden)]
pub trait SmallBoxed: Any {
    /// Whether `other` holds the same type and an equal value.
    fn eq_boxed(&self, other: &dyn SmallBoxed) -> bool;
}

impl<T: PartialEq + 'static> SmallBoxed for T {
    fn eq_boxed(&self, other: &dyn SmallBoxed) -> bool {
        let other: &dyn Any = other;
        other.downcast_ref::<T>().is_some_and(|other| self == other)
    }
}

/// The form the property store keeps the common value types in: a tag and the value inline for
/// the scalars, colors and geometry, and a shared box for other value types that name it as their
/// storage type ([`styled_boxed_value!`](crate::styled_boxed_value)). One storage type for all of
/// them, so that the store is compiled once for all of them.
///
/// Every read goes through a `match` on the tag or a checked downcast: a value read as another
/// type panics ([`SmallValue::foreign`]), and no value is reinterpreted. The comparison of the
/// scalars is that of their type, so `NaN` differs from itself here too; the property store treats
/// two `NaN` as the same value in `value_equals`, through [`SmallValue::both_nan`].
#[derive(Clone)]
pub enum SmallValue {
    /// A `bool`.
    Bool(bool),
    /// An `i32`.
    I32(i32),
    /// A `u32`, or a color as its ARGB value.
    U32(u32),
    /// An `i64`.
    I64(i64),
    /// A `u64`.
    U64(u64),
    /// An `f32`.
    F32(f32),
    /// An `f64`.
    F64(f64),
    /// Two coordinates: a point, a size or a vector.
    F64x2([f64; 2]),
    /// Four coordinates: a rectangle, a thickness or a corner radius.
    F64x4([f64; 4]),
    /// Any other value type that is kept here.
    Boxed(Rc<dyn SmallBoxed>),
}

impl PartialEq for SmallValue {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Bool(a), Self::Bool(b)) => a == b,
            (Self::I32(a), Self::I32(b)) => a == b,
            (Self::U32(a), Self::U32(b)) => a == b,
            (Self::I64(a), Self::I64(b)) => a == b,
            (Self::U64(a), Self::U64(b)) => a == b,
            (Self::F32(a), Self::F32(b)) => a == b,
            (Self::F64(a), Self::F64(b)) => a == b,
            (Self::F64x2(a), Self::F64x2(b)) => a == b,
            (Self::F64x4(a), Self::F64x4(b)) => a == b,
            (Self::Boxed(a), Self::Boxed(b)) => a.eq_boxed(&**b),
            _ => false,
        }
    }
}

impl SmallValue {
    /// `value` in a box of its own.
    #[inline]
    pub fn boxed<T: PartialEq + 'static>(value: T) -> Self {
        Self::Boxed(Rc::new(value))
    }

    /// The value of a box, as `T`. Panics for any other stored value.
    #[inline]
    pub fn unboxed<T: Clone + 'static>(self) -> T {
        self.boxed_ref::<T>().clone()
    }

    /// The value of a box, borrowed as `T`. Panics for any other stored value.
    #[inline]
    pub fn boxed_ref<T: 'static>(&self) -> &T {
        if let Self::Boxed(value) = self {
            let value: &dyn Any = &**value;
            if let Some(value) = value.downcast_ref::<T>() {
                return value;
            }
        }
        Self::foreign()
    }

    /// Whether both are the same floating point type and both are not a number.
    #[inline]
    pub fn both_nan(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::F64(a), Self::F64(b)) => a.is_nan() && b.is_nan(),
            (Self::F32(a), Self::F32(b)) => a.is_nan() && b.is_nan(),
            _ => false,
        }
    }

    /// A stored value read as a type it does not hold. The store never does that.
    #[cold]
    #[inline(never)]
    #[track_caller]
    pub fn foreign<T>() -> T {
        panic!("a stored value is read as another type")
    }
}

impl StyledValue for SmallValue {
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

/// Implements [`StyledValue`] for types the property store keeps in the shared box of a
/// [`SmallValue`].
#[macro_export]
macro_rules! styled_boxed_value {
    ($($type_:ty),* $(,)?) => {
        $(
            impl $crate::StyledValue for $type_ {
                type Storage = $crate::SmallValue;

                #[inline]
                fn into_storage(self) -> $crate::SmallValue {
                    $crate::SmallValue::boxed(self)
                }

                #[inline]
                fn from_storage(storage: $crate::SmallValue) -> Self {
                    storage.unboxed()
                }

                #[inline]
                fn with_storage<R>(storage: &$crate::SmallValue, read: impl FnOnce(&Self) -> R) -> R {
                    read(storage.boxed_ref::<Self>())
                }
            }
        )*
    };
}

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
    fn the_members_of_an_enumeration_are_found_by_their_stored_value() {
        for side in [Side::Left, Side::Right, Side::Top] {
            let member = side.into_storage().member_of(Side::MEMBERS).expect("a member");
            assert_eq!(Some(&side), member.downcast_ref::<Side>());
        }
        assert!(EnumValue::new(1).member_of(Side::MEMBERS).is_none());
        assert!(f64::MEMBERS.is_empty());
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
    fn a_small_value_is_read_back_as_its_type() {
        assert!(f64::from_storage(2.5f64.into_storage()) == 2.5);
        assert!(bool::from_storage(true.into_storage()));
        assert_eq!(7u32, u32::from_storage(7u32.into_storage()));
        assert!(SmallValue::SHARED_STORAGE);
        assert!(!f64::SHARED_STORAGE);
    }

    #[test]
    fn nan_differs_from_itself_as_a_small_value_and_both_nan_says_so() {
        let nan = f64::NAN.into_storage();
        assert!(nan != nan.clone());
        assert!(nan.both_nan(&nan.clone()));
        assert!(!nan.both_nan(&1.0f64.into_storage()));
        assert!(!f32::NAN.into_storage().both_nan(&nan));
    }

    #[test]
    fn a_boxed_value_compares_by_its_type_and_is_read_checked() {
        let (a, b) = (SmallValue::boxed(String::from("a")), SmallValue::boxed(String::from("a")));
        assert!(a == b);
        assert!(a != SmallValue::boxed(1u8));
        assert_eq!("a", a.boxed_ref::<String>());
        assert_eq!("a", b.unboxed::<String>());
    }

    #[test]
    #[should_panic(expected = "read as another type")]
    fn a_boxed_value_is_not_read_as_another_type() {
        let _ = SmallValue::boxed(1u8).boxed_ref::<u16>();
    }

    #[test]
    #[should_panic(expected = "read as another type")]
    fn a_small_value_is_not_read_as_another_type() {
        let _ = f64::from_storage(SmallValue::Bool(true));
    }

    #[test]
    fn an_option_of_a_type_that_is_its_own_storage_type_is_read_borrowed() {
        let stored = Some(String::from("text"));
        let address = Option::<String>::with_storage(&stored, |value| value as *const Option<String>);
        assert_eq!(&stored as *const Option<String>, address);
    }
}
