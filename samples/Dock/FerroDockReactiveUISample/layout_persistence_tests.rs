// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Original application models, contexts and commands after ordinary JSON restore.
use crate::{
    infrastructure::LayoutJsonSerialization,
    models::DemoData,
    services::{ILayoutInitialization, ILayoutSerialization},
    view_models::{
        docks::CustomDocumentDock,
        documents::DocumentViewModel,
        tools::*,
        views::{DashboardViewModel, HomeViewModel},
        DockFactory, MainWindowViewModel,
    },
};
use ferrodock_model::{controls::*, core::*};
use std::rc::Rc;

fn serializer() -> Rc<ferrodock_serializer_system_text_json::DockSerializer> {
    // Exercise the same explicit application catalog used by the injected service.
    super::infrastructure::create_layout_serializer().unwrap()
}
fn concrete<T: 'static>(value: Rc<dyn IDockable>) -> Rc<T> {
    value
        .boxed_reference()
        .downcast_ref::<Rc<T>>()
        .unwrap()
        .clone()
}
fn find(
    factory: &Rc<DockFactory>,
    root: &Rc<dyn IRootDock>,
    id: &'static str,
) -> Rc<dyn IDockable> {
    factory
        .find_dockable(
            Some(root.clone()),
            Some(Rc::new(move |v| v.is_some_and(|v| v.id() == id))),
        )
        .unwrap_or_else(|| panic!("missing sample dockable {id}"))
}

#[test]
fn all_twelve_application_class_defaults_match_unchanged_source_json() {
    let serializer = serializer();
    macro_rules! check {
        ($model:ty, $wire:literal) => {{
            let expected = include_str!($wire);
            assert_eq!(
                serializer.serialize(<$model>::new()).unwrap(),
                expected,
                stringify!($model)
            );
            let restored = serializer
                .deserialize::<Rc<$model>>(Some(expected))
                .unwrap()
                .unwrap();
            assert_eq!(
                serializer.serialize(restored).unwrap(),
                expected,
                stringify!($model)
            );
        }};
    }
    check!(
        DocumentViewModel,
        "../../../tests/Dock/data/serializer-json-sample.document.json"
    );
    check!(
        CustomDocumentDock,
        "../../../tests/Dock/data/serializer-json-sample.documents.json"
    );
    check!(
        HomeViewModel,
        "../../../tests/Dock/data/serializer-json-sample.home.json"
    );
    check!(
        DashboardViewModel,
        "../../../tests/Dock/data/serializer-json-sample.dashboard.json"
    );
    check!(
        Tool1ViewModel,
        "../../../tests/Dock/data/serializer-json-sample.tool1.json"
    );
    check!(
        Tool2ViewModel,
        "../../../tests/Dock/data/serializer-json-sample.tool2.json"
    );
    check!(
        Tool3ViewModel,
        "../../../tests/Dock/data/serializer-json-sample.tool3.json"
    );
    check!(
        Tool4ViewModel,
        "../../../tests/Dock/data/serializer-json-sample.tool4.json"
    );
    check!(
        Tool5ViewModel,
        "../../../tests/Dock/data/serializer-json-sample.tool5.json"
    );
    check!(
        Tool6ViewModel,
        "../../../tests/Dock/data/serializer-json-sample.tool6.json"
    );
    check!(
        Tool7ViewModel,
        "../../../tests/Dock/data/serializer-json-sample.tool7.json"
    );
    check!(
        Tool8ViewModel,
        "../../../tests/Dock/data/serializer-json-sample.tool8.json"
    );
}

