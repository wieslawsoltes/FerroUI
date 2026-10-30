// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::controls::*;
use crate::core::*;

/// The `IMdiDocument` docking contract.
pub trait IMdiDocument: IDocument {
    /// Gets or sets MDI window bounds in dock coordinates.
    fn mdi_bounds(&self) -> DockRect;
    /// Sets `MdiBounds`.
    fn set_mdi_bounds(&self, value: DockRect);
    /// Gets or sets MDI window state.
    fn mdi_state(&self) -> MdiWindowState;
    /// Sets `MdiState`.
    fn set_mdi_state(&self, value: MdiWindowState);
    /// Gets or sets MDI window Z index.
    fn mdi_z_index(&self) -> i32;
    /// Sets `MdiZIndex`.
    fn set_mdi_z_index(&self, value: i32);
}

impl PartialEq for dyn IMdiDocument {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
