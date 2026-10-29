use crate::controls::*;
use crate::core::DockWindow;
use crate::factory_init_tests::factory;
use crate::factory_parameter_tests::RecordingFactory;
use ferrodock_model::collections::ListHandle;
use ferrodock_model::core::*;
use ferrodock_model::FactoryBaseImpl;
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

fn tool(id: &str) -> Rc<dyn IDockable> {
    let item = Tool::new();
    item.set_id(id.into());
    item
}
fn splitter(id: &str) -> Rc<dyn IDockable> {
    let item = ProportionalDockSplitter::new();
    item.set_id(id.into());
    item
}
fn dock(ids: &[&str]) -> (Rc<ToolDock>, Vec<Rc<dyn IDockable>>) {
    let dock = ToolDock::new();
    let items: Vec<_> = ids.iter().map(|id| tool(id)).collect();
    for item in &items {
        item.set_owner(Some(dock.clone()));
    }
    dock.set_visible_dockables(Some(ListHandle::from_plain(
        (items.clone()).into_iter().map(Some).collect(),
    )));
    (dock, items)
}
fn ids(dock: &dyn IDock) -> Vec<String> {
    dock.visible_dockables().map_or_else(Vec::new, |items| {
        items
            .iter()
            .map(|item| item.as_ref().expect("nonnull source fixture").id())
            .collect()
    })
}
fn trace(factory: &RecordingFactory) -> Rc<RefCell<Vec<String>>> {
    let log = Rc::new(RefCell::new(Vec::new()));
    macro_rules! event {
        ($event:ident, $label:literal) => {{
            let log = log.clone();
            factory.$event(Rc::new(move |args| {
                log.borrow_mut()
                    .push(format!("{}:{}", $label, args.dockable().unwrap().id()))
            }));
        }};
    }
    event!(dockable_added, "added");
    event!(dockable_removed, "removed");
    event!(dockable_moved, "moved");
    event!(dockable_swapped, "swapped");
    let events = log.clone();
    factory.dockable_undocked(Rc::new(move |args| {
        assert_eq!(args.operation(), DockOperation::Fill);
        events
            .borrow_mut()
            .push(format!("undocked:{}", args.dockable().unwrap().id()));
    }));
    let events = log.clone();
    factory.dockable_docked(Rc::new(move |args| {
        assert_eq!(args.operation(), DockOperation::Fill);
        events
            .borrow_mut()
            .push(format!("docked:{}", args.dockable().unwrap().id()));
    }));
    log
}

#[test]
fn add_initializes_before_collection_mutation_and_preserves_generic_plain_list_override() {
    let factory = factory();
    factory.create_plain_lists.set(true);
    let dock = ToolDock::new();
    let item = tool("item");
    let events = Rc::new(RefCell::new(Vec::new()));
    let log = events.clone();
    let owner = dock.clone();
    factory.dockable_init(Rc::new(move |args| {
        assert!(args.dockable().unwrap().factory().is_some());
        assert!(owner.visible_dockables().is_none());
        log.borrow_mut().push("init");
    }));
    let log = events.clone();
    let owner = dock.clone();
    factory.dockable_added(Rc::new(move |args| {
        assert!(owner
            .visible_dockables()
            .unwrap()
            .contains(&Some((args.dockable().unwrap()).clone())));
        assert!(!owner.is_empty());
        log.borrow_mut().push("added");
    }));
    factory.add_dockable(Some(dock.clone()), Some(item.clone()));
    assert_eq!(*events.borrow(), ["init", "added"]);
    assert_eq!(factory.create_list_calls.get(), 1);
    assert!(dock
        .visible_dockables()
        .unwrap()
        .as_notify_collection_changed()
        .is_none());
    assert!(Rc::ptr_eq(
        &item.owner().unwrap(),
        &(dock.clone() as Rc<dyn IDockable>)
    ));
    assert_eq!(dock.opened_dockables_count(), 1);
}

#[test]
fn insert_negative_is_noop_and_invalid_positive_index_retains_completed_initialization() {
    let factory = factory();
    let dock = ToolDock::new();
    let item = tool("item");
    factory.insert_dockable(Some(dock.clone()), Some(item.clone()), -1);
    assert!(dock.visible_dockables().is_none() && item.factory().is_none());
    assert!(catch_unwind(AssertUnwindSafe(|| factory.insert_dockable(
        Some(dock.clone()),
        Some(item.clone()),
        1
    )))
    .is_err());
    assert!(item.factory().is_some() && item.owner().is_some());
    assert_eq!(dock.visible_dockables().unwrap().count(), 0);
    factory.insert_dockable(Some(dock.clone()), Some(item.clone()), 0);
    assert_eq!(ids(dock.as_ref()), ["item"]);
}

