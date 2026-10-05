// Portions of this source file are adapted from the Windows Presentation Foundation (WPF) project
// (https://github.com/dotnet/wpf) and from the WinUI project
// (https://github.com/microsoft/microsoft-ui-xaml/tree/winui3/main), both under the MIT License.

use crate::media::{BackgroundSizing, StreamGeometryContext, SweepDirection};
use crate::platform::IGeometryContext;
use crate::utilities::{math_utilities, MathUtilities};
use crate::{CornerRadius, Point, Rect, RoundedRect, Size, Thickness, Vector};

/// Same as the layout epsilon of the layout helper.
const EPSILON: f64 = 0.00000153;

/// Contains internal helpers used to build and draw various geometries.
pub struct GeometryBuilder;

impl GeometryBuilder {
    /// Draws a new rounded rectangle within the given geometry context.
    /// Warning: The caller must manage and dispose the [`StreamGeometryContext`] externally.
    ///
    /// WinUI: https://github.com/microsoft/microsoft-ui-xaml/blob/93742a178db8f625ba9299f62c21f656e0b195ad/dxaml/xcp/core/core/elements/geometry.cpp#L1072-L1079
    ///
    /// * `context` - The geometry context to draw into.
    /// * `keypoints` - The rounded rectangle keypoints defining the rectangle to draw.
    pub fn draw_rounded_corners_rectangle_keypoints(
        context: &mut StreamGeometryContext,
        keypoints: &RoundedRectKeypoints,
    ) {
        let mut radius_x;
        let mut radius_y;

        context.begin_figure(keypoints.top_left, true);

        // Top
        context.line_to(keypoints.top_right, true);

        // TopRight corner
        radius_x = keypoints.right_top.x - keypoints.top_right.x;
        radius_y = keypoints.top_right.y - keypoints.right_top.y;
        radius_x = if radius_x > 0.0 { radius_x } else { -radius_x };
        radius_y = if radius_y > 0.0 { radius_y } else { -radius_y };

        context.arc_to(
            keypoints.right_top,
            Size::new(radius_x, radius_y),
            0.0,
            false,
            SweepDirection::Clockwise,
            true,
        );

        // Right
        context.line_to(keypoints.right_bottom, true);

        // BottomRight corner
        radius_x = keypoints.right_bottom.x - keypoints.bottom_right.x;
        radius_y = keypoints.bottom_right.y - keypoints.right_bottom.y;
        radius_x = if radius_x > 0.0 { radius_x } else { -radius_x };
        radius_y = if radius_y > 0.0 { radius_y } else { -radius_y };

        if radius_x != 0.0 || radius_y != 0.0 {
            context.arc_to(
                keypoints.bottom_right,
                Size::new(radius_x, radius_y),
                0.0,
                false,
                SweepDirection::Clockwise,
                true,
            );
        }

        // Bottom
        context.line_to(keypoints.bottom_left, true);

        // BottomLeft corner
        radius_x = keypoints.bottom_left.x - keypoints.left_bottom.x;
        radius_y = keypoints.bottom_left.y - keypoints.left_bottom.y;
        radius_x = if radius_x > 0.0 { radius_x } else { -radius_x };
        radius_y = if radius_y > 0.0 { radius_y } else { -radius_y };

        if radius_x != 0.0 || radius_y != 0.0 {
            context.arc_to(
                keypoints.left_bottom,
                Size::new(radius_x, radius_y),
                0.0,
                false,
                SweepDirection::Clockwise,
                true,
            );
        }

        // Left
        context.line_to(keypoints.left_top, true);

        // TopLeft corner
        radius_x = keypoints.top_left.x - keypoints.left_top.x;
        radius_y = keypoints.top_left.y - keypoints.left_top.y;
        radius_x = if radius_x > 0.0 { radius_x } else { -radius_x };
        radius_y = if radius_y > 0.0 { radius_y } else { -radius_y };

        if radius_x != 0.0 || radius_y != 0.0 {
            context.arc_to(
                keypoints.top_left,
                Size::new(radius_x, radius_y),
                0.0,
                false,
                SweepDirection::Clockwise,
                true,
            );
        }

        context.end_figure(true);
    }

