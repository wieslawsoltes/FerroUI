use crate::controls::{Document, RootDock};
use crate::core::{DockWindow, DockWindowImpl};
use crate::factory_parameter_tests::RecordingFactory;
use crate::test_host::{HostCall, RecordingHost};
use ferrodock_model::adapters::HostAdapter;
use ferrodock_model::collections::ListHandle;
use ferrodock_model::core::*;
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::{reactive_object, IReactiveNotifyPropertyChanged};
use ferroui_system::component_model::INotifyPropertyChanged;
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, panic_any, AssertUnwindSafe};
use std::rc::{Rc, Weak};

#[test]
fn dock_window_defaults_are_correct() {
    let window = DockWindow::new();
    let contract: Rc<dyn IDockWindow> = window.clone();
    assert!(std::ptr::eq(
        contract
            .as_notify_property_changed()
            .unwrap()
            .property_changed(),
        window.property_changed()
    ));
    let host: Rc<dyn IHostWindow> = RecordingHost::new();
    assert!(host
        .boxed_interface(std::any::TypeId::of::<i32>())
        .is_none());
    assert_eq!(window.id(), "IDockWindow");
    assert_eq!(window.title(), "IDockWindow");
    assert_eq!(
        (window.x(), window.y(), window.width(), window.height()),
        (0.0, 0.0, 0.0, 0.0)
    );
    assert_eq!(window.window_state(), DockWindowState::Normal);
    assert!(!window.topmost());
    assert_eq!(window.owner_mode(), DockWindowOwnerMode::Default);
    assert!(window.parent_window().is_none());
    assert!(!window.is_modal());
    assert!(window.show_in_taskbar().is_none());
    assert!(
        window.owner().is_none()
            && window.factory().is_none()
            && window.layout().is_none()
            && window.host().is_none()
    );
    assert!(window.on_close() && window.on_move_drag_begin());
    window.on_move_drag();
    window.on_move_drag_end();
}

#[test]
fn save_updates_position_and_size_from_host() {
    let window = DockWindow::new();
    let host = RecordingHost::new();
    window.set_host(Some(host.clone()));
    host.position.set((10.0, 20.0));
    host.size.set((300.0, 200.0));
    host.state.set(DockWindowState::Maximized);
    let properties = Rc::new(RefCell::new(Vec::new()));
    let log = properties.clone();
    window.property_changed().add(Rc::new(move |property| {
        log.borrow_mut().push(property.to_owned())
    }));
    window.save();
    assert_eq!(
        (window.x(), window.y(), window.width(), window.height()),
        (10.0, 20.0, 300.0, 200.0)
    );
    assert_eq!(window.window_state(), DockWindowState::Maximized);
    assert_eq!(
        *properties.borrow(),
        ["WindowState", "X", "Y", "Width", "Height"]
    );
    assert_eq!(
        *host.calls.borrow(),
        [HostCall::GetState, HostCall::GetPosition, HostCall::GetSize]
    );
}

#[test]
fn present_creates_host_and_presents() {
    let factory = Rc::new(RecordingFactory::default());
    let host = RecordingHost::new();
    *factory.host_result.borrow_mut() = Some(host.clone());
    let window = DockWindow::new();
    let root = RootDock::new();
    window.set_factory(Some(factory.clone()));
    window.set_layout(Some(root.clone()));
    window.set_window_state(DockWindowState::Maximized);
    window.present(false);
    assert_eq!(*factory.legacy_host_requests.borrow(), ["IDockWindow"]);
    assert!(host.tracked.get());
    assert!(Rc::ptr_eq(
        &host.window().unwrap(),
        &(window.clone() as Rc<dyn IDockWindow>)
    ));
    assert!(Rc::ptr_eq(
        host.layout.borrow().as_ref().unwrap(),
        &(root as Rc<dyn IDock>)
    ));
    assert_eq!(host.state.get(), DockWindowState::Maximized);
    assert_eq!(host.title.borrow().as_deref(), Some("IDockWindow"));
    assert_eq!(
        *host.calls.borrow(),
        [
            HostCall::SetWindow,
            HostCall::Position(0.0, 0.0),
            HostCall::Size(0.0, 0.0),
            HostCall::State(DockWindowState::Maximized),
            HostCall::Title(Some("IDockWindow".into())),
            HostCall::Layout,
            HostCall::Tracked(true),
            HostCall::Present(false)
        ]
    );
}

