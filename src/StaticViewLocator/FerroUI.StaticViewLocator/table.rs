//! The tables a generated locator holds: the views of the view model types (`s_views`), the
//! messages of the view model types whose view is not found (`s_missingViews`), and the tables of
//! the generic type definitions of the legacy fallback.
//!
//! Not an upstream file: the generated source of the reference declares `Dictionary<Type, ..>`
//! fields with collection initializers. A Rust static cannot hold a `HashMap`, and
//! `TypeId::of` is not a constant function, so a generated table is a constant slice of entries,
//! each with the function that gives the `TypeId` of its view model type, indexed into a map the
//! first time it is read.

use std::any::TypeId;
use std::collections::HashMap;
use std::fmt;
use std::sync::OnceLock;

/// An entry of a [`TypeTable`]: the type, as the function that gives its `TypeId`
/// (`TypeId::of::<T>`), and the value of the type.
pub struct TypeEntry<V: 'static> {
    type_id: fn() -> TypeId,
    value: V,
}

impl<V: 'static> TypeEntry<V> {
    /// The entry of the type whose id `type_id` gives (`TypeId::of::<T>`).
    pub const fn new(type_id: fn() -> TypeId, value: V) -> Self {
        Self { type_id, value }
    }

    /// The id of the type.
    pub fn type_id(&self) -> TypeId {
        (self.type_id)()
    }

    /// The value of the type.
    pub fn value(&self) -> &V {
        &self.value
    }
}

/// A table of values by type (`Dictionary<Type, V>` of the generated source), declared as a
/// static of constant entries and indexed on its first lookup. Of two entries of one type the
/// first is the value.
pub struct TypeTable<V: Copy + Send + Sync + 'static> {
    entries: &'static [TypeEntry<V>],
    index: OnceLock<HashMap<TypeId, V>>,
}

impl<V: Copy + Send + Sync + 'static> TypeTable<V> {
    /// The table of `entries`.
    pub const fn new(entries: &'static [TypeEntry<V>]) -> Self {
        Self { entries, index: OnceLock::new() }
    }

    fn index(&self) -> &HashMap<TypeId, V> {
        self.index.get_or_init(|| {
            let mut index = HashMap::with_capacity(self.entries.len());
            for entry in self.entries {
                index.entry(entry.type_id()).or_insert(entry.value);
            }
            index
        })
    }

    /// The value of the type `type_id` (`TryGetValue`).
    pub fn get(&self, type_id: TypeId) -> Option<V> {
        if self.entries.is_empty() {
            return None;
        }
        self.index().get(&type_id).copied()
    }

    /// Whether the table has a value for the type `type_id` (`ContainsKey`).
    pub fn contains(&self, type_id: TypeId) -> bool {
        self.get(type_id).is_some()
    }

    /// The entries, in the order of the generated source.
    pub fn entries(&self) -> &'static [TypeEntry<V>] {
        self.entries
    }

    /// The number of types of the table (`Count`).
    pub fn len(&self) -> usize {
        if self.entries.is_empty() {
            return 0;
        }
        self.index().len()
    }

    /// Whether the table has no type.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<V: Copy + Send + Sync + 'static> fmt::Debug for TypeTable<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TypeTable").field("entries", &self.entries.len()).finish()
    }
}

/// A table of values by the path of a generic type definition (`type.GetGenericTypeDefinition()`
/// as the key of the legacy fallback): the path of the declaration of the type, as
/// `std::any::type_name` writes it without its generic arguments
/// (`my_app::view_models::WidgetViewModel`).
///
/// Not an upstream type: a Rust generic type has no run-time definition, and the `TypeId` of each
/// of its instantiations is its own. The legacy fallback of the reference finds the view of
/// `WidgetViewModel<int>` through the definition `WidgetViewModel<>`; the port finds it through
/// the path of the definition, which [`RuntimeType::generic_definition`](crate::RuntimeType::generic_definition)
/// gives for a value.
pub struct NameTable<V: Copy + 'static> {
    entries: &'static [(&'static str, V)],
}

impl<V: Copy + 'static> NameTable<V> {
    /// The table of `entries`, each a path and its value.
    pub const fn new(entries: &'static [(&'static str, V)]) -> Self {
        Self { entries }
    }

    /// The value of the path `path`, the first of the table.
    pub fn get(&self, path: &str) -> Option<V> {
        self.entries.iter().find(|(entry, _)| *entry == path).map(|(_, value)| *value)
    }

    /// The entries, in the order of the generated source.
    pub fn entries(&self) -> &'static [(&'static str, V)] {
        self.entries
    }

    /// The number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the table has no entry.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl<V: Copy + 'static> fmt::Debug for NameTable<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NameTable").field("entries", &self.entries.len()).finish()
    }
}