#[test]
fn add_reads_current_visible_list_after_initialization_observers_replace_it() {
    let factory = factory();
    let (dock, _) = dock(&["old"]);
    let replacement = ListHandle::from_plain(Vec::new());
    let list = replacement.clone();
    let owner = dock.clone();
    factory.dockable_init(Rc::new(move |_| {
        owner.set_visible_dockables(Some(list.clone()))
    }));
    factory.add_dockable(Some(dock.clone()), Some(tool("new")));
    assert!(replacement == dock.visible_dockables().unwrap());
    assert_eq!(ids(dock.as_ref()), ["new"]);
}

#[test]
fn protected_collection_helpers_update_state_without_initializing_or_raising_lifecycle_events() {
    let factory = factory();
    let dock = ToolDock::new();
    let item = tool("item");
    let base = factory.base.get().unwrap();
    let events = trace(&factory);
    base.remove_visible_dockable(Some(dock.clone()), Some(item.clone()));
    base.remove_visible_dockable_at(Some(dock.clone()), -1);
    base.remove_all_visible_dockables(Some(dock.clone()));
    assert!(dock.is_empty());
    base.add_visible_dockable(Some(dock.clone()), Some(item.clone()));
    base.insert_visible_dockable(Some(dock.clone()), 0, Some(tool("first")));
    assert_eq!(ids(dock.as_ref()), ["first", "item"]);
    assert!(!dock.is_empty());
    assert!(item.owner().is_none() && item.factory().is_none());
    assert!(events.borrow().is_empty());
    base.remove_visible_dockable(Some(dock.clone()), Some(tool("absent")));
    base.remove_visible_dockable_at(Some(dock.clone()), 0);
    assert_eq!(ids(dock.as_ref()), ["item"]);
    base.remove_all_visible_dockables(Some(dock.clone()));
    assert!(dock.is_empty());
    assert_eq!(dock.opened_dockables_count(), 1);
}

#[test]
fn replacement_selection_uses_previous_position_and_never_skips_over_a_splitter() {
    let factory = factory();
    let dock = ToolDock::new();
    let a = tool("a");
    let b = tool("b");
    dock.set_visible_dockables(Some(ListHandle::from_plain(vec![
        Some(a.clone()),
        Some(splitter("s")),
        Some(b.clone()),
    ])));
    for (index, expected) in [(0, Some("a")), (1, Some("a")), (2, None), (3, Some("b"))] {
        assert_eq!(
            FactoryBaseImpl::get_active_dockable_after_removal(
                factory.as_ref(),
                Some(dock.clone()),
                Some(tool("removed")),
                index
            )
            .map(|item| item.id()),
            expected.map(str::to_owned)
        );
    }
    dock.set_visible_dockables(None);
    assert!(factory
        .base
        .get()
        .unwrap()
        .get_active_dockable_after_removal(Some(dock), Some(a), 0)
        .is_none());
}

#[test]
fn splitter_cleanup_preserves_first_middle_splitter_and_source_removal_order() {
    let factory = factory();
    let dock = ToolDock::new();
    let items = vec![
        splitter("l1"),
        splitter("l2"),
        tool("a"),
        splitter("m1"),
        splitter("m2"),
        splitter("m3"),
        tool("b"),
        splitter("t1"),
        splitter("t2"),
    ];
    dock.set_active_dockable(Some(items[5].clone()));
    dock.set_visible_dockables(Some(ListHandle::from_plain(
        (items).into_iter().map(Some).collect(),
    )));
    let events = trace(&factory);
    factory
        .base
        .get()
        .unwrap()
        .cleanup_orphaned_splitters(Some(dock.clone()));
    assert_eq!(ids(dock.as_ref()), ["a", "m1", "b"]);
    assert_eq!(
        *events.borrow(),
        [
            "removed:l1",
            "removed:l2",
            "removed:t2",
            "removed:t1",
            "removed:m2",
            "removed:m3"
        ]
    );
    assert_eq!(dock.active_dockable().unwrap().id(), "a");
}

