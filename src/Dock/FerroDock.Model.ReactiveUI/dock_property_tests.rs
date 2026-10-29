//! Property contract tests: duplicate assignments, declared interfaces and concrete reactive identity.
use crate::controls::*;
use crate::test_support::TestWindow;
use ferrodock_model::collections::ListHandle;
use ferrodock_model::controls::*;
use ferrodock_model::core::*;
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::IReactiveNotifyPropertyChanged;
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn dock_dock_last_child_fill_uses_declared_interface_and_concrete_notifications() {
    let model = DockDock::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = model
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: bool = false;
    model.set_last_child_fill(first);
    model.set_last_child_fill(first);
    let erased: Rc<dyn IDockDock> = model.clone();
    let value = erased.last_child_fill();
    assert!(value == first);
    let second: bool = true;
    erased.set_last_child_fill(second);
    erased.set_last_child_fill(second);
    assert!(model.last_child_fill() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "LastChildFill");
        assert!(Rc::ptr_eq(&change.sender, &model));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn stack_dock_orientation_uses_declared_interface_and_concrete_notifications() {
    let model = StackDock::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = model
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: Orientation = Orientation::Vertical;
    model.set_orientation(first);
    model.set_orientation(first);
    let erased: Rc<dyn IStackDock> = model.clone();
    let value = erased.orientation();
    assert!(value == first);
    let second: Orientation = Orientation::Horizontal;
    erased.set_orientation(second);
    erased.set_orientation(second);
    assert!(model.orientation() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "Orientation");
        assert!(Rc::ptr_eq(&change.sender, &model));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn stack_dock_spacing_uses_declared_interface_and_concrete_notifications() {
    let model = StackDock::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = model
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: f64 = f64::NAN;
    model.set_spacing(first);
    model.set_spacing(first);
    let erased: Rc<dyn IStackDock> = model.clone();
    let value = erased.spacing();
    assert!(value.is_nan());
    let second: f64 = f64::INFINITY;
    erased.set_spacing(second);
    erased.set_spacing(second);
    assert!(model.spacing() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "Spacing");
        assert!(Rc::ptr_eq(&change.sender, &model));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn wrap_dock_orientation_uses_declared_interface_and_concrete_notifications() {
    let model = WrapDock::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = model
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: Orientation = Orientation::Vertical;
    model.set_orientation(first);
    model.set_orientation(first);
    let erased: Rc<dyn IWrapDock> = model.clone();
    let value = erased.orientation();
    assert!(value == first);
    let second: Orientation = Orientation::Horizontal;
    erased.set_orientation(second);
    erased.set_orientation(second);
    assert!(model.orientation() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "Orientation");
        assert!(Rc::ptr_eq(&change.sender, &model));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn uniform_grid_dock_rows_uses_declared_interface_and_concrete_notifications() {
    let model = UniformGridDock::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = model
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: i32 = 3;
    model.set_rows(first);
    model.set_rows(first);
    let erased: Rc<dyn IUniformGridDock> = model.clone();
    let value = erased.rows();
    assert!(value == first);
    let second: i32 = 0;
    erased.set_rows(second);
    erased.set_rows(second);
    assert!(model.rows() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "Rows");
        assert!(Rc::ptr_eq(&change.sender, &model));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn uniform_grid_dock_columns_uses_declared_interface_and_concrete_notifications() {
    let model = UniformGridDock::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = model
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: i32 = 3;
    model.set_columns(first);
    model.set_columns(first);
    let erased: Rc<dyn IUniformGridDock> = model.clone();
    let value = erased.columns();
    assert!(value == first);
    let second: i32 = 0;
    erased.set_columns(second);
    erased.set_columns(second);
    assert!(model.columns() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "Columns");
        assert!(Rc::ptr_eq(&change.sender, &model));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn grid_dock_column_definitions_uses_declared_interface_and_concrete_notifications() {
    let model = GridDock::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = model
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: Option<String> = Some("defined".to_owned());
    model.set_column_definitions(first.clone());
    model.set_column_definitions(first.clone());
    let erased: Rc<dyn IGridDock> = model.clone();
    let value = erased.column_definitions();
    assert!(value == first);
    let second: Option<String> = None;
    erased.set_column_definitions(second.clone());
    erased.set_column_definitions(second.clone());
    assert!(model.column_definitions() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "ColumnDefinitions");
        assert!(Rc::ptr_eq(&change.sender, &model));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn grid_dock_row_definitions_uses_declared_interface_and_concrete_notifications() {
    let model = GridDock::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = model
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: Option<String> = Some("defined".to_owned());
    model.set_row_definitions(first.clone());
    model.set_row_definitions(first.clone());
    let erased: Rc<dyn IGridDock> = model.clone();
    let value = erased.row_definitions();
    assert!(value == first);
    let second: Option<String> = None;
    erased.set_row_definitions(second.clone());
    erased.set_row_definitions(second.clone());
    assert!(model.row_definitions() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "RowDefinitions");
        assert!(Rc::ptr_eq(&change.sender, &model));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn proportional_dock_orientation_uses_declared_interface_and_concrete_notifications() {
    let model = ProportionalDock::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = model
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: Orientation = Orientation::Vertical;
    model.set_orientation(first);
    model.set_orientation(first);
    let erased: Rc<dyn IProportionalDock> = model.clone();
    let value = erased.orientation();
    assert!(value == first);
    let second: Orientation = Orientation::Horizontal;
    erased.set_orientation(second);
    erased.set_orientation(second);
    assert!(model.orientation() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "Orientation");
        assert!(Rc::ptr_eq(&change.sender, &model));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn tool_dock_alignment_uses_declared_interface_and_concrete_notifications() {
    let model = ToolDock::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = model
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: Alignment = Alignment::Left;
    model.set_alignment(first);
    model.set_alignment(first);
    let erased: Rc<dyn IToolDock> = model.clone();
    let value = erased.alignment();
    assert!(value == first);
    let second: Alignment = Alignment::Unset;
    erased.set_alignment(second);
    erased.set_alignment(second);
    assert!(model.alignment() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "Alignment");
        assert!(Rc::ptr_eq(&change.sender, &model));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn tool_dock_is_expanded_uses_declared_interface_and_concrete_notifications() {
    let model = ToolDock::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = model
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: bool = true;
    model.set_is_expanded(first);
    model.set_is_expanded(first);
    let erased: Rc<dyn IToolDock> = model.clone();
    let value = erased.is_expanded();
    assert!(value == first);
    let second: bool = false;
    erased.set_is_expanded(second);
    erased.set_is_expanded(second);
    assert!(model.is_expanded() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "IsExpanded");
        assert!(Rc::ptr_eq(&change.sender, &model));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn tool_dock_auto_hide_uses_declared_interface_and_concrete_notifications() {
    let model = ToolDock::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = model
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: bool = false;
    model.set_auto_hide(first);
    model.set_auto_hide(first);
    let erased: Rc<dyn IToolDock> = model.clone();
    let value = erased.auto_hide();
    assert!(value == first);
    let second: bool = true;
    erased.set_auto_hide(second);
    erased.set_auto_hide(second);
    assert!(model.auto_hide() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "AutoHide");
        assert!(Rc::ptr_eq(&change.sender, &model));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn tool_dock_grip_mode_uses_declared_interface_and_concrete_notifications() {
    let model = ToolDock::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = model
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: GripMode = GripMode::Hidden;
    model.set_grip_mode(first);
    model.set_grip_mode(first);
    let erased: Rc<dyn IToolDock> = model.clone();
    let value = erased.grip_mode();
    assert!(value == first);
    let second: GripMode = GripMode::Visible;
    erased.set_grip_mode(second);
    erased.set_grip_mode(second);
    assert!(model.grip_mode() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "GripMode");
        assert!(Rc::ptr_eq(&change.sender, &model));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn split_view_dock_compact_pane_length_uses_declared_interface_and_concrete_notifications() {
    let model = SplitViewDock::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = model
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: f64 = f64::NAN;
    model.set_compact_pane_length(first);
    model.set_compact_pane_length(first);
    let erased: Rc<dyn ISplitViewDock> = model.clone();
    let value = erased.compact_pane_length();
    assert!(value.is_nan());
    let second: f64 = f64::INFINITY;
    erased.set_compact_pane_length(second);
    erased.set_compact_pane_length(second);
    assert!(model.compact_pane_length() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "CompactPaneLength");
        assert!(Rc::ptr_eq(&change.sender, &model));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn split_view_dock_display_mode_uses_declared_interface_and_concrete_notifications() {
    let model = SplitViewDock::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = model
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: SplitViewDisplayMode = SplitViewDisplayMode::Inline;
    model.set_display_mode(first);
    model.set_display_mode(first);
    let erased: Rc<dyn ISplitViewDock> = model.clone();
    let value = erased.display_mode();
    assert!(value == first);
    let second: SplitViewDisplayMode = SplitViewDisplayMode::Overlay;
    erased.set_display_mode(second);
    erased.set_display_mode(second);
    assert!(model.display_mode() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "DisplayMode");
        assert!(Rc::ptr_eq(&change.sender, &model));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn split_view_dock_is_pane_open_uses_declared_interface_and_concrete_notifications() {
    let model = SplitViewDock::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = model
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: bool = true;
    model.set_is_pane_open(first);
    model.set_is_pane_open(first);
    let erased: Rc<dyn ISplitViewDock> = model.clone();
    let value = erased.is_pane_open();
    assert!(value == first);
    let second: bool = false;
    erased.set_is_pane_open(second);
    erased.set_is_pane_open(second);
    assert!(model.is_pane_open() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "IsPaneOpen");
        assert!(Rc::ptr_eq(&change.sender, &model));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn split_view_dock_open_pane_length_uses_declared_interface_and_concrete_notifications() {
    let model = SplitViewDock::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = model
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: f64 = f64::NAN;
    model.set_open_pane_length(first);
    model.set_open_pane_length(first);
    let erased: Rc<dyn ISplitViewDock> = model.clone();
    let value = erased.open_pane_length();
    assert!(value.is_nan());
    let second: f64 = f64::INFINITY;
    erased.set_open_pane_length(second);
    erased.set_open_pane_length(second);
    assert!(model.open_pane_length() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "OpenPaneLength");
        assert!(Rc::ptr_eq(&change.sender, &model));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn split_view_dock_pane_placement_uses_declared_interface_and_concrete_notifications() {
    let model = SplitViewDock::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = model
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: SplitViewPanePlacement = SplitViewPanePlacement::Right;
    model.set_pane_placement(first);
    model.set_pane_placement(first);
    let erased: Rc<dyn ISplitViewDock> = model.clone();
    let value = erased.pane_placement();
    assert!(value == first);
    let second: SplitViewPanePlacement = SplitViewPanePlacement::Left;
    erased.set_pane_placement(second);
    erased.set_pane_placement(second);
    assert!(model.pane_placement() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "PanePlacement");
        assert!(Rc::ptr_eq(&change.sender, &model));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn split_view_dock_use_light_dismiss_overlay_mode_uses_declared_interface_and_concrete_notifications(
) {
    let model = SplitViewDock::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = model
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: bool = true;
    model.set_use_light_dismiss_overlay_mode(first);
    model.set_use_light_dismiss_overlay_mode(first);
    let erased: Rc<dyn ISplitViewDock> = model.clone();
    let value = erased.use_light_dismiss_overlay_mode();
    assert!(value == first);
    let second: bool = false;
    erased.set_use_light_dismiss_overlay_mode(second);
    erased.set_use_light_dismiss_overlay_mode(second);
    assert!(model.use_light_dismiss_overlay_mode() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "UseLightDismissOverlayMode");
        assert!(Rc::ptr_eq(&change.sender, &model));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn split_view_dock_pane_dockable_uses_declared_interface_and_concrete_notifications() {
    let model = SplitViewDock::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = model
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: Option<Rc<dyn IDockable>> = Some(Document::new() as Rc<dyn IDockable>);
    model.set_pane_dockable(first.clone());
    model.set_pane_dockable(first.clone());
    let erased: Rc<dyn ISplitViewDock> = model.clone();
    let value = erased.pane_dockable();
    assert!(value == first);
    let second: Option<Rc<dyn IDockable>> = None;
    erased.set_pane_dockable(second.clone());
    erased.set_pane_dockable(second.clone());
    assert!(model.pane_dockable() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "PaneDockable");
        assert!(Rc::ptr_eq(&change.sender, &model));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn split_view_dock_content_dockable_uses_declared_interface_and_concrete_notifications() {
    let model = SplitViewDock::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = model
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: Option<Rc<dyn IDockable>> = Some(Document::new() as Rc<dyn IDockable>);
    model.set_content_dockable(first.clone());
    model.set_content_dockable(first.clone());
    let erased: Rc<dyn ISplitViewDock> = model.clone();
    let value = erased.content_dockable();
    assert!(value == first);
    let second: Option<Rc<dyn IDockable>> = None;
    erased.set_content_dockable(second.clone());
    erased.set_content_dockable(second.clone());
    assert!(model.content_dockable() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "ContentDockable");
        assert!(Rc::ptr_eq(&change.sender, &model));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn root_dock_is_focusable_root_uses_declared_interface_and_concrete_notifications() {
    let model = RootDock::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = model
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: bool = false;
    model.set_is_focusable_root(first);
    model.set_is_focusable_root(first);
    let erased: Rc<dyn IRootDock> = model.clone();
    let value = erased.is_focusable_root();
    assert!(value == first);
    let second: bool = true;
    erased.set_is_focusable_root(second);
    erased.set_is_focusable_root(second);
    assert!(model.is_focusable_root() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "IsFocusableRoot");
        assert!(Rc::ptr_eq(&change.sender, &model));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn root_dock_hidden_dockables_uses_declared_interface_and_concrete_notifications() {
    let model = RootDock::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = model
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: Option<ListHandle<Option<Rc<dyn IDockable>>>> =
        Some(ListHandle::<Option<Rc<dyn IDockable>>>::new());
    model.set_hidden_dockables(first.clone());
    model.set_hidden_dockables(first.clone());
    let erased: Rc<dyn IRootDock> = model.clone();
    let value = erased.hidden_dockables();
    assert!(value == first);
    let second: Option<ListHandle<Option<Rc<dyn IDockable>>>> = None;
    erased.set_hidden_dockables(second.clone());
    erased.set_hidden_dockables(second.clone());
    assert!(model.hidden_dockables() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "HiddenDockables");
        assert!(Rc::ptr_eq(&change.sender, &model));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn root_dock_left_pinned_dockables_uses_declared_interface_and_concrete_notifications() {
    let model = RootDock::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = model
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: Option<ListHandle<Option<Rc<dyn IDockable>>>> =
        Some(ListHandle::<Option<Rc<dyn IDockable>>>::new());
    model.set_left_pinned_dockables(first.clone());
    model.set_left_pinned_dockables(first.clone());
    let erased: Rc<dyn IRootDock> = model.clone();
    let value = erased.left_pinned_dockables();
    assert!(value == first);
    let second: Option<ListHandle<Option<Rc<dyn IDockable>>>> = None;
    erased.set_left_pinned_dockables(second.clone());
    erased.set_left_pinned_dockables(second.clone());
    assert!(model.left_pinned_dockables() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "LeftPinnedDockables");
        assert!(Rc::ptr_eq(&change.sender, &model));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn root_dock_right_pinned_dockables_uses_declared_interface_and_concrete_notifications() {
    let model = RootDock::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = model
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: Option<ListHandle<Option<Rc<dyn IDockable>>>> =
        Some(ListHandle::<Option<Rc<dyn IDockable>>>::new());
    model.set_right_pinned_dockables(first.clone());
    model.set_right_pinned_dockables(first.clone());
    let erased: Rc<dyn IRootDock> = model.clone();
    let value = erased.right_pinned_dockables();
    assert!(value == first);
    let second: Option<ListHandle<Option<Rc<dyn IDockable>>>> = None;
    erased.set_right_pinned_dockables(second.clone());
    erased.set_right_pinned_dockables(second.clone());
    assert!(model.right_pinned_dockables() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "RightPinnedDockables");
        assert!(Rc::ptr_eq(&change.sender, &model));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn root_dock_top_pinned_dockables_uses_declared_interface_and_concrete_notifications() {
    let model = RootDock::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = model
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: Option<ListHandle<Option<Rc<dyn IDockable>>>> =
        Some(ListHandle::<Option<Rc<dyn IDockable>>>::new());
    model.set_top_pinned_dockables(first.clone());
    model.set_top_pinned_dockables(first.clone());
    let erased: Rc<dyn IRootDock> = model.clone();
    let value = erased.top_pinned_dockables();
    assert!(value == first);
    let second: Option<ListHandle<Option<Rc<dyn IDockable>>>> = None;
    erased.set_top_pinned_dockables(second.clone());
    erased.set_top_pinned_dockables(second.clone());
    assert!(model.top_pinned_dockables() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "TopPinnedDockables");
        assert!(Rc::ptr_eq(&change.sender, &model));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn root_dock_bottom_pinned_dockables_uses_declared_interface_and_concrete_notifications() {
    let model = RootDock::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = model
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: Option<ListHandle<Option<Rc<dyn IDockable>>>> =
        Some(ListHandle::<Option<Rc<dyn IDockable>>>::new());
    model.set_bottom_pinned_dockables(first.clone());
    model.set_bottom_pinned_dockables(first.clone());
    let erased: Rc<dyn IRootDock> = model.clone();
    let value = erased.bottom_pinned_dockables();
    assert!(value == first);
    let second: Option<ListHandle<Option<Rc<dyn IDockable>>>> = None;
    erased.set_bottom_pinned_dockables(second.clone());
    erased.set_bottom_pinned_dockables(second.clone());
    assert!(model.bottom_pinned_dockables() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "BottomPinnedDockables");
        assert!(Rc::ptr_eq(&change.sender, &model));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn root_dock_pinned_dock_uses_declared_interface_and_concrete_notifications() {
    let model = RootDock::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = model
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: Option<Rc<dyn IToolDock>> = Some(ToolDock::new() as Rc<dyn IToolDock>);
    model.set_pinned_dock(first.clone());
    model.set_pinned_dock(first.clone());
    let erased: Rc<dyn IRootDock> = model.clone();
    let value = erased.pinned_dock();
    assert!(value == first);
    let second: Option<Rc<dyn IToolDock>> = None;
    erased.set_pinned_dock(second.clone());
    erased.set_pinned_dock(second.clone());
    assert!(model.pinned_dock() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "PinnedDock");
        assert!(Rc::ptr_eq(&change.sender, &model));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn root_dock_pinned_dock_display_mode_uses_declared_interface_and_concrete_notifications() {
    let model = RootDock::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = model
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: PinnedDockDisplayMode = PinnedDockDisplayMode::Inline;
    model.set_pinned_dock_display_mode(first);
    model.set_pinned_dock_display_mode(first);
    let erased: Rc<dyn IRootDock> = model.clone();
    let value = erased.pinned_dock_display_mode();
    assert!(value == first);
    let second: PinnedDockDisplayMode = PinnedDockDisplayMode::Overlay;
    erased.set_pinned_dock_display_mode(second);
    erased.set_pinned_dock_display_mode(second);
    assert!(model.pinned_dock_display_mode() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "PinnedDockDisplayMode");
        assert!(Rc::ptr_eq(&change.sender, &model));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn root_dock_window_uses_declared_interface_and_concrete_notifications() {
    let model = RootDock::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = model
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: Option<Rc<dyn IDockWindow>> = Some(TestWindow::new() as Rc<dyn IDockWindow>);
    model.set_window(first.clone());
    model.set_window(first.clone());
    let erased: Rc<dyn IRootDock> = model.clone();
    let value = erased.window();
    assert!(value == first);
    let second: Option<Rc<dyn IDockWindow>> = None;
    erased.set_window(second.clone());
    erased.set_window(second.clone());
    assert!(model.window() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "Window");
        assert!(Rc::ptr_eq(&change.sender, &model));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn root_dock_windows_uses_declared_interface_and_concrete_notifications() {
    let model = RootDock::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = model
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: Option<ListHandle<Option<Rc<dyn IDockWindow>>>> =
        Some(ListHandle::<Option<Rc<dyn IDockWindow>>>::new());
    model.set_windows(first.clone());
    model.set_windows(first.clone());
    let erased: Rc<dyn IRootDock> = model.clone();
    let value = erased.windows();
    assert!(value == first);
    let second: Option<ListHandle<Option<Rc<dyn IDockWindow>>>> = None;
    erased.set_windows(second.clone());
    erased.set_windows(second.clone());
    assert!(model.windows() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "Windows");
        assert!(Rc::ptr_eq(&change.sender, &model));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn root_dock_floating_window_host_mode_uses_declared_interface_and_concrete_notifications() {
    let model = RootDock::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = model
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: DockFloatingWindowHostMode = DockFloatingWindowHostMode::Managed;
    model.set_floating_window_host_mode(first);
    model.set_floating_window_host_mode(first);
    let erased: Rc<dyn IRootDock> = model.clone();
    let value = erased.floating_window_host_mode();
    assert!(value == first);
    let second: DockFloatingWindowHostMode = DockFloatingWindowHostMode::Default;
    erased.set_floating_window_host_mode(second);
    erased.set_floating_window_host_mode(second);
    assert!(model.floating_window_host_mode() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "FloatingWindowHostMode");
        assert!(Rc::ptr_eq(&change.sender, &model));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn root_dock_root_dock_capability_policy_uses_declared_interface_and_concrete_notifications() {
    let model = RootDock::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = model
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: Option<Rc<dyn DockCapabilityPolicyImpl>> =
        Some(DockCapabilityPolicy::new() as Rc<dyn DockCapabilityPolicyImpl>);
    model.set_root_dock_capability_policy(first.clone());
    model.set_root_dock_capability_policy(first.clone());
    let erased: Rc<dyn IRootDock> = model.clone();
    let value = erased.root_dock_capability_policy();
    assert!(value == first);
    let second: Option<Rc<dyn DockCapabilityPolicyImpl>> = None;
    erased.set_root_dock_capability_policy(second.clone());
    erased.set_root_dock_capability_policy(second.clone());
    assert!(model.root_dock_capability_policy() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "RootDockCapabilityPolicy");
        assert!(Rc::ptr_eq(&change.sender, &model));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn root_dock_enable_adaptive_global_dock_targets_uses_declared_interface_and_concrete_notifications(
) {
    let model = RootDock::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = model
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: bool = true;
    model.set_enable_adaptive_global_dock_targets(first);
    model.set_enable_adaptive_global_dock_targets(first);
    let erased: Rc<dyn IRootDock> = model.clone();
    let value = erased.enable_adaptive_global_dock_targets();
    assert!(value == first);
    let second: bool = false;
    erased.set_enable_adaptive_global_dock_targets(second);
    erased.set_enable_adaptive_global_dock_targets(second);
    assert!(model.enable_adaptive_global_dock_targets() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "EnableAdaptiveGlobalDockTargets");
        assert!(Rc::ptr_eq(&change.sender, &model));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn document_dock_can_create_document_uses_declared_interface_and_concrete_notifications() {
    let model = DocumentDock::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = model
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: bool = true;
    model.set_can_create_document(first);
    model.set_can_create_document(first);
    let erased: Rc<dyn IDocumentDock> = model.clone();
    let value = erased.can_create_document();
    assert!(value == first);
    let second: bool = false;
    erased.set_can_create_document(second);
    erased.set_can_create_document(second);
    assert!(model.can_create_document() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "CanCreateDocument");
        assert!(Rc::ptr_eq(&change.sender, &model));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn document_dock_enable_window_drag_uses_declared_interface_and_concrete_notifications() {
    let model = DocumentDock::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = model
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: bool = true;
    model.set_enable_window_drag(first);
    model.set_enable_window_drag(first);
    let erased: Rc<dyn IDocumentDock> = model.clone();
    let value = erased.enable_window_drag();
    assert!(value == first);
    let second: bool = false;
    erased.set_enable_window_drag(second);
    erased.set_enable_window_drag(second);
    assert!(model.enable_window_drag() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "EnableWindowDrag");
        assert!(Rc::ptr_eq(&change.sender, &model));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn document_dock_tabs_layout_uses_declared_interface_and_concrete_notifications() {
    let model = DocumentDock::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = model
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: DocumentTabLayout = DocumentTabLayout::Left;
    model.set_tabs_layout(first);
    model.set_tabs_layout(first);
    let erased: Rc<dyn IDocumentDock> = model.clone();
    let value = erased.tabs_layout();
    assert!(value == first);
    let second: DocumentTabLayout = DocumentTabLayout::Top;
    erased.set_tabs_layout(second);
    erased.set_tabs_layout(second);
    assert!(model.tabs_layout() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "TabsLayout");
        assert!(Rc::ptr_eq(&change.sender, &model));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn document_dock_layout_mode_uses_declared_interface_and_concrete_notifications() {
    let model = DocumentDock::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = model
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: DocumentLayoutMode = DocumentLayoutMode::Mdi;
    model.set_layout_mode(first);
    model.set_layout_mode(first);
    let erased: Rc<dyn IDocumentDock> = model.clone();
    let value = erased.layout_mode();
    assert!(value == first);
    let second: DocumentLayoutMode = DocumentLayoutMode::Tabbed;
    erased.set_layout_mode(second);
    erased.set_layout_mode(second);
    assert!(model.layout_mode() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "LayoutMode");
        assert!(Rc::ptr_eq(&change.sender, &model));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn document_dock_close_button_show_mode_uses_declared_interface_and_concrete_notifications() {
    let model = DocumentDock::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = model
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: DocumentCloseButtonShowMode = DocumentCloseButtonShowMode::Never;
    model.set_close_button_show_mode(first);
    model.set_close_button_show_mode(first);
    let erased: Rc<dyn IDocumentDock> = model.clone();
    let value = erased.close_button_show_mode();
    assert!(value == first);
    let second: DocumentCloseButtonShowMode = DocumentCloseButtonShowMode::Always;
    erased.set_close_button_show_mode(second);
    erased.set_close_button_show_mode(second);
    assert!(model.close_button_show_mode() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "CloseButtonShowMode");
        assert!(Rc::ptr_eq(&change.sender, &model));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn dock_dock_default_dockable_uses_declared_interface_and_concrete_notifications() {
    let model = DockDock::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = model
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: Option<Rc<dyn IDockable>> = Some(Document::new() as Rc<dyn IDockable>);
    model.set_default_dockable(first.clone());
    model.set_default_dockable(first.clone());
    let erased: Rc<dyn IDock> = model.clone();
    let value = erased.default_dockable();
    assert!(value == first);
    let second: Option<Rc<dyn IDockable>> = None;
    erased.set_default_dockable(second.clone());
    erased.set_default_dockable(second.clone());
    assert!(model.default_dockable() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "DefaultDockable");
        assert!(Rc::ptr_eq(&change.sender, &model));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn dock_dock_is_active_uses_declared_interface_and_concrete_notifications() {
    let model = DockDock::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = model
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: bool = true;
    model.set_is_active(first);
    model.set_is_active(first);
    let erased: Rc<dyn IDock> = model.clone();
    let value = erased.is_active();
    assert!(value == first);
    let second: bool = false;
    erased.set_is_active(second);
    erased.set_is_active(second);
    assert!(model.is_active() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "IsActive");
        assert!(Rc::ptr_eq(&change.sender, &model));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn dock_dock_opened_dockables_count_uses_declared_interface_and_concrete_notifications() {
    let model = DockDock::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = model
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: i32 = 3;
    model.set_opened_dockables_count(first);
    model.set_opened_dockables_count(first);
    let erased: Rc<dyn IDock> = model.clone();
    let value = erased.opened_dockables_count();
    assert!(value == first);
    let second: i32 = 0;
    erased.set_opened_dockables_count(second);
    erased.set_opened_dockables_count(second);
    assert!(model.opened_dockables_count() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "OpenedDockablesCount");
        assert!(Rc::ptr_eq(&change.sender, &model));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn dock_dock_can_close_last_dockable_uses_declared_interface_and_concrete_notifications() {
    let model = DockDock::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = model
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: bool = false;
    model.set_can_close_last_dockable(first);
    model.set_can_close_last_dockable(first);
    let erased: Rc<dyn IDock> = model.clone();
    let value = erased.can_close_last_dockable();
    assert!(value == first);
    let second: bool = true;
    erased.set_can_close_last_dockable(second);
    erased.set_can_close_last_dockable(second);
    assert!(model.can_close_last_dockable() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "CanCloseLastDockable");
        assert!(Rc::ptr_eq(&change.sender, &model));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn dock_dock_enable_global_docking_uses_declared_interface_and_concrete_notifications() {
    let model = DockDock::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = model
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: bool = false;
    model.set_enable_global_docking(first);
    model.set_enable_global_docking(first);
    let erased: Rc<dyn IDock> = model.clone();
    let value = erased.enable_global_docking();
    assert!(value == first);
    let second: bool = true;
    erased.set_enable_global_docking(second);
    erased.set_enable_global_docking(second);
    assert!(model.enable_global_docking() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "EnableGlobalDocking");
        assert!(Rc::ptr_eq(&change.sender, &model));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}
