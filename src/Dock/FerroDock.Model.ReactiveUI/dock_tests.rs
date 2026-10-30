use crate::controls::*;
use crate::core::{DockBase, DockableBaseImpl};
use crate::factory_parameter_tests::{ModelCall, RecordingFactory};
use crate::test_support::TestWindow;
use ferrodock_model::collections::ListHandle;
use ferrodock_model::core::*;
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::{reactive_object, ReactiveUIBindingExtensions};
use ferroui_system::component_model::INotifyPropertyChanged;
use ferroui_system::prop;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

#[test]
fn dock_dock_default_last_child_fill_true() {
    assert!(DockDock::new().last_child_fill());
}
#[test]
fn stack_dock_defaults() {
    let dock = StackDock::new();
    assert_eq!(dock.orientation(), Orientation::Horizontal);
    assert_eq!(dock.spacing(), 0.0);
}
#[test]
fn wrap_dock_defaults() {
    assert_eq!(WrapDock::new().orientation(), Orientation::Horizontal);
}
#[test]
fn uniform_grid_dock_defaults() {
    let dock = UniformGridDock::new();
    assert_eq!((dock.rows(), dock.columns()), (0, 0));
}
#[test]
fn grid_dock_defaults() {
    let dock = GridDock::new();
    assert!(dock.column_definitions().is_none() && dock.row_definitions().is_none());
}
#[test]
fn proportional_dock_default_orientation_horizontal() {
    assert_eq!(
        ProportionalDock::new().orientation(),
        Orientation::Horizontal
    );
}
#[test]
fn tool_dock_defaults() {
    let dock = ToolDock::new();
    assert_eq!(dock.alignment(), Alignment::Unset);
    assert!(!dock.is_expanded());
    assert!(dock.auto_hide());
    assert_eq!(dock.grip_mode(), GripMode::Visible);
}

fn all_docks() -> Vec<Rc<dyn IDock>> {
    vec![
        DockDock::new(),
        StackDock::new(),
        WrapDock::new(),
        UniformGridDock::new(),
        GridDock::new(),
        ProportionalDock::new(),
        ToolDock::new(),
        SplitViewDock::new(),
        RootDock::new(),
        DocumentDock::new(),
    ]
}

#[test]
fn every_container_preserves_the_base_defaults_and_dock_identity() {
    for dock in all_docks() {
        assert!(
            dock.visible_dockables().is_none()
                && dock.active_dockable().is_none()
                && dock.default_dockable().is_none()
                && dock.focused_dockable().is_none()
        );
        assert!(!dock.is_active());
        assert_eq!(dock.opened_dockables_count(), 0);
        assert!(dock.can_close_last_dockable() && dock.enable_global_docking());
        assert!(dock.dock_capability_policy().is_none());
        assert!(!dock.can_go_back() && !dock.can_go_forward());
        for command in [
            dock.go_back(),
            dock.go_forward(),
            dock.navigate(),
            dock.close(),
        ] {
            assert!(command.can_execute(None));
        }
        assert_eq!(dock.id(), "");
        assert!(dock.proportion().is_nan());
        assert!(dock.as_docking_window_state().is_none());
        assert!(dock.clone().as_dock().is_some());
    }
    assert!(DockDock::new().as_dock_dock().is_some());
    assert!(StackDock::new().as_stack_dock().is_some());
    assert!(WrapDock::new().as_wrap_dock().is_some());
    assert!(GridDock::new().as_grid_dock().is_some());
    assert!(UniformGridDock::new().as_uniform_grid_dock().is_some());
    assert!(ProportionalDock::new().as_proportional_dock().is_some());
    assert!(ToolDock::new().as_tool_dock().is_some());
    assert!(SplitViewDock::new().as_split_view_dock().is_some());
    assert!(RootDock::new().as_root_dock().is_some());
}

#[test]
fn visible_list_is_a_shared_observable_handle_with_identity_based_replacement() {
    let dock = StackDock::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    dock.property_changed()
        .add(Rc::new(move |name| log.borrow_mut().push(name.to_owned())));
    let list = ListHandle::new();
    let erased: Rc<dyn IDock> = dock.clone();
    erased.set_visible_dockables(Some(list.clone()));
    erased.set_visible_dockables(Some(list.clone()));
    list.add(Some(Document::new()));
    assert_eq!(erased.visible_dockables().unwrap().count(), 1);
    assert!(erased.visible_dockables().unwrap().ptr_eq(&list));
    erased.set_visible_dockables(Some(ListHandle::new()));
    erased.set_visible_dockables(None);
    assert_eq!(
        *changes.borrow(),
        ["VisibleDockables", "VisibleDockables", "VisibleDockables"]
    );
}