    /// Draws a new rounded rectangle within the given geometry context.
    /// Warning: The caller must manage and dispose the [`StreamGeometryContext`] externally.
    ///
    /// * `context` - The geometry context to draw into.
    /// * `rect` - The existing rectangle dimensions without corner radii.
    /// * `radius_x` - The radius on the X-axis used to round the corners of the rectangle.
    /// * `radius_y` - The radius on the Y-axis used to round the corners of the rectangle.
    pub fn draw_rounded_corners_rectangle(context: &mut StreamGeometryContext, rect: Rect, radius_x: f64, radius_y: f64) {
        let arc_size = Size::new(radius_x, radius_y);

        // The rectangle is constructed as follows:
        //
        //   (origin)
        //   Corner 4            Corner 1
        //   Top/Left  Line 1    Top/Right
        //      \_   __________   _/
        //          |          |
        //   Line 4 |          | Line 2
        //       _  |__________|  _
        //      /      Line 3      \
        //   Corner 3            Corner 2
        //   Bottom/Left         Bottom/Right
        //
        // - Lines 1,3 follow the deflated rectangle bounds minus RadiusX
        // - Lines 2,4 follow the deflated rectangle bounds minus RadiusY
        // - All corners are constructed using elliptical arcs

        context.begin_figure(Point::new(rect.left() + radius_x, rect.top()), true);

        // Line 1 + Corner 1
        context.line_to(Point::new(rect.right() - radius_x, rect.top()), true);
        context.arc_to(
            Point::new(rect.right(), rect.top() + radius_y),
            arc_size,
            0.0,
            false,
            SweepDirection::Clockwise,
            true,
        );

        // Line 2 + Corner 2
        context.line_to(Point::new(rect.right(), rect.bottom() - radius_y), true);
        context.arc_to(
            Point::new(rect.right() - radius_x, rect.bottom()),
            arc_size,
            0.0,
            false,
            SweepDirection::Clockwise,
            true,
        );

        // Line 3 + Corner 3
        context.line_to(Point::new(rect.left() + radius_x, rect.bottom()), true);
        context.arc_to(
            Point::new(rect.left(), rect.bottom() - radius_y),
            arc_size,
            0.0,
            false,
            SweepDirection::Clockwise,
            true,
        );

        // Line 4 + Corner 4
        context.line_to(Point::new(rect.left(), rect.top() + radius_y), true);
        context.arc_to(
            Point::new(rect.left() + radius_x, rect.top()),
            arc_size,
            0.0,
            false,
            SweepDirection::Clockwise,
            true,
        );

        context.end_figure(true);
    }

