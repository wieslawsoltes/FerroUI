// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::DockOperationMask;

/// Optional operation restrictions for drag sources and drop targets.
pub trait IDockableDockingRestrictions {
    /// Gets the operations allowed when this dockable is the drag source.
    fn allowed_dock_operations(&self) -> DockOperationMask;
    /// Sets the operations allowed when this dockable is the drag source.
    fn set_allowed_dock_operations(&self, value: DockOperationMask);
    /// Gets the operations allowed when this dockable is the drop target.
    fn allowed_drop_operations(&self) -> DockOperationMask;
    /// Sets the operations allowed when this dockable is the drop target.
    fn set_allowed_drop_operations(&self, value: DockOperationMask);
}
