//! Source split layout and share-allocation regressions through real reactive models.
use crate::controls::*;
use crate::factory_init_tests::factory;
use ferrodock_model::collections::ListHandle;
use ferrodock_model::core::*;
use std::cell::RefCell;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

fn same(a: &Rc<dyn IDockable>, b: &Rc<dyn IDockable>) -> bool {
    Rc::ptr_eq(a, b)
}

#[test]
fn split_wrapper_preserves_outer_share_balances_children_and_orders_all_four_directions() {
    for (operation, orientation, before) in [
        (DockOperation::Left, Orientation::Horizontal, true),
        (DockOperation::Right, Orientation::Horizontal, false),
        (DockOperation::Top, Orientation::Vertical, true),
        (DockOperation::Bottom, Orientation::Vertical, false),
    ] {
        let factory = factory();
        let dock = ToolDock::new();
        let incoming = DocumentDock::new();
        dock.set_proportion(0.7);
        dock.set_collapsed_proportion(0.9);
        incoming.set_proportion(0.2);
        incoming.set_collapsed_proportion(0.3);
        dock.set_dock_group(Some("group".into()));
        let added = Rc::new(RefCell::new(Vec::new()));
        let log = added.clone();
        factory.dockable_added(Rc::new(move |args| {
            log.borrow_mut().push(args.dockable().unwrap())
        }));
        let layout = factory.create_split_layout(dock.clone(), incoming.clone(), operation);
        assert_eq!(layout.proportion(), 0.7);
        assert_eq!(layout.collapsed_proportion(), 0.7);
        assert_eq!(layout.dock_group().as_deref(), Some("group"));
        assert_eq!(
            layout.clone().as_proportional_dock().unwrap().orientation(),
            orientation
        );
        for child in [dock.clone() as Rc<dyn IDockable>, incoming.clone()] {
            assert!(child.proportion().is_nan() && child.collapsed_proportion().is_nan());
            assert!(child.owner().is_none() && child.factory().is_none());
        }
        let items = layout.visible_dockables().unwrap();
        assert_eq!(items.count(), 3);
        assert!(same(
            (items.get(if before { 0 } else { 2 }))
                .as_ref()
                .expect("nonnull identity fixture"),
            &(incoming as Rc<dyn IDockable>)
        ));
        assert!(same(
            (items.get(if before { 2 } else { 0 }))
                .as_ref()
                .expect("nonnull identity fixture"),
            &(dock as Rc<dyn IDockable>)
        ));
        assert!(items
            .get(1)
            .and_then(|item| item.as_proportional_dock_splitter())
            .is_some());
        assert_eq!(
            items
                .get(1)
                .as_ref()
                .expect("nonnull source fixture")
                .title(),
            "IProportionalDockSplitter"
        );
        assert!(same(
            &layout.active_dockable().unwrap(),
            (items.get(2)).as_ref().expect("nonnull identity fixture")
        ));
        assert!(same(
            &added.borrow()[0],
            (items.get(0)).as_ref().expect("nonnull identity fixture")
        ));
        assert!(same(
            &added.borrow()[1],
            (items.get(1)).as_ref().expect("nonnull identity fixture")
        ));
        assert!(same(
            &added.borrow()[2],
            (items.get(2)).as_ref().expect("nonnull identity fixture")
        ));
    }
}

#[test]
fn leaf_wrapper_inherits_effective_group_and_is_selected_without_initializing_ownership() {
    let factory = factory();
    let owner = ToolDock::new();
    owner.set_dock_group(Some("inherited".into()));
    let item = Tool::new();
    item.set_owner(Some(owner.clone()));
    let dock = DocumentDock::new();
    let layout = factory.create_split_layout(dock, item.clone(), DockOperation::Right);
    let split = layout
        .visible_dockables()
        .unwrap()
        .get(2)
        .and_then(|item| item.as_dock())
        .unwrap();
    assert_eq!(split.dock_group().as_deref(), Some("inherited"));
    assert!(same(
        &split.active_dockable().unwrap(),
        &(item.clone() as Rc<dyn IDockable>)
    ));
    assert!(same(&item.owner().unwrap(), &(owner as Rc<dyn IDockable>)));
    assert!(split.owner().is_none() && split.factory().is_none());
}

