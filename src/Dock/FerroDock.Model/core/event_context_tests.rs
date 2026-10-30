use super::events::*;
use super::*;
use crate::test_support::{TestDock, TestHost, TestNode, TestRoot, TestWindow};
use ferroui_system::events::HandlerList;
use std::rc::Rc;

#[test]
fn focus_and_activation_contexts_retain_targets_but_resolve_the_current_host() {
    let dockable: Rc<dyn IDockable> = TestNode::new(None);
    let root: Rc<dyn crate::controls::IRootDock> = TestRoot::new(None);
    let window: Rc<dyn IDockWindow> = TestWindow::new();
    let active = ActiveDockableChangedEventArgs::with_context(
        Some(dockable.clone()),
        Some(root.clone()),
        Some(window.clone()),
    );
    let focused = FocusedDockableChangedEventArgs::with_context(
        Some(dockable.clone()),
        Some(root.clone()),
        Some(window.clone()),
    );
    assert!(active.host_window().is_none() && focused.host_window().is_none());
    let first: Rc<dyn IHostWindow> = Rc::new(TestHost::default());
    window.set_host(Some(first.clone()));
    assert!(Rc::ptr_eq(&active.host_window().unwrap(), &first));
    assert!(Rc::ptr_eq(&focused.host_window().unwrap(), &first));
    let second: Rc<dyn IHostWindow> = Rc::new(TestHost::default());
    window.set_host(Some(second.clone()));
    assert!(Rc::ptr_eq(&active.host_window().unwrap(), &second));
    assert!(Rc::ptr_eq(&focused.host_window().unwrap(), &second));
    assert!(Rc::ptr_eq(&active.dockable().unwrap(), &dockable));
    assert!(Rc::ptr_eq(&focused.dockable().unwrap(), &dockable));
    assert!(Rc::ptr_eq(&active.root_dock().unwrap(), &root));
    assert!(Rc::ptr_eq(&focused.root_dock().unwrap(), &root));
    assert!(Rc::ptr_eq(&active.window().unwrap(), &window));
    assert!(Rc::ptr_eq(&focused.window().unwrap(), &window));
    assert!(active == active.clone());
    assert!(focused == focused.clone());
    assert!(active != ActiveDockableChangedEventArgs::new(Some(dockable.clone())));
    assert!(focused != FocusedDockableChangedEventArgs::new(Some(dockable.clone())));
    for args in [
        ActiveDockableChangedEventArgs::new(None),
        ActiveDockableChangedEventArgs::new(Some(dockable.clone())),
    ] {
        assert!(
            args.root_dock().is_none() && args.window().is_none() && args.host_window().is_none()
        );
    }
    for args in [
        FocusedDockableChangedEventArgs::new(None),
        FocusedDockableChangedEventArgs::new(Some(dockable)),
    ] {
        assert!(
            args.root_dock().is_none() && args.window().is_none() && args.host_window().is_none()
        );
    }
    let weak_root = Rc::downgrade(&root);
    let weak_window = Rc::downgrade(&window);
    drop(root);
    drop(window);
    assert!(weak_root.upgrade().is_some() && weak_window.upgrade().is_some());
    drop(active);
    drop(focused);
    assert!(weak_root.upgrade().is_none() && weak_window.upgrade().is_none());
}

#[test]
fn owner_changing_args_capture_both_owners_without_value_equality() {
    let old: Rc<dyn IDockable> = TestNode::new(None);
    let new: Rc<dyn IDockable> = TestDock::new(None);
    let args = DockableOwnerChangingEventArgs::new(Some(old.clone()), Some(new.clone()));
    assert!(Rc::ptr_eq(&args.old_owner().unwrap(), &old));
    assert!(Rc::ptr_eq(&args.new_owner().unwrap(), &new));
    assert!(args == args.clone());
    assert!(args != DockableOwnerChangingEventArgs::new(Some(old), Some(new)));
    let empty = DockableOwnerChangingEventArgs::new(None, None);
    assert!(empty.old_owner().is_none() && empty.new_owner().is_none());
}

