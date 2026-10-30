// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Native SDK class construction and inherited callbacks over the original engine graph.
use ferrodock_model::{controls::*, core::*};
use ferrodock_model_reactiveui::{
    controls::{Document, DocumentDock, RootDock},
    Factory,
};
use ferrodock_serializer_system_text_json::JsonListType;
use std::rc::Rc;
include!(concat!(env!("OUT_DIR"), "/native_json_models.rs"));
include!(concat!(env!("OUT_DIR"), "/native_json_model_bindings.rs"));
#[test]
fn emitted_metadata_roundtrips_original_model_projection_and_initializes_the_same_engine() {
    let serializer = DockSystemTextJsonGenerated::create_serializer().unwrap();
    let wire = include_str!("../data/serializer-json-audit.model.root.json");
    let root = serializer
        .deserialize::<Rc<RootDock>>(Some(wire))
        .unwrap()
        .unwrap();
    assert_eq!(serializer.serialize(root.clone()).unwrap(), wire);
    let floating = root.windows().unwrap().get(0).unwrap();
    assert_eq!(floating.window_state().to_raw(), 123);
    assert_eq!(floating.owner_mode().to_raw(), -19);
    assert!(floating.topmost() && floating.is_modal());
    assert_eq!(floating.show_in_taskbar(), Some(false));
    assert!(Rc::ptr_eq(
        &floating.parent_window().unwrap(),
        &root.window().unwrap()
    ));
    let docs = root
        .visible_dockables()
        .unwrap()
        .get(0)
        .unwrap()
        .clone()
        .boxed_reference()
        .downcast_ref::<Rc<DocumentDock>>()
        .unwrap()
        .clone();
    let document = docs
        .visible_dockables()
        .unwrap()
        .get(0)
        .unwrap()
        .clone()
        .boxed_reference()
        .downcast_ref::<Rc<Document>>()
        .unwrap()
        .clone();
    assert_eq!(root.dock().to_raw(), -19);
    assert_eq!(root.pinned_dock_display_mode().to_raw(), 123);
    assert_eq!(root.floating_window_host_mode().to_raw(), -17);
    assert_eq!(docs.dock().to_raw(), i32::MIN);
    assert_eq!(document.dock().to_raw(), 123);
    assert!(root.pinned_dock_display_mode_override().is_none());
    assert_eq!(
        docs.pinned_dock_display_mode_override(),
        Some(PinnedDockDisplayMode::Inline)
    );
    assert_eq!(
        document
            .pinned_dock_display_mode_override()
            .map(|v| v.to_raw()),
        Some(i32::MIN)
    );
    assert!(Rc::ptr_eq(
        &document.owner().unwrap(),
        &(docs.clone() as Rc<dyn IDockable>)
    ));
    assert_eq!(document.proportion(), 0.125);
    assert_eq!(document.max_width(), f64::INFINITY);
    assert_eq!(document.docking_state(), DockingWindowState::DOCUMENT);
    assert_eq!(document.allowed_drop_operations().bits(), i32::MIN | 1);
    assert!(document
        .allowed_dock_operations()
        .allows(DockOperation::OuterLeft));
    assert!(!document
        .allowed_dock_operations()
        .allows(DockOperation::Right));
    assert!(document
        .allowed_drop_operations()
        .allows(DockOperation::Fill));
    assert!(!document
        .allowed_drop_operations()
        .allows(DockOperation::Window));
    let pinned = document.pinned_bounds().unwrap();
    assert_eq!(pinned.x.to_bits(), (-0.0f64).to_bits());
    assert_eq!(
        (pinned.y, pinned.width, pinned.height),
        (0.125, 512.5, 256.25)
    );
    assert!(Rc::ptr_eq(
        &root.root_dock_capability_policy().unwrap(),
        &docs.dock_capability_policy().unwrap()
    ));
    assert_eq!(
        document.dock_capability_overrides().unwrap().can_close(),
        Some(true)
    );
    let factory = Factory::new();
    factory.init_layout(root.clone());
    assert_eq!(document.dock().to_raw(), 123);
    assert_eq!(
        document
            .pinned_dock_display_mode_override()
            .map(|v| v.to_raw()),
        Some(i32::MIN)
    );
    assert_eq!(root.floating_window_host_mode().to_raw(), -17);
    assert!(Rc::ptr_eq(
        &document.factory().unwrap(),
        &(factory as Rc<dyn IFactory>)
    ));
    let capability = ferrodock_model::DockCapabilityResolver::evaluate(
        Some(document.clone()),
        DockCapability::Close,
    );
    assert!(capability.effective_value());
    assert_eq!(
        capability.effective_source(),
        DockCapabilityValueSource::DockableOverride
    );
    root.root_dock_capability_policy()
        .unwrap()
        .set_can_pin(Some(false));
    assert_eq!(
        docs.dock_capability_policy().unwrap().can_pin(),
        Some(false)
    );
    assert!(!ferrodock_model::DockCapabilityResolver::is_enabled(
        Some(document),
        DockCapability::Pin,
        Some(docs)
    ));
    root.root_dock_capability_policy()
        .unwrap()
        .set_can_pin(Some(true));
    assert_eq!(serializer.serialize(root.clone()).unwrap(), wire);
    assert!(Rc::ptr_eq(
        &root.window().unwrap().layout().unwrap(),
        &(root.clone() as Rc<dyn IRootDock>)
    ));
}
#[test]
fn emitted_factory_list_overload_uses_the_configured_original_list_creator() {
    let serializer =
        DockSystemTextJsonGenerated::create_serializer_with_list_type(Some(JsonListType::list()))
            .unwrap();
    let wire = include_str!("../data/serializer-json-audit.model.root.json");
    let root = serializer
        .deserialize::<Rc<RootDock>>(Some(wire))
        .unwrap()
        .unwrap();
    assert!(root
        .visible_dockables()
        .unwrap()
        .as_observable_collection()
        .is_none());
    assert!(root.windows().unwrap().as_observable_collection().is_none());
    assert_eq!(serializer.serialize(root).unwrap(), wire);
    assert!(DockSystemTextJsonGenerated::create_serializer_with_list_type(None).is_err());
}