#[test]
fn splitter_cleanup_rechecks_live_membership_including_duplicate_snapshot_candidates() {
    let factory = factory();
    let dock = ToolDock::new();
    let last = splitter("s3");
    dock.set_visible_dockables(Some(ListHandle::from_plain(vec![
        Some(tool("a")),
        Some(splitter("s1")),
        Some(splitter("s2")),
        Some(last.clone()),
        Some(tool("b")),
    ])));
    let list = dock.visible_dockables().unwrap();
    let inserted = Cell::new(false);
    let item = last.clone();
    let subscription = factory.dockable_removed(Rc::new(move |args| {
        if args.dockable().unwrap().id() == "s3" && !inserted.replace(true) {
            list.insert(2, Some(item.clone()));
        }
    }));
    let events = trace(&factory);
    factory
        .base
        .get()
        .unwrap()
        .cleanup_orphaned_splitters(Some(dock.clone()));
    assert_eq!(*events.borrow(), ["removed:s2", "removed:s3", "removed:s3"]);
    assert_eq!(ids(dock.as_ref()), ["a", "s1", "b"]);
    subscription.dispose();
}

#[test]
fn all_splitters_are_removed_but_initial_empty_list_returns_without_reselecting() {
    let factory = factory();
    let dock = ToolDock::new();
    let active = splitter("s");
    dock.set_active_dockable(Some(active.clone()));
    dock.set_visible_dockables(Some(ListHandle::from_plain(vec![
        Some(active.clone()),
        Some(splitter("s2")),
    ])));
    factory
        .base
        .get()
        .unwrap()
        .cleanup_orphaned_splitters(Some(dock.clone()));
    assert!(dock.visible_dockables().unwrap().count() == 0 && dock.active_dockable().is_none());
    dock.set_active_dockable(Some(active));
    factory
        .base
        .get()
        .unwrap()
        .cleanup_orphaned_splitters(Some(dock.clone()));
    assert!(dock.active_dockable().is_some());
}

#[test]
fn same_dock_move_overloads_preserve_distinct_event_and_selection_contracts() {
    for four_arguments in [false, true] {
        let factory = factory();
        let (dock, items) = dock(&["a", "b", "c"]);
        factory.init_dockable(Some(dock.clone()), None);
        dock.set_active_dockable(Some(items[1].clone()));
        let events = trace(&factory);
        let counts = Rc::new(RefCell::new(Vec::new()));
        let log = counts.clone();
        let owner = dock.clone();
        let moved = items[0].clone();
        factory.dockable_added(Rc::new(move |_| {
            log.borrow_mut().push(
                owner
                    .visible_dockables()
                    .unwrap()
                    .iter()
                    .filter(|item| {
                        Rc::ptr_eq(item.as_ref().expect("nonnull identity fixture"), &moved)
                    })
                    .count(),
            )
        }));
        if four_arguments {
            factory.move_dockable_between_docks(
                Some(dock.clone()),
                Some(dock.clone()),
                Some(items[0].clone()),
                Some(items[2].clone()),
            );
        } else {
            factory.move_dockable(
                Some(dock.clone()),
                Some(items[0].clone()),
                Some(items[2].clone()),
            );
        }
        assert_eq!(ids(dock.as_ref()), ["b", "c", "a"]);
        assert_eq!(
            dock.active_dockable().unwrap().id(),
            if four_arguments { "b" } else { "a" }
        );
        assert_eq!(*counts.borrow(), [if four_arguments { 2 } else { 1 }]);
        let expected = if four_arguments {
            vec!["added:a", "removed:a", "undocked:a", "moved:a", "docked:a"]
        } else {
            vec!["removed:a", "undocked:a", "added:a", "moved:a", "docked:a"]
        };
        assert_eq!(*events.borrow(), expected);
    }
}

#[test]
fn backward_same_owner_move_keeps_selection_and_noop_moves_leave_existing_splitters_alone() {
    let factory = factory();
    let (dock, items) = dock(&["a", "b", "c"]);
    dock.set_active_dockable(Some(items[1].clone()));
    factory.move_dockable_between_docks(
        Some(dock.clone()),
        Some(dock.clone()),
        Some(items[2].clone()),
        Some(items[0].clone()),
    );
    assert_eq!(ids(dock.as_ref()), ["c", "a", "b"]);
    assert_eq!(dock.active_dockable().unwrap().id(), "b");
    dock.visible_dockables()
        .unwrap()
        .insert(0, Some(splitter("leading")));
    let events = trace(&factory);
    factory.move_dockable(
        Some(dock.clone()),
        Some(items[0].clone()),
        Some(items[0].clone()),
    );
    factory.move_dockable(
        Some(dock.clone()),
        Some(tool("missing")),
        Some(items[0].clone()),
    );
    assert_eq!(ids(dock.as_ref()), ["leading", "c", "a", "b"]);
    assert!(events.borrow().is_empty());
}

