use super::*;
use ferrodock_model::core::*;
use ferrodock_model_reactiveui::controls::*;
#[test]
fn standard_catalog_constructs_original_models_without_caller_metadata() {
    let serializer = create_serializer().unwrap();
    let root = RootDock::new();
    root.set_id("root".into());
    let text = serializer.serialize(root).unwrap();
    let loaded = serializer
        .deserialize::<Rc<RootDock>>(Some(&text))
        .unwrap()
        .unwrap();
    assert_eq!(loaded.id(), "root");
    assert_eq!(serializer.serialize(loaded).unwrap(), text);
}

use ferrodock_model::{collections::ListHandle, controls::*};
use ferrodock_model_reactiveui::{core::DockWindow, Factory};
#[test]
fn all_standard_model_defaults_match_original_serializer_wire_and_read_back() {
    let serializer = create_serializer().unwrap();
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
        DockDock,
        "../../../tests/Dock/data/serializer-json-catalog.dock-dock.json"
    );
    check!(
        Document,
        "../../../tests/Dock/data/serializer-json-catalog.document.json"
    );
    check!(
        DocumentDock,
        "../../../tests/Dock/data/serializer-json-catalog.document-dock.json"
    );
    check!(
        GridDock,
        "../../../tests/Dock/data/serializer-json-catalog.grid-dock.json"
    );
    check!(
        GridDockSplitter,
        "../../../tests/Dock/data/serializer-json-catalog.grid-dock-splitter.json"
    );
    check!(
        ProportionalDock,
        "../../../tests/Dock/data/serializer-json-catalog.proportional-dock.json"
    );
    check!(
        ProportionalDockSplitter,
        "../../../tests/Dock/data/serializer-json-catalog.proportional-dock-splitter.json"
    );
    check!(
        RootDock,
        "../../../tests/Dock/data/serializer-json-catalog.root-dock.json"
    );
    check!(
        SplitViewDock,
        "../../../tests/Dock/data/serializer-json-catalog.split-view-dock.json"
    );
    check!(
        StackDock,
        "../../../tests/Dock/data/serializer-json-catalog.stack-dock.json"
    );
    check!(Tool, "../../../tests/Dock/data/serializer-json-catalog.tool.json");
    check!(
        ToolDock,
        "../../../tests/Dock/data/serializer-json-catalog.tool-dock.json"
    );
    check!(
        UniformGridDock,
        "../../../tests/Dock/data/serializer-json-catalog.uniform-grid-dock.json"
    );
    check!(
        WrapDock,
        "../../../tests/Dock/data/serializer-json-catalog.wrap-dock.json"
    );
    check!(
        DockWindow,
        "../../../tests/Dock/data/serializer-json-catalog.window.json"
    );
}
fn concrete<T: 'static>(value: Rc<dyn IDockable>) -> Rc<T> {
    value
        .boxed_reference()
        .downcast_ref::<Rc<T>>()
        .unwrap()
        .clone()
}
#[test]
fn complete_original_graph_roundtrips_and_restores_the_real_factory_engine() {
    let serializer = create_serializer().unwrap();
    let expected = include_str!("../../../tests/Dock/data/serializer-json-catalog.graph.json");
    let root = serializer
        .deserialize::<Rc<dyn IRootDock>>(Some(expected))
        .unwrap()
        .unwrap();
    assert_eq!(serializer.serialize(root.clone()).unwrap(), expected);
    let split = concrete::<ProportionalDock>(root.visible_dockables().unwrap().get(0).unwrap());
    let tools = concrete::<ToolDock>(split.visible_dockables().unwrap().get(0).unwrap());
    let documents = concrete::<DocumentDock>(split.visible_dockables().unwrap().get(2).unwrap());
    let document = concrete::<Document>(documents.visible_dockables().unwrap().get(0).unwrap());
    assert!(Rc::ptr_eq(
        &split.owner().unwrap(),
        &(root.clone() as Rc<dyn IDockable>)
    ));
    assert!(Rc::ptr_eq(
        &documents.active_dockable().unwrap(),
        &(document.clone() as Rc<dyn IDockable>)
    ));
    let windows = root.windows().unwrap();
    assert!(Rc::ptr_eq(
        &windows.get(0).unwrap(),
        &windows.get(1).unwrap()
    ));
    assert!(Rc::ptr_eq(
        &windows.get(0).unwrap().parent_window().unwrap(),
        &root.window().unwrap()
    ));
    assert!(Rc::ptr_eq(&root.window().unwrap().layout().unwrap(), &root));
    assert_eq!(
        root.left_pinned_dockables().unwrap().get(0).unwrap().id(),
        "pinned"
    );
    assert_eq!(
        root.hidden_dockables().unwrap().get(0).unwrap().id(),
        "hidden"
    );
    assert_eq!(document.mdi_z_index(), 7);
    assert_eq!(document.mdi_state(), MdiWindowState::Maximized);
    assert_eq!(document.column(), 2);
    assert_eq!(document.row_span(), 3);
    assert!(!document.show_in_selector());
    assert_eq!(
        document.selector_title().as_deref(),
        Some("Editor selector")
    );
    assert_eq!(tools.alignment(), Alignment::Left);
    assert_eq!(tools.grip_mode(), GripMode::AutoHide);
    assert!(!tools.auto_hide());
    assert_eq!(
        tools
            .visible_dockables()
            .unwrap()
            .get(0)
            .unwrap()
            .pinned_bounds()
            .unwrap()
            .width,
        240.0
    );
    let factory = Factory::new();
    factory.init_layout(root.clone());
    assert!(Rc::ptr_eq(
        &document.factory().unwrap(),
        &(factory.clone() as Rc<dyn IFactory>)
    ));
    assert!(Rc::ptr_eq(
        &documents.owner().unwrap(),
        &(split as Rc<dyn IDockable>)
    ));
    let added = Document::new();
    added.set_id("added".into());
    documents.add_document(added.clone());
    assert!(Rc::ptr_eq(
        &added.owner().unwrap(),
        &(documents.clone() as Rc<dyn IDockable>)
    ));
    assert!(Rc::ptr_eq(
        &documents.active_dockable().unwrap(),
        &(added as Rc<dyn IDockable>)
    ));
}
#[test]
fn list_overload_uses_the_requested_collection_and_preserves_null_entries() {
    assert!(create_serializer_with_list_type(None).is_err());
    for list_type in [JsonListType::observable_collection(), JsonListType::list()] {
        let notifying = list_type.name().contains("ObservableCollection");
        let serializer = create_serializer_with_list_type(Some(list_type)).unwrap();
        let root = RootDock::new();
        root.set_visible_dockables(Some(ListHandle::from_plain(vec![
            None,
            Some(Tool::new() as Rc<dyn IDockable>),
        ])));
        let text = serializer.serialize(root).unwrap();
        let loaded = serializer
            .deserialize::<Rc<RootDock>>(Some(&text))
            .unwrap()
            .unwrap();
        let list = loaded.visible_dockables().unwrap();
        assert_eq!(list.as_observable_collection().is_some(), notifying);
        assert!(list.get(0).is_none());
        assert!(list.get(1).unwrap().as_tool().is_some());
        assert_eq!(serializer.serialize(loaded).unwrap(), text);
    }
}

