// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::core::*;

/// The `IStackDock` docking contract.
pub trait IStackDock: IDock {
    /// Gets or sets layout orientation.
    fn orientation(&self) -> Orientation;
    /// Sets `Orientation`.
    fn set_orientation(&self, value: Orientation);
    /// Gets or sets spacing between items.
    fn spacing(&self) -> f64;
    /// Sets `Spacing`.
    fn set_spacing(&self, value: f64);
}

impl PartialEq for dyn IStackDock {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
