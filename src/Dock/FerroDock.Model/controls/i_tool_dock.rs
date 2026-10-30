// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::core::*;
use std::rc::Rc;

/// The `IToolDock` docking contract.
pub trait IToolDock: IDock + ILocalTarget {
    /// Gets or sets dock auto hide alignment.
    fn alignment(&self) -> Alignment;
    /// Sets `Alignment`.
    fn set_alignment(&self, value: Alignment);
    /// Gets or sets if the Dock is expanded.
    fn is_expanded(&self) -> bool;
    /// Sets `IsExpanded`.
    fn set_is_expanded(&self, value: bool);
    /// Gets or sets if the Dock auto hides dockable when pointer is not over.
    fn auto_hide(&self) -> bool;
    /// Sets `AutoHide`.
    fn set_auto_hide(&self, value: bool);
    /// Gets or sets if the tool Dock grip mode.
    fn grip_mode(&self) -> GripMode;
    /// Sets `GripMode`.
    fn set_grip_mode(&self, value: GripMode);
    /// Adds the specified tool to this dock and activates it.
    fn add_tool(&self, tool: Option<Rc<dyn IDockable>>);
}

impl PartialEq for dyn IToolDock {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
