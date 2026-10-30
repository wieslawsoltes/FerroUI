// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::DockingWindowState;

/// Optional mixin binding a view model to layout-backed window lifecycle state.
pub trait IDockingWindowState {
    /// Returns whether the dockable is open in the layout.
    fn is_open(&self) -> bool;
    /// Sets whether the dockable is open in the layout.
    fn set_is_open(&self, value: bool);
    /// Returns whether the dockable is active (focused).
    fn is_active(&self) -> bool;
    /// Sets whether the dockable is active (focused).
    fn set_is_active(&self, value: bool);
    /// Returns whether the dockable is selected in its owner dock.
    fn is_selected(&self) -> bool;
    /// Sets whether the dockable is selected in its owner dock.
    fn set_is_selected(&self, value: bool);
    /// Gets the dockable's logical docking state flags.
    fn docking_state(&self) -> DockingWindowState;
    /// Sets the dockable's logical docking state flags.
    fn set_docking_state(&self, value: DockingWindowState);
}
