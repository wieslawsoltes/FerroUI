// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::controls::*;
use crate::core::*;

/// The `IGridDockSplitter` docking contract.
pub trait IGridDockSplitter: ISplitter {
    /// Gets or sets resize direction.
    fn resize_direction(&self) -> GridResizeDirection;
    /// Sets `ResizeDirection`.
    fn set_resize_direction(&self, value: GridResizeDirection);
}

impl PartialEq for dyn IGridDockSplitter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
