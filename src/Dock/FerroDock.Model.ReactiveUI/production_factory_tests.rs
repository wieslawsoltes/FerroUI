//! Production factory construction, locators, dispatch and engine integration.
use crate::controls::*;
use crate::Factory;
use ferrodock_model::controls::*;
use ferrodock_model::core::*;
use ferrodock_model::BoxedValue;
use ferroui_system::collections::generic::Dictionary;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[test]
fn production_factory_has_fresh_shared_registries_and_observable_control_host_lists() {
    let factory = Factory::new();
    let other = Factory::new();
    let maps = [
        factory.visible_root_controls(),
        factory.pinned_root_controls(),
        factory.tab_root_controls(),
        factory.tool_controls(),
        factory.document_controls(),
    ];
    let document: Rc<dyn IDockable> = Document::new();
    let value: BoxedValue = Rc::new("control".to_owned());
    for (index, map) in maps.iter().enumerate() {
        assert!(map.borrow().is_empty());
        for earlier in &maps[..index] {
            assert!(!Rc::ptr_eq(map, earlier));
        }
        map.borrow_mut().insert(document.clone(), value.clone());
    }
    assert!(Rc::ptr_eq(&factory.visible_root_controls(), &maps[0]));
    assert!(Rc::ptr_eq(
        &factory
            .document_controls()
            .borrow()
            .get(&document)
            .unwrap()
            .clone(),
        &value
    ));
    assert!(other.visible_root_controls().borrow().is_empty());
    let controls = [
        factory.visible_dockable_controls(),
        factory.pinned_dockable_controls(),
        factory.tab_dockable_controls(),
    ];
    for (index, map) in controls.iter().enumerate() {
        assert!(map.borrow().is_empty());
        for earlier in &controls[..index] {
            assert!(!Rc::ptr_eq(map, earlier));
        }
    }
    assert!(Rc::ptr_eq(
        &factory.visible_dockable_controls(),
        &controls[0]
    ));
    assert!(factory
        .dock_controls()
        .as_notify_collection_changed()
        .is_some());
    assert!(factory
        .host_windows()
        .as_notify_collection_changed()
        .is_some());
    assert!(factory.dock_controls().ptr_eq(&factory.dock_controls()));
    assert!(factory.host_windows().ptr_eq(&factory.host_windows()));
}

#[test]
fn production_factory_constructs_every_model_with_concrete_identity_and_source_defaults() {
    let factory = Factory::new();
    let models: Vec<(Rc<dyn IDockable>, std::any::TypeId)> = vec![
        (
            factory.create_root_dock(),
            std::any::TypeId::of::<Rc<RootDock>>(),
        ),
        (
            factory.create_proportional_dock(),
            std::any::TypeId::of::<Rc<ProportionalDock>>(),
        ),
        (
            factory.create_dock_dock(),
            std::any::TypeId::of::<Rc<DockDock>>(),
        ),
        (
            factory.create_stack_dock(),
            std::any::TypeId::of::<Rc<StackDock>>(),
        ),
        (
            factory.create_grid_dock(),
            std::any::TypeId::of::<Rc<GridDock>>(),
        ),
        (
            factory.create_wrap_dock(),
            std::any::TypeId::of::<Rc<WrapDock>>(),
        ),
        (
            factory.create_uniform_grid_dock(),
            std::any::TypeId::of::<Rc<UniformGridDock>>(),
        ),
        (
            factory.create_proportional_dock_splitter(),
            std::any::TypeId::of::<Rc<ProportionalDockSplitter>>(),
        ),
        (
            factory.create_grid_dock_splitter(),
            std::any::TypeId::of::<Rc<GridDockSplitter>>(),
        ),
        (
            factory.create_tool_dock(),
            std::any::TypeId::of::<Rc<ToolDock>>(),
        ),
        (
            factory.create_document_dock(),
            std::any::TypeId::of::<Rc<DocumentDock>>(),
        ),
        (
            factory.create_split_view_dock(),
            std::any::TypeId::of::<Rc<SplitViewDock>>(),
        ),
        (
            factory.create_document(),
            std::any::TypeId::of::<Rc<Document>>(),
        ),
        (factory.create_tool(), std::any::TypeId::of::<Rc<Tool>>()),
    ];
    for (model, expected) in models {
        assert!(model.owner().is_none() && model.factory().is_none());
        assert_eq!(model.boxed_reference().as_ref().type_id(), expected);
    }
    assert!(factory.create_dock_dock().last_child_fill());
    let splitter = factory.create_proportional_dock_splitter();
    assert!(splitter.can_resize());
    assert!(!splitter.resize_preview());
    let tool = factory.create_tool();
    assert!(tool.min_width().is_nan() && tool.max_width().is_nan());
    assert!(tool.min_height().is_nan() && tool.max_height().is_nan());
    let window = factory.create_dock_window();
    assert!(window.factory().is_none() && window.owner().is_none());
}

