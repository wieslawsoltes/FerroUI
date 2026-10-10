//! Conversions between the geometry and the colours of Core Graphics and
//! UIKit and those of the framework.

use ferroui_base::media::Color;

/// A component of a colour as UIKit takes it, between 0 and 1.
pub fn color_component(c: u8) -> f32 {
    f32::from(c) / 255.0
}

/// The red, green, blue and alpha components of a colour, between 0 and 1.
pub fn color_components(color: Color) -> [f32; 4] {
    [color_component(color.r), color_component(color.g), color_component(color.b), color_component(color.a)]
}

#[cfg(target_os = "ios")]
pub use uikit::{to_cg_rect, to_point, to_rect, to_size, to_ui_color};

#[cfg(target_os = "ios")]
mod uikit {
    use super::color_components;
    use ferroui_base::media::Color;
    use ferroui_base::{Point, Rect, Size};
    use objc2::rc::Retained;
    use objc2_core_foundation::{CGFloat, CGPoint, CGRect, CGSize};
    use objc2_ui_kit::UIColor;

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

    /// The rectangle as a rectangle of Core Graphics.
    pub fn to_cg_rect(rect: Rect) -> CGRect {
        CGRect::new(CGPoint::new(rect.x, rect.y), CGSize::new(rect.width, rect.height))
    }

    /// The colour as a colour of UIKit.
    pub fn to_ui_color(color: Color) -> Retained<UIColor> {
        let [red, green, blue, alpha] = color_components(color);
        UIColor::colorWithRed_green_blue_alpha(
            CGFloat::from(red),
            CGFloat::from(green),
            CGFloat::from(blue),
            CGFloat::from(alpha),
        )
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of this file.
    use super::*;

    #[test]
    fn the_components_of_a_colour_are_between_zero_and_one() {
        assert_eq!(0.0, color_component(0));
        assert_eq!(1.0, color_component(255));
        assert_eq!(51.0 / 255.0, color_component(51));
        // Red, green, blue, alpha: the order of UIKit, not of the colour.
        assert_eq!([1.0, 0.0, 51.0 / 255.0, 128.0 / 255.0], color_components(Color::from_argb(128, 255, 0, 51)));
    }
}
