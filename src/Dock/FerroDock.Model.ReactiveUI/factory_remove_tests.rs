use crate::controls::*;
use crate::core::DockWindow;
use crate::factory_init_tests::factory;
use crate::factory_pin_tests::Layout;
use crate::test_host::RecordingHost;
use crate::test_support::TestControl;
use ferrodock_model::collections::ListHandle;
use ferrodock_model::core::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[test]
fn removal_reconciles_focus_and_default_selection_without_selecting_past_a_splitter() {
    let layout = Layout::new(Alignment::Left);
    let a = Tool::new();
    let splitter = ProportionalDockSplitter::new();
    layout
        .dock
        .set_visible_dockables(Some(ListHandle::from_plain(vec![
            Some(a.clone()),
            Some(splitter),
            Some(layout.tool.clone()),
        ])));
    layout
        .factory
        .init_dockable(Some(layout.root.clone()), None);
    layout.dock.set_active_dockable(Some(layout.tool.clone()));
    assert!(layout.root.focused_dockable().is_some());
    layout
        .factory
        .remove_dockable(Some(layout.tool.clone()), false);
    assert_eq!(layout.dock.visible_dockables().unwrap().count(), 1);
    assert!(layout.dock.active_dockable().is_none() && layout.root.focused_dockable().is_none());
    assert!(layout.tool.owner().is_some());
}

#[test]
fn removal_dispatches_custom_replacement_policy_after_removed_event() {
    let layout = Layout::new(Alignment::Left);
    let next = Tool::new();
    layout
        .factory
        .add_dockable(Some(layout.dock.clone()), Some(next.clone()));
    layout.dock.set_active_dockable(Some(layout.tool.clone()));
    let removed = Rc::new(Cell::new(false));
    let called = removed.clone();
    layout
        .factory
        .dockable_removed(Rc::new(move |_| called.set(true)));
    let called = removed.clone();
    let selected = next.clone();
    *layout.factory.on_selection_after_remove.borrow_mut() =
        Some(Rc::new(move |dock, item, index| {
            assert!(called.get());
            assert_eq!(index, 0);
            assert!(!dock
                .visible_dockables()
                .unwrap()
                .contains(&Some((item).clone())));
            Some(selected.clone())
        }));
    layout
        .factory
        .remove_dockable(Some(layout.tool.clone()), false);
    assert!(Rc::ptr_eq(
        &layout.dock.active_dockable().unwrap(),
        &(next as Rc<dyn IDockable>)
    ));
}

#[test]
fn hidden_removal_returns_before_collapse_and_resets_matching_root_focus() {
    let layout = Layout::new(Alignment::Left);
    layout.factory.hide_dockable(Some(layout.tool.clone()));
    layout.root.set_active_dockable(Some(layout.dock.clone()));
    layout.root.set_focused_dockable(Some(layout.tool.clone()));
    layout
        .factory
        .remove_dockable(Some(layout.tool.clone()), true);
    assert_eq!(layout.root.hidden_dockables().unwrap().count(), 0);
    assert!(layout
        .root
        .visible_dockables()
        .unwrap()
        .contains(&Some((layout.dock.clone() as Rc<dyn IDockable>).clone())));
    assert!(Rc::ptr_eq(
        &layout.root.focused_dockable().unwrap(),
        &(layout.dock as Rc<dyn IDockable>)
    ));
}

#[test]
fn collapsing_empty_child_prefers_previous_proportional_splitter_and_preserves_next() {
    let factory = factory();
    let root = RootDock::new();
    let parent = ProportionalDock::new();
    let empty = ToolDock::new();
    let previous = ProportionalDockSplitter::new();
    previous.set_id("previous".into());
    let next = ProportionalDockSplitter::new();
    next.set_id("next".into());
    empty.set_id("empty".into());
    parent.set_visible_dockables(Some(ListHandle::from_plain(vec![
        Some(Tool::new() as Rc<dyn IDockable>),
        Some(previous),
        Some(empty.clone()),
        Some(next.clone()),
        Some(Tool::new()),
    ])));
    root.set_visible_dockables(Some(ListHandle::from_plain(vec![Some(parent.clone())])));
    factory.init_dockable(Some(root.clone()), None);
    let removed = Rc::new(RefCell::new(Vec::new()));
    let log = removed.clone();
    factory.dockable_removed(Rc::new(move |args| {
        log.borrow_mut().push(args.dockable().unwrap().id())
    }));
    factory.collapse_dock(empty);
    assert_eq!(*removed.borrow(), ["previous", "empty"]);
    assert!(parent
        .visible_dockables()
        .unwrap()
        .contains(&Some((next as Rc<dyn IDockable>).clone())));
}

