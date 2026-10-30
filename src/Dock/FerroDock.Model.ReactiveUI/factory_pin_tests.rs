use crate::controls::*;
use crate::core::DockWindow;
use crate::factory_init_tests::factory;
use crate::factory_parameter_tests::RecordingFactory;
use ferrodock_model::collections::ListHandle;
use ferrodock_model::core::*;
use ferrodock_model::FactoryBaseImpl;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub(crate) struct Layout {
    pub(crate) factory: Rc<RecordingFactory>,
    pub(crate) root: Rc<RootDock>,
    pub(crate) dock: Rc<ToolDock>,
    pub(crate) tool: Rc<Tool>,
}
impl Layout {
    pub(crate) fn new(alignment: Alignment) -> Self {
        let factory = factory();
        let root = RootDock::new();
        let dock = ToolDock::new();
        let tool = Tool::new();
        dock.set_alignment(alignment);
        tool.set_id("tool".into());
        dock.set_visible_dockables(Some(ListHandle::from_plain(vec![Some(tool.clone())])));
        root.set_visible_dockables(Some(ListHandle::from_plain(vec![Some(dock.clone())])));
        factory.init_dockable(Some(root.clone()), None);
        Self {
            factory,
            root,
            dock,
            tool,
        }
    }
}
fn strip(root: &RootDock, alignment: Alignment) -> ListHandle<Option<Rc<dyn IDockable>>> {
    match alignment {
        Alignment::Left | Alignment::Unset => root.left_pinned_dockables(),
        Alignment::Right => root.right_pinned_dockables(),
        Alignment::Top => root.top_pinned_dockables(),
        Alignment::Bottom => root.bottom_pinned_dockables(),
    }
    .unwrap()
}

#[test]
fn pin_and_unpin_preserve_alignment_ownership_and_notification_order() {
    for alignment in [
        Alignment::Unset,
        Alignment::Left,
        Alignment::Right,
        Alignment::Top,
        Alignment::Bottom,
    ] {
        let layout = Layout::new(alignment);
        let events = Rc::new(RefCell::new(Vec::new()));
        let log = events.clone();
        layout
            .factory
            .dockable_removed(Rc::new(move |_| log.borrow_mut().push("removed")));
        let log = events.clone();
        layout
            .factory
            .dockable_pinned(Rc::new(move |_| log.borrow_mut().push("pinned")));
        let log = events.clone();
        layout
            .factory
            .dockable_unpinned(Rc::new(move |_| log.borrow_mut().push("unpinned")));
        let log = events.clone();
        layout
            .factory
            .dockable_added(Rc::new(move |_| log.borrow_mut().push("added")));
        layout.factory.pin_dockable(Some(layout.tool.clone()));
        assert!(strip(&layout.root, alignment)
            .contains(&Some((layout.tool.clone() as Rc<dyn IDockable>).clone())));
        assert_eq!(layout.dock.visible_dockables().unwrap().count(), 0);
        assert!(Rc::ptr_eq(
            &layout.tool.owner().unwrap(),
            &(layout.dock.clone() as Rc<dyn IDockable>)
        ));
        assert_eq!(layout.tool.docking_state(), DockingWindowState::PINNED);
        if alignment == Alignment::Unset {
            layout.dock.set_alignment(Alignment::Left);
        }
        layout.factory.unpin_dockable(Some(layout.tool.clone()));
        assert_eq!(strip(&layout.root, alignment).count(), 0);
        assert!(layout
            .dock
            .visible_dockables()
            .unwrap()
            .contains(&Some((layout.tool.clone() as Rc<dyn IDockable>).clone())));
        assert!(layout.tool.original_owner().is_none());
        assert_eq!(layout.tool.docking_state(), DockingWindowState::DOCKED);
        assert_eq!(*events.borrow(), ["removed", "pinned", "unpinned", "added"]);
    }
}

