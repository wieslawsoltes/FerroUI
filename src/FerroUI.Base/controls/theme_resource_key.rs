//! A resource key that carries the type of its value (`keys` of the theme crates).
//!
//! An addition to upstream, whose resource keys are objects. A constant of this type is the text
//! key of a resource of a theme, with the type of the value the theme defines for it, so that a
//! wrong key or value type does not compile.

use super::ResourceKey;
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

/// Declares typed resource keys, grouped by the type of their values; what
/// `scripts/generate_theme_keys.py` writes in the `keys` module of a theme. The constants hold
/// their names through `stringify!`, so that the module has no string literal of a key.
#[macro_export]
macro_rules! theme_keys {
    ($($value:ty => $($name:ident),* $(,)? ;)*) => {
        $($(
            pub const $name: $crate::controls::ThemeResourceKey<$value> =
                $crate::controls::ThemeResourceKey::new(stringify!($name));
        )*)*
    };
}