#[test]
fn factory_event_handlers_share_cancellation_and_context_mutations() {
    let handlers = HandlerList::<dyn Fn(&DockableClosingEventArgs)>::new();
    handlers.add(Rc::new(|args| args.set_cancel(true)));
    handlers.add(Rc::new(|args| assert!(args.cancel())));
    let args = DockableClosingEventArgs::new(None);
    for (_, handler) in handlers.snapshot().iter() {
        handler(&args);
    }
    assert!(args.cancel());
    let contexts = HandlerList::<dyn Fn(&DockableInitEventArgs)>::new();
    contexts.add(Rc::new(|args| {
        args.set_context(Some(Rc::new("resolved".to_owned())))
    }));
    contexts.add(Rc::new(|args| {
        assert_eq!(
            args.context()
                .unwrap()
                .downcast_ref::<String>()
                .map(String::as_str),
            Some("resolved")
        )
    }));
    let args = DockableInitEventArgs::new(None);
    for (_, handler) in contexts.snapshot().iter() {
        handler(&args);
    }
}

#[test]
fn replacing_context_releases_the_old_value_outside_the_refcell_borrow() {
    struct ReadOnDrop(DockableInitEventArgs);
    impl Drop for ReadOnDrop {
        fn drop(&mut self) {
            assert!(self.0.context().is_none());
        }
    }
    let args = DockableInitEventArgs::new(None);
    args.set_context(Some(Rc::new(ReadOnDrop(args.clone()))));
    args.set_context(None);
}

#[test]
fn global_context_snapshots_are_weak_and_the_empty_state_is_shared() {
    let dockable: Rc<dyn IDockable> = TestNode::new(None);
    let root: Rc<dyn crate::controls::IRootDock> = TestRoot::new(None);
    let window: Rc<dyn IDockWindow> = TestWindow::new();
    let host: Rc<dyn IHostWindow> = Rc::new(TestHost::default());
    window.set_host(Some(host.clone()));
    let state = GlobalDockTrackingState::new(
        Some(dockable.clone()),
        Some(root.clone()),
        Some(window.clone()),
    );
    assert!(Rc::ptr_eq(&state.dockable().unwrap(), &dockable));
    assert!(Rc::ptr_eq(&state.root_dock().unwrap(), &root));
    assert!(Rc::ptr_eq(&state.window().unwrap(), &window));
    assert!(Rc::ptr_eq(&state.host_window().unwrap(), &host));
    assert!(state == state.clone());
    assert!(
        state
            != GlobalDockTrackingState::new(
                Some(dockable.clone()),
                Some(root.clone()),
                Some(window.clone())
            )
    );
    let empty = GlobalDockTrackingState::empty();
    assert!(empty == GlobalDockTrackingState::empty());
    assert!(
        empty.dockable().is_none()
            && empty.root_dock().is_none()
            && empty.window().is_none()
            && empty.host_window().is_none()
    );
    let args = GlobalDockTrackingChangedEventArgs::new(
        empty.clone(),
        state.clone(),
        DockTrackingChangeReason::WindowActivated,
    );
    assert!(args.previous() == empty && args.current() == state);
    assert_eq!(args.reason(), DockTrackingChangeReason::WindowActivated);
    assert!(args == args.clone());
    assert!(
        args != GlobalDockTrackingChangedEventArgs::new(
            empty,
            state.clone(),
            DockTrackingChangeReason::WindowActivated
        )
    );
    drop(dockable);
    drop(root);
    drop(window);
    assert!(
        state.dockable().is_none()
            && state.root_dock().is_none()
            && state.window().is_none()
            && state.host_window().is_none()
    );
    assert!(args.current().dockable().is_none());
}

#[test]
fn dockable_activated_event_args_shares_the_original_event_instance() {
    let value: Rc<dyn IDockable> = TestNode::new(None);
    let args = DockableActivatedEventArgs::new(Some(value.clone()));
    let alias = args.clone();
    assert!(args == alias);
    assert!(args != DockableActivatedEventArgs::new(Some(value.clone())));
    assert!(Rc::ptr_eq(&alias.dockable().unwrap(), &value));
    assert!(DockableActivatedEventArgs::new(None).dockable().is_none());
}

#[test]
fn dockable_added_event_args_shares_the_original_event_instance() {
    let value: Rc<dyn IDockable> = TestNode::new(None);
    let args = DockableAddedEventArgs::new(Some(value.clone()));
    let alias = args.clone();
    assert!(args == alias);
    assert!(args != DockableAddedEventArgs::new(Some(value.clone())));
    assert!(Rc::ptr_eq(&alias.dockable().unwrap(), &value));
    assert!(DockableAddedEventArgs::new(None).dockable().is_none());
}