    /// Calculates the keypoints of a rounded rectangle based on the algorithm in WinUI.
    /// These keypoints may then be drawn or transformed into other types.
    ///
    /// * `outer_bounds` - The outer bounds of the rounded rectangle.
    ///   This should be the overall bounds and size of the shape/control without any
    ///   corner radii or border thickness adjustments.
    /// * `border_thickness` - The unadjusted border thickness of the rounded rectangle.
    /// * `corner_radius` - The unadjusted corner radii of the rounded rectangle.
    ///   The corner radius is defined to be the middle of the border stroke (center of the border).
    /// * `sizing` - The sizing mode used to calculate the final rounded rectangle size.
    ///
    /// Returns new rounded rectangle keypoints.
    pub fn calculate_rounded_corners_rectangle_win_ui(
        outer_bounds: Rect,
        border_thickness: Thickness,
        corner_radius: CornerRadius,
        sizing: BackgroundSizing,
    ) -> RoundedRectKeypoints {
        // This was initially derived from WinUI:
        //  - CGeometryBuilder::CalculateRoundedCornersRectangle
        //    https://github.com/microsoft/microsoft-ui-xaml/blob/93742a178db8f625ba9299f62c21f656e0b195ad/dxaml/xcp/core/core/elements/geometry.cpp#L862-L869
        //
        // It has been modified to accept a BackgroundSizing parameter directly as well
        // as to support BackgroundSizing::CenterBorder.
        //
        // Keep in mind:
        //   > In Xaml, the corner radius is defined to be the middle of the stroke
        //   > (i.e. half the border thickness extends to either side).

        let f_outer;
        let mut bound_rect = outer_bounds;

        if sizing == BackgroundSizing::InnerBorderEdge {
            bound_rect = outer_bounds.deflate_thickness(border_thickness);
            f_outer = false;
        } else if sizing == BackgroundSizing::OuterBorderEdge {
            f_outer = true;
        } else {
            // CenterBorder
            // This is a trick to support a 3rd state (CenterBorder) using the same WinUI-based algorithm.
            // The WinUI algorithm only supports the fOuter = True|False parameter.
            bound_rect = outer_bounds.deflate_thickness(border_thickness * 0.5);
            f_outer = false;
        }

        // Start of WinUI converted code
        // Doubles are used for calculation so multiple Point structs aren't
        // required during calculations -- everything can be done with these double variables.
        let f_left_top;
        let f_left_bottom;
        let f_top_left;
        let f_top_right;
        let f_right_top;
        let f_right_bottom;
        let f_bottom_left;
        let f_bottom_right;

        let left;
        let right;
        let top;
        let bottom;

        // If the caller wants to take the border into account
        // initialize the borders variables
        if border_thickness != Thickness::default() {
            left = 0.5 * border_thickness.left;
            right = 0.5 * border_thickness.right;
            top = 0.5 * border_thickness.top;
            bottom = 0.5 * border_thickness.bottom;
        } else {
            left = 0.0;
            right = 0.0;
            top = 0.0;
            bottom = 0.0;
        }

        // The following if/else block initializes the variables
        // of which the points of the path will be created
        // In case of outer, add the border - if any.
        // Otherwise (inner rectangle) subtract the border - if any
        if f_outer {
            if MathUtilities::are_close_eps(corner_radius.top_left, 0.0, EPSILON) {
                f_left_top = 0.0;
                f_top_left = 0.0;
            } else {
                f_left_top = corner_radius.top_left + left;
                f_top_left = corner_radius.top_left + top;
            }

            if MathUtilities::are_close_eps(corner_radius.top_right, 0.0, EPSILON) {
                f_top_right = 0.0;
                f_right_top = 0.0;
            } else {
                f_top_right = corner_radius.top_right + top;
                f_right_top = corner_radius.top_right + right;
            }

            if MathUtilities::are_close_eps(corner_radius.bottom_right, 0.0, EPSILON) {
                f_right_bottom = 0.0;
                f_bottom_right = 0.0;
            } else {
                f_right_bottom = corner_radius.bottom_right + right;
                f_bottom_right = corner_radius.bottom_right + bottom;
            }

            if MathUtilities::are_close_eps(corner_radius.bottom_left, 0.0, EPSILON) {
                f_bottom_left = 0.0;
                f_left_bottom = 0.0;
            } else {
                f_bottom_left = corner_radius.bottom_left + bottom;
                f_left_bottom = corner_radius.bottom_left + left;
            }
        } else {
            f_left_top = math_utilities::max(0.0, corner_radius.top_left - left);
            f_top_left = math_utilities::max(0.0, corner_radius.top_left - top);
            f_top_right = math_utilities::max(0.0, corner_radius.top_right - top);
            f_right_top = math_utilities::max(0.0, corner_radius.top_right - right);
            f_right_bottom = math_utilities::max(0.0, corner_radius.bottom_right - right);
            f_bottom_right = math_utilities::max(0.0, corner_radius.bottom_right - bottom);
            f_bottom_left = math_utilities::max(0.0, corner_radius.bottom_left - bottom);
            f_left_bottom = math_utilities::max(0.0, corner_radius.bottom_left - left);
        }

        let mut top_left_x = f_left_top;
        let top_left_y = 0.0;

        let mut top_right_x = bound_rect.width - f_right_top;
        let top_right_y = 0.0;

        let right_top_x = bound_rect.width;
        let mut right_top_y = f_top_right;

        let right_bottom_x = bound_rect.width;
        let mut right_bottom_y = bound_rect.height - f_bottom_right;

        let mut bottom_right_x = bound_rect.width - f_right_bottom;
        let bottom_right_y = bound_rect.height;

        let mut bottom_left_x = f_left_bottom;
        let bottom_left_y = bound_rect.height;

        let left_bottom_x = 0.0;
        let mut left_bottom_y = bound_rect.height - f_bottom_left;

        let left_top_x = 0.0;
        let mut left_top_y = f_top_left;

        // check keypoints for overlap and resolve by partitioning radii according to
        // the percentage of each one.

        // top edge
        if top_left_x > top_right_x {
            let v = (f_left_top) / (f_left_top + f_right_top) * bound_rect.width;
            top_left_x = v;
            top_right_x = v;
        }
        // right edge
        if right_top_y > right_bottom_y {
            let v = (f_top_right) / (f_top_right + f_bottom_right) * bound_rect.height;
            right_top_y = v;
            right_bottom_y = v;
        }
        // bottom edge
        if bottom_right_x < bottom_left_x {
            let v = (f_left_bottom) / (f_left_bottom + f_right_bottom) * bound_rect.width;
            bottom_right_x = v;
            bottom_left_x = v;
        }
        // left edge
        if left_bottom_y < left_top_y {
            let v = (f_top_left) / (f_top_left + f_bottom_left) * bound_rect.height;
            left_bottom_y = v;
            left_top_y = v;
        }

        // The above code does all calculations without taking into consideration X/Y absolute position.
        // In WinUI, this is compensated for in DrawRoundedCornersRectangle(); however, we do this here directly
        // when the final keypoints are being created.
        let mut keypoints = RoundedRectKeypoints::new();
        keypoints.top_left = Point::new(bound_rect.x + top_left_x, bound_rect.y + top_left_y);
        keypoints.top_right = Point::new(bound_rect.x + top_right_x, bound_rect.y + top_right_y);

        keypoints.right_top = Point::new(bound_rect.x + right_top_x, bound_rect.y + right_top_y);
        keypoints.right_bottom = Point::new(bound_rect.x + right_bottom_x, bound_rect.y + right_bottom_y);

        keypoints.bottom_right = Point::new(bound_rect.x + bottom_right_x, bound_rect.y + bottom_right_y);
        keypoints.bottom_left = Point::new(bound_rect.x + bottom_left_x, bound_rect.y + bottom_left_y);

        keypoints.left_bottom = Point::new(bound_rect.x + left_bottom_x, bound_rect.y + left_bottom_y);
        keypoints.left_top = Point::new(bound_rect.x + left_top_x, bound_rect.y + left_top_y);

        keypoints
    }
}

