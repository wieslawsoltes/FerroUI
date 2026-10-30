//! Floating-window construction, placement, policy and document-pane regressions.
use crate::controls::*;
use crate::core::DockWindow;
use crate::factory_init_tests::factory;
use crate::factory_parameter_tests::RecordingFactory;
use crate::test_host::{HostCall, RecordingHost};
use ferrodock_model::collections::ListHandle;
use ferrodock_model::core::*;
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

struct Scene {
    factory: Rc<RecordingFactory>,
    root: Rc<RootDock>,
    dock: Rc<DocumentDock>,
    first: Rc<Document>,
    second: Rc<Document>,
}
impl Scene {
    fn new() -> Self {
        let factory = factory();
        let root = RootDock::new();
        let dock = DocumentDock::new();
        dock.set_is_collapsable(false);
        let first = Document::new();
        let second = Document::new();
        dock.set_visible_dockables(Some(ListHandle::from_items(vec![
            Some(first.clone() as Rc<dyn IDockable>),
            Some(second.clone()),
        ])));
        dock.set_active_dockable(Some(first.clone()));
        root.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
            dock.clone() as Rc<dyn IDockable>
        )])));
        factory.init_dockable(Some(root.clone()), None);
        Self {
            factory,
            root,
            dock,
            first,
            second,
        }
    }
    fn window(&self) -> Rc<dyn IDockWindow> {
        self.root
            .windows()
            .unwrap()
            .get(0)
            .expect("nonnull window fixture")
    }
}
fn target(window: &Rc<dyn IDockWindow>) -> Rc<dyn IDock> {
    window
        .layout()
        .unwrap()
        .visible_dockables()
        .unwrap()
        .get(0)
        .and_then(|item| item.as_dock())
        .unwrap()
}

#[test]
fn document_window_creation_copies_source_metadata_and_preserves_uninitialized_owner_links() {
    let scene = Scene::new();
    scene.root.set_enable_adaptive_global_dock_targets(true);
    scene.dock.set_id("documents".into());
    scene.dock.set_can_create_document(true);
    scene.dock.set_enable_window_drag(false);
    scene.dock.set_layout_mode(DocumentLayoutMode::Mdi);
    scene.first.set_dock_group(Some("group".into()));
    let callback: DocumentFactory = Rc::new(|| Some(Document::new()));
    scene.dock.set_document_factory(Some(callback.clone()));
    let window = scene
        .factory
        .create_window_from(Some(scene.first.clone()))
        .unwrap();
    let layout = window.layout().unwrap();
    let dock = target(&window);
    let document = dock.clone().as_document_dock().unwrap();
    assert_eq!(dock.id(), "documents");
    assert_eq!(dock.title(), "IDocumentDock");
    assert_eq!(dock.dock_group().as_deref(), Some("group"));
    assert!(document.can_create_document());
    assert!(!document.enable_window_drag());
    assert_eq!(document.layout_mode(), DocumentLayoutMode::Tabbed);
    assert!(Rc::ptr_eq(
        &document
            .clone()
            .as_document_dock_factory()
            .unwrap()
            .document_factory()
            .unwrap(),
        &callback
    ));
    assert_eq!(dock.visible_dockables().unwrap().count(), 1);
    assert!(Rc::ptr_eq(
        &scene.first.owner().unwrap(),
        &(scene.dock.clone() as Rc<dyn IDockable>)
    ));
    assert_eq!(scene.dock.visible_dockables().unwrap().count(), 2);
    assert!(dock.owner().is_none() && dock.factory().is_none());
    assert!(layout.owner().is_none() && layout.factory().is_none());
    assert!(layout.enable_adaptive_global_dock_targets());
    assert_eq!(layout.title(), "IRootDock");
    assert!(Rc::ptr_eq(
        &layout.active_dockable().unwrap(),
        &(dock.clone() as Rc<dyn IDockable>)
    ));
    assert!(Rc::ptr_eq(
        &layout.default_dockable().unwrap(),
        &(dock as Rc<dyn IDockable>)
    ));
    assert!(Rc::ptr_eq(&layout.window().unwrap(), &window));
    assert_eq!(window.id(), "IDockWindow");
    assert!(window.title().is_empty());
    assert!(window.width().is_nan() && window.height().is_nan());
    assert!(window.owner().is_none() && window.factory().is_none() && window.host().is_none());
    assert!(scene.root.windows().is_none());
}