#[test]
fn dockable_closed_event_args_shares_the_original_event_instance() {
    let value: Rc<dyn IDockable> = TestNode::new(None);
    let args = DockableClosedEventArgs::new(Some(value.clone()));
    let alias = args.clone();
    assert!(args == alias);
    assert!(args != DockableClosedEventArgs::new(Some(value.clone())));
    assert!(Rc::ptr_eq(&alias.dockable().unwrap(), &value));
    assert!(DockableClosedEventArgs::new(None).dockable().is_none());
}

#[test]
fn dockable_closing_event_args_shares_the_original_event_instance() {
    let value: Rc<dyn IDockable> = TestNode::new(None);
    let args = DockableClosingEventArgs::new(Some(value.clone()));
    let alias = args.clone();
    assert!(args == alias);
    assert!(args != DockableClosingEventArgs::new(Some(value.clone())));
    assert!(Rc::ptr_eq(&alias.dockable().unwrap(), &value));
    assert!(DockableClosingEventArgs::new(None).dockable().is_none());
    assert!(!args.cancel());
    alias.set_cancel(true);
    assert!(args.cancel());
    args.set_cancel(false);
    assert!(!alias.cancel());
}

#[test]
fn dockable_deactivated_event_args_shares_the_original_event_instance() {
    let value: Rc<dyn IDockable> = TestNode::new(None);
    let args = DockableDeactivatedEventArgs::new(Some(value.clone()));
    let alias = args.clone();
    assert!(args == alias);
    assert!(args != DockableDeactivatedEventArgs::new(Some(value.clone())));
    assert!(Rc::ptr_eq(&alias.dockable().unwrap(), &value));
    assert!(DockableDeactivatedEventArgs::new(None).dockable().is_none());
}

#[test]
fn dockable_docked_event_args_shares_the_original_event_instance() {
    let value: Rc<dyn IDockable> = TestNode::new(None);
    let args = DockableDockedEventArgs::new(Some(value.clone()), DockOperation::OuterBottom);
    let alias = args.clone();
    assert!(args == alias);
    assert!(args != DockableDockedEventArgs::new(Some(value.clone()), DockOperation::OuterBottom));
    assert!(Rc::ptr_eq(&alias.dockable().unwrap(), &value));
    assert!(
        DockableDockedEventArgs::new(None, DockOperation::OuterBottom)
            .dockable()
            .is_none()
    );
    assert_eq!(alias.operation(), DockOperation::OuterBottom);
}

#[test]
fn dockable_hidden_event_args_shares_the_original_event_instance() {
    let value: Rc<dyn IDockable> = TestNode::new(None);
    let args = DockableHiddenEventArgs::new(Some(value.clone()));
    let alias = args.clone();
    assert!(args == alias);
    assert!(args != DockableHiddenEventArgs::new(Some(value.clone())));
    assert!(Rc::ptr_eq(&alias.dockable().unwrap(), &value));
    assert!(DockableHiddenEventArgs::new(None).dockable().is_none());
}

#[test]
fn dockable_init_event_args_shares_the_original_event_instance() {
    let value: Rc<dyn IDockable> = TestNode::new(None);
    let args = DockableInitEventArgs::new(Some(value.clone()));
    let alias = args.clone();
    assert!(args == alias);
    assert!(args != DockableInitEventArgs::new(Some(value.clone())));
    assert!(Rc::ptr_eq(&alias.dockable().unwrap(), &value));
    assert!(DockableInitEventArgs::new(None).dockable().is_none());
    assert!(args.context().is_none());
    alias.set_context(Some(Rc::new(42_i32)));
    assert_eq!(args.context().unwrap().downcast_ref::<i32>(), Some(&42));
    args.set_context(None);
    assert!(alias.context().is_none());
}

#[test]
fn dockable_moved_event_args_shares_the_original_event_instance() {
    let value: Rc<dyn IDockable> = TestNode::new(None);
    let args = DockableMovedEventArgs::new(Some(value.clone()));
    let alias = args.clone();
    assert!(args == alias);
    assert!(args != DockableMovedEventArgs::new(Some(value.clone())));
    assert!(Rc::ptr_eq(&alias.dockable().unwrap(), &value));
    assert!(DockableMovedEventArgs::new(None).dockable().is_none());
}