#[test]
fn activation_and_focus_subscriptions_use_the_current_factory_and_concrete_dock_identity() {
    let dock = ToolDock::new();
    let factory = Rc::new(RecordingFactory::default());
    dock.set_factory(Some(factory.clone()));
    assert!(factory.model_calls.borrow().is_empty()); // Factory assignment does not replay the observed values.
    let active = Tool::new();
    let other = Tool::new();
    dock.set_active_dockable(Some(active.clone()));
    dock.set_active_dockable(Some(active.clone()));
    dock.set_focused_dockable(Some(active.clone()));
    dock.set_focused_dockable(Some(active.clone()));
    dock.set_active_dockable(Some(other));
    dock.set_active_dockable(None);
    dock.set_focused_dockable(None);
    let calls = factory.model_calls.borrow();
    assert_eq!(calls.len(), 5);
    let erased: Rc<dyn IDock> = dock.clone();
    let active: Rc<dyn IDockable> = active;
    assert!(
        matches!(&calls[0], ModelCall::InitActive(Some(value), owner) if Rc::ptr_eq(value, &active) && Rc::ptr_eq(owner, &erased))
    );
    assert!(
        matches!(&calls[1], ModelCall::FocusedChanged(Some(value)) if Rc::ptr_eq(value, &active))
    );
    assert!(matches!(&calls[3], ModelCall::InitActive(None, owner) if Rc::ptr_eq(owner, &erased)));
    assert!(matches!(&calls[4], ModelCall::FocusedChanged(None)));
}

#[test]
fn factory_initialization_precedes_history_notifications_and_can_reenter_the_model() {
    let dock = StackDock::new();
    let factory = Rc::new(RecordingFactory::default());
    dock.set_factory(Some(factory.clone()));
    let events = Rc::new(RefCell::new(Vec::new()));
    let log = events.clone();
    *factory.on_init_active.borrow_mut() = Some(Rc::new(move |_, dock| {
        log.borrow_mut().push("factory".to_owned());
        dock.set_enable_global_docking(false);
    }));
    let log = events.clone();
    dock.property_changed()
        .add(Rc::new(move |name| log.borrow_mut().push(name.to_owned())));
    dock.set_active_dockable(Some(Document::new()));
    assert_eq!(
        *events.borrow(),
        [
            "factory",
            "EnableGlobalDocking",
            "CanGoBack",
            "CanGoForward",
            "ActiveDockable"
        ]
    );
}

#[test]
fn observed_active_changes_keep_distinct_filtering_during_notification_suppression() {
    use ferroui_reactiveui::IReactiveNotifyPropertyChanged;
    let dock = StackDock::new();
    let factory = Rc::new(RecordingFactory::default());
    dock.set_factory(Some(factory.clone()));
    let suppression = dock.suppress_change_notifications();
    dock.set_active_dockable(Some(Document::new()));
    dock.set_focused_dockable(Some(Document::new()));
    assert!(factory.model_calls.borrow().is_empty());
    suppression.dispose();
    assert!(factory.model_calls.borrow().is_empty());
    dock.set_active_dockable(None);
    // The observable last saw None; WhenAnyValue drops this repeat even though
    // the field changed while notifications were suppressed.
    assert!(factory.model_calls.borrow().is_empty());
    dock.set_active_dockable(Some(Document::new()));
    assert_eq!(factory.model_calls.borrow().len(), 1);
}