#[test]
fn proportional_cleanup_preserves_collapsed_parent_proportion_and_stops_at_workspace_root() {
    let factory = factory();
    let root = RootDock::new();
    let outer = ProportionalDock::new();
    let inner = ProportionalDock::new();
    let empty = ToolDock::new();
    let survivor = ToolDock::new();
    inner.set_proportion(0.37);
    survivor.set_proportion(0.63);
    survivor.set_visible_dockables(Some(ListHandle::from_plain(vec![Some(Tool::new())])));
    inner.set_visible_dockables(Some(ListHandle::from_plain(vec![
        Some(empty.clone()),
        Some(ProportionalDockSplitter::new()),
        Some(survivor.clone()),
    ])));
    outer.set_visible_dockables(Some(ListHandle::from_plain(vec![Some(inner.clone())])));
    root.set_visible_dockables(Some(ListHandle::from_plain(vec![Some(outer.clone())])));
    factory.init_dockable(Some(root.clone()), None);
    factory.collapse_dock(empty);
    assert!(Rc::ptr_eq(
        (outer.visible_dockables().unwrap().get(0))
            .as_ref()
            .expect("nonnull identity fixture"),
        &(survivor.clone() as Rc<dyn IDockable>)
    ));
    assert_eq!(
        (survivor.proportion(), survivor.collapsed_proportion()),
        (0.37, 0.37)
    );
    assert!(Rc::ptr_eq(
        (root.visible_dockables().unwrap().get(0))
            .as_ref()
            .expect("nonnull identity fixture"),
        &(outer as Rc<dyn IDockable>)
    ));
}

#[test]
fn any_nonempty_aligned_pin_strip_protects_an_empty_tool_dock_from_collapse() {
    let layout = Layout::new(Alignment::Left);
    layout.dock.set_visible_dockables(Some(ListHandle::new()));
    let unrelated = Tool::new();
    layout
        .root
        .set_left_pinned_dockables(Some(ListHandle::from_plain(vec![Some(unrelated)])));
    layout.factory.collapse_dock(layout.dock.clone());
    assert!(layout
        .root
        .visible_dockables()
        .unwrap()
        .contains(&Some((layout.dock as Rc<dyn IDockable>).clone())));
}

#[test]
fn floating_root_collapse_honors_host_cancellation_then_removes_window_on_success() {
    let factory = factory();
    let workspace = RootDock::new();
    let root = RootDock::new();
    let window = DockWindow::new();
    window.set_layout(Some(root.clone()));
    root.set_window(Some(window.clone()));
    factory.add_window(Some(workspace.clone()), Some(window.clone()));
    let host = RecordingHost::new();
    host.closed.set(Some(false));
    host.tracked.set(true);
    window.set_host(Some(host.clone()));
    factory.collapse_dock(root.clone());
    assert!(workspace
        .windows()
        .unwrap()
        .contains(&Some(window.clone() as Rc<dyn IDockWindow>)));
    assert!(root.window().is_some());
    host.closed.set(Some(true));
    factory.collapse_dock(root.clone());
    assert_eq!(workspace.windows().unwrap().count(), 0);
    assert!(root.window().is_none() && window.layout().is_none());
}

#[test]
fn cross_owner_move_uses_real_removal_and_collapse_before_or_after_target_commit_as_required() {
    let layout = Layout::new(Alignment::Left);
    let target = DocumentDock::new();
    layout
        .root
        .visible_dockables()
        .unwrap()
        .add(Some(target.clone()));
    layout
        .factory
        .init_dockable(Some(target.clone()), Some(layout.root.clone()));
    layout.factory.move_dockable_between_docks(
        Some(layout.dock.clone()),
        Some(target.clone()),
        Some(layout.tool.clone()),
        None,
    );
    assert!(!layout
        .root
        .visible_dockables()
        .unwrap()
        .contains(&Some((layout.dock.clone() as Rc<dyn IDockable>).clone())));
    assert!(target
        .visible_dockables()
        .unwrap()
        .contains(&Some((layout.tool.clone() as Rc<dyn IDockable>).clone())));
    assert_eq!(layout.tool.docking_state(), DockingWindowState::DOCUMENT);
}

