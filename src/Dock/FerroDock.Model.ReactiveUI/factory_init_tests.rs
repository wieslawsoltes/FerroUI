use crate::controls::*;
use crate::core::DockWindow;
use crate::factory_parameter_tests::RecordingFactory;
use crate::test_host::{HostCall, RecordingHost};
use ferrodock_model::collections::ListHandle;
use ferrodock_model::controls::*;
use ferrodock_model::core::*;
use ferrodock_model::BoxedValue;
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

pub(crate) fn factory() -> Rc<RecordingFactory> {
    let factory = RecordingFactory::with_base();
    factory.execute_base.set(true);
    *factory.tool_dock_creator.borrow_mut() = Some(Rc::new(|| ToolDock::new()));
    *factory.root_dock_creator.borrow_mut() = Some(Rc::new(|| RootDock::new()));
    *factory.proportional_dock_creator.borrow_mut() = Some(Rc::new(|| ProportionalDock::new()));
    *factory.dock_dock_creator.borrow_mut() = Some(Rc::new(|| DockDock::new()));
    *factory.document_dock_creator.borrow_mut() = Some(Rc::new(|| DocumentDock::new()));
    *factory.splitter_creator.borrow_mut() = Some(Rc::new(|| ProportionalDockSplitter::new()));
    *factory.window_creator.borrow_mut() = Some(Rc::new(|| DockWindow::new()));
    factory.supports_state_sync.set(true);
    factory
}

#[test]
fn recursive_initialization_normalizes_visible_lists_and_preserves_non_null_context() {
    let factory = factory();
    factory.create_plain_lists.set(true);
    let root = RootDock::new();
    root.set_id("root".into());
    let dock = ToolDock::new();
    let tool = Tool::new();
    tool.set_id("tool".into());
    let existing: BoxedValue = Rc::new("existing".to_owned());
    dock.set_context(Some(existing.clone()));
    let original = ListHandle::from_plain(vec![Some(tool.clone() as Rc<dyn IDockable>)]);
    dock.set_visible_dockables(Some(original.clone()));
    root.set_visible_dockables(Some(ListHandle::from_plain(vec![Some(dock.clone())])));
    let lookups = Rc::new(Cell::new(0));
    let calls = lookups.clone();
    factory.set_default_context_locator(Some(Rc::new(move || {
        calls.set(calls.get() + 1);
        Some(Rc::new(123_i32))
    })));
    factory.init_dockable(Some(root.clone()), None);
    assert_eq!(factory.create_list_calls.get(), 2);
    assert!(root
        .visible_dockables()
        .unwrap()
        .as_notify_collection_changed()
        .is_some());
    assert!(dock
        .visible_dockables()
        .unwrap()
        .as_notify_collection_changed()
        .is_some());
    assert!(original.as_notify_collection_changed().is_none());
    assert!(Rc::ptr_eq(&dock.context().unwrap(), &existing));
    assert_eq!(tool.context().unwrap().downcast_ref::<i32>(), Some(&123));
    assert_eq!(lookups.get(), 2);
    assert!(Rc::ptr_eq(
        &tool.owner().unwrap(),
        &(dock.clone() as Rc<dyn IDockable>)
    ));
    assert!(Rc::ptr_eq(
        &dock.owner().unwrap(),
        &(root.clone() as Rc<dyn IDockable>)
    ));
    assert!(Rc::ptr_eq(
        &tool.factory().unwrap(),
        &(factory.clone() as Rc<dyn IFactory>)
    ));
    let normalized = dock.visible_dockables().unwrap();
    factory.init_dockable(Some(root.clone()), None);
    assert!(normalized == dock.visible_dockables().unwrap());
    assert_eq!(factory.create_list_calls.get(), 2);
}

#[test]
fn initialization_events_run_children_first_with_mutable_shared_context() {
    let factory = factory();
    let root = RootDock::new();
    root.set_id("root".into());
    let dock = ToolDock::new();
    dock.set_id("dock".into());
    let tool = Tool::new();
    tool.set_id("tool".into());
    root.set_visible_dockables(Some(ListHandle::from_plain(vec![Some(dock.clone())])));
    dock.set_visible_dockables(Some(ListHandle::from_plain(vec![Some(tool.clone())])));
    let calls = Rc::new(RefCell::new(Vec::new()));
    let log = calls.clone();
    factory.dockable_init(Rc::new(move |args| {
        let item = args.dockable().unwrap();
        assert!(item.factory().is_some());
        log.borrow_mut().push(item.id());
        args.set_context(Some(Rc::new(item.id())));
    }));
    factory.init_dockable(Some(root.clone()), None);
    assert_eq!(*calls.borrow(), ["tool", "dock", "root"]);
    assert_eq!(
        tool.context().unwrap().downcast_ref::<String>().unwrap(),
        "tool"
    );
    assert_eq!(
        root.context().unwrap().downcast_ref::<String>().unwrap(),
        "root"
    );
}

