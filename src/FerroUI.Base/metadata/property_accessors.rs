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
//! accessor, the type whose `impl` block declares the function, its name,
//! its visibility and the definition it returns. The table of a type is
//! therefore complete as soon as the type is initialised, whatever ran
//! before; [`property_accessors`] initialises the types it looks at, so its
//! answer does not depend on what the thread did earlier.
//!
//! A definition can have several accessors (an owner added with `add_owner`
//! declares its own accessor returning the same definition); each of them
//! returns the identical definition. An accessor is a function of the type
//! whose `impl` block declares it, which is not always the owner the
//! declaration names (`ferro_property!(for StyledElement; ..)` written in
//! `impl ThemeVariant`); generated code calls it through the public Rust path
//! of that type ([`rust_path_of_type`]).
//!
//! Only with the `compiler-metadata` feature, which the emitter enables:
//! without it the macros record nothing and the table does not exist, so
//! shipped applications neither pay for the recording on the first call of
//! an accessor nor carry the table.

use std::any::TypeId;
#[cfg(feature = "compiler-metadata")]
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

#[cfg(feature = "compiler-metadata")]
use crate::{FerroProperty, TypeInfo};

/// An accessor of a property definition.
#[cfg(feature = "compiler-metadata")]
#[derive(Clone, Copy, Debug)]
pub struct PropertyAccessor {
    /// The type whose static initialisation registers the property (the
    /// owner the declaration names).
    pub owner: &'static TypeInfo,
    /// The type whose `impl` block declares the function.
    pub impl_type: TypeId,
    /// The name of the function.
    pub name: &'static str,
    /// Whether the function is `pub` (other crates, and generated code, can
    /// call it).
    pub public: bool,
}

#[cfg(feature = "compiler-metadata")]
thread_local! {
    /// The accessors each type declares, by the address of its runtime type,
    /// in declaration order, with the definition each returns.
    static ACCESSORS: RefCell<HashMap<usize, Vec<(PropertyAccessor, &'static FerroProperty)>>> =
        RefCell::new(HashMap::new());
}

#[cfg(feature = "compiler-metadata")]
fn key(type_: &'static TypeInfo) -> usize {
    type_ as *const TypeInfo as usize
}

/// Records that the function `name` of the type `impl_type`, an accessor of
/// the properties of `owner`, returns `property`; `visibility` is the
/// visibility the function is declared with, as text. Called by the accessor
/// declaration macros when the accessor first runs on a thread, which is the
/// registration of the properties of `owner`; recording the same accessor
/// again has no effect.
#[cfg(feature = "compiler-metadata")]
pub fn record_property_accessor(
    owner: &'static TypeInfo,
    impl_type: TypeId,
    name: &'static str,
    visibility: &'static str,
    property: &'static FerroProperty,
) {
    ACCESSORS.with(|accessors| {
        let mut accessors = accessors.borrow_mut();
        let list = accessors.entry(key(owner)).or_default();
        if !list.iter().any(|(known, _)| known.impl_type == impl_type && known.name == name) {
            list.push((PropertyAccessor { owner, impl_type, name, public: visibility == "pub" }, property));
        }
    });
}

/// The accessors declared for `type_` itself, in declaration order, with the
/// definition each returns. The type is initialised first, so the list is
/// complete. A type without a static initialisation of its own registers
/// nothing and declares no accessor this table can know of.
#[cfg(feature = "compiler-metadata")]
pub fn declared_property_accessors(type_: &'static TypeInfo) -> Vec<(PropertyAccessor, &'static FerroProperty)> {
    if !type_.has_class_init() {
        return Vec::new();
    }
    type_.ensure_class_init();
    ACCESSORS.with(|accessors| accessors.borrow().get(&key(type_)).cloned().unwrap_or_default())
}

/// The accessors of `property`, in a fixed order that depends only on the
/// declarations: those declared for `preferred` (the type the property was
/// resolved on) and then for each of its base types, then those declared for
/// the type that registered the property and its base types. Within a type
/// the order is the declaration order. Accessors declared for other types
/// (an `add_owner` in an unrelated class) are not listed.
#[cfg(feature = "compiler-metadata")]
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

fn type_paths() -> &'static RwLock<HashMap<TypeId, &'static str>> {
    static PATHS: OnceLock<RwLock<HashMap<TypeId, &'static str>>> = OnceLock::new();
    PATHS.get_or_init(Default::default)
}

/// Declares the public Rust paths of types by the `TypeId` of the type
/// itself (`Border`, `ThemeVariant`, `dyn IBrush`), the paths associated
/// functions of the types are called by. Called from the `register_types()`
/// of a crate with the table of its generated `rust_paths.rs`
/// ([`ferro_rust_paths!`](crate::ferro_rust_paths)).
pub fn register_type_rust_paths(paths: &[(fn() -> TypeId, &'static str)]) {
    let mut table = type_paths().write().unwrap_or_else(|e| e.into_inner());
    for (type_id, path) in paths {
        table.insert(type_id(), path);
    }
}

/// The public Rust path of the type with the `TypeId` `type_id`, if its
/// crate recorded one.
pub fn rust_path_of_type(type_id: TypeId) -> Option<&'static str> {
    type_paths().read().unwrap_or_else(|e| e.into_inner()).get(&type_id).copied()
}