#[test]
fn exit_saves_and_resets_host() {
    let window = DockWindow::new();
    let host = RecordingHost::new();
    window.set_layout(Some(RootDock::new()));
    window.set_host(Some(host.clone()));
    host.position.set((5.0, 6.0));
    host.size.set((100.0, 120.0));
    host.state.set(DockWindowState::FullScreen);
    host.tracked.set(true);
    window.exit();
    assert!(!host.tracked.get());
    assert!(window.host().is_none());
    assert_eq!(
        (window.x(), window.y(), window.width(), window.height()),
        (5.0, 6.0, 100.0, 120.0)
    );
    assert_eq!(window.window_state(), DockWindowState::FullScreen);
    assert_eq!(
        *host.calls.borrow(),
        [
            HostCall::IsTracked,
            HostCall::GetState,
            HostCall::GetPosition,
            HostCall::GetSize,
            HostCall::Tracked(false),
            HostCall::Exit
        ]
    );
}

#[test]
fn operations_without_host_or_layout_do_not_lookup_or_present_a_host() {
    let window = DockWindow::new();
    let factory = Rc::new(RecordingFactory::default());
    let host = RecordingHost::new();
    *factory.host_result.borrow_mut() = Some(host.clone());
    window.set_factory(Some(factory.clone()));
    window.save();
    window.exit();
    window.set_active();
    window.present(false);
    assert!(factory.legacy_host_requests.borrow().is_empty());
    window.set_host(Some(host.clone()));
    window.present(true);
    assert!(host.calls.borrow().is_empty());
    window.set_active();
    assert_eq!(*host.calls.borrow(), [HostCall::Active]);
}

#[test]
fn present_without_factory_or_with_a_null_locator_result_leaves_window_unhosted() {
    let window = DockWindow::new();
    window.set_layout(Some(RootDock::new()));
    window.present(false);
    assert!(window.host().is_none());
    let factory = Rc::new(RecordingFactory::default());
    window.set_factory(Some(factory.clone()));
    window.present(true);
    assert!(window.host().is_none());
    assert_eq!(*factory.legacy_host_requests.borrow(), ["IDockWindow"]);
}

#[test]
fn optional_host_factory_receives_same_model_and_is_preferred_over_id_locator() {
    let window = DockWindow::new();
    let factory = Rc::new(RecordingFactory::default());
    let host = RecordingHost::new();
    factory.supports_host_factory.set(true);
    *factory.host_factory_result.borrow_mut() = Some(host.clone());
    *factory.host_result.borrow_mut() = Some(RecordingHost::new());
    window.set_factory(Some(factory.clone()));
    window.set_layout(Some(RootDock::new()));
    window.present(false);
    assert!(factory.legacy_host_requests.borrow().is_empty());
    assert!(Rc::ptr_eq(
        &factory.host_factory_requests.borrow()[0],
        &(window.clone() as Rc<dyn IDockWindow>)
    ));
    assert!(Rc::ptr_eq(
        &window.host().unwrap(),
        &(host as Rc<dyn IHostWindow>)
    ));
}

#[test]
fn null_optional_factory_result_never_falls_back_to_id_locator() {
    let window = DockWindow::new();
    let factory = Rc::new(RecordingFactory::default());
    factory.supports_host_factory.set(true);
    *factory.host_result.borrow_mut() = Some(RecordingHost::new());
    window.set_factory(Some(factory.clone()));
    window.set_layout(Some(RootDock::new()));
    window.present(false);
    window.present(false);
    assert!(window.host().is_none());
    assert_eq!(factory.host_factory_requests.borrow().len(), 2);
    assert!(factory.legacy_host_requests.borrow().is_empty());
}

