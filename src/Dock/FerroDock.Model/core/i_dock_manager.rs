// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::controls::*;
use crate::core::*;
use std::rc::Rc;

/// The `IDockManager` docking contract.
pub trait IDockManager {
    /// Gets or sets pointer position relative to event source.
    fn position(&self) -> DockPoint;
    /// Sets `Position`.
    fn set_position(&self, value: DockPoint);
    /// Gets or sets pointer position relative to event source and translated to screen coordinates.
    fn screen_position(&self) -> DockPoint;
    /// Sets `ScreenPosition`.
    fn set_screen_position(&self, value: DockPoint);
    /// Gets or sets flag that prevents docking when fixed sizes conflict.
    fn prevent_size_conflicts(&self) -> bool;
    /// Sets `PreventSizeConflicts`.
    fn set_prevent_size_conflicts(&self, value: bool);
    /// Gets or sets whether docking interactions are enabled.
    fn is_docking_enabled(&self) -> bool;
    /// Sets `IsDockingEnabled`.
    fn set_is_docking_enabled(&self, value: bool);
    /// Gets the last capability evaluation that blocked a docking-related action.
    /// Returns null when the last validation did not fail due to capability policy.
    fn last_capability_evaluation(&self) -> Option<Rc<DockCapabilityEvaluation>>;
    /// Validates tool docking operation.
    fn validate_tool(
        &self,
        source_tool: Rc<dyn ITool>,
        target_dockable: Rc<dyn IDockable>,
        action: DragAction,
        operation: DockOperation,
        b_execute: bool,
    ) -> bool;
    /// Validates document docking operation.
    fn validate_document(
        &self,
        source_document: Rc<dyn IDocument>,
        target_dockable: Rc<dyn IDockable>,
        action: DragAction,
        operation: DockOperation,
        b_execute: bool,
    ) -> bool;
    /// Validates dock docking operation.
    fn validate_dock(
        &self,
        source_dock: Rc<dyn IDock>,
        target_dockable: Rc<dyn IDockable>,
        action: DragAction,
        operation: DockOperation,
        b_execute: bool,
    ) -> bool;
    /// Validates dockable docking operation.
    fn validate_dockable(
        &self,
        source_dockable: Rc<dyn IDockable>,
        target_dockable: Rc<dyn IDockable>,
        action: DragAction,
        operation: DockOperation,
        b_execute: bool,
    ) -> bool;
    /// Determines if a dock target indicator should be visible.
    fn is_dock_target_visible(
        &self,
        source_dockable: Rc<dyn IDockable>,
        target_dockable: Rc<dyn IDockable>,
        operation: DockOperation,
    ) -> bool;
}

impl PartialEq for dyn IDockManager {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
