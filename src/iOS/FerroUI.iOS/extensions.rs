//! Conversions between the geometry of Core Graphics and of the framework.
//!
//! Stage 2 of `docs/porting/ios-platform.md` adds the conversion of a
//! colour to a `UIColor`, which the text input is the first to need.

use ferroui_base::{Point, Rect, Size};
use objc2_core_foundation::{CGPoint, CGRect, CGSize};

/// The size as a size of the framework.
pub fn to_size(size: CGSize) -> Size {
    Size::new(size.width, size.height)
}

/// The point as a point of the framework.
pub fn to_point(point: CGPoint) -> Point {
    Point::new(point.x, point.y)
}

/// The rectangle as a rectangle of the framework.
pub fn to_rect(rect: CGRect) -> Rect {
    Rect::new(rect.origin.x, rect.origin.y, rect.size.width, rect.size.height)
}