#[test]
fn present_reuses_host_and_recreates_with_same_model_after_exit() {
    let window = DockWindow::new();
    let factory = Rc::new(RecordingFactory::default());
    let host = RecordingHost::new();
    factory.supports_host_factory.set(true);
    *factory.host_factory_result.borrow_mut() = Some(host.clone());
    window.set_factory(Some(factory.clone()));
    window.set_layout(Some(RootDock::new()));
    let adapter = HostAdapter::new(window.clone());
    adapter.present(false);
    adapter.present(true);
    assert_eq!(factory.host_factory_requests.borrow().len(), 1);
    assert_eq!(
        host.calls
            .borrow()
            .iter()
            .filter(|call| matches!(call, HostCall::Present(_)))
            .count(),
        2
    );
    assert_eq!(host.calls.borrow().last(), Some(&HostCall::Present(true)));
    adapter.exit();
    assert!(window.host().is_none() && !host.tracked.get());
    adapter.present(false);
    assert_eq!(factory.host_factory_requests.borrow().len(), 2);
    for request in factory.host_factory_requests.borrow().iter() {
        assert!(Rc::ptr_eq(
            request,
            &(window.clone() as Rc<dyn IDockWindow>)
        ));
    }
}

#[test]
fn present_existing_host_does_not_query_factory_or_rewrite_its_window_link() {
    let window = DockWindow::new();
    let factory = Rc::new(RecordingFactory::default());
    let host = RecordingHost::new();
    window.set_host(Some(host.clone()));
    window.set_factory(Some(factory.clone()));
    window.set_layout(Some(RootDock::new()));
    window.present(true);
    assert!(factory.legacy_host_requests.borrow().is_empty());
    assert!(host.window().is_none());
    assert_eq!(host.calls.borrow().last(), Some(&HostCall::Present(true)));
}

#[test]
fn host_property_notification_can_replace_new_host_before_window_link_is_assigned() {
    let window = DockWindow::new();
    let factory = Rc::new(RecordingFactory::default());
    let original = RecordingHost::new();
    let replacement = RecordingHost::new();
    *factory.host_result.borrow_mut() = Some(original.clone());
    window.set_factory(Some(factory.clone()));
    window.set_layout(Some(RootDock::new()));
    let weak = Rc::downgrade(&window);
    let next = replacement.clone();
    let once = Rc::new(Cell::new(false));
    window.property_changed().add(Rc::new(move |property| {
        if property == "Host" && !once.replace(true) {
            weak.upgrade().unwrap().set_host(Some(next.clone()));
        }
    }));
    window.present(false);
    assert!(original.calls.borrow().is_empty());
    assert!(Rc::ptr_eq(
        &replacement.window().unwrap(),
        &(window as Rc<dyn IDockWindow>)
    ));
    assert_eq!(
        replacement.calls.borrow().last(),
        Some(&HostCall::Present(false))
    );
}

#[test]
fn save_reloads_host_after_reentrant_window_property_notifications() {
    let window = DockWindow::new();
    let first = RecordingHost::new();
    let second = RecordingHost::new();
    let third = RecordingHost::new();
    first.state.set(DockWindowState::Maximized);
    second.position.set((10.0, 20.0));
    third.size.set((300.0, 200.0));
    window.set_host(Some(first.clone()));
    let weak = Rc::downgrade(&window);
    let b = second.clone();
    let c = third.clone();
    window.property_changed().add(Rc::new(move |name| {
        let window = weak.upgrade().unwrap();
        match name {
            "WindowState" => window.set_host(Some(b.clone())),
            "X" => window.set_host(Some(c.clone())),
            _ => {}
        }
    }));
    window.save();
    assert_eq!(*first.calls.borrow(), [HostCall::GetState]);
    assert_eq!(*second.calls.borrow(), [HostCall::GetPosition]);
    assert_eq!(*third.calls.borrow(), [HostCall::GetSize]);
    assert_eq!(
        (window.x(), window.y(), window.width(), window.height()),
        (10.0, 20.0, 300.0, 200.0)
    );
}

