// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::*;
use ferrodock_model::core::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
type Policy = Rc<dyn DockCapabilityPolicyImpl>;
type Contract = Rc<dyn IDockCapabilityPolicy>;
#[derive(Default)]
struct Holder {
    overrides: RefCell<Option<Rc<DockCapabilityOverrides>>>,
    policy: RefCell<Option<Policy>>,
    contract: RefCell<Option<Contract>>,
    bounds: Cell<Option<DockRect>>,
    other_bounds: Cell<Option<DockRect>>,
}
#[derive(Default)]
struct DollarProperty {
    value: Cell<i32>,
}
fn serializer(base_first: bool) -> Rc<DockSerializer> {
    let mut properties = vec![
        JsonPropertyInfo::new(
            "Overrides",
            |v: &Rc<Holder>| v.overrides.borrow().clone(),
            |v, x| {
                v.overrides.replace(x);
            },
        ),
        JsonPropertyInfo::new(
            "Policy",
            |v: &Rc<Holder>| v.policy.borrow().clone(),
            |v, x| {
                v.policy.replace(x);
            },
        ),
    ];
    if base_first {
        properties.swap(0, 1);
    } else {
        properties.extend([
            JsonPropertyInfo::new(
                "Contract",
                |v: &Rc<Holder>| v.contract.borrow().clone(),
                |v, x| {
                    v.contract.replace(x);
                },
            ),
            JsonPropertyInfo::new(
                "Bounds",
                |v: &Rc<Holder>| v.bounds.get(),
                |v, x| v.bounds.set(x),
            ),
            JsonPropertyInfo::new(
                "OtherBounds",
                |v: &Rc<Holder>| v.other_bounds.get(),
                |v, x| v.other_bounds.set(x),
            ),
        ]);
    }
    let mut types = core_model_type_metadata();
    types.push(JsonTypeInfo::object(
        || Rc::new(Holder::default()),
        |v: &Rc<Holder>| Rc::as_ptr(v) as usize,
        properties,
    ));
    types.push(JsonTypeInfo::object(
        || Rc::new(DollarProperty::default()),
        |v: &Rc<DollarProperty>| Rc::as_ptr(v) as usize,
        vec![JsonPropertyInfo::new(
            "$other",
            |v: &Rc<DollarProperty>| v.value.get(),
            |v, x| v.value.set(x),
        )],
    ));
    DockSerializer::with_resolver(Some(GeneratedJsonTypeInfoResolver::new(types))).unwrap()
}
fn check(name: &str, value: bool) {
    assert!(value, "{name}");
    assert!(
        include_str!("../../../tests/Dock/data/serializer-json-audit.core.tsv")
            .lines()
            .any(|row| row == format!("{name}\t1")),
        "missing original case {name}"
    );
}
#[test]
fn actual_core_value_and_policy_wire_and_source_constructors_match() {
    let serializer = serializer(false);
    let rect = DockRect::new(-0.0, 0.125, f64::INFINITY, f64::NAN);
    let text = serializer.serialize(rect).unwrap();
    assert_eq!(
        text,
        include_str!("../../../tests/Dock/data/serializer-json-audit.core.rect.json")
    );
    check("rect-value-no-reference-id", !text.contains("$id"));
    let read = serializer
        .deserialize::<DockRect>(Some(&text))
        .unwrap()
        .unwrap();
    check(
        "rect-components-and-signed-zero",
        read.x.to_bits() == (-0.0f64).to_bits()
            && read.y == 0.125
            && read.width == f64::INFINITY
            && read.height.is_nan(),
    );
    let policy = DockCapabilityPolicy::new();
    policy.set_can_close(Some(false));
    policy.set_can_pin(Some(true));
    let text = serializer.serialize(policy.clone()).unwrap();
    assert_eq!(
        text,
        include_str!("../../../tests/Dock/data/serializer-json-audit.core.policy.json")
    );
    check(
        "policy-null-values-omitted",
        !text.contains("CanDrag") && text.contains("false"),
    );
    assert_eq!(
        serializer.serialize(policy as Contract).unwrap(),
        include_str!("../../../tests/Dock/data/serializer-json-audit.core.contract.json")
    );
    let overrides = DockCapabilityOverrides::new();
    overrides.set_can_float(Some(false));
    overrides.set_can_drop(Some(true));
    let text = serializer.serialize(overrides).unwrap();
    assert_eq!(
        text,
        include_str!("../../../tests/Dock/data/serializer-json-audit.core.overrides.json")
    );
    check(
        "override-derived-readonly-property-ignored",
        !text.contains("HasAnyOverride"),
    );
    check(
        "policy-default-serialization",
        serializer.serialize(DockCapabilityPolicy::new()).unwrap() == "{\n  \"$id\": \"1\"\n}",
    );
    check(
        "policy-interface-read-has-no-fabricated-constructor",
        serializer.deserialize::<Contract>(Some("{}")).is_err(),
    );
}
#[test]
fn actual_override_upcasts_share_state_but_rectangles_are_copied_values() {
    let serializer = serializer(false);
    let wire = include_str!("../../../tests/Dock/data/serializer-json-audit.core.holder.json");
    let holder = serializer
        .deserialize::<Rc<Holder>>(Some(wire))
        .unwrap()
        .unwrap();
    assert_eq!(serializer.serialize(holder.clone()).unwrap(), wire);
    let overrides = holder.overrides.borrow().clone().unwrap();
    let policy = holder.policy.borrow().clone().unwrap();
    let contract = holder.contract.borrow().clone().unwrap();
    check(
        "override-upcast-shared-reference",
        Rc::ptr_eq(&(overrides.clone() as Policy), &policy)
            && Rc::ptr_eq(&(policy.clone() as Contract), &contract),
    );
    policy.set_can_close(Some(true));
    check(
        "same-original-override-state",
        overrides.can_close() == Some(true) && overrides.has_any_override(),
    );
    let bounds = holder.bounds.get().unwrap();
    check(
        "nullable-bounds-are-value-copies",
        holder.other_bounds.get() == Some(bounds) && bounds.x.to_bits() == (-0.0f64).to_bits(),
    );
    holder.bounds.set(Some(DockRect::default()));
    assert_eq!(holder.other_bounds.get(), Some(bounds));
    let base_first = self::serializer(true);
    check(
        "base-first-cannot-recover-derived-class",
        base_first
            .deserialize::<Rc<Holder>>(Some(include_str!(
                "../../../tests/Dock/data/serializer-json-audit.core.base-first.json"
            )))
            .is_err(),
    );
}
#[test]
fn source_core_read_corpus_preserves_duplicate_null_case_and_reserved_metadata_rules() {
    let serializer = serializer(false);
    let rows: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/Dock/data/serializer-json-audit.core.reads.json"
    ))
    .unwrap();
    let rows = rows.as_array().unwrap();
    assert_eq!(rows.len(), 49);
    for row in rows {
        let kind = row[0].as_str().unwrap();
        let input = row[1].as_str().unwrap();
        macro_rules! replay {
            ($t:ty) => {{
                let result = serializer.deserialize::<$t>(Some(input));
                if row[2] == "ERR" {
                    assert!(result.is_err(), "{kind} {input}");
                } else {
                    let result = result.unwrap_or_else(|e| panic!("{kind} {input}: {e}"));
                    let actual = match result {
                        Some(value) => serializer.serialize(value).unwrap(),
                        None => "null".into(),
                    };
                    assert_eq!(actual, row[3].as_str().unwrap(), "{kind} {input}");
                }
            }};
        }
        match kind {
            "RECT" => replay!(DockRect),
            "POLICY" => replay!(Rc<DockCapabilityPolicy>),
            "OVERRIDE" => replay!(Rc<DockCapabilityOverrides>),
            "DOLLAR" => replay!(Rc<DollarProperty>),
            _ => panic!("unexpected source case {kind}"),
        }
    }
}
#[test]
fn supplied_context_does_not_implicitly_gain_a_core_model_catalog() {
    let serializer =
        DockSerializer::with_resolver(Some(GeneratedJsonTypeInfoResolver::new(Vec::new())))
            .unwrap();
    assert!(serializer.serialize(DockRect::default()).is_err());
    assert!(serializer.serialize(DockPoint::default()).is_err());
    assert!(serializer.serialize(DockingWindowState::DOCKED).is_err());
    assert!(serializer.serialize(DockCapabilityPolicy::new()).is_err());
    assert_eq!(
        self::serializer(false).serialize(None::<DockRect>).unwrap(),
        "null"
    );
}
