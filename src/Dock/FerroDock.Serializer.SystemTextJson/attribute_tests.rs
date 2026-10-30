// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::{DockJsonSerializableAttribute, DockJsonSourceGenerationAttribute};
use std::{any::TypeId, rc::Rc};
crate::dock_json_source_generation!();
crate::dock_json_serializable!(String);
crate::dock_json_serializable!(String);
crate::dock_json_serializable!(Vec<Option<i32>>);
crate::dock_json_serializable!(std::convert::Infallible);
fn source_check(name: &str, value: bool) {
    assert!(value, "{name}");
    let source = include_str!("../../../tests/Dock/data/serializer-json-audit.attributes.tsv");
    assert!(
        source.lines().any(|line| line == format!("{name}\t1")),
        "missing source case {name}"
    );
}
#[test]
fn activation_attributes_are_original_distinct_reference_instances() {
    let first = DockJsonSourceGenerationAttribute::new();
    let alias = first.clone();
    source_check(
        "attribute-activation-shared-alias",
        Rc::ptr_eq(&first, &alias),
    );
    source_check(
        "attribute-activation-distinct-instances",
        !Rc::ptr_eq(&first, &DockJsonSourceGenerationAttribute::new()),
    );
    let marker = _DOCK_JSON_SOURCE_GENERATION_ATTRIBUTE();
    assert!(!Rc::ptr_eq(&first, &marker));
}
#[test]
fn serializable_attribute_retains_exact_type_or_null_without_instantiation() {
    let type_ = TypeId::of::<Vec<Option<i32>>>();
    let attribute = DockJsonSerializableAttribute::new(Some(type_));
    source_check(
        "attribute-registration-exact-closed-type",
        attribute.type_() == Some(type_) && attribute.type_() != Some(TypeId::of::<Vec<i32>>()),
    );
    source_check(
        "attribute-registration-shared-alias",
        Rc::ptr_eq(&attribute, &attribute.clone()),
    );
    source_check(
        "attribute-registration-distinct-instances",
        !Rc::ptr_eq(&attribute, &DockJsonSerializableAttribute::new(Some(type_))),
    );
    source_check(
        "attribute-registration-null-retained",
        DockJsonSerializableAttribute::new(None).type_().is_none(),
    );
    let type_ = TypeId::of::<std::convert::Infallible>();
    source_check(
        "attribute-registration-no-type-construction",
        DockJsonSerializableAttribute::new(Some(type_)).type_() == Some(type_),
    );
}
