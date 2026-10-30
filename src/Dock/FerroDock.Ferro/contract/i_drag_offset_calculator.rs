// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::controls::DockControl;
use ferroui_base::{PixelPoint, Point};
use ferroui_controls::Control;
/// Calculates the drag-preview offset relative to the originating dock control.
pub trait IDragOffsetCalculator {
    /// Returns the original offset in screen pixels.
    fn calculate_offset(
        &self,
        drag_control: &Control,
        dock_control: &DockControl,
        pointer: Point,
    ) -> PixelPoint;
}

impl PartialEq for dyn IDragOffsetCalculator {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
