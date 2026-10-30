// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Real SDK callbacks, original model instances and original source scalar wire values.
use ferrodock_model::core::*;
use ferrodock_model_reactiveui::controls::RootDock;
use ferrodock_serializer_system_text_json::*;
use std::rc::Rc;
include!(concat!(env!("OUT_DIR"), "/native_json_properties.rs"));

fn serializer() -> Rc<DockSerializer> {
    let mut metadata = core_model_type_metadata();
    metadata.push(JsonTypeInfo::object(
        RootDock::new,
        |model: &Rc<RootDock>| Rc::as_ptr(model) as usize,
        properties(),
    ));
    DockSerializer::with_resolver(Some(GeneratedJsonTypeInfoResolver::new(metadata))).unwrap()
}

#[test]
fn actual_native_sdk_inherited_properties_match_source_scalar_wire_on_original_models() {
    assert_eq!(
        HIERARCHY,
        [
            "RootDock",
            "DockBase",
            "DockableBase",
            "ReactiveBase",
            "ReactiveObject"
        ]
    );
    let source = JsonValue::parse(include_str!(
        "../data/serializer-json-audit.model.root.json"
    ))
    .unwrap();
    let JsonValue::Object(fields) = source else {
        panic!("original source object")
    };
    let names = [
        "$id",
        "PinnedDockDisplayMode",
        "FloatingWindowHostMode",
        "Id",
        "Title",
        "Proportion",
        "Dock",
        "DockingState",
        "CanClose",
        "CanPin",
        "PinnedDockDisplayModeOverride",
        "CanFloat",
        "CanDrag",
        "CanDrop",
        "CanDockAsDocument",
    ];
    let fields: Vec<_> = fields
        .into_iter()
        .filter(|(name, _)| names.contains(&name.as_str()))
        .collect();
    // The original serializer omits the null nullable override.
    assert_eq!(fields.len(), names.len() - 1);
    assert!(!fields
        .iter()
        .any(|(name, _)| name == "PinnedDockDisplayModeOverride"));
    let expected = JsonValue::Object(fields);
    let text = expected.to_json().unwrap();
    let serializer = serializer();
    let restored = serializer
        .deserialize::<Rc<RootDock>>(Some(&text))
        .unwrap()
        .unwrap();
    assert_eq!(restored.id(), "root");
    assert_eq!(restored.title(), "Root");
    assert_eq!(restored.pinned_dock_display_mode().to_raw(), 123);
    assert_eq!(restored.floating_window_host_mode().to_raw(), -17);
    assert_eq!(
        JsonValue::parse(&serializer.serialize(restored.clone()).unwrap()).unwrap(),
        expected
    );
    let second = RootDock::new();
    second.set_id("second".into());
    second.set_title("Second".into());
    restored.set_title("Live update".into());
    let live = JsonValue::parse(&serializer.serialize(restored.clone()).unwrap()).unwrap();
    assert_eq!(
        live.property("Title"),
        Some(&JsonValue::String("Live update".into()))
    );
    let distinct = JsonValue::parse(&serializer.serialize(second.clone()).unwrap()).unwrap();
    assert_eq!(
        distinct.property("Id"),
        Some(&JsonValue::String("second".into()))
    );
    assert_eq!(second.title(), "Second");
    assert!(!Rc::ptr_eq(&restored, &second));
}

#[test]
fn inherited_native_setters_mutate_the_original_instance_through_each_operation() {
    let original = RootDock::new();
    let configured = original.clone();
    let mut metadata = core_model_type_metadata();
    metadata.push(JsonTypeInfo::object(
        move || configured.clone(),
        |model: &Rc<RootDock>| Rc::as_ptr(model) as usize,
        properties(),
    ));
    let serializer =
        DockSerializer::with_resolver(Some(GeneratedJsonTypeInfoResolver::new(metadata))).unwrap();
    for (title, proportion) in [("first", 0.25), ("second", 0.75)] {
        let input = format!("{{\"Title\":\"{title}\",\"Proportion\":{proportion},\"CanClose\":false,\"PinnedDockDisplayMode\":-71,\"PinnedDockDisplayModeOverride\":123}}");
        let restored = serializer
            .deserialize::<Rc<RootDock>>(Some(&input))
            .unwrap()
            .unwrap();
        assert!(Rc::ptr_eq(&original, &restored));
        assert_eq!(original.title(), title);
        assert_eq!(original.proportion(), proportion);
        assert!(!original.can_close());
        assert_eq!(original.pinned_dock_display_mode().to_raw(), -71);
        assert_eq!(
            original
                .pinned_dock_display_mode_override()
                .unwrap()
                .to_raw(),
            123
        );
    }
}