#[test]
fn pin_bounds_use_the_complete_owner_pair_when_either_item_dimension_is_invalid() {
    for (width, height, owner_width, owner_height, expected) in [
        (12.0, 13.0, 100.0, 200.0, (0.0, 0.0, 12.0, 13.0)),
        (12.0, f64::NAN, 100.0, 200.0, (0.0, 0.0, 100.0, 200.0)),
        (f64::INFINITY, 13.0, 100.0, 200.0, (0.0, 0.0, 100.0, 200.0)),
        (-1.0, 13.0, 0.0, 200.0, (7.0, 8.0, 9.0, 10.0)),
        (0.001, 0.002, 100.0, 200.0, (0.0, 0.0, 0.001, 0.002)),
    ] {
        let layout = Layout::new(Alignment::Left);
        layout.tool.set_pinned_bounds(7.0, 8.0, 9.0, 10.0);
        layout.tool.set_visible_bounds(1.0, 2.0, width, height);
        layout
            .dock
            .set_visible_bounds(3.0, 4.0, owner_width, owner_height);
        layout.factory.pin_dockable(Some(layout.tool.clone()));
        assert_eq!(layout.tool.get_pinned_bounds(), expected);
    }
}

#[test]
fn public_pin_obeys_capability_but_remove_and_hide_use_core_cleanup() {
    for (hide, preview) in [(false, false), (false, true), (true, false), (true, true)] {
        let layout = Layout::new(Alignment::Left);
        layout.tool.set_can_pin(false);
        layout.factory.pin_dockable(Some(layout.tool.clone()));
        assert!(layout.root.left_pinned_dockables().is_none());
        FactoryBaseImpl::pin_dockable_core(layout.factory.as_ref(), Some(layout.tool.clone()));
        assert_eq!(layout.root.left_pinned_dockables().unwrap().count(), 1);
        if preview {
            layout
                .factory
                .preview_pinned_dockable(Some(layout.tool.clone()));
        }
        if hide {
            layout.factory.hide_dockable(Some(layout.tool.clone()));
        } else {
            layout
                .factory
                .remove_dockable(Some(layout.tool.clone()), false);
        }
        assert_eq!(layout.root.left_pinned_dockables().unwrap().count(), 0);
        assert_eq!(layout.dock.visible_dockables().unwrap().count(), 0);
        assert!(layout.root.pinned_dock().is_none());
        if hide {
            assert!(layout
                .root
                .hidden_dockables()
                .unwrap()
                .contains(&Some((layout.tool.clone() as Rc<dyn IDockable>).clone())));
        }
    }
}

#[test]
fn preview_keep_visible_is_respected_by_hide_but_not_explicit_switch_or_toggle() {
    let layout = Layout::new(Alignment::Right);
    let second = Tool::new();
    second.set_id("second".into());
    layout
        .factory
        .add_dockable(Some(layout.dock.clone()), Some(second.clone()));
    layout.factory.pin_dockable(Some(layout.tool.clone()));
    layout.factory.pin_dockable(Some(second.clone()));
    layout
        .factory
        .preview_pinned_dockable(Some(layout.tool.clone()));
    let preview = layout.root.pinned_dock().unwrap();
    assert_eq!(preview.alignment(), Alignment::Right);
    assert!(!preview.can_drop());
    assert!(Rc::ptr_eq(
        &layout.tool.owner().unwrap(),
        &(preview.clone() as Rc<dyn IDockable>)
    ));
    layout.tool.set_keep_pinned_dockable_visible(true);
    layout
        .factory
        .hide_previewing_dockables(Some(layout.root.clone()));
    assert!(layout.root.pinned_dock().is_some());
    layout.factory.preview_pinned_dockable(Some(second.clone()));
    assert!(Rc::ptr_eq(
        &layout.tool.owner().unwrap(),
        &(layout.dock.clone() as Rc<dyn IDockable>)
    ));
    assert!(layout.tool.original_owner().is_none());
    second.set_keep_pinned_dockable_visible(true);
    layout
        .factory
        .toggle_preview_pinned_dockable(Some(second.clone()));
    assert!(layout.root.pinned_dock().is_none() && second.original_owner().is_none());
}