#[test]
fn original_sample_class_graph_preserves_type_tags_aliases_and_constructor_command() {
    let serializer = serializer();
    let wire = include_str!("../../../tests/Dock/data/serializer-json-sample.graph.json");
    let root = serializer
        .deserialize::<Rc<dyn IRootDock>>(Some(wire))
        .unwrap()
        .unwrap();
    assert_eq!(serializer.serialize(root.clone()).unwrap(), wire);
    let home = concrete::<HomeViewModel>(root.visible_dockables().unwrap().get(1).unwrap());
    let documents =
        concrete::<CustomDocumentDock>(home.visible_dockables().unwrap().get(0).unwrap());
    concrete::<DashboardViewModel>(root.visible_dockables().unwrap().get(0).unwrap());
    concrete::<DocumentViewModel>(documents.visible_dockables().unwrap().get(0).unwrap());
    let tools = home
        .visible_dockables()
        .unwrap()
        .get(1)
        .unwrap()
        .as_tool_dock()
        .unwrap();
    concrete::<Tool1ViewModel>(tools.visible_dockables().unwrap().get(0).unwrap());
    concrete::<Tool8ViewModel>(tools.visible_dockables().unwrap().get(1).unwrap());
    assert!(Rc::ptr_eq(
        &root.active_dockable().unwrap(),
        &(home.clone() as Rc<dyn IDockable>)
    ));
    assert!(Rc::ptr_eq(
        &root.default_dockable().unwrap(),
        &(home.clone() as Rc<dyn IDockable>)
    ));
    assert!(Rc::ptr_eq(
        &documents.owner().unwrap(),
        &(home as Rc<dyn IDockable>)
    ));
    let factory = DockFactory::new(DemoData::new());
    ILayoutInitialization::initialize(factory.as_ref(), Some(root));
    documents.create_document().unwrap().execute(None);
    let added = documents.visible_dockables().unwrap().get(1).unwrap();
    concrete::<DocumentViewModel>(added.clone());
    assert_eq!(added.id(), "Document2");
    assert!(Rc::ptr_eq(&documents.active_dockable().unwrap(), &added));
}

#[test]
fn full_sample_restore_reconnects_live_factory_locators_and_fresh_contexts() {
    let factory = DockFactory::new(DemoData::new());
    let serialization = LayoutJsonSerialization::new().unwrap();
    let model =
        MainWindowViewModel::with_layout_services(factory.clone(), serialization, factory.clone());
    assert!(model.can_persist_layout());
    let before = model.layout().unwrap();
    let old_document = find(&factory, &before, "Document1");
    let old_context = old_document.context().unwrap();
    let wire = model.save_layout_json().unwrap();
    model.load_layout_json(&wire).unwrap();
    let root = model.layout().unwrap();
    assert!(!Rc::ptr_eq(&root, &before));
    let document = find(&factory, &root, "Document1");
    concrete::<DocumentViewModel>(document.clone());
    assert!(!Rc::ptr_eq(&document, &old_document));
    assert!(!Rc::ptr_eq(&document.context().unwrap(), &old_context));
    assert!(Rc::ptr_eq(
        &factory
            .get_dockable::<dyn IDockable>("Root".into())
            .unwrap(),
        &(root.clone() as Rc<dyn IDockable>)
    ));
    let documents = factory
        .find_dockable(
            Some(root.clone()),
            Some(Rc::new(|v| v.and_then(|v| v.as_document_dock()).is_some())),
        )
        .unwrap();
    assert!(Rc::ptr_eq(
        &factory
            .get_dockable::<dyn IDockable>("Documents".into())
            .unwrap(),
        &documents
    ));
    assert!(Rc::ptr_eq(
        find(&factory, &root, "Dashboard")
            .context()
            .unwrap()
            .downcast_ref::<Rc<dyn IDockable>>()
            .unwrap(),
        &(root.clone() as Rc<dyn IDockable>)
    ));
    let documents = concrete::<CustomDocumentDock>(documents);
    documents.create_document().unwrap().execute(None);
    assert_eq!(documents.visible_dockables().unwrap().count(), 4);
    concrete::<DocumentViewModel>(documents.active_dockable().unwrap());
    assert!(model.save_layout_json().is_ok());
}

#[test]
fn malformed_json_does_not_replace_layout_and_null_remains_distinct() {
    let factory = DockFactory::new(DemoData::new());
    let service = LayoutJsonSerialization::new().unwrap();
    assert_eq!(service.serialize(None).unwrap(), "null");
    assert!(service.deserialize("null").unwrap().is_none());
    let model =
        MainWindowViewModel::with_layout_services(factory.clone(), service, factory.clone());
    let root = model.layout().unwrap();
    assert!(model.load_layout_json("{").is_err());
    assert!(Rc::ptr_eq(&model.layout().unwrap(), &root));
    model.load_layout_json("null").unwrap();
    assert!(model.layout().is_none());
    assert!(factory
        .get_dockable::<dyn IDockable>("Root".into())
        .is_none());
    assert!(factory
        .get_dockable::<dyn IDockable>("Documents".into())
        .is_none());
    assert_eq!(model.global_status(), "Global: (none)");
    assert_eq!(model.save_layout_json().unwrap(), "null");
    let compatibility = MainWindowViewModel::new();
    assert!(!compatibility.can_persist_layout());
    assert!(compatibility.save_layout_json().is_err());
    assert!(compatibility.load_layout_json("null").is_err());
}