#[test]
fn dockable_pinned_event_args_shares_the_original_event_instance() {
    let value: Rc<dyn IDockable> = TestNode::new(None);
    let args = DockablePinnedEventArgs::new(Some(value.clone()));
    let alias = args.clone();
    assert!(args == alias);
    assert!(args != DockablePinnedEventArgs::new(Some(value.clone())));
    assert!(Rc::ptr_eq(&alias.dockable().unwrap(), &value));
    assert!(DockablePinnedEventArgs::new(None).dockable().is_none());
}

#[test]
fn dockable_removed_event_args_shares_the_original_event_instance() {
    let value: Rc<dyn IDockable> = TestNode::new(None);
    let args = DockableRemovedEventArgs::new(Some(value.clone()));
    let alias = args.clone();
    assert!(args == alias);
    assert!(args != DockableRemovedEventArgs::new(Some(value.clone())));
    assert!(Rc::ptr_eq(&alias.dockable().unwrap(), &value));
    assert!(DockableRemovedEventArgs::new(None).dockable().is_none());
}

#[test]
fn dockable_restored_event_args_shares_the_original_event_instance() {
    let value: Rc<dyn IDockable> = TestNode::new(None);
    let args = DockableRestoredEventArgs::new(Some(value.clone()));
    let alias = args.clone();
    assert!(args == alias);
    assert!(args != DockableRestoredEventArgs::new(Some(value.clone())));
    assert!(Rc::ptr_eq(&alias.dockable().unwrap(), &value));
    assert!(DockableRestoredEventArgs::new(None).dockable().is_none());
}

#[test]
fn dockable_swapped_event_args_shares_the_original_event_instance() {
    let value: Rc<dyn IDockable> = TestNode::new(None);
    let args = DockableSwappedEventArgs::new(Some(value.clone()));
    let alias = args.clone();
    assert!(args == alias);
    assert!(args != DockableSwappedEventArgs::new(Some(value.clone())));
    assert!(Rc::ptr_eq(&alias.dockable().unwrap(), &value));
    assert!(DockableSwappedEventArgs::new(None).dockable().is_none());
}

#[test]
fn dockable_undocked_event_args_shares_the_original_event_instance() {
    let value: Rc<dyn IDockable> = TestNode::new(None);
    let args = DockableUndockedEventArgs::new(Some(value.clone()), DockOperation::OuterBottom);
    let alias = args.clone();
    assert!(args == alias);
    assert!(
        args != DockableUndockedEventArgs::new(Some(value.clone()), DockOperation::OuterBottom)
    );
    assert!(Rc::ptr_eq(&alias.dockable().unwrap(), &value));
    assert!(
        DockableUndockedEventArgs::new(None, DockOperation::OuterBottom)
            .dockable()
            .is_none()
    );
    assert_eq!(alias.operation(), DockOperation::OuterBottom);
}

#[test]
fn dockable_unpinned_event_args_shares_the_original_event_instance() {
    let value: Rc<dyn IDockable> = TestNode::new(None);
    let args = DockableUnpinnedEventArgs::new(Some(value.clone()));
    let alias = args.clone();
    assert!(args == alias);
    assert!(args != DockableUnpinnedEventArgs::new(Some(value.clone())));
    assert!(Rc::ptr_eq(&alias.dockable().unwrap(), &value));
    assert!(DockableUnpinnedEventArgs::new(None).dockable().is_none());
}

#[test]
fn window_activated_event_args_shares_the_original_event_instance() {
    let value: Rc<dyn IDockWindow> = TestWindow::new();
    let args = WindowActivatedEventArgs::new(Some(value.clone()));
    let alias = args.clone();
    assert!(args == alias);
    assert!(args != WindowActivatedEventArgs::new(Some(value.clone())));
    assert!(Rc::ptr_eq(&alias.window().unwrap(), &value));
    assert!(WindowActivatedEventArgs::new(None).window().is_none());
}

#[test]
fn window_added_event_args_shares_the_original_event_instance() {
    let value: Rc<dyn IDockWindow> = TestWindow::new();
    let args = WindowAddedEventArgs::new(Some(value.clone()));
    let alias = args.clone();
    assert!(args == alias);
    assert!(args != WindowAddedEventArgs::new(Some(value.clone())));
    assert!(Rc::ptr_eq(&alias.window().unwrap(), &value));
    assert!(WindowAddedEventArgs::new(None).window().is_none());
}

