use crate::controls::*;
use crate::core::DockWindow;
use crate::factory_init_tests::factory;
use crate::factory_tracking_tests::Scene;
use crate::test_host::{HostCall, RecordingHost};
use ferrodock_model::collections::ListHandle;
use ferrodock_model::controls::*;
use ferrodock_model::core::*;
use ferrodock_model::FactoryBaseImpl;
use ferroui_system::component_model::INotifyPropertyChanged;
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

#[test]
fn add_and_insert_preserve_generic_list_override_and_raise_added_before_initialization() {
    let factory = factory();
    factory.create_plain_lists.set(true);
    let root = RootDock::new();
    let window = DockWindow::new();
    let count = Rc::new(Cell::new(0));
    let events = count.clone();
    let owner = root.clone();
    factory.window_added(Rc::new(move |args| {
        let window = args.window().unwrap();
        assert!(window.owner().is_none() && window.factory().is_none());
        assert!(owner.windows().unwrap().contains(&Some(window.clone())));
        events.set(events.get() + 1);
    }));
    factory.add_window(Some(root.clone()), Some(window.clone()));
    let second = DockWindow::new();
    factory.insert_window(Some(root.clone()), Some(second.clone()), 0);
    assert_eq!(count.get(), 2);
    assert_eq!(factory.create_list_calls.get(), 1);
    let windows = root.windows().unwrap();
    assert!(windows.as_notify_collection_changed().is_none());
    assert!(Rc::ptr_eq(
        &windows.get(0).expect("nonnull window fixture"),
        &(second as Rc<dyn IDockWindow>)
    ));
    assert!(Rc::ptr_eq(
        &window.owner().unwrap(),
        &(root as Rc<dyn IDockable>)
    ));
    assert!(Rc::ptr_eq(
        &window.factory().unwrap(),
        &(factory.clone() as Rc<dyn IFactory>)
    ));
}

#[test]
fn negative_insert_is_noop_and_out_of_range_retains_created_empty_list() {
    let factory = factory();
    let root = RootDock::new();
    let window = DockWindow::new();
    factory.insert_window(Some(root.clone()), Some(window.clone()), -1);
    assert!(root.windows().is_none());
    assert_eq!(factory.create_list_calls.get(), 0);
    assert!(catch_unwind(AssertUnwindSafe(|| factory.insert_window(
        Some(root.clone()),
        Some(window.clone()),
        1
    )))
    .is_err());
    assert_eq!(root.windows().unwrap().count(), 0);
    assert!(window.owner().is_none());
    factory.insert_window(Some(root.clone()), Some(window), 0);
    assert_eq!(factory.create_list_calls.get(), 1);
}

#[test]
fn added_event_failure_keeps_registration_and_skips_initialization() {
    let factory = factory();
    let root = RootDock::new();
    let window = DockWindow::new();
    factory.window_added(Rc::new(|_| panic!("added observer failure")));
    assert!(catch_unwind(AssertUnwindSafe(
        || factory.add_window(Some(root.clone()), Some(window.clone()))
    ))
    .is_err());
    assert!(root
        .windows()
        .unwrap()
        .contains(&Some(window.clone() as Rc<dyn IDockWindow>)));
    assert!(window.owner().is_none() && window.factory().is_none());
}

