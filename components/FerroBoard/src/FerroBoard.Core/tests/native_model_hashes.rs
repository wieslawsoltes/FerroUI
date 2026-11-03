//! Native hashes are opaque values used by Rust collections, never CLR integers.
use ferroboard_core::layouts::{
    ComponentName, CurrencySet, PopupMapping, Subtype, SubtypeLayoutMap, SubtypeLayoutMapValues,
};
use std::{
    collections::{HashMap, hash_map::DefaultHasher},
    hash::{Hash, Hasher},
    rc::Rc,
    sync::Arc,
};
fn hash(value: &impl Hash) -> u64 {
    let mut state = DefaultHasher::new();
    value.hash(&mut state);
    state.finish()
}

#[test]
fn layout_value_keys_work_in_native_hash_maps() {
    let values = SubtypeLayoutMapValues::default();
    let map = HashMap::from([(values.clone(), "qwerty")]);
    assert_eq!(map.get(&values), Some(&"qwerty"));
    let mut other = values.clone();
    other.characters = ComponentName::new("custom", "different");
    assert!(!map.contains_key(&other));
    let layout = SubtypeLayoutMap::from_values(values.clone());
    assert_eq!(hash(&layout), hash(&values));
    assert_eq!(hash(&layout), hash(&layout.clone()));
}

#[test]
fn subtype_hashes_follow_equal_native_values_and_nested_layouts() {
    let source = Subtype::default();
    let copy = source.clone();
    assert_eq!(source, copy);
    assert_eq!(hash(&source), hash(&copy));
    assert_eq!(hash(&source), hash(&source.values()));
    copy.set_layouts(Some(Arc::new(SubtypeLayoutMap::from_values(
        source.layouts().unwrap().values(),
    ))));
    assert_eq!(source, copy);
    assert_eq!(hash(&source), hash(&copy));
    copy.set_language_tag(Some(ferroboard_core::text::Utf16Text::from_units(vec![
        0xd800, 65,
    ])));
    let another = Subtype::from_values(copy.values());
    assert_eq!(hash(&copy), hash(&another));
}

#[test]
fn resource_references_keep_alias_identity_across_field_mutation() {
    let popup = Rc::new(PopupMapping::default());
    let alias = popup.clone();
    popup.set_id(Some("changed".into()));
    assert!(Rc::ptr_eq(&popup, &alias));
    assert_eq!(alias.id(), Some("changed".into()));
    assert!(popup == alias);
    assert!(popup != Rc::new(PopupMapping::default()));
    let currency = Rc::new(CurrencySet::default());
    let alias = currency.clone();
    currency.set_label(None);
    assert!(Rc::ptr_eq(&currency, &alias));
    assert!(alias.label().is_none());
    assert!(currency == alias);
    assert!(currency != Rc::new(CurrencySet::default()));
}