#[test]
fn present_reloads_host_between_each_host_operation() {
    let window = DockWindow::new();
    let first = RecordingHost::new();
    let second = RecordingHost::new();
    let third = RecordingHost::new();
    window.set_layout(Some(RootDock::new()));
    window.set_host(Some(first.clone()));
    window.set_x(10.0);
    window.set_y(20.0);
    window.set_width(300.0);
    window.set_height(200.0);
    let weak = Rc::downgrade(&window);
    let next = second.clone();
    *first.on_call.borrow_mut() = Some(Rc::new(move |call| {
        if matches!(call, HostCall::Position(..)) {
            weak.upgrade().unwrap().set_host(Some(next.clone()));
        }
    }));
    let weak = Rc::downgrade(&window);
    let next = third.clone();
    *second.on_call.borrow_mut() = Some(Rc::new(move |call| {
        if matches!(call, HostCall::Size(..)) {
            weak.upgrade().unwrap().set_host(Some(next.clone()));
        }
    }));
    window.present(true);
    assert_eq!(*first.calls.borrow(), [HostCall::Position(10.0, 20.0)]);
    assert_eq!(*second.calls.borrow(), [HostCall::Size(300.0, 200.0)]);
    assert_eq!(
        *third.calls.borrow(),
        [
            HostCall::State(DockWindowState::Normal),
            HostCall::Title(Some("IDockWindow".into())),
            HostCall::Layout,
            HostCall::Tracked(true),
            HostCall::Present(true)
        ]
    );
}

#[test]
fn canceled_close_preserves_host_and_restores_its_original_tracking_flag() {
    for tracked in [false, true] {
        let window = DockWindow::new();
        let host = RecordingHost::new();
        host.closed.set(Some(false));
        host.tracked.set(tracked);
        window.set_host(Some(host.clone()));
        window.exit();
        assert!(Rc::ptr_eq(
            &window.host().unwrap(),
            &(host.clone() as Rc<dyn IHostWindow>)
        ));
        assert_eq!(host.tracked.get(), tracked);
        assert_eq!(
            host.calls.borrow().last(),
            Some(&HostCall::Tracked(tracked))
        );
        host.closed.set(Some(true));
        window.exit();
        assert!(window.host().is_none());
    }
}

#[test]
fn already_closed_host_is_saved_and_cleared_without_another_exit() {
    let window = DockWindow::new();
    let host = RecordingHost::new();
    host.closed.set(Some(true));
    host.tracked.set(true);
    window.set_host(Some(host.clone()));
    window.exit();
    assert!(window.host().is_none() && !host.tracked.get());
    assert!(!host.calls.borrow().contains(&HostCall::Exit));
    assert!(host.calls.borrow().contains(&HostCall::GetSize));
}

#[test]
fn completed_close_clears_host_but_never_clears_a_replacement() {
    for report_state in [false, true] {
        let window = DockWindow::new();
        let host = RecordingHost::new();
        let replacement = RecordingHost::new();
        if report_state {
            host.closed.set(Some(false));
        }
        window.set_host(Some(host.clone()));
        let weak_window = Rc::downgrade(&window);
        let weak_host = Rc::downgrade(&host);
        let next = replacement.clone();
        *host.on_call.borrow_mut() = Some(Rc::new(move |call| {
            if *call == HostCall::Exit {
                if report_state {
                    weak_host.upgrade().unwrap().closed.set(Some(true));
                }
                weak_window.upgrade().unwrap().set_host(Some(next.clone()));
            }
        }));
        window.exit();
        assert!(Rc::ptr_eq(
            &window.host().unwrap(),
            &(replacement as Rc<dyn IHostWindow>)
        ));
        assert!(!host.tracked.get());
    }
}

#[test]
fn reentrant_exit_is_ignored_and_the_guard_is_reset_after_success() {
    let window = DockWindow::new();
    let host = RecordingHost::new();
    window.set_host(Some(host.clone()));
    let weak = Rc::downgrade(&window);
    *host.on_call.borrow_mut() = Some(Rc::new(move |call| {
        if *call == HostCall::Exit {
            weak.upgrade().unwrap().exit();
        }
    }));
    window.exit();
    assert_eq!(
        host.calls
            .borrow()
            .iter()
            .filter(|call| **call == HostCall::Exit)
            .count(),
        1
    );
    window.set_host(Some(host.clone()));
    window.exit();
    assert_eq!(
        host.calls
            .borrow()
            .iter()
            .filter(|call| **call == HostCall::Exit)
            .count(),
        2
    );
}

