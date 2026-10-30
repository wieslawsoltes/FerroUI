use crate::controls::*;
use crate::core::DockWindow;
use crate::factory_init_tests::factory;
use crate::test_host::RecordingHost;
use crate::test_support::TestControl;
use ferrodock_model::collections::ListHandle;
use ferrodock_model::controls::*;
use ferrodock_model::core::*;
use ferrodock_model::FactoryBase;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub(crate) struct Scene {
    pub(crate) root: Rc<RootDock>,
    pub(crate) window: Rc<DockWindow>,
    pub(crate) dock: Rc<DocumentDock>,
    pub(crate) document: Rc<Document>,
}
impl Scene {
    pub(crate) fn new() -> Self {
        let root = RootDock::new();
        let window = DockWindow::new();
        let dock = DocumentDock::new();
        let document = Document::new();
        dock.set_visible_dockables(Some(ListHandle::from_plain(vec![Some(document.clone())])));
        dock.set_owner(Some(root.clone()));
        dock.set_active_dockable(Some(document.clone()));
        dock.set_focused_dockable(Some(document.clone()));
        document.set_owner(Some(dock.clone()));
        root.set_visible_dockables(Some(ListHandle::from_plain(vec![Some(dock.clone())])));
        root.set_active_dockable(Some(dock.clone()));
        root.set_focused_dockable(Some(document.clone()));
        window.set_layout(Some(root.clone()));
        root.set_window(Some(window.clone()));
        Self {
            root,
            window,
            dock,
            document,
        }
    }
}

#[test]
fn window_activation_tracks_context_before_events_and_exposes_live_host_replacement() {
    let factory = factory();
    let scene = Scene::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let weak = Rc::downgrade(&factory);
    factory.global_dock_tracking_changed(Rc::new(move |args| {
        assert!(args.previous().dockable().is_none());
        assert_eq!(args.reason(), DockTrackingChangeReason::WindowActivated);
        assert!(Rc::ptr_eq(
            &args.current().dockable().unwrap(),
            &weak.upgrade().unwrap().current_dockable().unwrap()
        ));
        log.borrow_mut().push("tracking");
    }));
    let log = changes.clone();
    factory.focused_dockable_changed(Rc::new(move |_| log.borrow_mut().push("focus")));
    let log = changes.clone();
    factory.window_activated(Rc::new(move |_| log.borrow_mut().push("window")));
    factory.on_window_activated(Some(scene.window.clone()));
    assert_eq!(*changes.borrow(), ["tracking", "focus", "window"]);
    assert!(Rc::ptr_eq(
        &factory.current_dockable().unwrap(),
        &(scene.document.clone() as Rc<dyn IDockable>)
    ));
    assert!(Rc::ptr_eq(
        &factory.current_root_dock().unwrap(),
        &(scene.root as Rc<dyn IRootDock>)
    ));
    assert!(Rc::ptr_eq(
        &factory.current_dock_window().unwrap(),
        &(scene.window.clone() as Rc<dyn IDockWindow>)
    ));
    let host = RecordingHost::new();
    scene.window.set_host(Some(host.clone()));
    assert!(Rc::ptr_eq(
        &factory.current_host_window().unwrap(),
        &(host as Rc<dyn IHostWindow>)
    ));
    scene.window.set_host(None);
    assert!(factory.current_host_window().is_none());
}

#[test]
fn active_focused_and_activated_changes_from_another_live_root_keep_tracking_anchored() {
    let factory = factory();
    let first = Scene::new();
    let second = Scene::new();
    factory.on_window_activated(Some(first.window.clone()));
    factory.on_active_dockable_changed(Some(second.document.clone()));
    factory.on_focused_dockable_changed(Some(second.document.clone()));
    factory.set_active_dockable(Some(second.document.clone()));
    assert!(Rc::ptr_eq(
        &factory.current_dockable().unwrap(),
        &(first.document as Rc<dyn IDockable>)
    ));
    assert!(Rc::ptr_eq(
        &factory.current_root_dock().unwrap(),
        &(first.root as Rc<dyn IRootDock>)
    ));
    assert!(Rc::ptr_eq(
        &factory.current_dock_window().unwrap(),
        &(first.window as Rc<dyn IDockWindow>)
    ));
}

#[test]
fn null_active_and_focus_changes_re_resolve_the_existing_root_without_clearing_it() {
    let factory = factory();
    let scene = Scene::new();
    factory.on_window_activated(Some(scene.window.clone()));
    factory.on_active_dockable_changed(None);
    factory.on_focused_dockable_changed(None);
    assert!(Rc::ptr_eq(
        &factory.current_dockable().unwrap(),
        &(scene.document.clone() as Rc<dyn IDockable>)
    ));
    scene.root.set_focused_dockable(None);
    scene.dock.set_focused_dockable(None);
    scene.dock.set_active_dockable(None);
    factory.on_active_dockable_changed(None);
    assert!(Rc::ptr_eq(
        &factory.current_dockable().unwrap(),
        &(scene.dock as Rc<dyn IDockable>)
    ));
    assert!(Rc::ptr_eq(
        &factory.current_root_dock().unwrap(),
        &(scene.root as Rc<dyn IRootDock>)
    ));
}

