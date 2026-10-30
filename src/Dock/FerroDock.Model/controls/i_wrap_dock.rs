// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::core::*;

/// The `IWrapDock` docking contract.
pub trait IWrapDock: IDock {
    /// Gets or sets layout orientation.
    fn orientation(&self) -> Orientation;
    /// Sets `Orientation`.
    fn set_orientation(&self, value: Orientation);
}

impl PartialEq for dyn IWrapDock {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
