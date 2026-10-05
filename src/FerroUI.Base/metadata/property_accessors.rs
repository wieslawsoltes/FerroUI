//! The Rust accessors of registered property definitions, for the emitter of
//! Rust source of the markup compiler (docs/porting/xaml.md, section 9).
//!
//! Not a port of an upstream file: upstream's compiler loads a property
//! definition by reading its static field. In the port a definition is
//! returned by an accessor function (`Border::background_property()`), and
//! generated code must name that function. The accessor declaration macros
//! ([`ferro_property!`](crate::ferro_property) with an owner, and so every
//! accessor of a [`ferro_properties!`](crate::ferro_properties) block)
//! record here, the first time an accessor runs on a thread, which type
//! declares it, its name, and the definition it returns. A definition can
//! have several accessors (an owner added with `add_owner` declares its own
//! accessor returning the same definition); each of them returns the
//! identical definition.

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
    static ACCESSORS: RefCell<HashMap<usize, Vec<PropertyAccessor>>> = RefCell::new(HashMap::new());
}

fn key(property: &'static FerroProperty) -> usize {
    property as *const FerroProperty as usize
}

/// Records that the function `name` declared by `owner` returns `property`.
/// Called by the accessor declaration macros; recording the same accessor
/// again has no effect.
pub fn record_property_accessor(owner: &'static TypeInfo, name: &'static str, property: &'static FerroProperty) {
    ACCESSORS.with(|accessors| {
        let mut accessors = accessors.borrow_mut();
        let list = accessors.entry(key(property)).or_default();
        if !list.iter().any(|known| std::ptr::eq(known.owner, owner) && known.name == name) {
            list.push(PropertyAccessor { owner, name });
        }
    });
}

/// The recorded accessors of `property` on this thread, in the order they
/// first ran.
pub fn property_accessors(property: &'static FerroProperty) -> Vec<PropertyAccessor> {
    ACCESSORS.with(|accessors| accessors.borrow().get(&key(property)).cloned().unwrap_or_default())
}