#[test]
fn unsupported_layout_operation_retains_source_splitter_only_but_split_entry_rejects_it() {
    for operation in [DockOperation::Fill, DockOperation::Window] {
        let factory = factory();
        let dock = ToolDock::new();
        let incoming = DocumentDock::new();
        let layout = factory.create_split_layout(dock.clone(), incoming.clone(), operation);
        assert_eq!(layout.visible_dockables().unwrap().count(), 1);
        assert!(layout
            .visible_dockables()
            .unwrap()
            .get(0)
            .and_then(|item| item.as_proportional_dock_splitter())
            .is_some());
        assert!(layout.active_dockable().is_none());
        assert!(catch_unwind(AssertUnwindSafe(|| factory.split_to_dock(
            dock.clone(),
            incoming.clone(),
            operation
        )))
        .is_err());
    }
}

#[test]
fn compatible_owner_inserts_in_place_splits_share_and_preserves_parent_selection() {
    for (operation, orientation, before) in [
        (DockOperation::Left, Orientation::Horizontal, true),
        (DockOperation::Right, Orientation::Horizontal, false),
        (DockOperation::Top, Orientation::Vertical, true),
        (DockOperation::Bottom, Orientation::Vertical, false),
    ] {
        let factory = factory();
        let root = RootDock::new();
        let parent = ProportionalDock::new();
        parent.set_orientation(orientation);
        let dock = DocumentDock::new();
        let incoming = ToolDock::new();
        dock.set_proportion(0.6);
        incoming.set_proportion(99.0);
        parent.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
            dock.clone() as Rc<dyn IDockable>
        )])));
        parent.set_active_dockable(Some(dock.clone()));
        root.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
            parent.clone() as Rc<dyn IDockable>
        )])));
        factory.init_dockable(Some(root.clone()), None);
        factory.split_to_dock(dock.clone(), incoming.clone(), operation);
        let children = parent.visible_dockables().unwrap();
        assert_eq!(children.count(), 3);
        assert!(same(
            (children.get(if before { 0 } else { 2 }))
                .as_ref()
                .expect("nonnull identity fixture"),
            &(incoming.clone() as Rc<dyn IDockable>)
        ));
        assert!(same(
            (children.get(if before { 2 } else { 0 }))
                .as_ref()
                .expect("nonnull identity fixture"),
            &(dock.clone() as Rc<dyn IDockable>)
        ));
        for child in [dock.clone() as Rc<dyn IDockable>, incoming.clone()] {
            assert_eq!(child.proportion(), 0.3);
            assert_eq!(child.collapsed_proportion(), 0.3);
        }
        assert!(same(
            &incoming.owner().unwrap(),
            &(parent.clone() as Rc<dyn IDockable>)
        ));
        assert!(incoming.factory().is_some());
        // The optimized source initializes the incoming dock only, leaving the new splitter uninitialized.
        assert!(
            children
                .get(1)
                .as_ref()
                .expect("nonnull source fixture")
                .owner()
                .is_none()
                && children
                    .get(1)
                    .as_ref()
                    .expect("nonnull source fixture")
                    .factory()
                    .is_none()
        );
        assert!(same(
            &parent.active_dockable().unwrap(),
            &(dock as Rc<dyn IDockable>)
        ));
        assert!(same(
            (root.visible_dockables().unwrap().get(0))
                .as_ref()
                .expect("nonnull identity fixture"),
            &(parent as Rc<dyn IDockable>)
        ));
    }
}

#[test]
fn optimized_leaf_wrapper_does_not_copy_group_but_initializes_the_leaf() {
    let factory = factory();
    let root = RootDock::new();
    let parent = ProportionalDock::new();
    let dock = DocumentDock::new();
    parent.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
        dock.clone() as Rc<dyn IDockable>
    )])));
    root.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
        parent.clone() as Rc<dyn IDockable>
    )])));
    factory.init_dockable(Some(root.clone()), None);
    let incoming = Tool::new();
    incoming.set_dock_group(Some("leaf-only".into()));
    factory.split_to_dock(dock, incoming.clone(), DockOperation::Right);
    let split = parent
        .visible_dockables()
        .unwrap()
        .get(2)
        .and_then(|item| item.as_dock())
        .unwrap();
    assert!(split.dock_group().is_none());
    assert!(same(
        &incoming.owner().unwrap(),
        &(split as Rc<dyn IDockable>)
    ));
    assert!(incoming.factory().is_some());
}