#[test]
fn tool_priority_wins_over_document_membership_and_copies_all_tool_dock_flags() {
    let factory = factory();
    let root = RootDock::new();
    let dock = ToolDock::new();
    dock.set_id("tools".into());
    dock.set_alignment(Alignment::Bottom);
    dock.set_is_expanded(false);
    dock.set_auto_hide(false);
    dock.set_grip_mode(GripMode::Hidden);
    let tool = Tool::new();
    assert!((tool.clone() as Rc<dyn IDockable>).as_document().is_some());
    dock.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
        tool.clone() as Rc<dyn IDockable>
    )])));
    root.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
        dock.clone() as Rc<dyn IDockable>
    )])));
    factory.init_dockable(Some(root.clone()), None);
    let window = factory.create_window_from(Some(tool)).unwrap();
    let target = target(&window).as_tool_dock().unwrap();
    assert_eq!(target.title(), "IToolDock");
    assert_eq!(target.id(), "tools");
    assert_eq!(target.alignment(), Alignment::Bottom);
    assert!(!target.is_expanded() && !target.auto_hide());
    assert_eq!(target.grip_mode(), GripMode::Hidden);
}

#[test]
fn supported_containers_are_reused_and_other_container_types_are_rejected() {
    let factory = factory();
    let supported: Vec<Rc<dyn IDockable>> = vec![
        ToolDock::new(),
        DocumentDock::new(),
        ProportionalDock::new(),
        DockDock::new(),
    ];
    for source in supported {
        let window = factory.create_window_from(Some(source.clone())).unwrap();
        assert!(Rc::ptr_eq(&(target(&window) as Rc<dyn IDockable>), &source));
    }
    let unsupported: Vec<Rc<dyn IDockable>> = vec![
        StackDock::new(),
        GridDock::new(),
        WrapDock::new(),
        UniformGridDock::new(),
        SplitViewDock::new(),
        ProportionalDockSplitter::new(),
        GridDockSplitter::new(),
    ];
    for source in unsupported {
        assert!(factory.create_window_from(Some(source)).is_none());
    }
}

#[test]
fn root_creation_uses_only_active_child_and_allows_no_active_child() {
    let factory = factory();
    let root = RootDock::new();
    let child = ToolDock::new();
    root.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
        child.clone() as Rc<dyn IDockable>
    )])));
    let empty = factory.create_window_from(Some(root.clone())).unwrap();
    assert_eq!(
        empty.layout().unwrap().visible_dockables().unwrap().count(),
        0
    );
    assert!(empty.layout().unwrap().active_dockable().is_none());
    root.set_active_dockable(Some(child.clone()));
    let active = factory.create_window_from(Some(root.clone())).unwrap();
    assert!(Rc::ptr_eq(
        &(target(&active) as Rc<dyn IDockable>),
        &(child as Rc<dyn IDockable>)
    ));
    assert_eq!(root.visible_dockables().unwrap().count(), 1);
}