#[test]
fn swap_overloads_keep_their_different_notifications_even_with_identical_owner_arguments() {
    for four_arguments in [false, true] {
        let factory = factory();
        let (dock, items) = dock(&["a", "b"]);
        factory.init_dockable(Some(dock.clone()), None);
        let events = trace(&factory);
        if four_arguments {
            factory.swap_dockable_between_docks(
                Some(dock.clone()),
                Some(dock.clone()),
                Some(items[0].clone()),
                Some(items[1].clone()),
            );
        } else {
            factory.swap_dockable(
                Some(dock.clone()),
                Some(items[0].clone()),
                Some(items[1].clone()),
            );
        }
        assert_eq!(ids(dock.as_ref()), ["b", "a"]);
        assert_eq!(
            dock.active_dockable().unwrap().id(),
            if four_arguments { "a" } else { "b" }
        );
        let expected = if four_arguments {
            vec!["swapped:b", "swapped:a"]
        } else {
            vec!["removed:b", "added:a", "added:b", "swapped:a", "swapped:b"]
        };
        assert_eq!(*events.borrow(), expected);
    }
}

#[test]
fn cross_owner_swap_reinitializes_both_owners_before_notifying_and_selecting() {
    let factory = factory();
    let (source, a) = dock(&["a"]);
    let (target, b) = dock(&["b"]);
    factory.init_dockable(Some(source.clone()), None);
    factory.init_dockable(Some(target.clone()), None);
    let first = source.clone();
    let second = target.clone();
    let original_a = a[0].clone();
    let original_b = b[0].clone();
    factory.dockable_swapped(Rc::new(move |_| {
        assert!(Rc::ptr_eq(
            &original_a.owner().unwrap(),
            &(second.clone() as Rc<dyn IDockable>)
        ));
        assert!(Rc::ptr_eq(
            &original_b.owner().unwrap(),
            &(first.clone() as Rc<dyn IDockable>)
        ));
        assert!(first.active_dockable().is_none() && second.active_dockable().is_none());
    }));
    factory.swap_dockable_between_docks(
        Some(source.clone()),
        Some(target.clone()),
        Some(a[0].clone()),
        Some(b[0].clone()),
    );
    assert_eq!(ids(source.as_ref()), ["b"]);
    assert_eq!(ids(target.as_ref()), ["a"]);
    assert_eq!(source.active_dockable().unwrap().id(), "b");
    assert_eq!(target.active_dockable().unwrap().id(), "a");
}

#[test]
fn pinned_reordering_is_confined_to_one_strip_and_emits_only_moved() {
    for side in 0..4 {
        let factory = factory();
        let root = RootDock::new();
        let a = tool("a");
        let b = tool("b");
        let c = tool("c");
        a.set_owner(Some(root.clone()));
        let list = Some(ListHandle::from_plain(vec![
            Some(a.clone()),
            Some(b),
            Some(c.clone()),
        ]));
        match side {
            0 => root.set_left_pinned_dockables(list),
            1 => root.set_right_pinned_dockables(list),
            2 => root.set_top_pinned_dockables(list),
            _ => root.set_bottom_pinned_dockables(list),
        }
        let events = trace(&factory);
        factory.move_pinned_dockable(Some(a.clone()), Some(c));
        let list = match side {
            0 => root.left_pinned_dockables(),
            1 => root.right_pinned_dockables(),
            2 => root.top_pinned_dockables(),
            _ => root.bottom_pinned_dockables(),
        }
        .unwrap();
        assert_eq!(
            list.iter()
                .map(|item| item.as_ref().expect("nonnull source fixture").id())
                .collect::<Vec<_>>(),
            ["b", "c", "a"]
        );
        assert_eq!(*events.borrow(), ["moved:a"]);
        factory.move_pinned_dockable(Some(a), Some(tool("absent")));
        assert_eq!(events.borrow().len(), 1);
    }
}