#[test]
fn host_failures_restore_tracking_rethrow_same_payload_and_reset_exit_guard() {
    for failing_call in [HostCall::GetState, HostCall::Exit] {
        let window = DockWindow::new();
        let host = RecordingHost::new();
        host.tracked.set(true);
        window.set_host(Some(host.clone()));
        *host.on_call.borrow_mut() = Some(Rc::new(move |call| {
            if *call == failing_call {
                panic_any(42_usize);
            }
        }));
        let error = catch_unwind(AssertUnwindSafe(|| window.exit())).unwrap_err();
        assert_eq!(error.downcast_ref::<usize>(), Some(&42));
        assert!(host.tracked.get() && window.host().is_some());
        *host.on_call.borrow_mut() = None;
        window.exit();
        assert!(window.host().is_none());
    }
}

#[test]
fn failure_after_host_closed_does_not_restore_tracking_and_allows_cleanup_retry() {
    let window = DockWindow::new();
    let host = RecordingHost::new();
    host.tracked.set(true);
    host.closed.set(Some(false));
    window.set_host(Some(host.clone()));
    let weak = Rc::downgrade(&host);
    *host.on_call.borrow_mut() = Some(Rc::new(move |call| {
        if *call == HostCall::Exit {
            weak.upgrade().unwrap().closed.set(Some(true));
            panic_any(99_usize);
        }
    }));
    let error = catch_unwind(AssertUnwindSafe(|| window.exit())).unwrap_err();
    assert_eq!(error.downcast_ref::<usize>(), Some(&99));
    assert!(!host.tracked.get() && window.host().is_some());
    *host.on_call.borrow_mut() = None;
    window.exit();
    assert!(window.host().is_none());
}

#[test]
fn tracking_restore_failure_still_resets_guard_and_propagates_catch_failure() {
    let window = DockWindow::new();
    let host = RecordingHost::new();
    host.tracked.set(true);
    host.closed.set(Some(false));
    window.set_host(Some(host.clone()));
    let restorations = Rc::new(Cell::new(0_usize));
    let count = restorations.clone();
    *host.on_call.borrow_mut() = Some(Rc::new(move |call| {
        if *call == HostCall::Tracked(true) {
            count.set(count.get() + 1);
            panic_any(count.get());
        }
    }));
    let error = catch_unwind(AssertUnwindSafe(|| window.exit())).unwrap_err();
    assert_eq!(error.downcast_ref::<usize>(), Some(&2));
    assert_eq!(restorations.get(), 2);
    *host.on_call.borrow_mut() = None;
    host.closed.set(Some(true));
    window.exit();
    assert!(window.host().is_none());
}

#[test]
fn window_retains_serialized_parent_layout_and_host_until_links_are_cleared() {
    let window = DockWindow::new();
    let weak_window = Rc::downgrade(&window);
    let root = RootDock::new();
    let weak_root = Rc::downgrade(&root);
    let host = RecordingHost::new();
    let weak_host = Rc::downgrade(&host);
    root.set_window(Some(window.clone()));
    host.set_window(Some(window.clone()));
    window.set_layout(Some(root));
    window.set_host(Some(host));
    let parent = DockWindow::new();
    let weak_parent = Rc::downgrade(&parent);
    window.set_parent_window(Some(parent));
    let owner = Document::new();
    let weak_owner = Rc::downgrade(&owner);
    window.set_owner(Some(owner));
    let factory = Rc::new(RecordingFactory::default());
    let weak_factory = Rc::downgrade(&factory);
    window.set_factory(Some(factory));
    assert!(
        weak_parent.upgrade().is_some()
            && weak_owner.upgrade().is_none()
            && weak_factory.upgrade().is_none()
    );
    assert!(weak_root.upgrade().is_some() && weak_host.upgrade().is_some());
    // Isolate ownership from the separately tracked CLR cycle-collection gap.
    window.layout().unwrap().set_window(None);
    drop(window);
    assert!(weak_parent.upgrade().is_none());
    assert!(
        weak_window.upgrade().is_none()
            && weak_root.upgrade().is_none()
            && weak_host.upgrade().is_none()
    );
}

#[test]
fn standalone_adapter_retains_its_source_window_until_released() {
    let window = DockWindow::new();
    let weak = Rc::downgrade(&window);
    let adapter = HostAdapter::new(window);
    assert!(weak.upgrade().is_some());
    adapter.save();
    drop(adapter);
    assert!(weak.upgrade().is_none());
}