#[test]
fn navigation_commands_accept_ids_erased_handles_and_boxed_model_parameters() {
    let dock = StackDock::new();
    let first = Document::new();
    first.set_id("first".into());
    let second = Document::new();
    second.set_id("second".into());
    dock.set_visible_dockables(Some(ListHandle::from_items(vec![
        Some(first.clone() as Rc<dyn IDockable>),
        Some(second.clone()),
    ])));
    let navigate = dock.navigate();
    assert!(Rc::ptr_eq(&navigate, &dock.navigate()));
    navigate.execute(Some(&"first".to_owned()));
    assert!(Rc::ptr_eq(
        &dock.active_dockable().unwrap(),
        &(first.clone() as Rc<dyn IDockable>)
    ));
    navigate.execute(Some(&(second.clone() as Rc<dyn IDockable>)));
    assert!(dock.can_go_back());
    dock.go_back().execute(None);
    assert!(dock.can_go_forward());
    dock.go_forward().execute(None);
    assert!(Rc::ptr_eq(
        &dock.active_dockable().unwrap(),
        &(second.clone() as Rc<dyn IDockable>)
    ));
    navigate.execute(Some(&first.boxed()));
    assert!(Rc::ptr_eq(
        &dock.active_dockable().unwrap(),
        &(first as Rc<dyn IDockable>)
    ));
    navigate.execute(None);
    assert!(dock.active_dockable().is_none());
    assert!(!dock.can_go_back() && !dock.can_go_forward());
}

#[test]
fn concrete_reactive_paths_keep_the_container_as_notification_sender() {
    let dock = GridDock::new();
    let values = Rc::new(RefCell::new(Vec::new()));
    let log = values.clone();
    let subscription = dock
        .when_any_value(prop!(GridDock => active_dockable))
        .subscribe_next(move |value: Option<Rc<dyn IDockable>>| log.borrow_mut().push(value));
    dock.set_active_dockable(Some(Document::new()));
    dock.set_active_dockable(None);
    assert_eq!(values.borrow().len(), 3);
    assert!(values.borrow()[0].is_none() && values.borrow()[2].is_none());
    subscription.dispose();
}

#[test]
fn policy_assignment_compares_reference_identity_even_for_equal_values() {
    let dock = GridDock::new();
    let changes = Rc::new(Cell::new(0));
    let log = changes.clone();
    dock.property_changed()
        .add(Rc::new(move |_| log.set(log.get() + 1)));
    let first = DockCapabilityPolicy::new();
    let second = DockCapabilityPolicy::new();
    dock.set_dock_capability_policy(Some(first.clone()));
    dock.set_dock_capability_policy(Some(first.clone()));
    dock.set_dock_capability_policy(Some(second));
    dock.set_dock_capability_policy(None);
    assert_eq!(changes.get(), 3);
    let erased: Rc<dyn DockCapabilityPolicyImpl> = first.clone();
    assert!(erased == first as Rc<dyn DockCapabilityPolicyImpl>);
}

#[test]
fn commands_and_observations_do_not_keep_the_containing_dock_alive() {
    let dock = StackDock::new();
    let weak = Rc::downgrade(&dock);
    let command = dock.navigate();
    dock.set_active_dockable(Some(Document::new()));
    drop(dock);
    assert!(weak.upgrade().is_none());
    command.execute(None); // Native weak callbacks remain safe after their owner is released.
}

#[test]
fn split_view_defaults_and_actions_preserve_reactive_pane_state() {
    let dock = SplitViewDock::new();
    assert_eq!(dock.compact_pane_length(), 48.0);
    assert_eq!(dock.open_pane_length(), 320.0);
    assert_eq!(dock.display_mode(), SplitViewDisplayMode::Overlay);
    assert_eq!(dock.pane_placement(), SplitViewPanePlacement::Left);
    assert!(!dock.is_pane_open() && !dock.use_light_dismiss_overlay_mode());
    assert!(dock.pane_dockable().is_none() && dock.content_dockable().is_none());
    let values = Rc::new(RefCell::new(Vec::new()));
    let log = values.clone();
    let subscription = dock
        .when_any_value(prop!(SplitViewDock => is_pane_open))
        .subscribe_next(move |value| log.borrow_mut().push(value));
    dock.open_pane();
    dock.open_pane();
    dock.toggle_pane();
    dock.toggle_pane();
    dock.close_pane();
    assert_eq!(*values.borrow(), [false, true, false, true, false]);
    subscription.dispose();
}

#[test]
fn navigation_reaches_concrete_split_view_pane_and_content_children() {
    let root = RootDock::new();
    let split = SplitViewDock::new();
    let pane = Tool::new();
    let content = Document::new();
    pane.set_id("pane".into());
    content.set_id("content".into());
    split.set_pane_dockable(Some(pane.clone()));
    split.set_content_dockable(Some(content.clone()));
    root.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
        split as Rc<dyn IDockable>,
    )])));
    root.navigate().execute(Some(&"pane".to_owned()));
    assert!(Rc::ptr_eq(
        &root.active_dockable().unwrap(),
        &(pane as Rc<dyn IDockable>)
    ));
    root.navigate().execute(Some(&"content".to_owned()));
    assert!(Rc::ptr_eq(
        &root.active_dockable().unwrap(),
        &(content as Rc<dyn IDockable>)
    ));
}