#[test]
fn sdk_metadata_constructors_return_distinct_actual_engine_classes_and_live_properties() {
    assert_eq!(NATIVE_MODEL_BINDINGS.len(), 4);
    assert_eq!(NATIVE_MODEL_SCHEMAS.len(), 7);
    assert_eq!(
        NATIVE_MODEL_SCHEMAS
            .iter()
            .map(|(_, count)| count)
            .sum::<usize>(),
        101
    );
    for (name, count) in NATIVE_MODEL_BINDINGS {
        assert!(*count > 0, "native bindings for {name}");
    }
    let serializer = DockSystemTextJsonGenerated::create_serializer().unwrap();
    macro_rules! check {
        ($model:ty, $view:ty) => {{
            let first = serializer
                .deserialize::<Rc<$model>>(Some("{}"))
                .unwrap()
                .unwrap();
            let second = serializer
                .deserialize::<Rc<$model>>(Some("{}"))
                .unwrap()
                .unwrap();
            assert!(!Rc::ptr_eq(&first, &second));
            let view = <$view>::new(first.clone());
            let original = view.model().boxed_reference();
            assert!(Rc::ptr_eq(
                &first,
                original.downcast_ref::<Rc<$model>>().unwrap()
            ));
            first.set_title("native constructor".into());
            assert!(serializer
                .serialize(first.clone())
                .unwrap()
                .contains("native constructor"));
            second.set_title("separate original".into());
            assert_eq!(first.title(), "native constructor");
        }};
    }
    use ferrodock_model_reactiveui_ferro::reactive_metadata::*;
    check!(RootDock, RootDockReference);
    check!(DocumentDock, DocumentDockReference);
    check!(Document, DocumentReference);
    check!(
        ferrodock_model_reactiveui::core::DockWindow,
        DockWindowReference
    );
}