#[test]
fn root_initializes_hidden_pinned_preview_and_window_layouts_without_original_owners() {
    let factory = factory();
    let root = RootDock::new();
    let hidden = Tool::new();
    let pinned: Vec<Rc<dyn IDockable>> = (0..4).map(|_| Tool::new() as Rc<dyn IDockable>).collect();
    root.set_hidden_dockables(Some(ListHandle::from_plain(vec![Some(hidden.clone())])));
    root.set_left_pinned_dockables(Some(ListHandle::from_plain(vec![Some(pinned[0].clone())])));
    root.set_right_pinned_dockables(Some(ListHandle::from_plain(vec![Some(pinned[1].clone())])));
    root.set_top_pinned_dockables(Some(ListHandle::from_plain(vec![Some(pinned[2].clone())])));
    root.set_bottom_pinned_dockables(Some(ListHandle::from_plain(vec![Some(pinned[3].clone())])));
    let preview = ToolDock::new();
    root.set_pinned_dock(Some(preview.clone()));
    let window = DockWindow::new();
    let layout = RootDock::new();
    let original = ToolDock::new();
    layout.set_owner(Some(original.clone()));
    window.set_layout(Some(layout.clone()));
    root.set_windows(Some(ListHandle::from_plain(vec![Some(window.clone())])));
    factory.init_dockable(Some(root.clone()), None);
    for item in pinned {
        assert!(Rc::ptr_eq(
            &item.owner().unwrap(),
            &(root.clone() as Rc<dyn IDockable>)
        ));
        assert!(item.original_owner().is_none());
        assert!(item.as_docking_window_state().unwrap().is_open());
        assert_eq!(item.docking_state(), DockingWindowState::PINNED);
    }
    assert!(hidden.docking_state().contains(DockingWindowState::HIDDEN));
    assert!(!hidden.is_open());
    assert!(Rc::ptr_eq(
        &preview.owner().unwrap(),
        &(root.clone() as Rc<dyn IDockable>)
    ));
    assert!(Rc::ptr_eq(
        &window.owner().unwrap(),
        &(root as Rc<dyn IDockable>)
    ));
    assert!(Rc::ptr_eq(
        &layout.owner().unwrap(),
        &(original as Rc<dyn IDockable>)
    ));
    assert!(layout.factory().is_some());
}

#[test]
fn split_view_initialization_avoids_duplicate_pane_and_content_extras() {
    let factory = factory();
    let split = SplitViewDock::new();
    split.set_id("split".into());
    let pane = Tool::new();
    pane.set_id("pane".into());
    let content = Tool::new();
    content.set_id("content".into());
    split.set_visible_dockables(Some(ListHandle::from_plain(vec![
        Some(pane.clone()),
        Some(pane.clone()),
    ])));
    split.set_pane_dockable(Some(pane.clone()));
    split.set_content_dockable(Some(content.clone()));
    let calls = Rc::new(RefCell::new(Vec::new()));
    let log = calls.clone();
    factory.dockable_init(Rc::new(move |args| {
        log.borrow_mut().push(args.dockable().unwrap().id())
    }));
    factory.init_dockable(Some(split.clone()), None);
    assert_eq!(*calls.borrow(), ["pane", "pane", "content", "split"]);
    assert!(Rc::ptr_eq(
        &content.owner().unwrap(),
        &(split.clone() as Rc<dyn IDockable>)
    ));
    calls.borrow_mut().clear();
    split.set_visible_dockables(None);
    split.set_pane_dockable(Some(content));
    factory.init_dockable(Some(split), None);
    assert_eq!(*calls.borrow(), ["content", "split"]);
}

#[test]
fn layout_uses_default_selection_then_initializes_tracking_without_changing_restored_focus() {
    let factory = factory();
    let root = RootDock::new();
    let dock = DocumentDock::new();
    let document = Document::new();
    dock.set_visible_dockables(Some(ListHandle::from_plain(vec![Some(document.clone())])));
    dock.set_owner(Some(root.clone()));
    dock.set_active_dockable(Some(document.clone()));
    document.set_owner(Some(dock.clone()));
    root.set_visible_dockables(Some(ListHandle::from_plain(vec![Some(dock.clone())])));
    root.set_active_dockable(Some(dock.clone()));
    root.set_default_dockable(Some(dock));
    root.set_focused_dockable(Some(document.clone()));
    let window = DockWindow::new();
    window.set_layout(Some(root.clone()));
    root.set_window(Some(window.clone()));
    let reasons = Rc::new(RefCell::new(Vec::new()));
    let log = reasons.clone();
    factory.global_dock_tracking_changed(Rc::new(move |args| log.borrow_mut().push(args.reason())));
    factory.init_layout(root.clone());
    assert_eq!(
        *reasons.borrow(),
        [DockTrackingChangeReason::LayoutInitialized]
    );
    assert!(Rc::ptr_eq(
        &factory.current_dockable().unwrap(),
        &(document as Rc<dyn IDockable>)
    ));
    assert!(Rc::ptr_eq(
        &factory.current_root_dock().unwrap(),
        &(root as Rc<dyn IRootDock>)
    ));
    assert!(Rc::ptr_eq(
        &factory.current_dock_window().unwrap(),
        &(window as Rc<dyn IDockWindow>)
    ));
}