#[test]
fn repeated_previews_of_cross_root_pins_preserve_the_floating_original_owner() {
    let layout = Layout::new(Alignment::Top);
    let workspace = RootDock::new();
    let window = DockWindow::new();
    window.set_layout(Some(layout.root.clone()));
    layout.root.set_window(Some(window.clone()));
    layout
        .factory
        .add_window(Some(workspace.clone()), Some(window.clone()));
    FactoryBaseImpl::pin_dockable_core_with_root(
        layout.factory.as_ref(),
        Some(layout.tool.clone()),
        Some(workspace.clone()),
        None,
    );
    assert!(Rc::ptr_eq(
        &layout.tool.owner().unwrap(),
        &(workspace.clone() as Rc<dyn IDockable>)
    ));
    for _ in 0..2 {
        layout
            .factory
            .preview_pinned_dockable(Some(layout.tool.clone()));
        layout
            .factory
            .hide_previewing_dockables(Some(workspace.clone()));
        assert!(Rc::ptr_eq(
            &layout.tool.owner().unwrap(),
            &(workspace.clone() as Rc<dyn IDockable>)
        ));
        assert!(Rc::ptr_eq(
            &layout.tool.original_owner().unwrap(),
            &(layout.dock.clone() as Rc<dyn IDockable>)
        ));
    }
    FactoryBaseImpl::pin_dockable_core_with_root(
        layout.factory.as_ref(),
        Some(layout.tool.clone()),
        Some(workspace),
        None,
    );
    assert!(Rc::ptr_eq(
        &layout.tool.owner().unwrap(),
        &(layout.dock.clone() as Rc<dyn IDockable>)
    ));
    assert!(layout.tool.original_owner().is_none());
    assert!(layout
        .tool
        .docking_state()
        .contains(DockingWindowState::FLOATING));
}

#[test]
fn initial_root_owned_pin_creates_a_real_aligned_tool_dock_when_structure_is_empty() {
    let factory = factory();
    let root = RootDock::new();
    let tool = Tool::new();
    root.set_right_pinned_dockables(Some(ListHandle::from_plain(vec![Some(tool.clone())])));
    factory.init_dockable(Some(root.clone()), None);
    factory.unpin_dockable(Some(tool.clone()));
    let owner = tool.owner().unwrap().as_tool_dock().unwrap();
    assert_eq!(owner.alignment(), Alignment::Right);
    assert_eq!(owner.title(), "IToolDock");
    assert_eq!(root.visible_dockables().unwrap().count(), 1);
    assert!(Rc::ptr_eq(
        (root.visible_dockables().unwrap().get(0))
            .as_ref()
            .expect("nonnull identity fixture"),
        &(owner as Rc<dyn IDockable>)
    ));
    assert_eq!(root.right_pinned_dockables().unwrap().count(), 0);
}

#[test]
fn unpin_explicit_destination_bypasses_automatic_alignment_and_attachment_selection() {
    let layout = Layout::new(Alignment::Left);
    let target = ToolDock::new();
    target.set_alignment(Alignment::Bottom);
    layout.factory.pin_dockable(Some(layout.tool.clone()));
    FactoryBaseImpl::pin_dockable_core_with_root(
        layout.factory.as_ref(),
        Some(layout.tool.clone()),
        Some(layout.root.clone()),
        Some(target.clone()),
    );
    assert!(Rc::ptr_eq(
        &layout.tool.owner().unwrap(),
        &(target.clone() as Rc<dyn IDockable>)
    ));
    assert!(target
        .visible_dockables()
        .unwrap()
        .contains(&Some((layout.tool.clone() as Rc<dyn IDockable>).clone())));
}

#[test]
fn duplicate_pin_cleanup_retains_source_one_pass_counts_and_one_unpinned_event() {
    let layout = Layout::new(Alignment::Left);
    layout.dock.set_visible_dockables(Some(ListHandle::new()));
    layout
        .root
        .set_left_pinned_dockables(Some(ListHandle::from_plain(vec![
            Some(layout.tool.clone()),
            Some(layout.tool.clone()),
            Some(layout.tool.clone()),
        ])));
    let count = Rc::new(Cell::new(0));
    let calls = count.clone();
    layout
        .factory
        .dockable_unpinned(Rc::new(move |_| calls.set(calls.get() + 1)));
    FactoryBaseImpl::pin_dockable_core(layout.factory.as_ref(), Some(layout.tool.clone()));
    assert_eq!(layout.root.left_pinned_dockables().unwrap().count(), 1);
    assert_eq!(count.get(), 1);
}

#[test]
fn preview_without_a_pin_strip_uses_owner_alignment_and_stale_history_falls_back_to_root() {
    let layout = Layout::new(Alignment::Bottom);
    layout
        .factory
        .preview_pinned_dockable(Some(layout.tool.clone()));
    assert_eq!(
        layout.root.pinned_dock().unwrap().alignment(),
        Alignment::Bottom
    );
    let detached = ToolDock::new();
    layout.tool.set_original_owner(Some(detached.clone()));
    layout
        .factory
        .hide_previewing_dockables(Some(layout.root.clone()));
    assert!(layout.root.pinned_dock().is_none());
    assert!(Rc::ptr_eq(
        &layout.tool.owner().unwrap(),
        &(layout.root.clone() as Rc<dyn IDockable>)
    ));
    assert!(layout.tool.original_owner().is_none());
}

