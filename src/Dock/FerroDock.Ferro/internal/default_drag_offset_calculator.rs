// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::{contract::IDragOffsetCalculator, controls::DockControl};
use ferroui_base::{PixelPoint, Point};
use ferroui_controls::Control;
pub(crate) struct DefaultDragOffsetCalculator;
impl IDragOffsetCalculator for DefaultDragOffsetCalculator {
    fn calculate_offset(&self, drag: &Control, dock: &DockControl, pointer: Point) -> PixelPoint {
        let screen = dock.point_to_screen(pointer);
        if drag.name().as_deref() != Some("PART_Grip") {
            let corner = drag.point_to_screen(Point::default());
            return PixelPoint::new(
                corner.x.wrapping_sub(screen.x),
                corner.y.wrapping_sub(screen.y),
            );
        }
        PixelPoint::default()
    }
}