#[test]
fn roots_and_layouts_have_four_distinct_observable_pin_lists_and_no_implicit_initialization() {
    let factory = Factory::new();
    for root in [factory.create_root_dock(), factory.create_layout().unwrap()] {
        let strips = [
            root.left_pinned_dockables().unwrap(),
            root.right_pinned_dockables().unwrap(),
            root.top_pinned_dockables().unwrap(),
            root.bottom_pinned_dockables().unwrap(),
        ];
        for (index, list) in strips.iter().enumerate() {
            assert_eq!(list.count(), 0);
            assert!(list.as_notify_collection_changed().is_some());
            for earlier in &strips[..index] {
                assert!(!list.ptr_eq(earlier));
            }
        }
        assert!(root.visible_dockables().is_none());
        assert!(root.factory().is_none() && root.owner().is_none());
    }
}

#[test]
fn generic_lists_preserve_native_values_without_initializing_dockables() {
    #[derive(Clone, PartialEq)]
    struct Entry(u32);
    let factory = Factory::new();
    let list = factory.create_list(vec![Entry(1), Entry(2)]);
    assert!(list.as_notify_collection_changed().is_some());
    assert_eq!(list.get(1).0, 2);
    let document: Rc<dyn IDockable> = factory.create_document();
    let documents = factory.create_list(vec![document.clone()]);
    assert!(Rc::ptr_eq(&documents.get(0), &document));
    assert!(document.factory().is_none() && document.owner().is_none());
}

#[test]
fn production_factory_references_use_identity_when_stored_in_native_lists() {
    let factory = Factory::new();
    let other = Factory::new();
    let values = factory.create_list(vec![factory.clone()]);
    assert!(values.contains(&factory));
    assert!(!values.contains(&other));
}

#[test]
fn typed_locators_preserve_concrete_and_interface_identity_and_reject_mismatches() {
    let factory = Factory::new();
    let document = Document::new();
    let calls = Rc::new(Cell::new(0));
    let log = calls.clone();
    let located = document.clone();
    let map: DockableLocatorMap = Rc::new(RefCell::new(Dictionary::new()));
    map.borrow_mut().insert(
        "doc".into(),
        Some(Rc::new(move || {
            log.set(log.get() + 1);
            Some(located.clone())
        })),
    );
    factory.set_dockable_locator(Some(map));
    assert!(factory.get_dockable::<Document>(String::new()).is_none());
    assert!(factory.get_dockable::<Document>("absent".into()).is_none());
    assert_eq!(calls.get(), 0);
    assert!(Rc::ptr_eq(
        &factory.get_dockable::<Document>("doc".into()).unwrap(),
        &document
    ));
    assert!(Rc::ptr_eq(
        &factory.get_dockable::<dyn IDocument>("doc".into()).unwrap(),
        &(document.clone() as Rc<dyn IDocument>)
    ));
    assert!(Rc::ptr_eq(
        &factory.get_dockable::<dyn IDockable>("doc".into()).unwrap(),
        &(document.clone() as Rc<dyn IDockable>)
    ));
    assert!(factory.get_dockable::<dyn ITool>("doc".into()).is_none());
    assert!(factory.get_dockable::<Tool>("doc".into()).is_none());
    assert_eq!(calls.get(), 5);
}

