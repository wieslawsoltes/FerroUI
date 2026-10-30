// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::*;
use ferrodock_model::core::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
#[derive(Default)]
struct Holder {
    options: RefCell<Option<Rc<DockManagerOptions>>>,
    other_options: RefCell<Option<Rc<DockManagerOptions>>>,
    point: Cell<Option<DockPoint>>,
    state: Cell<DockingWindowState>,
    mask: Cell<DockOperationMask>,
    action: Cell<DragAction>,
}
fn serializer() -> Rc<DockSerializer> {
    let mut types = core_model_type_metadata();
    types.push(JsonTypeInfo::object(
        || Rc::new(Holder::default()),
        |v: &Rc<Holder>| Rc::as_ptr(v) as usize,
        vec![
            JsonPropertyInfo::new(
                "Options",
                |v: &Rc<Holder>| v.options.borrow().clone(),
                |v, x| {
                    v.options.replace(x);
                },
            ),
            JsonPropertyInfo::new(
                "OtherOptions",
                |v: &Rc<Holder>| v.other_options.borrow().clone(),
                |v, x| {
                    v.other_options.replace(x);
                },
            ),
            JsonPropertyInfo::new(
                "Point",
                |v: &Rc<Holder>| v.point.get(),
                |v, x| v.point.set(x),
            ),
            JsonPropertyInfo::new(
                "State",
                |v: &Rc<Holder>| v.state.get(),
                |v, x| v.state.set(x),
            ),
            JsonPropertyInfo::new("Mask", |v: &Rc<Holder>| v.mask.get(), |v, x| v.mask.set(x)),
            JsonPropertyInfo::new(
                "Action",
                |v: &Rc<Holder>| v.action.get(),
                |v, x| v.action.set(x),
            ),
        ],
    ));
    DockSerializer::with_resolver(Some(GeneratedJsonTypeInfoResolver::new(types))).unwrap()
}
fn check(name: &str, value: bool) {
    assert!(value, "{name}");
    assert!(
        include_str!("../../../tests/Dock/data/serializer-json-audit.values.tsv")
            .lines()
            .any(|row| row == format!("{name}\t1")),
        "missing source case {name}"
    );
}
#[test]
fn original_readonly_construction_flags_and_shared_options_wire_are_preserved() {
    let serializer = serializer();
    let point = DockPoint::new(-0.0, f64::INFINITY);
    let text = serializer.serialize(point).unwrap();
    assert_eq!(
        text,
        include_str!("../../../tests/Dock/data/serializer-json-audit.values.point.json")
    );
    let point = serializer
        .deserialize::<DockPoint>(Some(&text))
        .unwrap()
        .unwrap();
    check(
        "point-readonly-source-default-construction",
        point == DockPoint::default(),
    );
    check(
        "point-write-getters-no-reference-id",
        !text.contains("$id") && text.contains("Infinity"),
    );
    check(
        "point-readonly-input-values-skipped",
        serializer
            .deserialize::<DockPoint>(Some("{\"X\":null,\"Y\":{\"not\":\"a number\"}}"))
            .unwrap()
            .unwrap()
            == DockPoint::default(),
    );
    check(
        "manager-read-constructor-retains-enabled-default",
        serializer
            .deserialize::<Rc<DockManagerOptions>>(Some("{}"))
            .unwrap()
            .unwrap()
            .is_docking_enabled(),
    );
    let options = DockManagerOptions::new();
    options.set_is_docking_enabled(false);
    assert_eq!(
        serializer.serialize(options.clone()).unwrap(),
        include_str!("../../../tests/Dock/data/serializer-json-audit.values.manager.json")
    );
    let holder = Rc::new(Holder::default());
    holder.options.replace(Some(options.clone()));
    holder.other_options.replace(Some(options));
    holder.point.set(Some(DockPoint::new(-0.0, f64::INFINITY)));
    holder
        .state
        .set(DockingWindowState::from_bits_retain(i32::MIN | 4));
    holder.mask.set(DockOperationMask::from_bits_retain(-1));
    holder.action.set(DragAction::from_bits_retain(i32::MAX));
    let wire = include_str!("../../../tests/Dock/data/serializer-json-audit.values.holder.json");
    assert_eq!(serializer.serialize(holder).unwrap(), wire);
    let read = serializer
        .deserialize::<Rc<Holder>>(Some(wire))
        .unwrap()
        .unwrap();
    // Get-only point properties intentionally keep the source default after reading.
    assert_eq!(
        serializer.serialize(read.clone()).unwrap(),
        include_str!("../../../tests/Dock/data/serializer-json-audit.values.holder.roundtrip.json")
    );
    let options = read.options.borrow().clone().unwrap();
    let other = read.other_options.borrow().clone().unwrap();
    check(
        "manager-shared-original-instance",
        Rc::ptr_eq(&options, &other),
    );
    options.set_is_docking_enabled(true);
    check(
        "manager-shared-original-mutation",
        other.is_docking_enabled(),
    );
    check(
        "flags-undefined-signed-bits-retained",
        read.state.get().bits() == i32::MIN | 4
            && read.mask.get().bits() == -1
            && read.action.get().bits() == i32::MAX,
    );
    check(
        "flags-default-format-is-numeric",
        serializer
            .serialize(DockOperationMask::FILL | DockOperationMask::LEFT)
            .unwrap()
            == "3",
    );
    check(
        "nullable-point-source-null",
        serializer
            .deserialize::<Option<DockPoint>>(Some("null"))
            .unwrap()
            .unwrap()
            .is_none(),
    );
}
#[test]
fn original_value_and_flags_read_corpus_matches_unknown_bits_range_and_token_rules() {
    let serializer = serializer();
    let rows: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/Dock/data/serializer-json-audit.values.reads.json"
    ))
    .unwrap();
    let rows = rows.as_array().unwrap();
    assert_eq!(rows.len(), 490);
    for row in rows {
        let kind = row[0].as_str().unwrap();
        let input = row[1].as_str().unwrap();
        macro_rules! replay {
            ($t:ty) => {{
                let result = serializer.deserialize::<$t>(Some(input));
                if row[2] == "ERR" {
                    assert!(result.is_err(), "{kind} {input}");
                } else {
                    let actual = match result.unwrap_or_else(|e| panic!("{kind} {input}: {e}")) {
                        Some(value) => serializer.serialize(value).unwrap(),
                        None => "null".into(),
                    };
                    assert_eq!(actual, row[3].as_str().unwrap(), "{kind} {input}");
                }
            }};
        }
        match kind {
            "POINT" => replay!(DockPoint),
            "MANAGER" => replay!(Rc<DockManagerOptions>),
            "STATE" => replay!(DockingWindowState),
            "MASK" => replay!(DockOperationMask),
            "ACTION" => replay!(DragAction),
            "OPTION_POINT" => replay!(Option<DockPoint>),
            "OPTION_STATE" => replay!(Option<DockingWindowState>),
            "OPTION_MASK" => replay!(Option<DockOperationMask>),
            "OPTION_ACTION" => replay!(Option<DragAction>),
            _ => panic!("unexpected source case {kind}"),
        }
    }
}
#[test]
fn numeric_enum_converter_is_independent_of_a_custom_underlying_integer_codec() {
    let mut types = vec![JsonTypeInfo::converter(
        |_: &i32, _| Ok(JsonValue::String("custom-int32".into())),
        |_, _| Ok(Some(123)),
    )];
    types.extend(core_model_type_metadata());
    let serializer =
        DockSerializer::with_resolver(Some(GeneratedJsonTypeInfoResolver::new(types))).unwrap();
    check(
        "flags-independent-of-int32-codec",
        serializer.serialize(42i32).unwrap() == "\"custom-int32\""
            && serializer.deserialize::<i32>(Some("42")).unwrap() == Some(123)
            && serializer
                .serialize(DockOperationMask::from_bits_retain(-1))
                .unwrap()
                == "-1"
            && serializer
                .deserialize::<DockOperationMask>(Some("-1"))
                .unwrap()
                .unwrap()
                .bits()
                == -1,
    );
}