#[test]
fn cross_owner_transfer_preserves_target_index_rule_and_event_order() {
    for target_kind in 0..4 {
        let factory = factory();
        let (source, moved) = dock(&["a"]);
        let (target, targets) = dock(if target_kind == 0 {
            &[]
        } else {
            &["x", "y", "z"]
        });
        factory.init_dockable(Some(source.clone()), None);
        factory.init_dockable(Some(target.clone()), None);
        let owner = source.clone();
        let weak = Rc::downgrade(&factory);
        *factory.on_remove.borrow_mut() = Some(Rc::new(move |item, collapse| {
            assert!(collapse);
            let factory = weak.upgrade().unwrap();
            factory
                .base
                .get()
                .unwrap()
                .remove_visible_dockable(Some(owner.clone()), Some(item.clone()));
            factory.on_dockable_removed(Some(item));
        }));
        let target_item = match target_kind {
            2 => Some(targets[0].clone()),
            3 => Some(tool("missing")),
            _ => None,
        };
        let events = trace(&factory);
        factory.move_dockable_between_docks(
            Some(source.clone()),
            Some(target.clone()),
            Some(moved[0].clone()),
            target_item,
        );
        let expected = match target_kind {
            0 => vec!["a"],
            2 => vec!["x", "a", "y", "z"],
            _ => vec!["x", "y", "a", "z"],
        };
        assert_eq!(ids(target.as_ref()), expected);
        assert!(ids(source.as_ref()).is_empty());
        assert!(Rc::ptr_eq(
            &moved[0].owner().unwrap(),
            &(target.clone() as Rc<dyn IDockable>)
        ));
        assert_eq!(target.active_dockable().unwrap().id(), "a");
        assert_eq!(
            *events.borrow(),
            ["removed:a", "undocked:a", "added:a", "moved:a", "docked:a"]
        );
    }
}

#[test]
fn floating_source_root_collapse_occurs_only_after_target_commit_and_not_after_observer_failure() {
    for fail in [false, true] {
        let factory = factory();
        let source = RootDock::new();
        let window = DockWindow::new();
        source.set_window(Some(window.clone()));
        let item = tool("a");
        item.set_owner(Some(source.clone()));
        source.set_visible_dockables(Some(ListHandle::from_plain(vec![Some(item.clone())])));
        let (target, _) = dock(&[]);
        factory.init_dockable(Some(target.clone()), None);
        let old = source.clone();
        let weak = Rc::downgrade(&factory);
        *factory.on_remove.borrow_mut() = Some(Rc::new(move |item, collapse| {
            assert!(!collapse);
            weak.upgrade()
                .unwrap()
                .base
                .get()
                .unwrap()
                .remove_visible_dockable(Some(old.clone()), Some(item));
        }));
        let collapsed = Rc::new(Cell::new(false));
        let called = collapsed.clone();
        let destination = target.clone();
        *factory.on_collapse.borrow_mut() = Some(Rc::new(move |_| {
            assert_eq!(ids(destination.as_ref()), ["a"]);
            assert_eq!(destination.active_dockable().unwrap().id(), "a");
            called.set(true);
        }));
        if fail {
            factory.dockable_added(Rc::new(|_| panic!("target observer failure")));
        }
        let result = catch_unwind(AssertUnwindSafe(|| {
            factory.move_dockable_between_docks(Some(source), Some(target), Some(item), None)
        }));
        assert_eq!(result.is_err(), fail);
        assert_eq!(collapsed.get(), !fail);
    }
}