#[test]
fn create_window_options_capture_existing_parent_and_apply_before_registration() {
    let scene = Scene::new();
    let parent = DockWindow::new();
    scene.root.set_window(Some(parent.clone()));
    let options = DockWindowOptions::new();
    options.set_owner_mode(DockWindowOwnerMode::DockableWindow);
    options.set_is_modal(true);
    options.set_show_in_taskbar(Some(false));
    let window = scene
        .factory
        .create_window_from_with_options(Some(scene.first), Some(options.clone()))
        .unwrap();
    assert!(Rc::ptr_eq(
        &options.parent_window().unwrap(),
        &(parent.clone() as Rc<dyn IDockWindow>)
    ));
    assert!(Rc::ptr_eq(
        &window.parent_window().unwrap(),
        &(parent as Rc<dyn IDockWindow>)
    ));
    assert_eq!(window.owner_mode(), DockWindowOwnerMode::DockableWindow);
    assert!(window.is_modal());
    assert_eq!(window.show_in_taskbar(), Some(false));
}

#[test]
fn split_to_window_registers_before_init_then_presents_and_reports_docked_before_focusing() {
    let scene = Scene::new();
    let host = RecordingHost::new();
    scene.factory.supports_host_factory.set(true);
    *scene.factory.host_factory_result.borrow_mut() = Some(host.clone());
    let events = Rc::new(RefCell::new(Vec::new()));
    let log = events.clone();
    scene.factory.dockable_undocked(Rc::new(move |args| {
        assert_eq!(args.operation(), DockOperation::Window);
        log.borrow_mut().push("undocked");
    }));
    let log = events.clone();
    scene.factory.window_added(Rc::new(move |args| {
        let window = args.window().unwrap();
        assert!(window.factory().is_none() && window.owner().is_none());
        log.borrow_mut().push("window-added");
    }));
    let log = events.clone();
    let expected_host = host.clone();
    scene.factory.dockable_docked(Rc::new(move |args| {
        assert_eq!(args.operation(), DockOperation::Window);
        assert!(expected_host
            .calls
            .borrow()
            .contains(&HostCall::Present(true)));
        log.borrow_mut().push("docked");
    }));
    let options = DockWindowOptions::new();
    options.set_is_modal(true);
    scene.factory.split_to_window_with_options(
        Some(scene.dock.clone()),
        Some(scene.first.clone()),
        11.0,
        22.0,
        333.0,
        444.0,
        Some(options),
    );
    let window = scene.window();
    assert_eq!(
        (window.x(), window.y(), window.width(), window.height()),
        (11.0, 22.0, 333.0, 444.0)
    );
    assert_eq!(scene.dock.visible_dockables().unwrap().count(), 1);
    assert!(Rc::ptr_eq(
        &window.owner().unwrap(),
        &(scene.root.clone() as Rc<dyn IDockable>)
    ));
    assert!(Rc::ptr_eq(
        &scene.first.owner().unwrap(),
        &(target(&window) as Rc<dyn IDockable>)
    ));
    assert!(Rc::ptr_eq(
        &window.layout().unwrap().focused_dockable().unwrap(),
        &(scene.first as Rc<dyn IDockable>)
    ));
    assert_eq!(*events.borrow(), ["undocked", "window-added", "docked"]);
    assert_eq!(host.position.get(), (11.0, 22.0));
    assert_eq!(host.size.get(), (333.0, 444.0));
}

#[test]
fn split_without_root_is_noop_but_unsupported_window_target_is_removed_and_undocked() {
    let factory = factory();
    let dock = DockDock::new();
    let item = StackDock::new();
    dock.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
        item.clone() as Rc<dyn IDockable>
    )])));
    item.set_owner(Some(dock.clone()));
    factory.split_to_window(Some(dock.clone()), Some(item.clone()), 1.0, 2.0, 3.0, 4.0);
    assert_eq!(dock.visible_dockables().unwrap().count(), 1);
    let root = RootDock::new();
    dock.set_is_collapsable(false);
    root.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
        dock.clone() as Rc<dyn IDockable>
    )])));
    factory.init_dockable(Some(root.clone()), None);
    let undocked = Rc::new(Cell::new(0));
    let log = undocked.clone();
    factory.dockable_undocked(Rc::new(move |_| log.set(log.get() + 1)));
    factory.split_to_window(Some(dock.clone()), Some(item), 1.0, 2.0, 3.0, 4.0);
    assert_eq!(dock.visible_dockables().unwrap().count(), 0);
    assert!(root.windows().is_none());
    assert_eq!(undocked.get(), 1);
}

