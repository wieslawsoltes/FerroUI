// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::core::*;
use std::rc::Rc;

/// The `IDockService` docking contract.
pub trait IDockService {
    /// Moves dockable to the target dock.
    fn move_dockable_between_docks(
        &self,
        source_dockable: Rc<dyn IDockable>,
        source_dockable_owner: Rc<dyn IDock>,
        target_dock: Rc<dyn IDock>,
        b_execute: bool,
    ) -> bool;
    /// Swaps dockable with target dock.
    fn swap_dockable_between_docks(
        &self,
        source_dockable: Rc<dyn IDockable>,
        source_dockable_owner: Rc<dyn IDock>,
        target_dock: Rc<dyn IDock>,
        b_execute: bool,
    ) -> bool;
    /// Splits dockable using the given operation.
    fn split_dockable(
        &self,
        source_dockable: Rc<dyn IDockable>,
        source_dockable_owner: Rc<dyn IDock>,
        target_dock: Rc<dyn IDock>,
        operation: DockOperation,
        b_execute: bool,
    ) -> bool;
    /// Docks the dockable into a window.
    fn dock_dockable_into_window(
        &self,
        source_dockable: Rc<dyn IDockable>,
        target_dockable: Rc<dyn IDockable>,
        screen_position: DockPoint,
        b_execute: bool,
    ) -> bool;
    /// Docks a dockable into another dockable.
    fn dock_dockable_into_dockable(
        &self,
        source_dockable: Rc<dyn IDockable>,
        target_dockable: Rc<dyn IDockable>,
        action: DragAction,
        b_execute: bool,
    ) -> bool;
}

impl PartialEq for dyn IDockService {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