#[test]
fn layout_picks_first_direct_non_splitter_and_focuses_its_owner() {
    let factory = factory();
    let root = RootDock::new();
    let splitter = ProportionalDockSplitter::new();
    let dock = ToolDock::new();
    root.set_visible_dockables(Some(ListHandle::from_plain(vec![
        Some(splitter),
        Some(dock.clone()),
    ])));
    factory.init_layout(root.clone());
    assert!(Rc::ptr_eq(
        &root.active_dockable().unwrap(),
        &(dock.clone() as Rc<dyn IDockable>)
    ));
    assert!(Rc::ptr_eq(
        &root.focused_dockable().unwrap(),
        &(dock as Rc<dyn IDockable>)
    ));
    assert!(root.is_active());
}

#[test]
fn empty_and_non_collapsable_docks_follow_source_empty_and_open_count_rules() {
    let factory = factory();
    let root = RootDock::new();
    let proportional = ProportionalDock::new();
    let empty = ToolDock::new();
    let retained = DocumentDock::new();
    retained.set_is_collapsable(false);
    proportional.set_visible_dockables(Some(ListHandle::from_plain(vec![
        Some(empty.clone()),
        Some(retained.clone()),
    ])));
    root.set_visible_dockables(Some(ListHandle::from_plain(vec![Some(
        proportional.clone(),
    )])));
    factory.init_dockable(Some(root.clone()), None);
    assert!(empty.is_empty() && retained.is_empty());
    assert!(!proportional.is_empty() && !root.is_empty());
    assert_eq!(
        (
            empty.opened_dockables_count(),
            retained.opened_dockables_count(),
            proportional.opened_dockables_count(),
            root.opened_dockables_count()
        ),
        (1, 1, 2, 2)
    );
    retained.set_is_collapsable(true);
    factory.init_dockable(Some(root.clone()), None);
    assert!(proportional.is_empty() && root.is_empty());
}

#[test]
fn split_empty_state_includes_unlisted_pane_and_content_before_recursing() {
    let factory = factory();
    let split = SplitViewDock::new();
    split.set_pane_dockable(Some(ProportionalDockSplitter::new()));
    split.set_content_dockable(Some(Tool::new()));
    factory.init_dockable(Some(split.clone()), None);
    assert!(!split.is_empty());
    split.set_content_dockable(None);
    factory.init_dockable(Some(split.clone()), None);
    assert!(split.is_empty());
}

#[test]
fn layout_registers_default_host_locator_once_and_presents_floating_windows() {
    let factory = factory();
    let root = RootDock::new();
    let window = DockWindow::new();
    window.set_layout(Some(RootDock::new()));
    root.set_windows(Some(ListHandle::from_plain(vec![Some(window.clone())])));
    let host = RecordingHost::new();
    let result = host.clone();
    factory.set_default_host_window_locator(Some(Rc::new(move || Some(result.clone()))));
    factory.init_layout(root.clone());
    assert!(host.tracked.get());
    assert!(host
        .calls
        .borrow()
        .iter()
        .any(|call| matches!(call, HostCall::Present(false))));
    let registry = factory.host_window_locator().unwrap();
    factory.set_default_host_window_locator(Some(Rc::new(|| None)));
    factory.init_layout(root);
    assert!(Rc::ptr_eq(
        &registry,
        &factory.host_window_locator().unwrap()
    ));
    assert!(factory.get_host_window("IDockWindow".into()).is_some());
}

#[test]
fn hidden_plain_list_mutation_from_init_event_fails_fast_and_subscription_disposes() {
    let factory = factory();
    let root = RootDock::new();
    let hidden = ListHandle::from_plain(vec![Some(Tool::new() as Rc<dyn IDockable>)]);
    root.set_hidden_dockables(Some(hidden.clone()));
    let changed = Cell::new(false);
    let subscription = factory.dockable_init(Rc::new(move |args| {
        if args.dockable().unwrap().clone().as_tool().is_some() && !changed.replace(true) {
            hidden.add(Some(Tool::new()));
        }
    }));
    assert!(catch_unwind(AssertUnwindSafe(
        || factory.init_dockable(Some(root.clone()), None)
    ))
    .is_err());
    subscription.dispose();
    subscription.dispose();
    factory.init_dockable(Some(root.clone()), None);
    assert_eq!(root.hidden_dockables().unwrap().count(), 2);
}
