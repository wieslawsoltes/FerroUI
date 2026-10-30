// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::core::*;
use std::rc::Rc;

/// The `IDockingWindowStateSync` docking contract.
pub trait IDockingWindowStateSync {
    /// Handles a property change request from a docking window state mixin.
    fn on_docking_window_state_property_changed(
        &self,
        dockable: Rc<dyn IDockable>,
        property: DockingWindowStateProperty,
    );
}

impl PartialEq for dyn IDockingWindowStateSync {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