#[test]
fn floating_from_nested_window_registers_in_outer_workspace_and_captures_parent_before_removal() {
    let scene = Scene::new();
    let workspace = RootDock::new();
    let previous = DockWindow::new();
    previous.set_layout(Some(scene.root.clone()));
    scene.root.set_window(Some(previous.clone()));
    scene
        .factory
        .add_window(Some(workspace.clone()), Some(previous.clone()));
    let options = DockWindowOptions::new();
    options.set_owner_mode(DockWindowOwnerMode::DockableWindow);
    scene
        .factory
        .float_dockable_with_options(Some(scene.first.clone()), Some(options.clone()));
    let windows = workspace.windows().unwrap();
    assert_eq!(windows.count(), 2);
    assert!(scene.root.windows().is_none());
    let created = windows.get(1).expect("nonnull window fixture");
    assert!(Rc::ptr_eq(
        &created.owner().unwrap(),
        &(workspace as Rc<dyn IDockable>)
    ));
    assert!(Rc::ptr_eq(
        &created.parent_window().unwrap(),
        &(previous as Rc<dyn IDockWindow>)
    ));
    assert_eq!(created.owner_mode(), DockWindowOwnerMode::DockableWindow);
}

#[test]
fn floating_position_uses_per_axis_pointer_then_item_bounds_owner_bounds_and_zero() {
    for (item_pointer, owner_pointer, item_bounds, owner_bounds, expected) in [
        (
            (1.0, f64::NAN),
            (20.0, 30.0),
            (40.0, 50.0),
            (60.0, 70.0),
            (1.0, 30.0),
        ),
        (
            (f64::NAN, f64::NAN),
            (f64::NAN, f64::NAN),
            (40.0, f64::NAN),
            (60.0, 70.0),
            (40.0, 70.0),
        ),
        (
            (f64::NAN, f64::NAN),
            (f64::NAN, f64::NAN),
            (f64::NAN, f64::NAN),
            (f64::NAN, f64::NAN),
            (0.0, 0.0),
        ),
        (
            (f64::INFINITY, f64::NEG_INFINITY),
            (20.0, 30.0),
            (40.0, 50.0),
            (60.0, 70.0),
            (f64::INFINITY, f64::NEG_INFINITY),
        ),
    ] {
        let scene = Scene::new();
        scene
            .first
            .set_pointer_screen_position(item_pointer.0, item_pointer.1);
        scene
            .dock
            .set_pointer_screen_position(owner_pointer.0, owner_pointer.1);
        scene
            .first
            .set_visible_bounds(item_bounds.0, item_bounds.1, 500.0, 600.0);
        scene
            .dock
            .set_visible_bounds(owner_bounds.0, owner_bounds.1, 700.0, 800.0);
        scene.factory.float_dockable(Some(scene.first.clone()));
        let window = scene.window();
        assert_eq!((window.x(), window.y()), expected);
        assert_eq!((window.width(), window.height()), (500.0, 600.0));
    }
}

#[test]
fn floating_dimensions_require_finite_values_strictly_above_sixteen_and_fall_back_per_axis() {
    for (item, owner, expected) in [
        ((16.0, 16.1), (200.0, 300.0), (200.0, 16.1)),
        (
            (f64::INFINITY, f64::NAN),
            (f64::NEG_INFINITY, 15.0),
            (300.0, 400.0),
        ),
        ((-5.0, 0.0), (17.0, 18.0), (17.0, 18.0)),
        ((500.0, 600.0), (700.0, 800.0), (500.0, 600.0)),
    ] {
        let scene = Scene::new();
        scene.first.set_visible_bounds(0.0, 0.0, item.0, item.1);
        scene.dock.set_visible_bounds(0.0, 0.0, owner.0, owner.1);
        scene.factory.float_dockable(Some(scene.first.clone()));
        assert_eq!((scene.window().width(), scene.window().height()), expected);
    }
}

