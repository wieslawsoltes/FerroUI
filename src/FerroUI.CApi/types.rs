//! The entries of the classes (#976, docs/c-abi.md, section 16): one exported function per class
//! of the API model (`ferro_type_button()`, written into `api_types.rs` by
//! `tests/types_tests.rs`), which registers the class (its type, its default constructor) and the
//! units of the Fluent theme it needs, and returns its type ID.
//!
//! Without the tables of types (the by-use mode, the crate built without `type-tables`) nothing
//! names a class but its entry: an application, or the generated bindings of its selection, call
//! the entries of the classes they use, and the linker of an application that links the library
//! statically removes every other class, its markup metadata, and the units of the theme only it
//! needs. With the tables (the full mode, the default) every class is known by name from the start
//! and an entry only returns the ID; the theme is the whole theme.

use crate::error::{status_or_value, Result};
use ferroui_base::{DefaultConstructible, StaticType, TypeInfo};
use std::sync::Mutex;

/// A unit of the Fluent theme, or nothing without the desktop platform (no theme).
#[cfg(feature = "desktop")]
pub type Unit = ferroui_markup_xaml::xaml_il::runtime::compiled::ThemeUnit;
/// A unit of the Fluent theme, or nothing without the desktop platform (no theme).
#[cfg(not(feature = "desktop"))]
pub type Unit = ();

/// The units of the theme a class needs beyond the ambient ones: the closure of the class over
/// the unit graph of the theme (`ferroui-build`'s `kept_units`), computed when `api_types.rs` is
/// written.
pub type Units = &'static [&'static Unit];

/// The units of the theme the entries called so far need (the by-use mode), in no order.
static REGISTERED_UNITS: Mutex<Vec<&'static Unit>> = Mutex::new(Vec::new());

fn remember(units: Units) {
    if !units.is_empty() {
        let mut registered = REGISTERED_UNITS.lock().unwrap_or_else(|error| error.into_inner());
        for unit in units {
            if !registered.iter().any(|known| std::ptr::eq(*known, *unit)) {
                registered.push(unit);
            }
        }
    }
}

/// The units of the theme the registered classes need, with the ambient ones, in canonical order:
/// the parts the application constructs its theme with in the by-use mode.
#[cfg(all(feature = "desktop", not(feature = "type-tables")))]
pub(crate) fn theme_parts(ambient: Units) -> &'static [&'static Unit] {
    let mut parts: Vec<&'static Unit> = ambient.to_vec();
    parts.extend(REGISTERED_UNITS.lock().unwrap_or_else(|error| error.into_inner()).iter().copied());
    parts.sort_by_key(|unit| unit.index);
    parts.dedup_by_key(|unit| unit.index);
    Box::leak(parts.into_boxed_slice())
}

/// Registers the class `T` of the model ID `id` with its default constructor.
pub(crate) fn entry<T: DefaultConstructible>(id: i64, units: Units) -> i64 {
    status_or_value(|| {
        let type_ = TypeInfo::register_constructor::<T>();
        Ok(register(id, type_, units))
    })
}

/// Registers the type `T` (a class without a default constructor, or a type that owns attached
/// properties) of the model ID `id`.
pub(crate) fn entry_static<T: StaticType>(id: i64, units: Units) -> i64 {
    status_or_value(|| -> Result<i64> { Ok(register(id, T::TYPE, units)) })
}

fn register(id: i64, type_: &'static TypeInfo, units: Units) -> i64 {
    TypeInfo::register(type_);
    crate::ids::register_model_type(id, type_);
    remember(units);
    id
}
