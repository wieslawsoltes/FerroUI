// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::controls::*;

/// The `IProportionalDockSplitter` docking contract.
pub trait IProportionalDockSplitter: ISplitter {
    /// Gets or sets whether the splitter allows resizing.
    fn can_resize(&self) -> bool;
    /// Sets `CanResize`.
    fn set_can_resize(&self, value: bool);
    /// Gets or sets whether resizing occurs only after the pointer is released.
    /// When true a drag preview is shown during movement.
    fn resize_preview(&self) -> bool;
    /// Sets `ResizePreview`.
    fn set_resize_preview(&self, value: bool);
}

impl PartialEq for dyn IProportionalDockSplitter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