#[test]
fn denied_float_and_dock_as_document_capabilities_leave_layout_untouched() {
    let scene = Scene::new();
    scene.first.set_can_float(false);
    scene.first.set_can_dock_as_document(false);
    scene.factory.float_dockable(Some(scene.first.clone()));
    scene.factory.float_all_dockables(Some(scene.first.clone()));
    scene.factory.dock_as_document(scene.first.clone());
    assert!(scene.root.windows().is_none());
    assert_eq!(scene.dock.visible_dockables().unwrap().count(), 2);
}

#[test]
fn float_all_moves_snapshot_preserves_non_collapsable_source_and_uses_owner_geometry_and_last_selection(
) {
    let scene = Scene::new();
    scene.dock.set_title("source".into());
    scene.dock.set_id("documents".into());
    scene.dock.set_can_create_document(true);
    scene.dock.set_enable_window_drag(false);
    scene.dock.set_layout_mode(DocumentLayoutMode::Mdi);
    scene.dock.set_pointer_screen_position(f64::NAN, 22.0);
    scene.dock.set_visible_bounds(11.0, 44.0, 15.0, 700.0);
    scene.first.set_pointer_screen_position(100.0, 200.0);
    scene.first.set_visible_bounds(100.0, 200.0, 800.0, 900.0);
    let moved = Rc::new(RefCell::new(Vec::new()));
    let log = moved.clone();
    scene.factory.dockable_moved(Rc::new(move |args| {
        log.borrow_mut().push(args.dockable().unwrap())
    }));
    let docked = Rc::new(Cell::new(0));
    let log = docked.clone();
    scene
        .factory
        .dockable_docked(Rc::new(move |_| log.set(log.get() + 1)));
    let collapse = Rc::new(RefCell::new(Vec::new()));
    let log = collapse.clone();
    let weak = Rc::downgrade(&scene.factory);
    *scene.factory.on_remove.borrow_mut() = Some(Rc::new(move |item, flag| {
        log.borrow_mut().push(flag);
        weak.upgrade()
            .unwrap()
            .base
            .get()
            .unwrap()
            .remove_dockable(Some(item), flag);
    }));
    scene.factory.float_all_dockables(Some(scene.first.clone()));
    let window = scene.window();
    let target = target(&window);
    assert_eq!(target.title(), "source");
    assert_eq!(target.id(), "documents");
    let document = target.clone().as_document_dock().unwrap();
    assert!(document.can_create_document() && !document.enable_window_drag());
    assert_eq!(document.layout_mode(), DocumentLayoutMode::Tabbed);
    assert_eq!(
        (window.x(), window.y(), window.width(), window.height()),
        (11.0, 22.0, 300.0, 700.0)
    );
    assert_eq!(target.visible_dockables().unwrap().count(), 2);
    assert_eq!(scene.dock.visible_dockables().unwrap().count(), 0);
    assert!(scene
        .root
        .visible_dockables()
        .unwrap()
        .contains(&Some((scene.dock.clone() as Rc<dyn IDockable>).clone())));
    assert!(Rc::ptr_eq(
        &target.active_dockable().unwrap(),
        &(scene.second.clone() as Rc<dyn IDockable>)
    ));
    assert!(Rc::ptr_eq(
        &window.layout().unwrap().focused_dockable().unwrap(),
        &(scene.second as Rc<dyn IDockable>)
    ));
    assert_eq!(*collapse.borrow(), [false, false]);
    assert_eq!(moved.borrow().len(), 2);
    assert!(Rc::ptr_eq(
        &moved.borrow()[0],
        &(scene.first as Rc<dyn IDockable>)
    ));
    assert_eq!(docked.get(), 0);
}

