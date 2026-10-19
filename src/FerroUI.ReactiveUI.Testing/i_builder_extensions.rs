//! `IBuilderExtensions` (`ReactiveUI.Testing/IBuilderExtensions.cs`).

use crate::IBuilder;
use std::collections::HashMap;
use std::hash::Hash;

/// The fluent setters of a builder of test data (`IBuilderExtensions`): each sets or fills a field
/// of the builder and returns the builder.
///
/// Deviation (DEVIATIONS.md, ReactiveUI): upstream passes the field by reference (`out` or `ref`)
/// next to the builder, which is a reference type; a Rust builder is moved through the chain, so
/// the field is named by an accessor, a closure that gives the field of the builder:
///
/// ```
/// use ferroui_reactiveui_testing::{IBuilder, IBuilderExtensions};
///
/// #[derive(Default)]
/// struct PersonBuilder {
///     name: String,
///     tags: Vec<String>,
/// }
///
/// impl IBuilder for PersonBuilder {}
///
/// impl PersonBuilder {
///     fn with_name(self, name: &str) -> Self {
///         self.with(|builder| &mut builder.name, name.to_string())
///     }
///
///     fn with_tag(self, tag: &str) -> Self {
///         self.with_value(|builder| &mut builder.tags, tag.to_string())
///     }
/// }
///
/// let builder = PersonBuilder::default().with_name("Ada").with_tag("first");
/// assert_eq!(builder.name, "Ada");
/// assert_eq!(builder.tags, ["first"]);
/// ```
///
/// The overloads get names of their own: `with` (`With<TField>(out TField, TField)`),
/// `with_values` and `with_value` for a list, `with_pair`, `with_key_value` and `with_dictionary`
/// for a dictionary. Upstream declares the last one for every type, not only builders; here it
/// is a method of builders as the others are.
///
/// Not ported: the `ArgumentException` of a list or dictionary field that is null - argument
/// validation (the fields cannot be null).
pub trait IBuilderExtensions: IBuilder + Sized {
    /// Sets the field to `value` (`With<TField>(out TField field, TField value)`).
    fn with<F>(mut self, field: impl FnOnce(&mut Self) -> &mut F, value: F) -> Self {
        *field(&mut self) = value;
        self
    }

    /// Adds `values` to the list (`With<TField>(ref List<TField>? field, IEnumerable<TField>
    /// values)`).
    fn with_values<F>(mut self, field: impl FnOnce(&mut Self) -> &mut Vec<F>, values: impl IntoIterator<Item = F>) -> Self {
        field(&mut self).extend(values);
        self
    }

    /// Adds `value` to the list (`With<TField>(ref List<TField>? field, TField value)`).
    fn with_value<F>(mut self, field: impl FnOnce(&mut Self) -> &mut Vec<F>, value: F) -> Self {
        field(&mut self).push(value);
        self
    }

    /// Adds the key and value of `key_value_pair` to the dictionary
    /// (`With<TKey, TField>(ref Dictionary<TKey, TField>, KeyValuePair<TKey, TField>)`).
    ///
    /// # Panics
    /// When the dictionary holds the key, as `Dictionary.Add` throws.
    fn with_pair<K: Eq + Hash, F>(self, dictionary: impl FnOnce(&mut Self) -> &mut HashMap<K, F>, key_value_pair: (K, F)) -> Self {
        let (key, value) = key_value_pair;
        self.with_key_value(dictionary, key, value)
    }

    /// Adds `key` with `value` to the dictionary
    /// (`With<TKey, TField>(ref Dictionary<TKey, TField>, TKey, TField)`).
    ///
    /// # Panics
    /// When the dictionary holds the key, as `Dictionary.Add` throws.
    fn with_key_value<K: Eq + Hash, F>(mut self, dictionary: impl FnOnce(&mut Self) -> &mut HashMap<K, F>, key: K, value: F) -> Self {
        let dictionary = dictionary(&mut self);
        assert!(!dictionary.contains_key(&key), "An item with the same key has already been added.");
        dictionary.insert(key, value);
        self
    }

    /// Replaces the dictionary with a copy of `values`
    /// (`With<TKey, TField>(ref Dictionary<TKey, TField>, IDictionary<TKey, TField>)`).
    fn with_dictionary<K: Eq + Hash + Clone, F: Clone>(mut self, dictionary: impl FnOnce(&mut Self) -> &mut HashMap<K, F>, values: &HashMap<K, F>) -> Self {
        *dictionary(&mut self) = values.clone();
        self
    }
}

impl<B: IBuilder> IBuilderExtensions for B {}