#[test]
fn remove_exits_host_clears_tracking_and_notifies_before_clearing_model_links() {
    let factory = factory();
    let root = RootDock::new();
    let window = DockWindow::new();
    let layout = RootDock::new();
    let parent = DockWindow::new();
    window.set_layout(Some(layout.clone()));
    layout.set_window(Some(window.clone()));
    window.set_parent_window(Some(parent.clone()));
    factory.add_window(Some(root.clone()), Some(window.clone()));
    let host = RecordingHost::new();
    window.set_host(Some(host.clone()));
    factory.on_window_activated(Some(window.clone()));
    let observed = Rc::new(Cell::new(false));
    let called = observed.clone();
    let owner = root.clone();
    factory.window_removed(Rc::new(move |args| {
        let window = args.window().unwrap();
        assert!(!owner.windows().unwrap().contains(&Some(window.clone())));
        assert!(
            window.owner().is_some() && window.factory().is_some() && window.layout().is_some()
        );
        assert!(window.parent_window().is_some());
        called.set(true);
    }));
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    window
        .property_changed()
        .add(Rc::new(move |name| log.borrow_mut().push(name.to_string())));
    factory.remove_window(Some(window.clone()));
    assert!(observed.get());
    assert!(factory.current_dock_window().is_none());
    assert!(host.calls.borrow().contains(&HostCall::Exit));
    assert!(
        window.owner().is_none()
            && window.factory().is_none()
            && window.layout().is_none()
            && window.parent_window().is_none()
            && window.host().is_none()
    );
    assert!(layout.window().is_none());
    assert!(changes.borrow().ends_with(&[
        "ParentWindow".into(),
        "Owner".into(),
        "Factory".into(),
        "Layout".into()
    ]));
}

#[test]
fn cancelled_host_close_preserves_registration_tracking_and_model_links() {
    let factory = factory();
    let root = RootDock::new();
    let window = DockWindow::new();
    window.set_layout(Some(RootDock::new()));
    factory.add_window(Some(root.clone()), Some(window.clone()));
    let host = RecordingHost::new();
    host.closed.set(Some(false));
    host.tracked.set(true);
    window.set_host(Some(host.clone()));
    factory.on_window_activated(Some(window.clone()));
    let count = Rc::new(Cell::new(0));
    let calls = count.clone();
    factory.window_removed(Rc::new(move |_| calls.set(calls.get() + 1)));
    factory.remove_window(Some(window.clone()));
    assert!(root
        .windows()
        .unwrap()
        .contains(&Some(window.clone() as Rc<dyn IDockWindow>)));
    assert!(window.owner().is_some() && window.factory().is_some() && window.layout().is_some());
    assert!(factory.current_dock_window().is_some() && host.tracked.get());
    assert_eq!(count.get(), 0);
}

#[test]
fn owner_change_during_exit_stops_outer_removal() {
    let factory = factory();
    let root = RootDock::new();
    let new_owner = RootDock::new();
    let window = DockWindow::new();
    window.set_layout(Some(RootDock::new()));
    factory.add_window(Some(root.clone()), Some(window.clone()));
    let host = RecordingHost::new();
    window.set_host(Some(host.clone()));
    let model = Rc::downgrade(&window);
    let owner = new_owner.clone();
    *host.on_call.borrow_mut() = Some(Rc::new(move |call| {
        if *call == HostCall::Exit {
            model.upgrade().unwrap().set_owner(Some(owner.clone()));
        }
    }));
    factory.remove_window(Some(window.clone()));
    assert!(root
        .windows()
        .unwrap()
        .contains(&Some(window.clone() as Rc<dyn IDockWindow>)));
    assert!(Rc::ptr_eq(
        &window.owner().unwrap(),
        &(new_owner as Rc<dyn IDockable>)
    ));
    assert!(window.factory().is_some() && window.layout().is_some());
}

#[test]
fn reentrant_removal_from_host_exit_notifies_once() {
    let factory = factory();
    let root = RootDock::new();
    let window = DockWindow::new();
    factory.add_window(Some(root.clone()), Some(window.clone()));
    let host = RecordingHost::new();
    window.set_host(Some(host.clone()));
    let owner = Rc::downgrade(&factory);
    let model = Rc::downgrade(&window);
    let reentered = Cell::new(false);
    *host.on_call.borrow_mut() = Some(Rc::new(move |call| {
        if *call == HostCall::Exit && !reentered.replace(true) {
            owner
                .upgrade()
                .unwrap()
                .remove_window(Some(model.upgrade().unwrap()));
        }
    }));
    let count = Rc::new(Cell::new(0));
    let calls = count.clone();
    factory.window_removed(Rc::new(move |_| calls.set(calls.get() + 1)));
    factory.remove_window(Some(window.clone()));
    assert_eq!(count.get(), 1);
    assert_eq!(root.windows().unwrap().count(), 0);
    assert!(window.owner().is_none());
}

