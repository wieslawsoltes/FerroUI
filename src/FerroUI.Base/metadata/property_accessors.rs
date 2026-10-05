//! The Rust accessors of registered property definitions, for the emitter of
//! Rust source of the markup compiler (docs/porting/xaml.md, section 9).
//!
//! Not a port of an upstream file: upstream's compiler loads a property
//! definition by reading its static field. In the port a definition is
//! returned by an accessor function (`Border::background_property()`), and
//! generated code must name that function.
//!
//! The accessor declaration macros ([`ferro_property!`](crate::ferro_property)
//! with an owner, and so every accessor of a
//! [`ferro_properties!`](crate::ferro_properties) block) record the accessor
//! when the type that declares it registers its properties: the static
//! initialisation of the type runs every accessor of the type once, in
//! declaration order, and that first run records which type declares the
//! accessor, its name, and the definition it returns. The table of a type is
//! therefore complete as soon as the type is initialised, whatever ran
//! before; [`property_accessors`] initialises the types it looks at, so its
//! answer does not depend on what the thread did earlier.
//!
//! A definition can have several accessors (an owner added with `add_owner`
//! declares its own accessor returning the same definition); each of them
//! returns the identical definition.
//!
//! Only with the `compiler-metadata` feature, which the emitter enables:
//! without it the macros record nothing and the table does not exist, so
//! shipped applications neither pay for the recording on the first call of
//! an accessor nor carry the table.

use std::cell::RefCell;
use std::collections::HashMap;

use crate::{FerroProperty, TypeInfo};

/// An accessor of a property definition: the type whose `impl` block
/// declares it and the name of the function.
#[derive(Clone, Copy, Debug)]
pub struct PropertyAccessor {
    pub owner: &'static TypeInfo,
    pub name: &'static str,
}

thread_local! {
    /// The accessors each type declares, by the address of its runtime type,
    /// in declaration order, with the definition each returns.
    static ACCESSORS: RefCell<HashMap<usize, Vec<(&'static str, &'static FerroProperty)>>> =
        RefCell::new(HashMap::new());
}

fn key(type_: &'static TypeInfo) -> usize {
    type_ as *const TypeInfo as usize
}

/// Records that the function `name` declared by `owner` returns `property`.
/// Called by the accessor declaration macros when the accessor first runs
/// on a thread, which is the registration of the properties of `owner`;
/// recording the same accessor again has no effect.
pub fn record_property_accessor(owner: &'static TypeInfo, name: &'static str, property: &'static FerroProperty) {
    ACCESSORS.with(|accessors| {
        let mut accessors = accessors.borrow_mut();
        let list = accessors.entry(key(owner)).or_default();
        if !list.iter().any(|(known, _)| *known == name) {
            list.push((name, property));
        }
    });
}

/// The accessors declared by `type_` itself, in declaration order, with the
/// definition each returns. The type is initialised first, so the list is
/// complete. A type without a static initialisation of its own registers
/// nothing and declares no accessor this table can know of.
pub fn declared_property_accessors(type_: &'static TypeInfo) -> Vec<(PropertyAccessor, &'static FerroProperty)> {
    if !type_.has_class_init() {
        return Vec::new();
    }
    type_.ensure_class_init();
    ACCESSORS.with(|accessors| {
        accessors
            .borrow()
            .get(&key(type_))
            .map(|list| {
                list.iter()
                    .map(|&(name, property)| (PropertyAccessor { owner: type_, name }, property))
                    .collect()
            })
            .unwrap_or_default()
    })
}

/// The accessors of `property`, in a fixed order that depends only on the
/// declarations: those declared by `preferred` (the type the property was
/// resolved on) and then by each of its base types, then those declared by
/// the type that registered the property and its base types. Within a type
/// the order is the declaration order. Accessors declared by other types
/// (an `add_owner` in an unrelated class) are not listed.
pub fn property_accessors(property: &'static FerroProperty, preferred: Option<&'static TypeInfo>) -> Vec<PropertyAccessor> {
    let mut types: Vec<&'static TypeInfo> = Vec::new();
    for start in preferred.into_iter().chain(std::iter::once(property.owner_type())) {
        let mut current = Some(start);
        while let Some(type_) = current {
            if !types.iter().any(|known| std::ptr::eq(*known, type_)) {
                types.push(type_);
            }
            current = type_.base_type();
        }
    }
    types
        .into_iter()
        .flat_map(declared_property_accessors)
        .filter(|(_, returned)| std::ptr::eq(*returned, property))
        .map(|(accessor, _)| accessor)
        .collect()
}
