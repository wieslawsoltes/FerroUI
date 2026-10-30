// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferroui_base::{layout::Orientation, Vector};
use ferroui_controls::ScrollViewer;
pub(crate) struct TabStripMouseWheelScrollHelper;
impl TabStripMouseWheelScrollHelper {
    pub(crate) fn try_handle(
        scroll_viewer: Option<&ScrollViewer>,
        orientation: Orientation,
        delta: Vector,
    ) -> bool {
        let Some(scroll_viewer) = scroll_viewer else {
            return false;
        };
        let delta_y = delta.y;
        // Double.Epsilon is the smallest positive subnormal, not Rust f64::EPSILON.
        if delta_y.abs() <= f64::from_bits(1) {
            return false;
        }
        let steps = 1.max(delta_y.abs().ceil() as i32);
        match orientation {
            Orientation::Horizontal => {
                if scroll_viewer.extent().width <= scroll_viewer.viewport().width {
                    return false;
                }
                let initial = scroll_viewer.offset();
                for _ in 0..steps {
                    if delta_y > 0.0 {
                        scroll_viewer.line_left();
                    } else {
                        scroll_viewer.line_right();
                    }
                }
                scroll_viewer.offset() != initial
            }
            Orientation::Vertical => {
                if scroll_viewer.extent().height <= scroll_viewer.viewport().height {
                    return false;
                }
                let initial = scroll_viewer.offset();
                for _ in 0..steps {
                    if delta_y > 0.0 {
                        scroll_viewer.line_up();
                    } else {
                        scroll_viewer.line_down();
                    }
                }
                scroll_viewer.offset() != initial
            }
        }
    }
}
