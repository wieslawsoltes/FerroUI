// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::{controls::*, core::*};
use ferrodock_serializer_system_text_json::JsonPropertyInfo;
use std::rc::Rc;

macro_rules! fields {
    ($t:ty; $( $name:literal => $get:ident, $set:ident );* $(;)?) => {
        vec![$(JsonPropertyInfo::new($name, |v: &Rc<$t>| v.$get(), |v, x| v.$set(x))),*]
    };
}
pub(crate) use fields;
pub(crate) fn dockable<T: IDockable + ?Sized + 'static>() -> Vec<JsonPropertyInfo<Rc<T>>> {
    fields!(T;
        "Id" => id, set_id;
        "Title" => title, set_title;
        "Owner" => owner, set_owner;
        "IsEmpty" => is_empty, set_is_empty;
        "IsCollapsable" => is_collapsable, set_is_collapsable;
        "Proportion" => proportion, set_proportion;
        "Dock" => dock, set_dock;
        "DockingState" => docking_state, set_docking_state;
        "Column" => column, set_column;
        "Row" => row, set_row;
        "ColumnSpan" => column_span, set_column_span;
        "RowSpan" => row_span, set_row_span;
        "IsSharedSizeScope" => is_shared_size_scope, set_is_shared_size_scope;
        "CollapsedProportion" => collapsed_proportion, set_collapsed_proportion;
        "MinWidth" => min_width, set_min_width;
        "MaxWidth" => max_width, set_max_width;
        "MinHeight" => min_height, set_min_height;
        "MaxHeight" => max_height, set_max_height;
        "CanClose" => can_close, set_can_close;
        "CanPin" => can_pin, set_can_pin;
        "KeepPinnedDockableVisible" => keep_pinned_dockable_visible, set_keep_pinned_dockable_visible;
        "PinnedDockDisplayModeOverride" => pinned_dock_display_mode_override, set_pinned_dock_display_mode_override;
        "PinnedBounds" => pinned_bounds, set_pinned_bounds_rect;
        "CanFloat" => can_float, set_can_float;
        "CanDrag" => can_drag, set_can_drag;
        "CanDrop" => can_drop, set_can_drop;
        "CanDockAsDocument" => can_dock_as_document, set_can_dock_as_document;
        "DockCapabilityOverrides" => dock_capability_overrides, set_dock_capability_overrides;
        "IsModified" => is_modified, set_is_modified;
        "DockGroup" => dock_group, set_dock_group;
    )
}

pub(crate) fn concrete_dockable<
    T: IDockable + IDockableDockingRestrictions + IDockSelectorInfo + 'static,
>() -> Vec<JsonPropertyInfo<Rc<T>>> {
    let mut fields = dockable::<T>();
    fields.extend(fields!(T;
        "AllowedDockOperations" => allowed_dock_operations, set_allowed_dock_operations;
        "AllowedDropOperations" => allowed_drop_operations, set_allowed_drop_operations;
        "ShowInSelector" => show_in_selector, set_show_in_selector;
        "SelectorTitle" => selector_title, set_selector_title;
    ));
    fields
}
pub(crate) fn dock_own<T: IDock + ?Sized + 'static>() -> Vec<JsonPropertyInfo<Rc<T>>> {
    fields!(T;
        "VisibleDockables" => visible_dockables, set_visible_dockables;
        "ActiveDockable" => active_dockable, set_active_dockable;
        "DefaultDockable" => default_dockable, set_default_dockable;
        "FocusedDockable" => focused_dockable, set_focused_dockable;
        "IsActive" => is_active, set_is_active;
        "OpenedDockablesCount" => opened_dockables_count, set_opened_dockables_count;
        "CanCloseLastDockable" => can_close_last_dockable, set_can_close_last_dockable;
        "DockCapabilityPolicy" => dock_capability_policy, set_dock_capability_policy;
        "EnableGlobalDocking" => enable_global_docking, set_enable_global_docking;
    )
}

pub(crate) fn concrete_dock<
    T: IDock + IDockableDockingRestrictions + IDockSelectorInfo + 'static,
>() -> Vec<JsonPropertyInfo<Rc<T>>> {
    let mut fields = dock_own::<T>();
    fields.extend(concrete_dockable::<T>());
    fields
}
pub(crate) fn root_own<T: IRootDock + ?Sized + 'static>() -> Vec<JsonPropertyInfo<Rc<T>>> {
    fields!(T;
        "IsFocusableRoot" => is_focusable_root, set_is_focusable_root;
        "HiddenDockables" => hidden_dockables, set_hidden_dockables;
        "LeftPinnedDockables" => left_pinned_dockables, set_left_pinned_dockables;
        "RightPinnedDockables" => right_pinned_dockables, set_right_pinned_dockables;
        "TopPinnedDockables" => top_pinned_dockables, set_top_pinned_dockables;
        "BottomPinnedDockables" => bottom_pinned_dockables, set_bottom_pinned_dockables;
        "PinnedDock" => pinned_dock, set_pinned_dock;
        "PinnedDockDisplayMode" => pinned_dock_display_mode, set_pinned_dock_display_mode;
        "Window" => window, set_window;
        "Windows" => windows, set_windows;
        "FloatingWindowHostMode" => floating_window_host_mode, set_floating_window_host_mode;
        "RootDockCapabilityPolicy" => root_dock_capability_policy, set_root_dock_capability_policy;
        "EnableAdaptiveGlobalDockTargets" => enable_adaptive_global_dock_targets, set_enable_adaptive_global_dock_targets;
    )
}

pub(crate) fn window<T: IDockWindow + ?Sized + 'static>() -> Vec<JsonPropertyInfo<Rc<T>>> {
    fields!(T;
        "Id" => id, set_id;
        "X" => x, set_x;
        "Y" => y, set_y;
        "Width" => width, set_width;
        "Height" => height, set_height;
        "WindowState" => window_state, set_window_state;
        "Topmost" => topmost, set_topmost;
        "Title" => title, set_title;
        "OwnerMode" => owner_mode, set_owner_mode;
        "ParentWindow" => parent_window, set_parent_window;
        "IsModal" => is_modal, set_is_modal;
        "ShowInTaskbar" => show_in_taskbar, set_show_in_taskbar;
        "Layout" => layout, set_layout;
    )
}

pub(crate) fn mdi<T: IMdiDocument + ?Sized + 'static>() -> Vec<JsonPropertyInfo<Rc<T>>> {
    fields!(T;
        "MdiBounds" => mdi_bounds, set_mdi_bounds;
        "MdiState" => mdi_state, set_mdi_state;
        "MdiZIndex" => mdi_z_index, set_mdi_z_index;
    )
}

pub(crate) fn tool_dock_own<T: IToolDock + ?Sized + 'static>() -> Vec<JsonPropertyInfo<Rc<T>>> {
    fields!(T;
        "Alignment" => alignment, set_alignment;
        "IsExpanded" => is_expanded, set_is_expanded;
        "AutoHide" => auto_hide, set_auto_hide;
        "GripMode" => grip_mode, set_grip_mode;
    )
}

pub(crate) fn document_dock_own<T: IDocumentDock + ?Sized + 'static>(
) -> Vec<JsonPropertyInfo<Rc<T>>> {
    fields!(T;
        "CanCreateDocument" => can_create_document, set_can_create_document;
        "EnableWindowDrag" => enable_window_drag, set_enable_window_drag;
        "TabsLayout" => tabs_layout, set_tabs_layout;
        "LayoutMode" => layout_mode, set_layout_mode;
        "CloseButtonShowMode" => close_button_show_mode, set_close_button_show_mode;
    )
}