#[test]
fn float_all_reads_collapse_policy_for_each_snapshot_item_even_after_callback_mutation() {
    let scene = Scene::new();
    let dock = scene.dock.clone();
    let weak = Rc::downgrade(&scene.factory);
    let policies = Rc::new(RefCell::new(Vec::new()));
    let log = policies.clone();
    *scene.factory.on_remove.borrow_mut() = Some(Rc::new(move |item, flag| {
        log.borrow_mut().push(flag);
        weak.upgrade()
            .unwrap()
            .base
            .get()
            .unwrap()
            .remove_dockable(Some(item), false);
        dock.set_is_collapsable(true);
    }));
    scene.factory.float_all_dockables(Some(scene.first.clone()));
    assert_eq!(*policies.borrow(), [false, true]);
    assert_eq!(
        target(&scene.window()).visible_dockables().unwrap().count(),
        2
    );
}

#[test]
fn float_all_from_generic_dock_uses_dock_dock_and_missing_geometry_defaults() {
    let factory = factory();
    let root = RootDock::new();
    let source = StackDock::new();
    source.set_is_collapsable(false);
    let item = Tool::new();
    source.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
        item.clone() as Rc<dyn IDockable>
    )])));
    root.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
        source.clone() as Rc<dyn IDockable>
    )])));
    factory.init_dockable(Some(root.clone()), None);
    factory.float_all_dockables(Some(item));
    let window = root
        .windows()
        .unwrap()
        .get(0)
        .expect("nonnull window fixture");
    assert!(target(&window).as_dock_dock().is_some());
    assert_eq!(
        (window.x(), window.y(), window.width(), window.height()),
        (0.0, 0.0, 300.0, 400.0)
    );
}

#[test]
fn dock_as_document_uses_first_document_dock_and_moves_after_its_last_item() {
    let scene = Scene::new();
    let tools = ToolDock::new();
    tools.set_is_collapsable(false);
    let tool = Tool::new();
    tools.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
        tool.clone() as Rc<dyn IDockable>
    )])));
    scene
        .factory
        .add_dockable(Some(scene.root.clone()), Some(tools.clone()));
    scene.factory.dock_as_document(tool.clone());
    let items = scene.dock.visible_dockables().unwrap();
    assert_eq!(items.count(), 3);
    assert!(Rc::ptr_eq(
        (items.get(2)).as_ref().expect("nonnull identity fixture"),
        &(tool.clone() as Rc<dyn IDockable>)
    ));
    assert!(Rc::ptr_eq(
        &scene.dock.active_dockable().unwrap(),
        &(tool as Rc<dyn IDockable>)
    ));
    assert_eq!(tools.visible_dockables().unwrap().count(), 0);
}

#[test]
fn new_document_panes_copy_layout_mode_split_in_requested_direction_and_move_the_item() {
    for horizontal in [true, false] {
        let scene = Scene::new();
        scene.dock.set_id("original".into());
        scene.dock.set_can_create_document(true);
        scene.dock.set_enable_window_drag(false);
        scene.dock.set_layout_mode(DocumentLayoutMode::Mdi);
        if horizontal {
            scene
                .factory
                .new_horizontal_document_dock(scene.first.clone());
        } else {
            scene
                .factory
                .new_vertical_document_dock(scene.first.clone());
        }
        let layout = scene
            .root
            .visible_dockables()
            .unwrap()
            .get(0)
            .and_then(|item| item.as_proportional_dock())
            .unwrap();
        assert_eq!(
            layout.orientation(),
            if horizontal {
                Orientation::Horizontal
            } else {
                Orientation::Vertical
            }
        );
        let target = layout
            .visible_dockables()
            .unwrap()
            .get(2)
            .and_then(|item| item.as_document_dock())
            .unwrap();
        assert_eq!(target.id(), "original");
        assert_eq!(target.layout_mode(), DocumentLayoutMode::Mdi);
        assert!(target.can_create_document() && !target.enable_window_drag());
        assert_eq!(target.visible_dockables().unwrap().count(), 1);
        assert!(Rc::ptr_eq(
            &scene.first.owner().unwrap(),
            &(target.clone() as Rc<dyn IDockable>)
        ));
        assert!(Rc::ptr_eq(
            &target.active_dockable().unwrap(),
            &(scene.first as Rc<dyn IDockable>)
        ));
        assert_eq!(scene.dock.visible_dockables().unwrap().count(), 1);
    }
}