#[test]
fn root_defaults_and_window_commands_use_the_existing_navigation_adapter() {
    let root = RootDock::new();
    assert!(root.is_focusable_root());
    assert_eq!(
        root.pinned_dock_display_mode(),
        PinnedDockDisplayMode::Overlay
    );
    assert_eq!(
        root.floating_window_host_mode(),
        DockFloatingWindowHostMode::Default
    );
    assert!(!root.enable_adaptive_global_dock_targets());
    assert!(
        root.hidden_dockables().is_none()
            && root.left_pinned_dockables().is_none()
            && root.right_pinned_dockables().is_none()
            && root.top_pinned_dockables().is_none()
            && root.bottom_pinned_dockables().is_none()
    );
    assert!(
        root.pinned_dock().is_none()
            && root.window().is_none()
            && root.windows().is_none()
            && root.root_dock_capability_policy().is_none()
    );
    assert!(root.show_windows().can_execute(None) && root.exit_windows().can_execute(None));
    let window = TestWindow::new();
    window.window.is_modal.set(true);
    root.set_windows(Some(ListHandle::from_items(vec![Some(
        window.clone() as Rc<dyn IDockWindow>
    )])));
    root.show_windows().execute(None);
    root.close().execute(None);
    assert_eq!(
        *window.window.calls.borrow(),
        ["PresentModal", "Save", "Exit"]
    );
}

#[test]
fn root_window_link_retains_original_reference_until_cleared() {
    let root = RootDock::new();
    let window = TestWindow::new();
    let weak = Rc::downgrade(&window);
    root.set_window(Some(window.clone()));
    assert!(root.window().is_some());
    drop(window);
    assert!(weak.upgrade().is_some() && root.window().is_some());
    root.set_window(None);
    assert!(weak.upgrade().is_none());
}

#[test]
fn tool_add_uses_source_order_and_rechecks_the_factory_between_calls() {
    let dock = ToolDock::new();
    let tool = Tool::new();
    dock.add_tool(tool.clone()); // No factory leaves the model unchanged.
    let first = Rc::new(RecordingFactory::default());
    let second = Rc::new(RecordingFactory::default());
    let replacement = second.clone();
    *first.on_add.borrow_mut() = Some(Rc::new(move |dock, _| {
        dock.set_factory(Some(replacement.clone()))
    }));
    dock.set_factory(Some(first.clone()));
    dock.add_tool(tool.clone());
    let owner: Rc<dyn IDock> = dock;
    let tool: Rc<dyn IDockable> = tool;
    assert!(
        matches!(&first.model_calls.borrow()[0], ModelCall::Add(d, value) if Rc::ptr_eq(d, &owner) && Rc::ptr_eq(value, &tool))
    );
    let calls = second.model_calls.borrow();
    assert_eq!(calls.len(), 2);
    assert!(matches!(&calls[0], ModelCall::SetActive(value) if Rc::ptr_eq(value, &tool)));
    assert!(
        matches!(&calls[1], ModelCall::SetFocused(d, Some(value)) if Rc::ptr_eq(d, &owner) && Rc::ptr_eq(value, &tool))
    );
}

struct UserDock {
    base: DockBase,
}
reactive_object!(UserDock, base);
crate::impl_dock!(UserDock);
impl DockableBaseImpl for UserDock {}

#[test]
fn custom_dock_initialization_raises_initial_history_notifications_once() {
    let dock = Rc::new_cyclic(|weak: &Weak<UserDock>| UserDock {
        base: DockBase::construct(weak.clone()),
    });
    let events = Rc::new(RefCell::new(Vec::new()));
    let log = events.clone();
    dock.property_changed()
        .add(Rc::new(move |name| log.borrow_mut().push(name.to_owned())));
    dock.base.initialize(&dock);
    dock.base.initialize(&dock);
    assert_eq!(*events.borrow(), ["CanGoBack", "CanGoForward"]);
    dock.navigate().execute(Some(&Document::new().boxed()));
    assert!(dock.active_dockable().is_some());
}