#[test]
fn window_deactivation_clears_only_the_tracked_window_and_deactivates_its_focused_pane() {
    let factory = factory();
    let first = Scene::new();
    let other = Scene::new();
    factory.on_window_activated(Some(first.window.clone()));
    other.dock.set_is_active(true);
    factory.on_window_deactivated(Some(other.window.clone()));
    assert!(!other.dock.is_active());
    assert!(factory.current_dock_window().is_some());
    let reasons = Rc::new(RefCell::new(Vec::new()));
    let log = reasons.clone();
    factory.global_dock_tracking_changed(Rc::new(move |args| log.borrow_mut().push(args.reason())));
    factory.on_window_deactivated(Some(first.window));
    assert_eq!(
        *reasons.borrow(),
        [DockTrackingChangeReason::WindowDeactivated]
    );
    assert!(
        factory.current_dockable().is_none()
            && factory.current_root_dock().is_none()
            && factory.current_dock_window().is_none()
    );
    assert!(!first.dock.is_active());
}

#[test]
fn identical_context_does_not_raise_tracking_event_and_subscription_disposal_is_explicit() {
    let factory = factory();
    let scene = Scene::new();
    let count = Rc::new(Cell::new(0));
    let changes = count.clone();
    let subscription =
        factory.global_dock_tracking_changed(Rc::new(move |_| changes.set(changes.get() + 1)));
    let base = factory.base.get().unwrap();
    base.update_global_dock_tracking(
        Some(scene.document.clone()),
        None,
        None,
        DockTrackingChangeReason::Unknown,
    );
    base.update_global_dock_tracking(
        Some(scene.document.clone()),
        None,
        None,
        DockTrackingChangeReason::FocusedDockableChanged,
    );
    assert_eq!(count.get(), 1);
    subscription.dispose();
    subscription.dispose();
    base.update_global_dock_tracking(None, None, None, DockTrackingChangeReason::WindowClosed);
    assert_eq!(count.get(), 1);
    assert!(factory.current_root_dock().is_none());
}

#[test]
fn window_clear_checks_both_window_identity_and_layout_identity() {
    let factory = factory();
    let scene = Scene::new();
    let other = Scene::new();
    factory.on_window_activated(Some(scene.window.clone()));
    let base = factory.base.get().unwrap();
    base.clear_global_dock_tracking_for_window(None, DockTrackingChangeReason::WindowRemoved);
    base.clear_global_dock_tracking_for_window(
        Some(other.window.clone()),
        DockTrackingChangeReason::WindowRemoved,
    );
    assert!(factory.current_dock_window().is_some());
    other.window.set_layout(Some(scene.root));
    base.clear_global_dock_tracking_for_window(
        Some(other.window),
        DockTrackingChangeReason::WindowRemoved,
    );
    assert!(factory.current_dock_window().is_none());
}

#[test]
fn different_root_replaces_tracking_only_when_previous_window_or_registration_is_stale() {
    for case in 0..6 {
        let factory = factory();
        let first = Scene::new();
        let second = Scene::new();
        if case >= 2 {
            first.root.set_window(None);
        }
        factory.base.get().unwrap().update_global_dock_tracking(
            Some(first.document.clone()),
            Some(first.root.clone()),
            None,
            DockTrackingChangeReason::Unknown,
        );
        let replacement = DockWindow::new();
        match case {
            0 => first.window.set_layout(Some(second.root.clone())),
            1 => first.root.set_window(Some(replacement.clone())),
            2 => first.root.set_window(Some(first.window.clone())),
            3 => factory
                .controls
                .add(TestControl::new(Some(second.root.clone()))),
            4 => factory
                .controls
                .add(TestControl::new(Some(first.root.clone()))),
            _ => {}
        }
        factory.on_active_dockable_changed(Some(second.document.clone()));
        let expected: Rc<dyn IRootDock> = if case < 4 { second.root } else { first.root };
        assert!(
            Rc::ptr_eq(&factory.current_root_dock().unwrap(), &expected),
            "case {case}"
        );
    }
}

#[test]
fn tracked_resolver_prefers_focus_and_terminates_on_active_reference_cycles() {
    assert!(FactoryBase::resolve_tracked_dockable(None).is_none());
    let scene = Scene::new();
    let other = Document::new();
    scene.dock.set_focused_dockable(Some(other.clone()));
    assert!(Rc::ptr_eq(
        &FactoryBase::resolve_tracked_dockable(Some(scene.dock.clone())).unwrap(),
        &(other as Rc<dyn IDockable>)
    ));
    let first = ToolDock::new();
    let second = ToolDock::new();
    first.set_active_dockable(Some(second.clone()));
    second.set_active_dockable(Some(first.clone()));
    assert!(Rc::ptr_eq(
        &FactoryBase::resolve_tracked_dockable(Some(first.clone())).unwrap(),
        &(first.clone() as Rc<dyn IDockable>)
    ));
    first.set_active_dockable(None);
    second.set_active_dockable(None);
}