#[test]
fn window_closed_event_args_shares_the_original_event_instance() {
    let value: Rc<dyn IDockWindow> = TestWindow::new();
    let args = WindowClosedEventArgs::new(Some(value.clone()));
    let alias = args.clone();
    assert!(args == alias);
    assert!(args != WindowClosedEventArgs::new(Some(value.clone())));
    assert!(Rc::ptr_eq(&alias.window().unwrap(), &value));
    assert!(WindowClosedEventArgs::new(None).window().is_none());
}

#[test]
fn window_closing_event_args_shares_the_original_event_instance() {
    let value: Rc<dyn IDockWindow> = TestWindow::new();
    let args = WindowClosingEventArgs::new(Some(value.clone()));
    let alias = args.clone();
    assert!(args == alias);
    assert!(args != WindowClosingEventArgs::new(Some(value.clone())));
    assert!(Rc::ptr_eq(&alias.window().unwrap(), &value));
    assert!(WindowClosingEventArgs::new(None).window().is_none());
    assert!(!args.cancel());
    alias.set_cancel(true);
    assert!(args.cancel());
    args.set_cancel(false);
    assert!(!alias.cancel());
}

#[test]
fn window_deactivated_event_args_shares_the_original_event_instance() {
    let value: Rc<dyn IDockWindow> = TestWindow::new();
    let args = WindowDeactivatedEventArgs::new(Some(value.clone()));
    let alias = args.clone();
    assert!(args == alias);
    assert!(args != WindowDeactivatedEventArgs::new(Some(value.clone())));
    assert!(Rc::ptr_eq(&alias.window().unwrap(), &value));
    assert!(WindowDeactivatedEventArgs::new(None).window().is_none());
}

#[test]
fn window_move_drag_begin_event_args_shares_the_original_event_instance() {
    let value: Rc<dyn IDockWindow> = TestWindow::new();
    let args = WindowMoveDragBeginEventArgs::new(Some(value.clone()));
    let alias = args.clone();
    assert!(args == alias);
    assert!(args != WindowMoveDragBeginEventArgs::new(Some(value.clone())));
    assert!(Rc::ptr_eq(&alias.window().unwrap(), &value));
    assert!(WindowMoveDragBeginEventArgs::new(None).window().is_none());
    assert!(!args.cancel());
    alias.set_cancel(true);
    assert!(args.cancel());
    args.set_cancel(false);
    assert!(!alias.cancel());
}

#[test]
fn window_move_drag_end_event_args_shares_the_original_event_instance() {
    let value: Rc<dyn IDockWindow> = TestWindow::new();
    let args = WindowMoveDragEndEventArgs::new(Some(value.clone()));
    let alias = args.clone();
    assert!(args == alias);
    assert!(args != WindowMoveDragEndEventArgs::new(Some(value.clone())));
    assert!(Rc::ptr_eq(&alias.window().unwrap(), &value));
    assert!(WindowMoveDragEndEventArgs::new(None).window().is_none());
}

#[test]
fn window_move_drag_event_args_shares_the_original_event_instance() {
    let value: Rc<dyn IDockWindow> = TestWindow::new();
    let args = WindowMoveDragEventArgs::new(Some(value.clone()));
    let alias = args.clone();
    assert!(args == alias);
    assert!(args != WindowMoveDragEventArgs::new(Some(value.clone())));
    assert!(Rc::ptr_eq(&alias.window().unwrap(), &value));
    assert!(WindowMoveDragEventArgs::new(None).window().is_none());
}

#[test]
fn window_opened_event_args_shares_the_original_event_instance() {
    let value: Rc<dyn IDockWindow> = TestWindow::new();
    let args = WindowOpenedEventArgs::new(Some(value.clone()));
    let alias = args.clone();
    assert!(args == alias);
    assert!(args != WindowOpenedEventArgs::new(Some(value.clone())));
    assert!(Rc::ptr_eq(&alias.window().unwrap(), &value));
    assert!(WindowOpenedEventArgs::new(None).window().is_none());
}

#[test]
fn window_removed_event_args_shares_the_original_event_instance() {
    let value: Rc<dyn IDockWindow> = TestWindow::new();
    let args = WindowRemovedEventArgs::new(Some(value.clone()));
    let alias = args.clone();
    assert!(args == alias);
    assert!(args != WindowRemovedEventArgs::new(Some(value.clone())));
    assert!(Rc::ptr_eq(&alias.window().unwrap(), &value));
    assert!(WindowRemovedEventArgs::new(None).window().is_none());
}
