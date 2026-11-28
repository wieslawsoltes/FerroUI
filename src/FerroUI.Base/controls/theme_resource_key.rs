//! A resource key that carries the type of its value (`keys` of the theme crates).
//!
//! An addition to upstream, whose resource keys are objects. A constant of this type is the text
//! key of a resource of a theme, with the type of the value the theme defines for it, so that a
//! wrong key or value type does not compile.

use super::{ResourceKey, ResourceValue};
use crate::metadata::from_markup_value;
use crate::styling::ThemeVariant;
use crate::BoxedValue;
use std::any::TypeId;
use std::fmt;
use std::marker::PhantomData;

/// The key of a resource whose value is a `T`.
pub struct ThemeResourceKey<T> {
    name: &'static str,
    value: PhantomData<fn() -> T>,
}

impl<T> ThemeResourceKey<T> {
    /// The key with the text `name`.
    pub const fn new(name: &'static str) -> Self {
        Self { name, value: PhantomData }
    }

    /// The text of the key.
    pub const fn name(&self) -> &'static str {
        self.name
    }
}

impl<T> Clone for ThemeResourceKey<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for ThemeResourceKey<T> {}

impl<T> PartialEq for ThemeResourceKey<T> {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl<T> Eq for ThemeResourceKey<T> {}

impl<T> fmt::Debug for ThemeResourceKey<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ThemeResourceKey({})", self.name)
    }
}

impl<T> From<ThemeResourceKey<T>> for ResourceKey {
    fn from(key: ThemeResourceKey<T>) -> Self {
        ResourceKey::from(key.name)
    }
}

/// A generated key with the check of its value type, for the test of a theme that resolves every
/// key ([`check_theme_keys`]).
#[doc(hidden)]
pub struct ThemeKeyEntry {
    name: &'static str,
    value_type: fn() -> TypeId,
    type_name: fn() -> &'static str,
    accepts: fn(&ResourceValue) -> bool,
}

impl ThemeKeyEntry {
    /// The entry of the key `name`, whose value is declared to be a `T`.
    pub const fn of<T: Clone + 'static>(name: &'static str) -> Self {
        fn accepts<T: Clone + 'static>(value: &ResourceValue) -> bool {
            from_markup_value::<T>(value).is_some()
        }
        Self { name, value_type: TypeId::of::<T>, type_name: std::any::type_name::<T>, accepts: accepts::<T> }
    }
}

/// What resolving the generated keys of a theme found.
#[doc(hidden)]
#[derive(Debug, Default)]
pub struct ThemeKeyReport {
    /// The keys found in at least one variant.
    pub resolved: usize,
    /// The values whose declared type was checked, over all variants.
    pub checked_values: usize,
    /// The keys declared with no single value type: found, and their values not checked.
    pub key_only: usize,
    /// The keys found in no variant.
    pub missing: Vec<&'static str>,
    /// The values that are not of the declared type: key, variant and declared type.
    pub wrong: Vec<String>,
}

/// Resolves every key of `entries` in every variant through `find`, and checks each value found
/// against the declared type of its key. A key needs a value in at least one variant.
#[doc(hidden)]
pub fn check_theme_keys(
    entries: &[ThemeKeyEntry],
    variants: &[ThemeVariant],
    find: &dyn Fn(&ResourceKey, &ThemeVariant) -> Option<ResourceValue>,
) -> ThemeKeyReport {
    let mut report = ThemeKeyReport::default();
    for entry in entries {
        let key = ResourceKey::from(entry.name);
        let typed = (entry.value_type)() != TypeId::of::<BoxedValue>();
        let mut found = false;
        for variant in variants {
            let Some(value) = find(&key, variant) else { continue };
            found = true;
            if typed {
                report.checked_values += 1;
                if !(entry.accepts)(&value) {
                    report.wrong.push(format!("{} in {variant:?} is not a {}", entry.name, (entry.type_name)()));
                }
            }
        }
        if found {
            report.resolved += 1;
            report.key_only += usize::from(!typed);
        } else {
            report.missing.push(entry.name);
        }
    }
    report
}

/// Declares typed resource keys, grouped by the type of their values; what
/// `scripts/generate_theme_keys.py` writes in the `keys` module of a theme. The constants hold
/// their names through `stringify!`, so that the module has no string literal of a key. In the
/// tests of the crate, `ALL_KEYS` lists them for [`check_theme_keys`].
#[macro_export]
macro_rules! theme_keys {
    ($($value:ty => $($name:ident),* $(,)? ;)*) => {
        $($(
            pub const $name: $crate::controls::ThemeResourceKey<$value> =
                $crate::controls::ThemeResourceKey::new(stringify!($name));
        )*)*

        #[cfg(test)]
        pub(crate) const ALL_KEYS: &[$crate::controls::ThemeKeyEntry] = &[
            $($($crate::controls::ThemeKeyEntry::of::<$value>(stringify!($name)),)*)*
        ];
    };
}