/// Represents the keypoints of a rounded rectangle.
/// These keypoints can be shared between methods and turned into geometry.
///
/// A rounded rectangle is the base geometric shape used when drawing borders.
/// It is a superset of a simple rectangle (which has corner radii set to zero).
/// These keypoints can be combined together to produce geometries for both background
/// and border elements.
//
// The following keypoints are defined for a rounded rectangle:
//
//       TopLeft                                  TopRight
//              *--------------------------------*
// (start)     /                                  \
//    LeftTop *                                    * RightTop
//            |                                    |
//            |                                    |
// LeftBottom *                                    * RightBottom
//             \                                  /
//              *--------------------------------*
//    BottomLeft                                  BottomRight
//
// Or, for a simple rectangle without corner radii:
//
//    TopLeft = LeftTop                   TopRight = RightTop
//  (start)   *------------------------------------*
//            |                                    |
//            |                                    |
//            *------------------------------------*
// BottomLeft = LeftBottom             BottomRight = RightBottom
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct RoundedRectKeypoints {
    /// The topmost point in the left line segment of the rectangle.
    pub left_top: Point,
    /// The leftmost point in the top line segment of the rectangle.
    pub top_left: Point,
    /// The rightmost point in the top line segment of the rectangle.
    pub top_right: Point,
    /// The topmost point in the right line segment of the rectangle.
    pub right_top: Point,
    /// The bottommost point in the right line segment of the rectangle.
    pub right_bottom: Point,
    /// The rightmost point in the bottom line segment of the rectangle.
    pub bottom_right: Point,
    /// The leftmost point in the bottom line segment of the rectangle.
    pub bottom_left: Point,
    /// The bottommost point in the left line segment of the rectangle.
    pub left_bottom: Point,
}

