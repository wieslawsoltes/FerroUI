use crate::controls::*;
use crate::factory_init_tests::factory;
use crate::factory_tracking_tests::Scene;
use crate::test_support::TestControl;
use ferrodock_model::core::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[test]
fn focus_transfer_changes_activation_without_changing_pane_selection() {
    let factory = factory();
    let scene = Scene::new();
    let other = Document::new();
    other.set_owner(Some(scene.dock.clone()));
    scene
        .dock
        .visible_dockables()
        .unwrap()
        .add(Some(other.clone()));
    factory.init_dockable(Some(scene.root.clone()), None);
    factory.set_focused_dockable(Some(scene.dock.clone()), Some(other.clone()));
    assert!(Rc::ptr_eq(
        &scene.root.focused_dockable().unwrap(),
        &(other.clone() as Rc<dyn IDockable>)
    ));
    assert!(scene.dock.is_active() && other.is_active() && !other.is_selected());
    assert!(!scene.document.is_active() && scene.document.is_selected());
}

#[test]
fn same_focus_reactivates_inactive_owner_and_raises_focus_event_only_once() {
    let factory = factory();
    let scene = Scene::new();
    factory.init_dockable(Some(scene.root.clone()), None);
    scene.dock.set_is_active(false);
    let count = Rc::new(Cell::new(0));
    let log = count.clone();
    factory.focused_dockable_changed(Rc::new(move |_| log.set(log.get() + 1)));
    factory.set_focused_dockable(Some(scene.dock.clone()), Some(scene.document.clone()));
    factory.set_focused_dockable(Some(scene.dock.clone()), Some(scene.document.clone()));
    assert_eq!(count.get(), 1);
    assert!(scene.dock.is_active() && scene.document.is_active());
}

#[test]
fn clearing_focus_deactivates_previous_owner_and_preserves_selection() {
    let factory = factory();
    let scene = Scene::new();
    factory.init_dockable(Some(scene.root.clone()), None);
    factory.set_focused_dockable(Some(scene.dock.clone()), Some(scene.document.clone()));
    factory.set_focused_dockable(Some(scene.dock.clone()), None);
    assert!(scene.root.focused_dockable().is_none());
    assert!(!scene.dock.is_active() && !scene.document.is_active());
    assert!(scene.document.is_selected());
}

#[test]
fn focus_transfer_deactivates_other_registered_focusable_roots_without_clearing_their_focus() {
    let factory = factory();
    let first = Scene::new();
    let second = Scene::new();
    let third = Scene::new();
    third.root.set_is_focusable_root(false);
    for scene in [&first, &second, &third] {
        factory.init_dockable(Some(scene.root.clone()), None);
        scene.dock.set_is_active(true);
        factory
            .controls
            .add(TestControl::new(Some(scene.root.clone())));
    }
    factory.set_focused_dockable(Some(first.dock.clone()), Some(first.document.clone()));
    assert!(first.dock.is_active() && first.document.is_active());
    assert!(!second.dock.is_active() && !second.document.is_active());
    assert!(Rc::ptr_eq(
        &second.root.focused_dockable().unwrap(),
        &(second.document as Rc<dyn IDockable>)
    ));
    assert!(third.dock.is_active());
}

#[test]
fn non_focusable_root_does_not_accept_focus_requests() {
    let factory = factory();
    let scene = Scene::new();
    scene.root.set_is_focusable_root(false);
    factory.set_focused_dockable(Some(scene.dock.clone()), None);
    assert!(scene.root.focused_dockable().is_some());
    assert!(!scene.dock.is_active());
}

#[test]
fn deactivation_precedes_focus_change_and_observers_receive_current_model_context() {
    let factory = factory();
    let scene = Scene::new();
    let other = Document::new();
    other.set_owner(Some(scene.dock.clone()));
    scene
        .dock
        .visible_dockables()
        .unwrap()
        .add(Some(other.clone()));
    factory.init_dockable(Some(scene.root.clone()), None);
    scene.dock.set_is_active(true);
    let events = Rc::new(RefCell::new(Vec::new()));
    let log = events.clone();
    let old = scene.document.clone();
    factory.dockable_deactivated(Rc::new(move |args| {
        assert!(Rc::ptr_eq(
            &args.dockable().unwrap(),
            &(old.clone() as Rc<dyn IDockable>)
        ));
        log.borrow_mut().push("deactivated");
    }));
    let log = events.clone();
    let root = scene.root.clone();
    factory.focused_dockable_changed(Rc::new(move |args| {
        assert!(Rc::ptr_eq(
            &args.root_dock().unwrap(),
            &(root.clone() as Rc<dyn ferrodock_model::controls::IRootDock>)
        ));
        log.borrow_mut().push("focused");
    }));
    factory.set_focused_dockable(Some(scene.dock), Some(other));
    assert_eq!(*events.borrow(), ["deactivated", "focused"]);
}