#[test]
fn removal_observer_failure_leaves_links_after_unregistration_in_source_order() {
    let factory = factory();
    let root = RootDock::new();
    let window = DockWindow::new();
    window.set_layout(Some(RootDock::new()));
    factory.add_window(Some(root.clone()), Some(window.clone()));
    factory.window_removed(Rc::new(|_| panic!("removed observer failure")));
    assert!(catch_unwind(AssertUnwindSafe(
        || factory.remove_window(Some(window.clone()))
    ))
    .is_err());
    assert_eq!(root.windows().unwrap().count(), 0);
    assert!(window.owner().is_some() && window.factory().is_some() && window.layout().is_some());
}

#[test]
fn close_window_requests_descendants_before_containers_and_uses_snapshots() {
    let factory = factory();
    factory.simulate_close.set(true);
    let root = RootDock::new();
    let window = DockWindow::new();
    window.set_layout(Some(root.clone()));
    let dock = DocumentDock::new();
    dock.set_id("dock".into());
    let first = Document::new();
    first.set_id("first".into());
    let removed = Document::new();
    removed.set_id("removed".into());
    let last = Tool::new();
    last.set_id("last".into());
    let children = ListHandle::from_plain(vec![Some(first as Rc<dyn IDockable>), Some(removed)]);
    dock.set_visible_dockables(Some(children.clone()));
    let outer = ListHandle::from_plain(vec![Some(dock as Rc<dyn IDockable>), Some(last)]);
    root.set_visible_dockables(Some(outer.clone()));
    *factory.on_operation.borrow_mut() = Some(Rc::new(move |name, item| {
        assert_eq!(name, "close_dockable");
        if item.id() == "first" {
            children.clear();
            outer.clear();
        }
    }));
    factory.close_window(Some(window.clone()));
    assert_eq!(
        factory
            .calls
            .borrow()
            .iter()
            .map(|(_, item)| item.id())
            .collect::<Vec<_>>(),
        ["first", "removed", "dock", "last"]
    );
    window.set_layout(None);
    factory.close_window(Some(window));
    assert_eq!(factory.calls.borrow().len(), 4);
}

#[test]
fn collection_root_follows_nested_window_owners_and_stops_on_cycles() {
    let factory = factory();
    let top = RootDock::new();
    let middle = RootDock::new();
    let bottom = RootDock::new();
    // Root.Window is weak; retain the windows independently just as a real workspace does.
    let middle_window = DockWindow::new();
    middle_window.set_owner(Some(top.clone()));
    middle.set_window(Some(middle_window.clone()));
    let bottom_window = DockWindow::new();
    bottom_window.set_owner(Some(middle.clone()));
    bottom.set_window(Some(bottom_window.clone()));
    assert!(Rc::ptr_eq(
        &FactoryBaseImpl::resolve_window_collection_root(factory.as_ref(), Some(bottom.clone()))
            .expect("nonnull collection root fixture"),
        &(top.clone() as Rc<dyn IRootDock>)
    ));
    let top_window = DockWindow::new();
    top_window.set_owner(Some(bottom.clone()));
    top.set_window(Some(top_window.clone()));
    assert!(Rc::ptr_eq(
        &factory
            .base
            .get()
            .unwrap()
            .resolve_window_collection_root(Some(bottom))
            .expect("nonnull collection root fixture"),
        &(top as Rc<dyn IRootDock>)
    ));
}