#[test]
fn hide_restore_preserves_owner_history_and_restores_hidden_descendant_states() {
    let factory = factory();
    let root = RootDock::new();
    let dock = DocumentDock::new();
    let document = Document::new();
    dock.set_visible_dockables(Some(ListHandle::from_plain(vec![Some(document.clone())])));
    root.set_visible_dockables(Some(ListHandle::from_plain(vec![Some(dock.clone())])));
    factory.init_dockable(Some(root.clone()), None);
    factory.hide_dockable(Some(dock.clone()));
    assert_eq!(
        document.docking_state(),
        DockingWindowState::DOCUMENT | DockingWindowState::HIDDEN
    );
    factory.restore_dockable(Some(dock.clone()));
    assert_eq!(document.docking_state(), DockingWindowState::DOCUMENT);
    assert!(dock.original_owner().is_none());
    assert!(root
        .visible_dockables()
        .unwrap()
        .contains(&Some((dock as Rc<dyn IDockable>).clone())));
}

#[test]
fn hide_restore_id_wrappers_use_registered_visible_roots_and_restoration_can_clear_owner() {
    let layout = Layout::new(Alignment::Bottom);
    layout
        .factory
        .controls
        .add(TestControl::new(Some(layout.root.clone())));
    layout.factory.hide_dockable_by_id("tool".into());
    assert!(layout
        .root
        .hidden_dockables()
        .unwrap()
        .contains(&Some((layout.tool.clone() as Rc<dyn IDockable>).clone())));
    assert!(Rc::ptr_eq(
        &layout
            .factory
            .restore_dockable_by_id("tool".into())
            .unwrap(),
        &(layout.tool.clone() as Rc<dyn IDockable>)
    ));
    layout.factory.hide_dockable(Some(layout.tool.clone()));
    layout.tool.set_original_owner(None);
    layout.factory.restore_dockable(Some(layout.tool.clone()));
    assert!(layout.tool.owner().is_none());
    assert!(layout
        .factory
        .restore_dockable_by_id("absent".into())
        .is_none());
}

#[test]
fn grid_splitters_do_not_match_the_proportional_neighbor_type_test() {
    let factory = factory();
    let root = RootDock::new();
    let parent = ProportionalDock::new();
    let empty = ToolDock::new();
    let grid = GridDockSplitter::new();
    let proportional = ProportionalDockSplitter::new();
    parent.set_visible_dockables(Some(ListHandle::from_plain(vec![
        Some(Tool::new() as Rc<dyn IDockable>),
        Some(grid.clone()),
        Some(empty.clone()),
        Some(proportional),
        Some(Tool::new()),
    ])));
    root.set_visible_dockables(Some(ListHandle::from_plain(vec![Some(parent.clone())])));
    factory.init_dockable(Some(root.clone()), None);
    factory.collapse_dock(empty);
    assert!(parent
        .visible_dockables()
        .unwrap()
        .contains(&Some((grid as Rc<dyn IDockable>).clone())));
    assert_eq!(parent.visible_dockables().unwrap().count(), 3);
}

#[test]
fn missing_visible_item_only_reconciles_matching_focus_without_removal_or_splitter_cleanup() {
    let layout = Layout::new(Alignment::Left);
    let missing = Tool::new();
    missing.set_owner(Some(layout.dock.clone()));
    layout
        .dock
        .visible_dockables()
        .unwrap()
        .insert(0, Some(ProportionalDockSplitter::new()));
    layout.dock.set_active_dockable(Some(layout.tool.clone()));
    layout.root.set_focused_dockable(Some(missing.clone()));
    let count = Rc::new(Cell::new(0));
    let calls = count.clone();
    layout
        .factory
        .dockable_removed(Rc::new(move |_| calls.set(calls.get() + 1)));
    layout.factory.remove_dockable(Some(missing), true);
    assert_eq!(count.get(), 0);
    assert_eq!(layout.dock.visible_dockables().unwrap().count(), 2);
    assert!(Rc::ptr_eq(
        &layout.root.focused_dockable().unwrap(),
        &(layout.tool as Rc<dyn IDockable>)
    ));
}

