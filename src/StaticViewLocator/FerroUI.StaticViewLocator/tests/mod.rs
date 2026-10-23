//! The tests of the tables and of the run-time type of a view model. The generated locators are
//! tested by `tests/FerroUI.StaticViewLocator.Tests`.

use crate::{MissingViewTable, NameTable, RuntimeType, TypeEntry, TypeSet};
use ferroui_base::BoxedValue;
use std::any::TypeId;
use std::rc::Rc;

#[derive(PartialEq)]
struct First;

#[derive(PartialEq)]
struct Second;

#[derive(PartialEq)]
struct Generic<T>(T);

static MISSING: MissingViewTable = MissingViewTable::new(&[
    TypeEntry::new(TypeId::of::<First>, "first"),
    TypeEntry::new(TypeId::of::<Second>, "second"),
    TypeEntry::new(TypeId::of::<First>, "ignored"),
]);

static EMPTY: TypeSet = TypeSet::new(&[]);

#[test]
fn a_table_finds_the_first_value_of_a_type() {
    assert_eq!(MISSING.get(TypeId::of::<First>()), Some("first"));
    assert_eq!(MISSING.get(TypeId::of::<Second>()), Some("second"));
    assert_eq!(MISSING.get(TypeId::of::<String>()), None);
    assert!(MISSING.contains(TypeId::of::<Second>()));
    assert_eq!(MISSING.len(), 2);
    assert_eq!(MISSING.entries().len(), 3);
    assert_eq!(MISSING.entries()[1].type_id(), TypeId::of::<Second>());
}

#[test]
fn an_empty_table_has_no_type() {
    assert!(EMPTY.is_empty());
    assert!(!EMPTY.contains(TypeId::of::<First>()));
}

#[test]
fn a_name_table_finds_a_generic_definition() {
    let table: NameTable<u8> = NameTable::new(&[("a::Generic", 1), ("a::Other", 2)]);
    assert_eq!(table.get("a::Generic"), Some(1));
    assert_eq!(table.get("a::Missing"), None);
    assert_eq!(table.len(), 2);
}

#[test]
fn the_runtime_type_of_a_value_is_the_type_of_the_view_model() {
    let value: BoxedValue = Rc::new(First);
    let type_ = RuntimeType::of_value(&value);
    assert_eq!(type_.id(), TypeId::of::<First>());
    assert!(type_.name().ends_with("First"));
    assert_eq!(type_.generic_definition(), None);
}

#[test]
fn the_generic_definition_is_the_path_without_the_arguments() {
    let value: BoxedValue = Rc::new(Generic(1_i32));
    let type_ = RuntimeType::of_value(&value);
    assert_eq!(type_.id(), TypeId::of::<Generic<i32>>());
    assert_eq!(type_.generic_definition(), Some(std::any::type_name::<Generic<i32>>().split('<').next().unwrap()));
    assert_eq!(RuntimeType::of::<Generic<u8>>().generic_definition(), type_.generic_definition());
}