#[test]
fn locator_callback_can_mutate_its_registry_and_return_null_without_borrow_conflicts() {
    let factory = Factory::new();
    let registry: DockableLocatorMap = Rc::new(RefCell::new(Dictionary::new()));
    let weak = Rc::downgrade(&registry);
    registry.borrow_mut().insert(
        "self".into(),
        Some(Rc::new(move || {
            weak.upgrade().unwrap().borrow_mut().remove("self");
            None
        })),
    );
    factory.set_dockable_locator(Some(registry.clone()));
    assert!(factory
        .get_dockable::<dyn IDockable>("self".into())
        .is_none());
    assert!(registry.borrow().is_empty());
}

#[test]
fn typed_locator_supports_every_core_dockable_container_splitter_and_mdi_contract() {
    let factory = Factory::new();
    macro_rules! check {
        ($contract:ty,$create:ident) => {{
            let item: Rc<dyn IDockable> = factory.$create();
            let located = item.clone();
            let registry: DockableLocatorMap = Rc::new(RefCell::new(Dictionary::new()));
            registry
                .borrow_mut()
                .insert("item".into(), Some(Rc::new(move || Some(located.clone()))));
            factory.set_dockable_locator(Some(registry));
            let actual: Rc<dyn IDockable> =
                factory.get_dockable::<$contract>("item".into()).unwrap();
            assert!(Rc::ptr_eq(&actual, &item));
        }};
    }
    check!(dyn IDock, create_document_dock);
    check!(dyn IDocument, create_document);
    check!(dyn ITool, create_tool);
    check!(dyn IMdiDocument, create_document);
    check!(dyn ISplitter, create_proportional_dock_splitter);
    check!(
        dyn IProportionalDockSplitter,
        create_proportional_dock_splitter
    );
    check!(dyn IGridDockSplitter, create_grid_dock_splitter);
    check!(dyn IRootDock, create_root_dock);
    check!(dyn IToolDock, create_tool_dock);
    check!(dyn IDocumentDock, create_document_dock);
    check!(dyn IProportionalDock, create_proportional_dock);
    check!(dyn IDockDock, create_dock_dock);
    check!(dyn IStackDock, create_stack_dock);
    check!(dyn IWrapDock, create_wrap_dock);
    check!(dyn IGridDock, create_grid_dock);
    check!(dyn IUniformGridDock, create_uniform_grid_dock);
    check!(dyn ISplitViewDock, create_split_view_dock);
}

#[test]
fn production_factory_resolves_hosts_and_presents_modal_windows_through_real_engine_bodies() {
    let factory = Factory::new();
    let host = crate::test_host::RecordingHost::new();
    let located = host.clone();
    factory.set_default_host_window_locator(Some(Rc::new(move || Some(located.clone()))));
    let root = factory.create_layout().unwrap();
    let dock = factory.create_tool_dock();
    let tool = factory.create_tool();
    factory.add_dockable(Some(root.clone()), Some(dock.clone()));
    factory.add_dockable(Some(dock), Some(tool.clone()));
    factory.init_layout(root.clone());
    let options = DockWindowOptions::new();
    options.set_is_modal(true);
    factory.float_dockable_with_options(Some(tool), Some(options));
    let window = root
        .windows()
        .unwrap()
        .get(0)
        .expect("nonnull window fixture");
    assert!(window.is_modal());
    assert!(Rc::ptr_eq(
        &window.host().unwrap(),
        &(host.clone() as Rc<dyn IHostWindow>)
    ));
    assert!(host
        .calls
        .borrow()
        .contains(&crate::test_host::HostCall::Present(true)));
}