#[test]
fn embedded_adapter_keeps_a_nonowning_window_link() {
    let window = DockWindow::new();
    let weak = Rc::downgrade(&window);
    let erased: Rc<dyn IDockWindow> = window;
    let _adapter = HostAdapter::from_weak(Rc::downgrade(&erased));
    drop(erased);
    assert!(weak.upgrade().is_none());
}

struct CustomWindow {
    base: DockWindow,
}
reactive_object!(CustomWindow, base);
crate::impl_dock_window!(CustomWindow);
impl DockWindowImpl for CustomWindow {
    fn on_close(_this: &Self) -> bool {
        false
    }
    fn on_move_drag_begin(_this: &Self) -> bool {
        false
    }
    fn on_move_drag(this: &Self) {
        this.set_title("moving".into());
    }
    fn on_move_drag_end(this: &Self) {
        this.set_title("ended".into());
    }
}

#[test]
fn composed_window_preserves_virtual_callbacks_and_concrete_reactive_sender() {
    let model = Rc::new_cyclic(|weak: &Weak<CustomWindow>| CustomWindow {
        base: DockWindow::construct(weak.clone()),
    });
    let log = Rc::new(RefCell::new(Vec::new()));
    let events = log.clone();
    let weak = Rc::downgrade(&model);
    let subscription = model.changed().subscribe_next(move |change| {
        assert!(Rc::ptr_eq(&change.sender, &weak.upgrade().unwrap()));
        events.borrow_mut().push(change.property_name);
    });
    assert!(!model.base.on_close() && !model.base.on_move_drag_begin());
    model.base.on_move_drag();
    assert_eq!(model.title(), "moving");
    model.base.on_move_drag_end();
    assert_eq!(model.title(), "ended");
    model.base.set_width(500.0);
    let erased: Rc<dyn IDockWindow> = model.clone();
    assert!(std::ptr::eq(
        erased
            .as_notify_property_changed()
            .unwrap()
            .property_changed(),
        model.property_changed()
    ));
    assert!(!erased.on_close());
    erased.on_move_drag();
    assert_eq!(model.title(), "moving");
    assert_eq!(*log.borrow(), ["Title", "Title", "Width", "Title"]);
    subscription.dispose();
}

#[test]
fn exit_keeps_the_original_host_for_close_even_when_save_replaces_it() {
    let window = DockWindow::new();
    let original = RecordingHost::new();
    let replacement = RecordingHost::new();
    original.tracked.set(true);
    original.state.set(DockWindowState::Maximized);
    replacement.position.set((10.0, 20.0));
    replacement.size.set((300.0, 200.0));
    window.set_host(Some(original.clone()));
    let weak = Rc::downgrade(&window);
    let next = replacement.clone();
    window.property_changed().add(Rc::new(move |name| {
        if name == "WindowState" {
            weak.upgrade().unwrap().set_host(Some(next.clone()));
        }
    }));
    window.exit();
    assert_eq!(
        *original.calls.borrow(),
        [
            HostCall::IsTracked,
            HostCall::GetState,
            HostCall::Tracked(false),
            HostCall::Exit
        ]
    );
    assert_eq!(
        *replacement.calls.borrow(),
        [HostCall::GetPosition, HostCall::GetSize]
    );
    assert!(Rc::ptr_eq(
        &window.host().unwrap(),
        &(replacement as Rc<dyn IHostWindow>)
    ));
}

#[test]
fn clearing_host_during_save_restores_the_captured_hosts_tracking_and_resets_guard() {
    let window = DockWindow::new();
    let host = RecordingHost::new();
    host.tracked.set(true);
    host.state.set(DockWindowState::Maximized);
    window.set_host(Some(host.clone()));
    let weak = Rc::downgrade(&window);
    window.property_changed().add(Rc::new(move |name| {
        if name == "WindowState" {
            weak.upgrade().unwrap().set_host(None);
        }
    }));
    assert!(catch_unwind(AssertUnwindSafe(|| window.exit())).is_err());
    assert!(window.host().is_none() && host.tracked.get());
    window.set_host(Some(host.clone()));
    window.exit();
    assert!(window.host().is_none() && !host.tracked.get());
}