#[test]
fn options_capture_source_window_before_moving_and_keep_explicit_parent() {
    let factory = factory();
    let source = Scene::new();
    let target = DockWindow::new();
    let options = DockWindowOptions::new();
    let base = factory.base.get().unwrap();
    base.prepare_window_options_for_dockable(Some(source.document.clone()), None);
    base.prepare_window_options_for_dockable(Some(source.document.clone()), Some(options.clone()));
    assert_eq!(factory.find_root_calls.get(), 0);
    options.set_owner_mode(DockWindowOwnerMode::DockableWindow);
    options.set_is_modal(true);
    options.set_show_in_taskbar(Some(false));
    FactoryBaseImpl::prepare_window_options_for_dockable(
        factory.as_ref(),
        Some(source.document.clone()),
        Some(options.clone()),
    );
    assert!(Rc::ptr_eq(
        &options.parent_window().unwrap(),
        &(source.window.clone() as Rc<dyn IDockWindow>)
    ));
    source.document.set_owner(None);
    FactoryBaseImpl::apply_window_options(
        factory.as_ref(),
        Some(target.clone()),
        Some(options.clone()),
    );
    assert!(Rc::ptr_eq(
        &target.parent_window().unwrap(),
        &(source.window.clone() as Rc<dyn IDockWindow>)
    ));
    assert!(target.is_modal());
    assert_eq!(target.show_in_taskbar(), Some(false));
    let reads = factory.find_root_calls.get();
    base.prepare_window_options_for_dockable(Some(source.document), Some(options));
    assert_eq!(factory.find_root_calls.get(), reads);
    base.apply_window_options(Some(target.clone()), None);
    assert!(target.is_modal());
}

#[test]
fn removal_uses_layout_captured_before_exit_and_respects_changed_layout_window_links() {
    for replace_link in [false, true] {
        let factory = factory();
        let root = RootDock::new();
        let window = DockWindow::new();
        let original = RootDock::new();
        original.set_window(Some(window.clone()));
        window.set_layout(Some(original.clone()));
        factory.add_window(Some(root.clone()), Some(window.clone()));
        let replacement = RootDock::new();
        replacement.set_window(Some(window.clone()));
        let other_window = DockWindow::new();
        let host = RecordingHost::new();
        window.set_host(Some(host.clone()));
        let weak = Rc::downgrade(&window);
        let layout = replacement.clone();
        *host.on_call.borrow_mut() = Some(Rc::new(move |call| {
            if *call == HostCall::Exit {
                weak.upgrade().unwrap().set_layout(Some(layout.clone()));
            }
        }));
        if replace_link {
            let layout = original.clone();
            let other = other_window.clone();
            factory.window_removed(Rc::new(move |_| layout.set_window(Some(other.clone()))));
        }
        factory.remove_window(Some(window.clone()));
        if replace_link {
            assert!(Rc::ptr_eq(
                &original.window().unwrap(),
                &(other_window as Rc<dyn IDockWindow>)
            ));
        } else {
            assert!(original.window().is_none());
        }
        assert!(replacement.window().is_some());
        assert!(window.layout().is_none());
    }
}

#[test]
fn failed_host_exit_stops_removal_before_registration_or_link_changes() {
    let factory = factory();
    let root = RootDock::new();
    let window = DockWindow::new();
    factory.add_window(Some(root.clone()), Some(window.clone()));
    let host = RecordingHost::new();
    host.tracked.set(true);
    window.set_host(Some(host.clone()));
    *host.on_call.borrow_mut() = Some(Rc::new(|call| {
        if *call == HostCall::Exit {
            panic!("host failure");
        }
    }));
    assert!(catch_unwind(AssertUnwindSafe(
        || factory.remove_window(Some(window.clone()))
    ))
    .is_err());
    assert!(host.tracked.get());
    assert!(root
        .windows()
        .unwrap()
        .contains(&Some(window.clone() as Rc<dyn IDockWindow>)));
    assert!(window.owner().is_some() && window.factory().is_some());
}