#[test]
fn restored_initial_preview_is_initialized_without_losing_its_selection() {
    let factory = factory();
    let root = RootDock::new();
    let preview = ToolDock::new();
    let tool = Tool::new();
    root.set_left_pinned_dockables(Some(ListHandle::from_plain(vec![Some(tool.clone())])));
    preview.set_visible_dockables(Some(ListHandle::from_plain(vec![Some(tool.clone())])));
    preview.set_active_dockable(Some(tool.clone()));
    preview.set_is_empty(true);
    root.set_pinned_dock(Some(preview.clone()));
    factory.init_layout(root.clone());
    assert!(!preview.is_empty());
    assert!(preview.factory().is_some());
    assert!(Rc::ptr_eq(
        &preview.owner().unwrap(),
        &(root as Rc<dyn IDockable>)
    ));
    assert!(Rc::ptr_eq(
        &preview.active_dockable().unwrap(),
        &(tool as Rc<dyn IDockable>)
    ));
}

#[test]
fn initial_root_owned_pin_reuses_an_attached_aligned_tool_dock() {
    let factory = factory();
    let root = RootDock::new();
    let tool = Tool::new();
    let existing = ToolDock::new();
    existing.set_alignment(Alignment::Bottom);
    root.set_visible_dockables(Some(ListHandle::from_plain(vec![Some(existing.clone())])));
    root.set_bottom_pinned_dockables(Some(ListHandle::from_plain(vec![Some(tool.clone())])));
    factory.init_dockable(Some(root.clone()), None);
    *factory.tool_dock_creator.borrow_mut() =
        Some(Rc::new(|| panic!("aligned existing dock must be reused")));
    factory.unpin_dockable(Some(tool.clone()));
    assert!(Rc::ptr_eq(
        &tool.owner().unwrap(),
        &(existing as Rc<dyn IDockable>)
    ));
    assert_eq!(root.visible_dockables().unwrap().count(), 1);
}

#[test]
fn initial_root_owned_pin_creates_an_aligned_tool_dock_beside_the_existing_anchor() {
    for (alignment, orientation, target_index) in [
        (Alignment::Left, Orientation::Horizontal, 0),
        (Alignment::Right, Orientation::Horizontal, 2),
        (Alignment::Top, Orientation::Vertical, 0),
        (Alignment::Bottom, Orientation::Vertical, 2),
    ] {
        let factory = factory();
        let root = RootDock::new();
        let anchor = DocumentDock::new();
        let tool = Tool::new();
        root.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
            anchor.clone() as Rc<dyn IDockable>
        )])));
        let pinned = Some(ListHandle::from_items(vec![Some(
            tool.clone() as Rc<dyn IDockable>
        )]));
        match alignment {
            Alignment::Left => root.set_left_pinned_dockables(pinned),
            Alignment::Right => root.set_right_pinned_dockables(pinned),
            Alignment::Top => root.set_top_pinned_dockables(pinned),
            Alignment::Bottom => root.set_bottom_pinned_dockables(pinned),
            Alignment::Unset => unreachable!(),
        }
        factory.init_dockable(Some(root.clone()), None);
        factory.unpin_dockable(Some(tool.clone()));
        let owner = tool.owner().unwrap().as_tool_dock().unwrap();
        assert_eq!(owner.alignment(), alignment);
        assert_eq!(strip(&root, alignment).count(), 0);
        let layout = root
            .visible_dockables()
            .unwrap()
            .get(0)
            .and_then(|item| item.as_proportional_dock())
            .unwrap();
        assert_eq!(layout.orientation(), orientation);
        assert!(Rc::ptr_eq(
            (layout.visible_dockables().unwrap().get(target_index))
                .as_ref()
                .expect("nonnull identity fixture"),
            &(owner.clone() as Rc<dyn IDockable>)
        ));
        assert!(Rc::ptr_eq(
            (layout.visible_dockables().unwrap().get(2 - target_index))
                .as_ref()
                .expect("nonnull identity fixture"),
            &(anchor as Rc<dyn IDockable>)
        ));
        assert!(Rc::ptr_eq(
            &owner.active_dockable().unwrap(),
            &(tool as Rc<dyn IDockable>)
        ));
        assert!(owner.factory().is_some());
    }
}