#[test]
fn production_factory_runs_full_tool_document_pin_float_hide_restore_state_transitions() {
    let factory = Factory::new();
    let root = factory.create_root_dock();
    root.set_visible_dockables(Some(factory.create_list(Vec::new())));
    root.set_hidden_dockables(Some(factory.create_list(Vec::new())));
    root.set_windows(Some(factory.create_list(Vec::new())));
    let tools = factory.create_tool_dock();
    let documents = factory.create_document_dock();
    factory.add_dockable(Some(root.clone()), Some(tools.clone()));
    factory.add_dockable(Some(root.clone()), Some(documents.clone()));
    let tool = factory.create_tool();
    let document = factory.create_document();
    factory.add_dockable(Some(tools.clone()), Some(tool.clone()));
    factory.add_dockable(Some(documents.clone()), Some(document.clone()));
    factory.init_layout(root.clone());
    assert_eq!(tool.docking_state(), DockingWindowState::DOCKED);
    assert_eq!(document.docking_state(), DockingWindowState::DOCUMENT);
    factory.dock_as_document(tool.clone());
    assert_eq!(tool.docking_state(), DockingWindowState::DOCUMENT);
    factory.move_dockable_between_docks(
        Some(documents.clone()),
        Some(tools.clone()),
        Some(tool.clone()),
        None,
    );
    assert_eq!(tool.docking_state(), DockingWindowState::DOCKED);
    factory.pin_dockable(Some(tool.clone()));
    assert_eq!(tool.docking_state(), DockingWindowState::PINNED);
    factory.unpin_dockable(Some(tool.clone()));
    assert_eq!(tool.docking_state(), DockingWindowState::DOCKED);
    factory.float_dockable(Some(tool.clone()));
    let floating = DockingWindowState::DOCKED | DockingWindowState::FLOATING;
    assert_eq!(tool.docking_state(), floating);
    factory.hide_dockable(Some(tool.clone()));
    assert_eq!(tool.docking_state(), floating | DockingWindowState::HIDDEN);
    factory.restore_dockable(Some(tool.clone()));
    assert_eq!(tool.docking_state(), floating);
    factory.pin_dockable(Some(tool.clone()));
    assert_eq!(
        tool.docking_state(),
        DockingWindowState::PINNED | DockingWindowState::FLOATING
    );
    factory.unpin_dockable(Some(tool.clone()));
    assert_eq!(tool.docking_state(), floating);
    factory.float_dockable(Some(document.clone()));
    let floating = DockingWindowState::DOCUMENT | DockingWindowState::FLOATING;
    assert_eq!(document.docking_state(), floating);
    factory.hide_dockable(Some(document.clone()));
    assert_eq!(
        document.docking_state(),
        floating | DockingWindowState::HIDDEN
    );
    factory.restore_dockable(Some(document.clone()));
    assert_eq!(document.docking_state(), floating);
    assert!(Rc::ptr_eq(
        &tool.factory().unwrap(),
        &(factory.clone() as Rc<dyn IFactory>)
    ));
    assert_eq!(root.windows().unwrap().count(), 2);
}

#[test]
fn production_factory_splits_moves_closes_and_reconciles_focus_through_real_event_dispatch() {
    let factory = Factory::new();
    let root = factory.create_layout().unwrap();
    let dock = factory.create_document_dock();
    dock.set_is_collapsable(false);
    factory.add_dockable(Some(root.clone()), Some(dock.clone()));
    let document = factory.create_document();
    factory.add_dockable(Some(dock.clone()), Some(document.clone()));
    factory.init_layout(root.clone());
    factory.new_vertical_document_dock(document.clone());
    let target = document.owner().unwrap().as_document_dock().unwrap();
    assert!(!Rc::ptr_eq(&target, &dock));
    assert_eq!(target.visible_dockables().unwrap().count(), 1);
    let closed = Rc::new(Cell::new(0));
    let log = closed.clone();
    factory.dockable_closed(Rc::new(move |_| log.set(log.get() + 1)));
    factory.close_dockable(Some(document.clone()));
    assert_eq!(closed.get(), 1);
    assert!(!root
        .focused_dockable()
        .is_some_and(|item| Rc::ptr_eq(&item, &(document.clone() as Rc<dyn IDockable>))));
    assert!(target.visible_dockables().unwrap().count() == 0);
    assert!(!factory
        .current_dockable()
        .is_some_and(|item| Rc::ptr_eq(&item, &(document.clone() as Rc<dyn IDockable>))));
}
