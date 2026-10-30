//! Original factory focus helpers exercised against the shared engine and live models.
use crate::{
    controls::*, core::DockWindow, factory_init_tests::factory, test_host::RecordingHost,
    test_support::TestControl,
};
use ferrodock_model::{collections::ListHandle, controls::*, core::*, FactoryBaseImpl};
use std::rc::Rc;

#[test]
fn null_factory_returns_null_and_close_is_a_no_op() {
    assert!(FactoryExtensions::get_active_root::<dyn IFactory>(None).is_none());
    assert!(FactoryExtensions::get_current_document::<dyn IFactory>(None).is_none());
    assert!(FactoryExtensions::get_current_dock_window::<dyn IFactory>(None).is_none());
    assert!(FactoryExtensions::get_current_host_window::<dyn IFactory>(None).is_none());
    FactoryExtensions::close_focused_dockable::<dyn IFactory>(None);
}
#[test]
fn root_fallback_keeps_factory_search_order_and_stops_before_later_controls() {
    let factory = factory();
    assert!(factory.get_active_root().is_none());
    assert!(factory.get_current_document().is_none());
    factory.close_focused_dockable();
    assert!(factory.calls.borrow().is_empty());
    let inactive = RootDock::new();
    let first = RootDock::new();
    first.set_is_active(true);
    let later = RootDock::new();
    later.set_is_active(true);
    let controls = [
        TestControl::new(Some(inactive)),
        TestControl::new(Some(first.clone())),
        TestControl::new(Some(later)),
    ];
    for control in &controls {
        factory.dock_controls().add(control.clone());
    }
    let handle: Rc<dyn IFactory> = factory.clone();
    assert!(Rc::ptr_eq(
        &handle.get_active_root().unwrap(),
        &(first as Rc<dyn IRootDock>)
    ));
    assert_eq!(controls[2].reads.get(), 0);
}
#[test]
fn tracked_root_wins_even_when_inactive_without_enumerating_controls() {
    let factory = factory();
    let tracked = RootDock::new();
    let active = RootDock::new();
    active.set_is_active(true);
    let control = TestControl::new(Some(active));
    factory.dock_controls().add(control.clone());
    factory.update_global_dock_tracking(
        None,
        Some(tracked.clone()),
        None,
        DockTrackingChangeReason::LayoutInitialized,
    );
    assert!(Rc::ptr_eq(
        &factory.get_active_root().unwrap(),
        &(tracked as Rc<dyn IRootDock>)
    ));
    assert_eq!(control.reads.get(), 0);
}
#[test]
fn current_document_uses_current_cast_then_root_focus_and_preserves_tool_document_contract() {
    let factory = factory();
    let root = RootDock::new();
    let focus = Document::new();
    root.set_focused_dockable(Some(focus.clone()));
    let current = Document::new();
    factory.update_global_dock_tracking(
        Some(current.clone()),
        Some(root.clone()),
        None,
        DockTrackingChangeReason::LayoutInitialized,
    );
    assert!(Rc::ptr_eq(
        &factory.get_current_document().unwrap(),
        &(current as Rc<dyn IDocument>)
    ));
    let splitter = ProportionalDockSplitter::new();
    factory.update_global_dock_tracking(
        Some(splitter.clone()),
        Some(root.clone()),
        None,
        DockTrackingChangeReason::LayoutInitialized,
    );
    assert!(Rc::ptr_eq(
        &factory.get_current_document().unwrap(),
        &(focus as Rc<dyn IDocument>)
    ));
    let tool = Tool::new();
    factory.update_global_dock_tracking(
        Some(tool.clone()),
        Some(root.clone()),
        None,
        DockTrackingChangeReason::LayoutInitialized,
    );
    assert!(Rc::ptr_eq(
        &factory.get_current_document().unwrap(),
        &(tool as Rc<dyn IDocument>)
    ));
    root.set_focused_dockable(None);
    factory.update_global_dock_tracking(
        None,
        Some(root.clone()),
        None,
        DockTrackingChangeReason::LayoutInitialized,
    );
    assert!(factory.get_current_document().is_none());
}
#[test]
fn close_uses_current_dockable_without_document_filtering_then_focused_fallback() {
    let factory = factory();
    factory.simulate_close.set(true);
    let root = RootDock::new();
    let focused = Document::new();
    root.set_focused_dockable(Some(focused.clone()));
    let current = ProportionalDockSplitter::new();
    current.set_can_close(false);
    factory.update_global_dock_tracking(
        Some(current.clone()),
        Some(root.clone()),
        None,
        DockTrackingChangeReason::LayoutInitialized,
    );
    factory.close_focused_dockable();
    assert_eq!(factory.calls.borrow()[0].0, "close_dockable");
    assert!(Rc::ptr_eq(
        &factory.calls.borrow()[0].1,
        &(current as Rc<dyn IDockable>)
    ));
    factory.update_global_dock_tracking(
        None,
        Some(root.clone()),
        None,
        DockTrackingChangeReason::LayoutInitialized,
    );
    factory.close_focused_dockable();
    assert!(Rc::ptr_eq(
        &factory.calls.borrow()[1].1,
        &(focused as Rc<dyn IDockable>)
    ));
    root.set_focused_dockable(None);
    factory.close_focused_dockable();
    assert_eq!(factory.calls.borrow().len(), 2);
}
#[test]
fn window_helpers_read_the_tracked_window_and_current_host_without_search() {
    let factory = factory();
    let window = DockWindow::new();
    factory.update_global_dock_tracking(
        None,
        None,
        Some(window.clone()),
        DockTrackingChangeReason::LayoutInitialized,
    );
    assert!(Rc::ptr_eq(
        &factory.get_current_dock_window().unwrap(),
        &(window.clone() as Rc<dyn IDockWindow>)
    ));
    assert!(factory.get_current_host_window().is_none());
    let first = RecordingHost::new();
    window.set_host(Some(first.clone()));
    assert!(Rc::ptr_eq(
        &factory.get_current_host_window().unwrap(),
        &(first as Rc<dyn IHostWindow>)
    ));
    let second = RecordingHost::new();
    window.set_host(Some(second.clone()));
    assert!(Rc::ptr_eq(
        &factory.get_current_host_window().unwrap(),
        &(second as Rc<dyn IHostWindow>)
    ));
    window.set_host(None);
    assert!(factory.get_current_host_window().is_none());
}
#[test]
fn close_helper_dispatches_through_the_original_engine_and_mutates_the_real_layout() {
    let factory = factory();
    let root = RootDock::new();
    let dock = DocumentDock::new();
    dock.set_can_close_last_dockable(true);
    dock.set_is_collapsable(false);
    let document = Document::new();
    dock.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
        document.clone() as Rc<dyn IDockable>
    )])));
    dock.set_active_dockable(Some(document.clone()));
    root.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
        dock.clone() as Rc<dyn IDockable>
    )])));
    root.set_active_dockable(Some(dock.clone()));
    root.set_focused_dockable(Some(document.clone()));
    factory.set_hide_documents_on_close(false);
    factory.init_layout(root.clone());
    assert!(Rc::ptr_eq(
        &factory.current_dockable().unwrap(),
        &(document as Rc<dyn IDockable>)
    ));
    factory.close_focused_dockable();
    assert_eq!(dock.visible_dockables().unwrap().count(), 0);
}