#[test]
fn undocked_event_failure_keeps_source_removal_but_prevents_window_creation() {
    let scene = Scene::new();
    scene
        .factory
        .dockable_undocked(Rc::new(|_| panic!("observer failure")));
    assert!(catch_unwind(AssertUnwindSafe(|| scene
        .factory
        .float_dockable(Some(scene.first.clone()))))
    .is_err());
    assert_eq!(scene.dock.visible_dockables().unwrap().count(), 1);
    assert!(scene.root.windows().is_none());
}

#[test]
fn floating_pinned_tool_unpins_restores_its_owner_and_initializes_its_new_window_state() {
    let layout = crate::factory_pin_tests::Layout::new(Alignment::Left);
    layout.factory.pin_dockable(Some(layout.tool.clone()));
    assert_eq!(layout.tool.docking_state(), DockingWindowState::PINNED);
    let events = Rc::new(RefCell::new(Vec::new()));
    let log = events.clone();
    layout
        .factory
        .dockable_unpinned(Rc::new(move |_| log.borrow_mut().push("unpinned")));
    let log = events.clone();
    layout
        .factory
        .dockable_undocked(Rc::new(move |_| log.borrow_mut().push("undocked")));
    layout.factory.float_dockable(Some(layout.tool.clone()));
    assert_eq!(layout.root.left_pinned_dockables().unwrap().count(), 0);
    let window = layout
        .root
        .windows()
        .unwrap()
        .get(0)
        .expect("nonnull window fixture");
    assert!(Rc::ptr_eq(
        &layout.tool.owner().unwrap(),
        &(target(&window) as Rc<dyn IDockable>)
    ));
    assert_eq!(
        layout.tool.docking_state(),
        DockingWindowState::DOCKED | DockingWindowState::FLOATING
    );
    assert_eq!(*events.borrow(), ["unpinned", "undocked"]);
}

#[test]
fn floating_previewed_tool_uses_the_owner_captured_before_unpinning_for_geometry() {
    let layout = crate::factory_pin_tests::Layout::new(Alignment::Left);
    layout.factory.pin_dockable(Some(layout.tool.clone()));
    layout
        .factory
        .preview_pinned_dockable(Some(layout.tool.clone()));
    let preview = layout.root.pinned_dock().unwrap();
    preview.set_pointer_screen_position(91.0, 82.0);
    preview.set_visible_bounds(0.0, 0.0, 701.0, 602.0);
    layout.dock.set_pointer_screen_position(11.0, 22.0);
    layout.dock.set_visible_bounds(0.0, 0.0, 301.0, 402.0);
    layout.factory.float_dockable(Some(layout.tool.clone()));
    let window = layout
        .root
        .windows()
        .unwrap()
        .get(0)
        .expect("nonnull window fixture");
    assert_eq!(
        (window.x(), window.y(), window.width(), window.height()),
        (91.0, 82.0, 701.0, 602.0)
    );
    assert!(layout.root.pinned_dock().is_none());
    assert_eq!(layout.root.left_pinned_dockables().unwrap().count(), 0);
    assert!(Rc::ptr_eq(
        &layout.tool.owner().unwrap(),
        &(target(&window) as Rc<dyn IDockable>)
    ));
}