#[test]
fn layout_tracking_preserves_a_focused_container_while_generic_resolver_descends_it() {
    let factory = factory();
    let scene = Scene::new();
    scene.root.set_focused_dockable(Some(scene.dock.clone()));
    factory.init_layout(scene.root.clone());
    assert!(Rc::ptr_eq(
        &factory.current_dockable().unwrap(),
        &(scene.dock.clone() as Rc<dyn IDockable>)
    ));
    assert!(Rc::ptr_eq(
        &FactoryBase::resolve_tracked_dockable(Some(scene.dock)).unwrap(),
        &(scene.document as Rc<dyn IDockable>)
    ));
}

#[test]
fn deactivation_retains_source_fallback_without_recursively_resolving_it() {
    let factory = factory();
    let scene = Scene::new();
    factory.on_window_activated(Some(scene.window.clone()));
    factory.on_dockable_deactivated(Some(scene.document));
    assert!(Rc::ptr_eq(
        &factory.current_dockable().unwrap(),
        &(scene.dock as Rc<dyn IDockable>)
    ));
}

#[test]
fn tracking_snapshot_does_not_retain_layouts_windows_or_dockables() {
    let factory = factory();
    let scene = Scene::new();
    // Test snapshot ownership independently of CLR collection of model cycles.
    scene.root.set_window(None);
    scene.dock.set_owner(None);
    scene.document.set_owner(None);
    let weak = Rc::downgrade(&scene.root);
    factory.base.get().unwrap().update_global_dock_tracking(
        Some(scene.document.clone()),
        Some(scene.root.clone()),
        Some(scene.window.clone()),
        DockTrackingChangeReason::Unknown,
    );
    let snapshot = factory.global_dock_tracking_state();
    drop(scene);
    assert!(weak.upgrade().is_none());
    assert!(
        snapshot.dockable().is_none()
            && snapshot.root_dock().is_none()
            && snapshot.window().is_none()
    );
}

#[test]
fn tracking_observers_can_reenter_updates_without_borrow_conflicts_or_rewriting_event_snapshots() {
    let factory = factory();
    let first = Scene::new();
    let second = Scene::new();
    first.document.set_id("first".into());
    second.document.set_id("second".into());
    let events = Rc::new(RefCell::new(Vec::new()));
    let log = events.clone();
    let weak = Rc::downgrade(&factory);
    let changed = Cell::new(false);
    factory.global_dock_tracking_changed(Rc::new(move |args| {
        let current = args.current();
        log.borrow_mut().push(current.dockable().unwrap().id());
        if !changed.replace(true) {
            let factory = weak.upgrade().unwrap();
            factory.base.get().unwrap().update_global_dock_tracking(
                Some(second.document.clone()),
                Some(second.root.clone()),
                Some(second.window.clone()),
                DockTrackingChangeReason::FocusedDockableChanged,
            );
            assert_eq!(current.dockable().unwrap().id(), "first");
            assert_eq!(factory.current_dockable().unwrap().id(), "second");
        }
    }));
    factory.base.get().unwrap().update_global_dock_tracking(
        Some(first.document),
        Some(first.root),
        Some(first.window),
        DockTrackingChangeReason::ActiveDockableChanged,
    );
    assert_eq!(*events.borrow(), ["first", "second"]);
}

#[test]
fn active_change_event_keeps_requested_root_context_when_global_tracking_is_anchored_elsewhere() {
    let factory = factory();
    let first = Scene::new();
    let second = Scene::new();
    factory.on_window_activated(Some(first.window.clone()));
    let observed = Rc::new(Cell::new(false));
    let changed = observed.clone();
    let root = second.root.clone();
    let window = second.window.clone();
    factory.active_dockable_changed(Rc::new(move |args| {
        assert!(Rc::ptr_eq(
            &args.root_dock().unwrap(),
            &(root.clone() as Rc<dyn IRootDock>)
        ));
        assert!(Rc::ptr_eq(
            &args.window().unwrap(),
            &(window.clone() as Rc<dyn IDockWindow>)
        ));
        changed.set(true);
    }));
    factory.on_active_dockable_changed(Some(second.document));
    assert!(observed.get());
    assert!(Rc::ptr_eq(
        &factory.current_root_dock().unwrap(),
        &(first.root as Rc<dyn IRootDock>)
    ));
}

#[test]
fn activation_and_window_deactivation_events_raise_after_their_tracking_effects() {
    let factory = factory();
    let scene = Scene::new();
    let count = Rc::new(Cell::new(0));
    let called = count.clone();
    factory.dockable_activated(Rc::new(move |args| {
        assert!(args.dockable().is_some());
        called.set(called.get() + 1);
    }));
    factory.on_dockable_activated(Some(scene.document));
    assert_eq!(count.get(), 1);
    let called = count.clone();
    let weak = Rc::downgrade(&factory);
    factory.window_deactivated(Rc::new(move |args| {
        assert!(args.window().is_some());
        assert!(weak.upgrade().unwrap().current_root_dock().is_none());
        called.set(called.get() + 1);
    }));
    factory.on_window_deactivated(Some(scene.window));
    assert_eq!(count.get(), 2);
}
