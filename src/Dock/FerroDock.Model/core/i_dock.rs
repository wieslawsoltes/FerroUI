// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::core::*;
use ferrodock_model::collections::ListHandle;
use ferroui_reactiveui::ICommand;
use std::rc::Rc;

/// The `IDock` docking contract.
pub trait IDock: IDockable {
    /// Gets or sets visible dockables.
    fn visible_dockables(&self) -> Option<ListHandle<Option<Rc<dyn IDockable>>>>;
    /// Sets `VisibleDockables`.
    fn set_visible_dockables(&self, value: Option<ListHandle<Option<Rc<dyn IDockable>>>>);
    /// Gets or sets active dockable.
    fn active_dockable(&self) -> Option<Rc<dyn IDockable>>;
    /// Sets `ActiveDockable`.
    fn set_active_dockable(&self, value: Option<Rc<dyn IDockable>>);
    /// Gets or sets default dockable.
    fn default_dockable(&self) -> Option<Rc<dyn IDockable>>;
    /// Sets `DefaultDockable`.
    fn set_default_dockable(&self, value: Option<Rc<dyn IDockable>>);
    /// Gets or sets the focused dockable.
    fn focused_dockable(&self) -> Option<Rc<dyn IDockable>>;
    /// Sets `FocusedDockable`.
    fn set_focused_dockable(&self, value: Option<Rc<dyn IDockable>>);
    /// Gets or sets if the dockable is the currently active.
    fn is_active(&self) -> bool;
    /// Sets `IsActive`.
    fn set_is_active(&self, value: bool);
    /// Gets the number of currently opened and visible dockables
    fn opened_dockables_count(&self) -> i32;
    /// Sets `OpenedDockablesCount`.
    fn set_opened_dockables_count(&self, value: i32);
    /// Gets or sets whether the last remaining dockable can be closed.
    fn can_close_last_dockable(&self) -> bool;
    /// Sets `CanCloseLastDockable`.
    fn set_can_close_last_dockable(&self, value: bool);
    /// Gets or sets optional per-dock capability policy values.
    /// Null values inherit root policy and dockable base flag values.
    fn dock_capability_policy(&self) -> Option<Rc<dyn DockCapabilityPolicyImpl>>;
    /// Sets `DockCapabilityPolicy`.
    fn set_dock_capability_policy(&self, value: Option<Rc<dyn DockCapabilityPolicyImpl>>);
    /// Gets a value that indicates whether there is at least one entry in back navigation history.
    fn can_go_back(&self) -> bool;
    /// Gets a value that indicates whether there is at least one entry in forward navigation history.
    fn can_go_forward(&self) -> bool;
    /// Navigates to the most recent entry in back navigation history, if there is one.
    fn go_back(&self) -> Rc<dyn ICommand>;
    /// Navigate to the most recent entry in forward navigation history, if there is one.
    fn go_forward(&self) -> Rc<dyn ICommand>;
    /// Navigate to content that is contained by an object.
    fn navigate(&self) -> Rc<dyn ICommand>;
    /// Close layout.
    fn close(&self) -> Rc<dyn ICommand>;
    /// Gets or sets whether global docking is enabled for this dock.
    /// When true, dockables can be dropped onto other dock controls.
    /// Enabled by default.
    fn enable_global_docking(&self) -> bool;
    /// Sets `EnableGlobalDocking`.
    fn set_enable_global_docking(&self, value: bool);
}

impl PartialEq for dyn IDock {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