#[test]
fn float_all_keeps_its_snapshot_when_removal_callbacks_change_remaining_siblings() {
    let scene = Scene::new();
    let weak = Rc::downgrade(&scene.factory);
    let source = scene.dock.clone();
    let first = scene.first.clone();
    let second = scene.second.clone();
    let appended = Document::new();
    let extra = appended.clone();
    *scene.factory.on_remove.borrow_mut() = Some(Rc::new(move |item, flag| {
        weak.upgrade()
            .unwrap()
            .base
            .get()
            .unwrap()
            .remove_dockable(Some(item.clone()), flag);
        if Rc::ptr_eq(&item, &(first.clone() as Rc<dyn IDockable>)) {
            source
                .visible_dockables()
                .unwrap()
                .remove(&Some((second.clone() as Rc<dyn IDockable>).clone()));
            source.visible_dockables().unwrap().add(Some(extra.clone()));
        }
    }));
    scene.factory.float_all_dockables(Some(scene.first.clone()));
    let target = target(&scene.window());
    let items = target.visible_dockables().unwrap();
    assert_eq!(items.count(), 2);
    assert!(items.contains(&Some((scene.first as Rc<dyn IDockable>).clone())));
    assert!(items.contains(&Some((scene.second as Rc<dyn IDockable>).clone())));
    assert!(!items.contains(&Some((appended.clone() as Rc<dyn IDockable>).clone())));
    assert_eq!(scene.dock.visible_dockables().unwrap().count(), 1);
    assert!(scene
        .dock
        .visible_dockables()
        .unwrap()
        .contains(&Some((appended as Rc<dyn IDockable>).clone())));
}

#[test]
fn float_all_options_present_modally_and_apply_explicit_parent_and_taskbar_visibility() {
    let scene = Scene::new();
    let host = RecordingHost::new();
    scene.factory.supports_host_factory.set(true);
    *scene.factory.host_factory_result.borrow_mut() = Some(host.clone());
    let parent = DockWindow::new();
    let options = DockWindowOptions::new();
    options.set_owner_mode(DockWindowOwnerMode::ParentWindow);
    options.set_parent_window(Some(parent.clone()));
    options.set_is_modal(true);
    options.set_show_in_taskbar(Some(false));
    scene
        .factory
        .float_all_dockables_with_options(Some(scene.first.clone()), Some(options));
    let window = scene.window();
    assert!(window.is_modal());
    assert_eq!(window.show_in_taskbar(), Some(false));
    assert_eq!(window.owner_mode(), DockWindowOwnerMode::ParentWindow);
    assert!(Rc::ptr_eq(
        &window.parent_window().unwrap(),
        &(parent as Rc<dyn IDockWindow>)
    ));
    assert!(host.calls.borrow().contains(&HostCall::Present(true)));
}

#[test]
fn floating_and_hidden_container_state_propagates_to_all_descendants() {
    let scene = Scene::new();
    scene.factory.float_dockable(Some(scene.dock.clone()));
    let state = DockingWindowState::DOCUMENT | DockingWindowState::FLOATING;
    assert_eq!(scene.dock.docking_state(), state);
    assert_eq!(scene.first.docking_state(), state);
    assert_eq!(scene.second.docking_state(), state);
    scene.factory.hide_dockable(Some(scene.dock.clone()));
    assert_eq!(
        scene.dock.docking_state(),
        state | DockingWindowState::HIDDEN
    );
    assert_eq!(
        scene.first.docking_state(),
        state | DockingWindowState::HIDDEN
    );
    assert_eq!(
        scene.second.docking_state(),
        state | DockingWindowState::HIDDEN
    );
    scene.factory.restore_dockable(Some(scene.dock.clone()));
    assert_eq!(scene.dock.docking_state(), state);
    assert_eq!(scene.first.docking_state(), state);
    assert_eq!(scene.second.docking_state(), state);
}