#[test]
fn inferred_shares_ignore_only_proportional_splitters_and_handle_exhausted_and_infinite_shares() {
    for (current, sibling, expected) in [
        (f64::NAN, 0.6, 0.2),
        (0.0, f64::NAN, 0.25),
        (-5.0, 1.2, 0.25),
        (f64::NAN, f64::INFINITY, 0.25),
        (f64::INFINITY, f64::NAN, f64::INFINITY),
        (0.8, 0.3, 0.4),
    ] {
        let factory = factory();
        let parent = ProportionalDock::new();
        let dock = DocumentDock::new();
        dock.set_proportion(current);
        let other = ToolDock::new();
        other.set_proportion(sibling);
        let splitter = ProportionalDockSplitter::new();
        splitter.set_proportion(1000.0);
        parent.set_visible_dockables(Some(ListHandle::from_items(vec![
            Some(dock.clone() as Rc<dyn IDockable>),
            Some(splitter),
            Some(other),
        ])));
        dock.set_owner(Some(parent.clone()));
        factory.split_to_dock(dock.clone(), ToolDock::new(), DockOperation::Right);
        assert_eq!(dock.proportion(), expected);
    }
    let factory = factory();
    let parent = ProportionalDock::new();
    let dock = DocumentDock::new();
    let grid_splitter = GridDockSplitter::new();
    grid_splitter.set_proportion(0.6);
    parent.set_visible_dockables(Some(ListHandle::from_items(vec![
        Some(dock.clone() as Rc<dyn IDockable>),
        Some(grid_splitter),
    ])));
    dock.set_owner(Some(parent.clone()));
    factory.split_to_dock(dock.clone(), ToolDock::new(), DockOperation::Right);
    assert_eq!(dock.proportion(), 0.2);
}

#[test]
fn incompatible_orientation_replaces_default_and_initializes_the_entire_wrapper() {
    let factory = factory();
    let root = RootDock::new();
    let parent = ProportionalDock::new();
    parent.set_orientation(Orientation::Horizontal);
    let dock = DocumentDock::new();
    dock.set_proportion(0.8);
    parent.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
        dock.clone() as Rc<dyn IDockable>
    )])));
    parent.set_default_dockable(Some(dock.clone()));
    root.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
        parent.clone() as Rc<dyn IDockable>
    )])));
    factory.init_dockable(Some(root.clone()), None);
    let incoming = ToolDock::new();
    let removed = Rc::new(RefCell::new(Vec::new()));
    let log = removed.clone();
    factory.dockable_removed(Rc::new(move |args| {
        log.borrow_mut().push(args.dockable().unwrap())
    }));
    factory.split_to_dock(dock.clone(), incoming.clone(), DockOperation::Bottom);
    let layout = parent
        .visible_dockables()
        .unwrap()
        .get(0)
        .and_then(|item| item.as_dock())
        .unwrap();
    assert_eq!(parent.visible_dockables().unwrap().count(), 1);
    assert_eq!(layout.proportion(), 0.8);
    assert_eq!(
        layout.clone().as_proportional_dock().unwrap().orientation(),
        Orientation::Vertical
    );
    assert!(same(
        &parent.active_dockable().unwrap(),
        &(layout.clone() as Rc<dyn IDockable>)
    ));
    assert!(same(
        &parent.default_dockable().unwrap(),
        &(layout.clone() as Rc<dyn IDockable>)
    ));
    for child in layout.visible_dockables().unwrap().iter() {
        assert!(same(
            &child
                .as_ref()
                .expect("nonnull source fixture")
                .owner()
                .unwrap(),
            &(layout.clone() as Rc<dyn IDockable>)
        ));
        assert!(child
            .as_ref()
            .expect("nonnull source fixture")
            .factory()
            .is_some());
    }
    // Source fallback reports the incoming item in Removed, although it physically removed the old dock.
    assert_eq!(removed.borrow().len(), 1);
    assert!(same(&removed.borrow()[0], &(incoming as Rc<dyn IDockable>)));
    assert!(dock.proportion().is_nan());
}

#[test]
fn unattached_and_absent_docks_leave_incoming_and_lists_untouched() {
    let factory = factory();
    let dock = DocumentDock::new();
    let incoming = ToolDock::new();
    incoming.set_proportion(0.7);
    factory.split_to_dock(dock.clone(), incoming.clone(), DockOperation::Right);
    let parent = RootDock::new();
    dock.set_owner(Some(parent.clone()));
    factory.split_to_dock(dock.clone(), incoming.clone(), DockOperation::Right);
    parent.set_visible_dockables(Some(ListHandle::new()));
    factory.split_to_dock(dock, incoming.clone(), DockOperation::Right);
    assert!(incoming.owner().is_none());
    assert_eq!(incoming.proportion(), 0.7);
    assert_eq!(parent.visible_dockables().unwrap().count(), 0);
    assert_eq!(factory.create_list_calls.get(), 0);
}