impl RoundedRectKeypoints {
    /// Initializes a new instance of the [`RoundedRectKeypoints`] struct.
    pub fn new() -> Self {
        Self::default()
    }

    /// Initializes a new instance of the [`RoundedRectKeypoints`] struct.
    ///
    /// * `rounded_rect` - An existing [`RoundedRect`] to initialize keypoints with.
    pub fn from_rounded_rect(rounded_rect: RoundedRect) -> Self {
        let rect = rounded_rect.rect;
        Self {
            left_top: Point::new(rect.top_left().x, rect.top_left().y + rounded_rect.radii_top_left.y),
            top_left: Point::new(rect.top_left().x + rounded_rect.radii_top_left.x, rect.top_left().y),
            top_right: Point::new(rect.top_right().x - rounded_rect.radii_top_right.x, rect.top_right().y),
            right_top: Point::new(rect.top_right().x, rect.top_right().y + rounded_rect.radii_top_right.y),
            right_bottom: Point::new(
                rect.bottom_right().x,
                rect.bottom_right().y - rounded_rect.radii_bottom_right.y,
            ),
            bottom_right: Point::new(
                rect.bottom_right().x - rounded_rect.radii_bottom_right.x,
                rect.bottom_right().y,
            ),
            bottom_left: Point::new(rect.bottom_left().x + rounded_rect.radii_bottom_left.x, rect.bottom_left().y),
            left_bottom: Point::new(rect.bottom_left().x, rect.bottom_right().y - rounded_rect.radii_bottom_left.y),
        }
    }

    /// Gets a value indicating whether the rounded rectangle is actually rounded on
    /// any corner. If false the key points represent a simple rectangle.
    pub fn is_rounded(&self) -> bool {
        self.top_left != self.left_top
            || self.top_right != self.right_top
            || self.bottom_left != self.left_bottom
            || self.bottom_right != self.right_bottom
    }

    /// Converts the keypoints into a simple rectangle (with no corners).
    /// This is equivalent to the outer rectangle with zero corner radii.
    ///
    /// Warning: This will force the keypoints into a simple rectangle without
    /// any rounded corners. Use [`is_rounded`](Self::is_rounded) to determine if corner
    /// information is otherwise available.
    pub fn to_rect(&self) -> Rect {
        Rect::from_points(
            Point::new(self.left_top.x, self.top_left.y),
            Point::new(self.right_bottom.x, self.bottom_right.y),
        )
    }

