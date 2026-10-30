// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::BoxedValue;

/// The `INavigateAdapter` docking contract.
pub trait INavigateAdapter {
    /// Gets a value that indicates whether there is at least one entry in back navigation history.
    fn can_go_back(&self) -> bool;
    /// Gets a value that indicates whether there is at least one entry in forward navigation history.
    fn can_go_forward(&self) -> bool;
    /// Navigates to the most recent entry in back navigation history, if there is one.
    fn go_back(&self);
    /// Navigate to the most recent entry in forward navigation history, if there is one.
    fn go_forward(&self);
    /// Implementation of the `IDock.Navigate` method.
    fn navigate(&self, root: Option<BoxedValue>, b_snapshot: bool);
    /// Implementation of the `IRootDock.ShowWindows` method.
    fn show_windows(&self);
    /// Implementation of the `IRootDock.ExitWindows` method.
    fn exit_windows(&self);
    /// Implementation of the `IDock.Close` method.
    fn close(&self);
}

impl PartialEq for dyn INavigateAdapter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