#[test]
fn restoration_event_and_added_event_precede_final_owner_history_updates() {
    let layout = Layout::new(Alignment::Right);
    let other = Tool::new();
    layout
        .factory
        .add_dockable(Some(layout.dock.clone()), Some(other.clone()));
    layout.dock.set_active_dockable(Some(layout.tool.clone()));
    layout.factory.hide_dockable(Some(layout.tool.clone()));
    assert!(Rc::ptr_eq(
        &layout.dock.active_dockable().unwrap(),
        &(other.clone() as Rc<dyn IDockable>)
    ));
    let events = Rc::new(RefCell::new(Vec::new()));
    let log = events.clone();
    let root = layout.root.clone();
    layout.factory.dockable_restored(Rc::new(move |args| {
        assert!(Rc::ptr_eq(
            &args.dockable().unwrap().owner().unwrap(),
            &(root.clone() as Rc<dyn IDockable>)
        ));
        log.borrow_mut().push("restored");
    }));
    let log = events.clone();
    let root = layout.root.clone();
    layout.factory.dockable_added(Rc::new(move |args| {
        assert!(Rc::ptr_eq(
            &args.dockable().unwrap().owner().unwrap(),
            &(root.clone() as Rc<dyn IDockable>)
        ));
        log.borrow_mut().push("added");
    }));
    layout.factory.restore_dockable(Some(layout.tool.clone()));
    assert_eq!(*events.borrow(), ["restored", "added"]);
    assert!(layout.tool.original_owner().is_none());
    assert!(Rc::ptr_eq(
        &layout.dock.active_dockable().unwrap(),
        &(other as Rc<dyn IDockable>)
    ));
}

#[test]
fn floating_root_move_closes_its_host_only_after_target_docking_events() {
    let factory = factory();
    let workspace = RootDock::new();
    let target = DocumentDock::new();
    workspace.set_visible_dockables(Some(ListHandle::from_plain(vec![Some(target.clone())])));
    factory.init_dockable(Some(workspace.clone()), None);
    let floating = RootDock::new();
    let tool = Tool::new();
    floating.set_visible_dockables(Some(ListHandle::from_plain(vec![Some(tool.clone())])));
    let window = DockWindow::new();
    window.set_layout(Some(floating.clone()));
    floating.set_window(Some(window.clone()));
    factory.add_window(Some(workspace.clone()), Some(window.clone()));
    let host = RecordingHost::new();
    host.tracked.set(true);
    window.set_host(Some(host.clone()));
    let source_host = host.clone();
    factory.dockable_docked(Rc::new(move |_| assert!(source_host.tracked.get())));
    factory.move_dockable_between_docks(
        Some(floating.clone()),
        Some(target.clone()),
        Some(tool.clone()),
        None,
    );
    assert!(window.owner().is_none() && window.layout().is_none() && window.host().is_none());
    assert!(floating.window().is_none());
    assert_eq!(workspace.windows().unwrap().count(), 0);
    assert!(Rc::ptr_eq(
        &tool.owner().unwrap(),
        &(target as Rc<dyn IDockable>)
    ));
}

#[test]
fn split_view_collapse_checks_unlisted_content_instead_of_only_the_visible_list() {
    let factory = factory();
    let root = RootDock::new();
    let split = SplitViewDock::new();
    split.set_content_dockable(Some(Tool::new()));
    root.set_visible_dockables(Some(ListHandle::from_plain(vec![Some(split.clone())])));
    factory.init_dockable(Some(root.clone()), None);
    factory.collapse_dock(split.clone());
    assert_eq!(root.visible_dockables().unwrap().count(), 1);
    split.set_content_dockable(None);
    factory.collapse_dock(split);
    assert_eq!(root.visible_dockables().unwrap().count(), 0);
}