#[test]
fn save_and_load_use_original_stream_contract_and_complete_catalog() {
    use ferrodock_model::io::{MemoryStream, Stream};
    use std::cell::RefCell;
    let serializer = create_serializer().unwrap();
    let wire = include_str!("../../../tests/Dock/data/serializer-json-catalog.graph.json");
    let root = serializer
        .deserialize::<Rc<dyn IRootDock>>(Some(wire))
        .unwrap()
        .unwrap();
    let output = MemoryStream::new();
    serializer.save(Some(output.clone()), root).unwrap();
    assert!(!output.can_write());
    let bytes = output.to_array();
    assert!(bytes.starts_with(&[0xef, 0xbb, 0xbf]));
    let input = MemoryStream::from_buffer(Rc::new(RefCell::new(bytes.into_boxed_slice())), false);
    let loaded = serializer
        .load::<Rc<dyn IRootDock>>(Some(input.clone()))
        .unwrap()
        .unwrap();
    assert!(!input.can_read());
    assert_eq!(serializer.serialize(loaded).unwrap(), wire);
}

#[test]
fn malformed_layout_and_unsupported_closed_enum_values_return_errors() {
    let serializer = create_serializer().unwrap();
    for wire in ["{", r#"{"TabsLayout":999}"#, r#"{"LayoutMode":"Tabbed"}"#] {
        assert!(serializer
            .deserialize::<Rc<DocumentDock>>(Some(wire))
            .is_err());
    }
}

#[test]
fn configured_layout_containers_match_original_wire_and_values() {
    let serializer = create_serializer().unwrap();
    let wire = include_str!("../../../tests/Dock/data/serializer-json-catalog.containers.json");
    let root = serializer
        .deserialize::<Rc<RootDock>>(Some(wire))
        .unwrap()
        .unwrap();
    assert_eq!(serializer.serialize(root.clone()).unwrap(), wire);
    let items = root.visible_dockables().unwrap();
    assert!(!concrete::<DockDock>(items.get(0).unwrap()).last_child_fill());
    assert_eq!(
        concrete::<GridDock>(items.get(1).unwrap())
            .column_definitions()
            .as_deref(),
        Some("Auto,*,2*")
    );
    assert_eq!(
        concrete::<GridDockSplitter>(items.get(2).unwrap()).resize_direction(),
        GridResizeDirection::Rows
    );
    assert_eq!(
        concrete::<ProportionalDock>(items.get(3).unwrap()).orientation(),
        Orientation::Vertical
    );
    assert!(!concrete::<ProportionalDockSplitter>(items.get(4).unwrap()).can_resize());
    let split = concrete::<SplitViewDock>(items.get(5).unwrap());
    assert!(split.is_pane_open());
    assert_eq!(split.compact_pane_length(), 32.0);
    assert_eq!(split.display_mode(), SplitViewDisplayMode::CompactInline);
    assert_eq!(split.open_pane_length(), 256.0);
    assert_eq!(split.pane_placement(), SplitViewPanePlacement::Right);
    assert!(split.use_light_dismiss_overlay_mode());
    assert_eq!(split.pane_dockable().unwrap().id(), "pane");
    assert_eq!(split.content_dockable().unwrap().id(), "content");
    assert_eq!(concrete::<StackDock>(items.get(6).unwrap()).spacing(), 12.5);
    assert_eq!(
        concrete::<UniformGridDock>(items.get(7).unwrap()).columns(),
        3
    );
    assert_eq!(
        concrete::<WrapDock>(items.get(8).unwrap()).orientation(),
        Orientation::Vertical
    );
}