#[test]
fn close_ranges_walk_live_lists_backwards_and_preserve_exclusion_identity() {
    for operation in 0..4 {
        let factory = factory();
        factory.simulate_close.set(true);
        let (dock, items) = dock(&["a", "b", "c", "d"]);
        let list = dock.visible_dockables().unwrap();
        *factory.on_operation.borrow_mut() = Some(Rc::new(move |name, item| {
            assert_eq!(name, "close_dockable");
            list.remove(&Some((item).clone()));
        }));
        match operation {
            0 => factory.close_other_dockables(items[1].clone()),
            1 => factory.close_all_dockables(items[1].clone()),
            2 => factory.close_left_dockables(items[1].clone()),
            _ => factory.close_right_dockables(items[1].clone()),
        }
        let closed: Vec<_> = factory
            .calls
            .borrow()
            .iter()
            .filter(|(name, _)| *name == "close_dockable")
            .map(|(_, item)| item.id())
            .collect();
        let expected = match operation {
            0 => vec!["d", "c", "a"],
            1 => vec!["d", "c", "b", "a"],
            2 => vec!["a"],
            _ => vec!["d", "c"],
        };
        assert_eq!(closed, expected);
    }
    let factory = factory();
    factory.simulate_close.set(true);
    let (dock, items) = dock(&["same", "same", "same"]);
    let list = dock.visible_dockables().unwrap();
    *factory.on_operation.borrow_mut() = Some(Rc::new(move |_, item| {
        list.remove(&Some((item).clone()));
    }));
    factory.close_other_dockables(items[1].clone());
    assert_eq!(dock.visible_dockables().unwrap().count(), 1);
    assert!(Rc::ptr_eq(
        (dock.visible_dockables().unwrap().get(0))
            .as_ref()
            .expect("nonnull identity fixture"),
        &items[1]
    ));
}

#[test]
fn close_range_reads_current_indices_instead_of_snapshotting_siblings() {
    let factory = factory();
    factory.simulate_close.set(true);
    let (dock, items) = dock(&["a", "b", "c", "d"]);
    let list = dock.visible_dockables().unwrap();
    let first = items[0].clone();
    *factory.on_operation.borrow_mut() = Some(Rc::new(move |_, item| {
        list.remove(&Some((item).clone()));
        list.remove(&Some((first).clone()));
    }));
    assert!(catch_unwind(AssertUnwindSafe(
        || factory.close_all_dockables(items[1].clone())
    ))
    .is_err());
    assert_eq!(ids(dock.as_ref()), ["b", "c"]);
    let closed: Vec<_> = factory
        .calls
        .borrow()
        .iter()
        .filter(|(name, _)| *name == "close_dockable")
        .map(|(_, item)| item.id())
        .collect();
    assert_eq!(closed, ["d"]);
}

#[test]
fn move_observer_failure_keeps_the_source_overloads_different_partial_mutations() {
    for four_arguments in [false, true] {
        let factory = factory();
        let (dock, items) = dock(&["a", "b", "c"]);
        if four_arguments {
            factory.dockable_added(Rc::new(|_| panic!("added observer failure")));
        } else {
            factory.dockable_removed(Rc::new(|_| panic!("removed observer failure")));
        }
        assert!(catch_unwind(AssertUnwindSafe(|| {
            if four_arguments {
                factory.move_dockable_between_docks(
                    Some(dock.clone()),
                    Some(dock.clone()),
                    Some(items[0].clone()),
                    Some(items[2].clone()),
                );
            } else {
                factory.move_dockable(
                    Some(dock.clone()),
                    Some(items[0].clone()),
                    Some(items[2].clone()),
                );
            }
        }))
        .is_err());
        let expected = if four_arguments {
            vec!["a", "b", "c", "a"]
        } else {
            vec!["b", "c"]
        };
        assert_eq!(ids(dock.as_ref()), expected);
    }
}

#[test]
fn document_layout_wrappers_dispatch_virtual_setters_and_ignore_other_model_types() {
    let factory = factory();
    let dock = DocumentDock::new();
    factory.set_document_dock_tabs_layout_left(dock.clone());
    assert_eq!(dock.tabs_layout(), DocumentTabLayout::Left);
    factory.set_document_dock_tabs_layout_top(dock.clone());
    assert_eq!(dock.tabs_layout(), DocumentTabLayout::Top);
    factory.set_document_dock_tabs_layout_right(dock.clone());
    assert_eq!(dock.tabs_layout(), DocumentTabLayout::Right);
    factory.set_document_dock_layout_mode_mdi(dock.clone());
    assert_eq!(dock.layout_mode(), DocumentLayoutMode::Mdi);
    factory.set_document_dock_layout_mode_tabbed(dock.clone());
    assert_eq!(dock.layout_mode(), DocumentLayoutMode::Tabbed);
    assert_eq!(factory.tabs_layout_requests.borrow().len(), 3);
    assert_eq!(factory.layout_mode_requests.borrow().len(), 2);
    factory.set_document_dock_tabs_layout(tool("tool"), DocumentTabLayout::Left);
    factory.set_document_dock_layout_mode(tool("tool"), DocumentLayoutMode::Mdi);
}