#[test]
fn close_state_is_queried_again_after_the_exit_decision() {
    let window = DockWindow::new();
    let host = RecordingHost::new();
    host.closed.set(Some(false));
    host.tracked.set(true);
    window.set_host(Some(host.clone()));
    let weak = Rc::downgrade(&host);
    let queries = Rc::new(Cell::new(0));
    let count = queries.clone();
    *host.on_call.borrow_mut() = Some(Rc::new(move |call| {
        if *call == HostCall::IsClosed {
            count.set(count.get() + 1);
            weak.upgrade().unwrap().closed.set(Some(count.get() == 1));
        }
    }));
    window.exit();
    assert_eq!(queries.get(), 2);
    assert!(!host.calls.borrow().contains(&HostCall::Exit));
    assert!(host.tracked.get() && window.host().is_some());
}

#[test]
fn present_uses_the_explicit_dialog_argument() {
    let window = DockWindow::new();
    let host = RecordingHost::new();
    window.set_host(Some(host.clone()));
    window.set_layout(Some(RootDock::new()));
    window.set_is_modal(true);
    window.present(false);
    assert_eq!(host.calls.borrow().last(), Some(&HostCall::Present(false)));
    assert!(window.is_modal());
}

#[test]
fn window_options_apply_to_concrete_reactive_model_in_source_order() {
    let window = DockWindow::new();
    let parent = DockWindow::new();
    let options = DockWindowOptions::new();
    options.set_owner_mode(DockWindowOwnerMode::ParentWindow);
    options.set_parent_window(Some(parent.clone()));
    options.set_is_modal(true);
    options.set_show_in_taskbar(Some(false));
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    window
        .property_changed()
        .add(Rc::new(move |name| log.borrow_mut().push(name.to_owned())));
    options.apply_to(Some(window.as_ref()));
    assert_eq!(
        *changes.borrow(),
        ["OwnerMode", "ParentWindow", "IsModal", "ShowInTaskbar"]
    );
    assert!(Rc::ptr_eq(
        &window.parent_window().unwrap(),
        &(parent as Rc<dyn IDockWindow>)
    ));
    assert!(window.is_modal());
    assert_eq!(window.show_in_taskbar(), Some(false));
    DockWindowOptions::new().apply_to(Some(window.as_ref()));
    assert_eq!(window.show_in_taskbar(), Some(false));
}

#[test]
fn root_window_commands_present_and_save_concrete_floating_window_models() {
    let workspace = RootDock::new();
    let window = DockWindow::new();
    let floating = RootDock::new();
    let factory = Rc::new(RecordingFactory::default());
    let host = RecordingHost::new();
    *factory.host_result.borrow_mut() = Some(host.clone());
    window.set_factory(Some(factory.clone()));
    window.set_layout(Some(floating.clone()));
    floating.set_window(Some(window.clone()));
    window.set_is_modal(true);
    workspace.set_windows(Some(ListHandle::from_items(vec![Some(
        window.clone() as Rc<dyn IDockWindow>
    )])));
    workspace.show_windows().execute(None);
    assert_eq!(host.calls.borrow().last(), Some(&HostCall::Present(true)));
    host.position.set((20.0, 30.0));
    host.size.set((400.0, 500.0));
    host.state.set(DockWindowState::Maximized);
    workspace.exit_windows().execute(None);
    assert_eq!(
        (window.x(), window.y(), window.width(), window.height()),
        (20.0, 30.0, 400.0, 500.0)
    );
    assert_eq!(window.window_state(), DockWindowState::Maximized);
    assert!(window.host().is_none() && !host.tracked.get());
    assert_eq!(
        host.calls
            .borrow()
            .iter()
            .filter(|call| **call == HostCall::GetState)
            .count(),
        2
    ); // NavigateAdapter saves before DockWindow.Exit saves again.
}

