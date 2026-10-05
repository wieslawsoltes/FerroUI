use super::{HorizontalAlignment, VerticalAlignment};
use crate::Rect;

/// Extension methods for layout types.
pub struct LayoutExtensions;

impl LayoutExtensions {
    /// Aligns a rect in a constraining rect according to horizontal and
    /// vertical alignment settings.
    pub fn align(
        mut rect: Rect,
        constraint: Rect,
        horizontal_alignment: HorizontalAlignment,
        vertical_alignment: VerticalAlignment,
    ) -> Rect {
        match horizontal_alignment {
            HorizontalAlignment::Center => rect = rect.with_x((constraint.width - rect.width) / 2.0),
            HorizontalAlignment::Right => rect = rect.with_x(constraint.width - rect.width),
            HorizontalAlignment::Stretch => {
                rect = Rect::new(0.0, rect.y, constraint.width.max(rect.width), rect.height)
            }
            HorizontalAlignment::Left => {}
        }

        match vertical_alignment {
            VerticalAlignment::Center => rect = rect.with_y((constraint.height - rect.height) / 2.0),
            VerticalAlignment::Bottom => rect = rect.with_y(constraint.height - rect.height),
            VerticalAlignment::Stretch => {
                rect = Rect::new(rect.x, 0.0, rect.width, constraint.height.max(rect.height))
            }
            VerticalAlignment::Top => {}
        }

        rect
    }
}