    /// Converts the keypoints into a rounded rectangle with elliptical corner radii.
    ///
    /// Elliptical corner radius (represented by [`Vector`]) is more powerful
    /// than circular corner radius (represented by a [`CornerRadius`]).
    /// Elliptical is a superset of circular.
    pub fn to_rounded_rect(&self) -> RoundedRect {
        RoundedRect::new(
            self.to_rect(),
            Vector::new(self.top_left.x - self.left_top.x, self.left_top.y - self.top_left.y),
            Vector::new(self.right_top.x - self.top_right.x, self.right_top.y - self.top_right.y),
            Vector::new(self.right_bottom.x - self.bottom_right.x, self.bottom_right.y - self.right_bottom.y),
            Vector::new(self.bottom_left.x - self.left_bottom.x, self.bottom_left.y - self.left_bottom.y),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::FillRule;
    use crate::platform::IStreamGeometryContextImpl;
    use std::cell::RefCell;
    use std::rc::Rc;

    fn inner_border_edge_borders_larger_than_corners(uniform_borders: f64, uniform_corners: f64) {
        let bounds = Rect::from_size(Size::new(100.0, 100.0));
        let border_thickness = Thickness::uniform(uniform_borders);
        let corner_radius = CornerRadius::uniform(uniform_corners);

        let points = GeometryBuilder::calculate_rounded_corners_rectangle_win_ui(
            bounds,
            border_thickness,
            corner_radius,
            BackgroundSizing::InnerBorderEdge,
        );

        assert_eq!(Point::new(uniform_borders, uniform_borders), points.left_top);
        assert_eq!(Point::new(uniform_borders, uniform_borders), points.top_left);
        assert_eq!(Point::new(100.0 - uniform_borders, uniform_borders), points.top_right);
        assert_eq!(Point::new(100.0 - uniform_borders, uniform_borders), points.right_top);
        assert_eq!(Point::new(100.0 - uniform_borders, 100.0 - uniform_borders), points.right_bottom);
        assert_eq!(Point::new(100.0 - uniform_borders, 100.0 - uniform_borders), points.bottom_right);
        assert_eq!(Point::new(uniform_borders, 100.0 - uniform_borders), points.bottom_left);
        assert_eq!(Point::new(uniform_borders, 100.0 - uniform_borders), points.left_bottom);

        assert!(!points.is_rounded());
    }

    #[test]
    fn calculate_rounded_corners_rectangle_win_ui_inner_border_edge_borders_larger_than_corners_test() {
        for (uniform_borders, uniform_corners) in [(20.0, 10.0), (10.0, 5.0), (2.0, 1.0), (1.0, 0.0)] {
            inner_border_edge_borders_larger_than_corners(uniform_borders, uniform_corners);
        }
    }

    fn outer_border_edge_borders_larger_than_corners(uniform_borders: f64, uniform_corners: f64) {
        let bounds = Rect::from_size(Size::new(100.0, 100.0));
        let border_thickness = Thickness::uniform(uniform_borders);
        let corner_radius = CornerRadius::uniform(uniform_corners);

        let points = GeometryBuilder::calculate_rounded_corners_rectangle_win_ui(
            bounds,
            border_thickness,
            corner_radius,
            BackgroundSizing::OuterBorderEdge,
        );

        assert_eq!(Point::new(0.0, uniform_borders), points.left_top);
        assert_eq!(Point::new(uniform_borders, 0.0), points.top_left);
        assert_eq!(Point::new(100.0 - uniform_borders, 0.0), points.top_right);
        assert_eq!(Point::new(100.0, uniform_borders), points.right_top);
        assert_eq!(Point::new(100.0, 100.0 - uniform_borders), points.right_bottom);
        assert_eq!(Point::new(100.0 - uniform_borders, 100.0), points.bottom_right);
        assert_eq!(Point::new(uniform_borders, 100.0), points.bottom_left);
        assert_eq!(Point::new(0.0, 100.0 - uniform_borders), points.left_bottom);

        assert!(points.is_rounded());
    }

    #[test]
    fn calculate_rounded_corners_rectangle_win_ui_outer_border_edge_borders_larger_than_corners_test() {
        for (uniform_borders, uniform_corners) in [(20.0, 10.0), (10.0, 5.0), (2.0, 1.0)] {
            outer_border_edge_borders_larger_than_corners(uniform_borders, uniform_corners);
        }
    }

    // --- The tests below are not upstream tests; expected values are derived by hand from the
    // --- upstream algorithm.

    #[derive(Default)]
    struct Recorder {
        log: Rc<RefCell<Vec<String>>>,
    }

    impl IGeometryContext for Recorder {
        fn arc_to(&mut self, point: Point, size: Size, angle: f64, large: bool, sweep: SweepDirection, stroked: bool) {
            self.log.borrow_mut().push(format!("arc {point} {size} {angle} {large} {sweep:?} {stroked}"));
        }
        fn begin_figure(&mut self, start_point: Point, is_filled: bool) {
            self.log.borrow_mut().push(format!("begin {start_point} {is_filled}"));
        }
        fn cubic_bezier_to(&mut self, p1: Point, p2: Point, p3: Point, stroked: bool) {
            self.log.borrow_mut().push(format!("cubic {p1} {p2} {p3} {stroked}"));
        }
        fn quadratic_bezier_to(&mut self, p1: Point, p2: Point, stroked: bool) {
            self.log.borrow_mut().push(format!("quad {p1} {p2} {stroked}"));
        }
        fn line_to(&mut self, point: Point, stroked: bool) {
            self.log.borrow_mut().push(format!("line {point} {stroked}"));
        }
        fn end_figure(&mut self, is_closed: bool) {
            self.log.borrow_mut().push(format!("end {is_closed}"));
        }
        fn set_fill_rule(&mut self, _: FillRule) {}
        fn dispose(&mut self) {}
    }

    impl IStreamGeometryContextImpl for Recorder {}

    fn record(draw: impl FnOnce(&mut StreamGeometryContext)) -> Vec<String> {
        let log = Rc::new(RefCell::new(Vec::new()));
        let mut context = StreamGeometryContext::new(Box::new(Recorder { log: log.clone() }));
        draw(&mut context);
        drop(context);
        let result = log.borrow().clone();
        result
    }

    #[test]
    fn keypoints_from_rounded_rect_round_trip() {
        let rounded_rect = RoundedRect::new(
            Rect::new(10.0, 20.0, 100.0, 50.0),
            Vector::new(1.0, 2.0),
            Vector::new(3.0, 4.0),
            Vector::new(5.0, 6.0),
            Vector::new(7.0, 8.0),
        );

        let keypoints = RoundedRectKeypoints::from_rounded_rect(rounded_rect);

        assert_eq!(Point::new(10.0, 22.0), keypoints.left_top);
        assert_eq!(Point::new(11.0, 20.0), keypoints.top_left);
        assert_eq!(Point::new(107.0, 20.0), keypoints.top_right);
        assert_eq!(Point::new(110.0, 24.0), keypoints.right_top);
        assert_eq!(Point::new(110.0, 64.0), keypoints.right_bottom);
        assert_eq!(Point::new(105.0, 70.0), keypoints.bottom_right);
        assert_eq!(Point::new(17.0, 70.0), keypoints.bottom_left);
        assert_eq!(Point::new(10.0, 62.0), keypoints.left_bottom);
        assert!(keypoints.is_rounded());
        assert_eq!(Rect::new(10.0, 20.0, 100.0, 50.0), keypoints.to_rect());
        assert_eq!(rounded_rect, keypoints.to_rounded_rect());
    }

    #[test]
    fn default_keypoints_are_not_rounded() {
        let keypoints = RoundedRectKeypoints::new();
        assert!(!keypoints.is_rounded());
        assert_eq!(Rect::default(), keypoints.to_rect());
    }

    #[test]
    fn center_border_deflates_by_half_the_border() {
        let points = GeometryBuilder::calculate_rounded_corners_rectangle_win_ui(
            Rect::new(10.0, 10.0, 100.0, 60.0),
            Thickness::uniform(4.0),
            CornerRadius::uniform(10.0),
            BackgroundSizing::CenterBorder,
        );

        // bounds deflated by 2 -> (12, 12, 96, 56); radii reduced by half the border -> 8
        assert_eq!(Point::new(12.0, 20.0), points.left_top);
        assert_eq!(Point::new(20.0, 12.0), points.top_left);
        assert_eq!(Point::new(100.0, 12.0), points.top_right);
        assert_eq!(Point::new(108.0, 20.0), points.right_top);
        assert_eq!(Point::new(108.0, 60.0), points.right_bottom);
        assert_eq!(Point::new(100.0, 68.0), points.bottom_right);
        assert_eq!(Point::new(20.0, 68.0), points.bottom_left);
        assert_eq!(Point::new(12.0, 60.0), points.left_bottom);
    }

    #[test]
    fn overlapping_radii_are_partitioned_proportionally() {
        let points = GeometryBuilder::calculate_rounded_corners_rectangle_win_ui(
            Rect::new(0.0, 0.0, 100.0, 100.0),
            Thickness::default(),
            CornerRadius::new(90.0, 30.0, 30.0, 90.0),
            BackgroundSizing::OuterBorderEdge,
        );

        // top edge: 90 + 30 > 100 -> split at 90 / 120 * 100 = 75
        assert_eq!(Point::new(75.0, 0.0), points.top_left);
        assert_eq!(Point::new(75.0, 0.0), points.top_right);
        // bottom edge: same split
        assert_eq!(Point::new(75.0, 100.0), points.bottom_left);
        assert_eq!(Point::new(75.0, 100.0), points.bottom_right);
        // right edge: 30 + 30 <= 100 -> untouched
        assert_eq!(Point::new(100.0, 30.0), points.right_top);
        assert_eq!(Point::new(100.0, 70.0), points.right_bottom);
        // left edge: 90 + 90 > 100 -> split at 90 / 180 * 100 = 50
        assert_eq!(Point::new(0.0, 50.0), points.left_top);
        assert_eq!(Point::new(0.0, 50.0), points.left_bottom);
    }

    #[test]
    fn keypoints_are_drawn_with_arcs_only_on_rounded_corners() {
        let keypoints = RoundedRectKeypoints::from_rounded_rect(RoundedRect::new(
            Rect::new(0.0, 0.0, 100.0, 50.0),
            Vector::new(10.0, 5.0),
            Vector::new(0.0, 0.0),
            Vector::new(20.0, 10.0),
            Vector::new(0.0, 0.0),
        ));

        let log = record(|context| GeometryBuilder::draw_rounded_corners_rectangle_keypoints(context, &keypoints));

        assert_eq!(
            vec![
                "begin 10, 0 true",
                "line 100, 0 true",
                // The top right corner is always drawn, even with zero radii (which the upstream
                // sign flip turns into negative zeros).
                "arc 100, 0 -0, -0 0 false Clockwise true",
                "line 100, 40 true",
                "arc 80, 50 20, 10 0 false Clockwise true",
                "line 0, 50 true",
                "line 0, 5 true",
                "arc 10, 0 10, 5 0 false Clockwise true",
                "end true",
            ],
            log
        );
    }

    #[test]
    fn uniform_radii_rectangle_is_drawn_with_four_arcs() {
        let log = record(|context| {
            GeometryBuilder::draw_rounded_corners_rectangle(context, Rect::new(0.0, 0.0, 100.0, 50.0), 10.0, 5.0)
        });

        assert_eq!(
            vec![
                "begin 10, 0 true",
                "line 90, 0 true",
                "arc 100, 5 10, 5 0 false Clockwise true",
                "line 100, 45 true",
                "arc 90, 50 10, 5 0 false Clockwise true",
                "line 10, 50 true",
                "arc 0, 45 10, 5 0 false Clockwise true",
                "line 0, 5 true",
                "arc 10, 0 10, 5 0 false Clockwise true",
                "end true",
            ],
            log
        );
    }
}