#[test]
fn factory_base_initializes_optional_host_links_and_passes_existing_layout_owner() {
    use crate::factory_parameter_tests::ModelCall;
    let factory = RecordingFactory::with_base();
    factory.supports_host_factory.set(true);
    let host = RecordingHost::new();
    *factory.host_factory_result.borrow_mut() = Some(host.clone());
    let window = DockWindow::new();
    let layout = RootDock::new();
    let layout_owner = RootDock::new();
    let owner = RootDock::new();
    layout.set_owner(Some(layout_owner.clone()));
    window.set_layout(Some(layout.clone()));
    factory.init_dock_window(Some(window.clone()), Some(owner.clone()));
    let erased: Rc<dyn IDockWindow> = window.clone();
    assert!(Rc::ptr_eq(
        &factory.host_factory_requests.borrow()[0],
        &erased
    ));
    assert!(factory.legacy_host_requests.borrow().is_empty());
    assert!(Rc::ptr_eq(&host.window().unwrap(), &erased));
    assert!(Rc::ptr_eq(
        &window.owner().unwrap(),
        &(owner as Rc<dyn IDockable>)
    ));
    assert!(Rc::ptr_eq(
        &window.factory().unwrap(),
        &(factory.clone() as Rc<dyn IFactory>)
    ));
    assert!(
        matches!(&factory.model_calls.borrow()[0], ModelCall::InitDockable(value, Some(parent)) if Rc::ptr_eq(value, &(layout as Rc<dyn IDockable>)) && Rc::ptr_eq(parent, &(layout_owner as Rc<dyn IDockable>)))
    );
}

#[test]
fn explicit_factory_base_host_initialization_skips_optional_creation_and_accepts_absent_layout() {
    let factory = RecordingFactory::with_base();
    factory.supports_host_factory.set(true);
    let window = DockWindow::new();
    let host = RecordingHost::new();
    factory.init_dock_window_with_host(Some(window.clone()), None, Some(host.clone()));
    assert!(
        factory.host_factory_requests.borrow().is_empty()
            && factory.legacy_host_requests.borrow().is_empty()
    );
    assert!(window.owner().is_none() && window.factory().is_some());
    assert!(Rc::ptr_eq(
        &window.host().unwrap(),
        &(host as Rc<dyn IHostWindow>)
    ));
    assert!(factory.model_calls.borrow().is_empty());
}

#[test]
fn factory_base_and_host_adapter_share_the_source_legacy_locator_path() {
    use ferroui_system::collections::generic::Dictionary;
    let factory = RecordingFactory::with_base();
    let host = RecordingHost::new();
    let result = host.clone();
    let locators = Rc::new(RefCell::new(Dictionary::new()));
    locators.borrow_mut().add(
        "special".into(),
        Some(Rc::new(move || Some(result.clone() as Rc<dyn IHostWindow>)) as HostWindowFactory),
    );
    factory.set_host_window_locator(Some(locators));
    let window = DockWindow::new();
    window.set_id("special".into());
    window.set_layout(Some(RootDock::new()));
    factory.init_dock_window(Some(window.clone()), None);
    window.exit();
    window.present(false);
    assert_eq!(
        *factory.legacy_host_requests.borrow(),
        ["special", "special"]
    );
    assert!(Rc::ptr_eq(
        &window.host().unwrap(),
        &(host as Rc<dyn IHostWindow>)
    ));
}

#[test]
fn null_optional_host_in_factory_base_initialization_never_uses_a_second_locator() {
    let factory = RecordingFactory::with_base();
    factory.supports_host_factory.set(true);
    factory.set_default_host_window_locator(Some(Rc::new(|| panic!("unexpected legacy fallback"))));
    let window = DockWindow::new();
    window.set_layout(Some(RootDock::new()));
    factory.init_dock_window(Some(window.clone()), None);
    window.present(false);
    assert!(window.host().is_none());
    assert_eq!(factory.host_factory_requests.borrow().len(), 2);
    assert!(factory.legacy_host_requests.borrow().is_empty());
}

#[test]
fn presenting_windows_uses_plain_list_enumeration_instead_of_a_snapshot() {
    use ferrodock_model::adapters::NavigateAdapter;
    let root = RootDock::new();
    let window = DockWindow::new();
    let host = RecordingHost::new();
    window.set_layout(Some(RootDock::new()));
    window.set_host(Some(host.clone()));
    let windows = ListHandle::from_plain(vec![Some(window as Rc<dyn IDockWindow>)]);
    root.set_windows(Some(windows.clone()));
    let weak = Rc::downgrade(&windows.as_list());
    *host.on_call.borrow_mut() = Some(Rc::new(move |call| {
        if matches!(call, HostCall::Present(_)) {
            weak.upgrade().unwrap().clear();
        }
    }));
    let adapter = NavigateAdapter::new(root);
    assert!(catch_unwind(AssertUnwindSafe(|| adapter.show_windows())).is_err());
    assert!(windows.is_empty());
}
