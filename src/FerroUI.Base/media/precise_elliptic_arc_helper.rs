// Copyright (c) 2003-2004, Luc Maisonobe
// 2015 - Alexey Rozanov <thehdotx@gmail.com> - Adaptations and oval center computations
// 2022 - Alexey Rozanov <thehdotx@gmail.com> - Fix for arcs sometimes drawn in inverted order.
// All rights reserved.
//
// Redistribution and use in source and binary forms, with
// or without modification, are permitted provided that
// the following conditions are met:
//
//    Redistributions of source code must retain the
//    above copyright notice, this list of conditions and
//    the following disclaimer.
//    Redistributions in binary form must reproduce the
//    above copyright notice, this list of conditions and
//    the following disclaimer in the documentation
//    and/or other materials provided with the
//    distribution.
//    Neither the names of spaceroots.org, spaceroots.com
//    nor the names of their contributors may be used to
//    endorse or promote products derived from this
//    software without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND
// CONTRIBUTORS "AS IS" AND ANY EXPRESS OR IMPLIED
// WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
// WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A
// PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL
// THE COPYRIGHT OWNER OR CONTRIBUTORS BE LIABLE FOR ANY
// DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
// CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,
// PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF
// USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
// HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER
// IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING
// NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE
// USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
// POSSIBILITY OF SUCH DAMAGE.
//
// Adapted from http://www.spaceroots.org/documents/ellipse/EllipticalArc.java
// (see NOTICE.md next to this file for the unabridged upstream header).

use std::f64::consts::PI;
use std::ops::Mul;

use crate::media::{StreamGeometryContext, SweepDirection};
use crate::platform::IGeometryContext;
use crate::utilities::math_utilities;
use crate::{Point, Rect, Size, Vector};

const TWO_PI: f64 = 2.0 * PI;

/// Coefficients for error estimation while using quadratic Bezier curves for approximation,
/// 0 <= b/a <= 0.25
const COEFFS_2_LOW: [[[f64; 4]; 4]; 2] = [
    [
        [3.92478, -13.5822, -0.233377, 0.0128206],
        [-1.08814, 0.859987, 3.62265E-4, 2.29036E-4],
        [-0.942512, 0.390456, 0.0080909, 0.00723895],
        [-0.736228, 0.20998, 0.0129867, 0.0103456],
    ],
    [
        [-0.395018, 6.82464, 0.0995293, 0.0122198],
        [-0.545608, 0.0774863, 0.0267327, 0.0132482],
        [0.0534754, -0.0884167, 0.012595, 0.0343396],
        [0.209052, -0.0599987, -0.00723897, 0.00789976],
    ],
];

/// Coefficients for error estimation while using quadratic Bezier curves for approximation,
/// 0.25 <= b/a <= 1
const COEFFS_2_HIGH: [[[f64; 4]; 4]; 2] = [
    [
        [0.0863805, -11.5595, -2.68765, 0.181224],
        [0.242856, -1.81073, 1.56876, 1.68544],
        [0.233337, -0.455621, 0.222856, 0.403469],
        [0.0612978, -0.104879, 0.0446799, 0.00867312],
    ],
    [
        [0.028973, 6.68407, 0.171472, 0.0211706],
        [0.0307674, -0.0517815, 0.0216803, -0.0749348],
        [-0.0471179, 0.1288, -0.0781702, 2.0],
        [-0.0309683, 0.0531557, -0.0227191, 0.0434511],
    ],
];

/// Safety factor to convert the "best" error approximation into a "max bound" error
const SAFETY_2: [f64; 4] = [0.02, 2.83, 0.125, 0.01];

/// Coefficients for error estimation while using cubic Bezier curves for approximation,
/// 0 <= b/a <= 0.25
const COEFFS_3_LOW: [[[f64; 4]; 4]; 2] = [
    [
        [3.85268, -21.229, -0.330434, 0.0127842],
        [-1.61486, 0.706564, 0.225945, 0.263682],
        [-0.910164, 0.388383, 0.00551445, 0.00671814],
        [-0.630184, 0.192402, 0.0098871, 0.0102527],
    ],
    [
        [-0.162211, 9.94329, 0.13723, 0.0124084],
        [-0.253135, 0.00187735, 0.0230286, 0.01264],
        [-0.0695069, -0.0437594, 0.0120636, 0.0163087],
        [-0.0328856, -0.00926032, -0.00173573, 0.00527385],
    ],
];

/// Coefficients for error estimation while using cubic Bezier curves for approximation,
/// 0.25 <= b/a <= 1
const COEFFS_3_HIGH: [[[f64; 4]; 4]; 2] = [
    [
        [0.0899116, -19.2349, -4.11711, 0.183362],
        [0.138148, -1.45804, 1.32044, 1.38474],
        [0.230903, -0.450262, 0.219963, 0.414038],
        [0.0590565, -0.101062, 0.0430592, 0.0204699],
    ],
    [
        [0.0164649, 9.89394, 0.0919496, 0.00760802],
        [0.0191603, -0.0322058, 0.0134667, -0.0825018],
        [0.0156192, -0.017535, 0.00326508, -0.228157],
        [-0.0236752, 0.0405821, -0.0173086, 0.176187],
    ],
];

/// Safety factor to convert the "best" error approximation into a "max bound" error
const SAFETY_3: [f64; 4] = [0.0010, 4.98, 0.207, 0.0067];

/// Draws elliptic arcs as polylines, quadratic or cubic Bezier curves.
pub(crate) struct PreciseEllipticArcHelper;

impl PreciseEllipticArcHelper {
    /// Draws an arc from `current_point` to `point` into the context.
    ///
    /// `rotation_angle` is in degrees.
    pub(crate) fn arc_to(
        stream_geometry_context_impl: &mut StreamGeometryContext,
        current_point: Point,
        point: Point,
        size: Size,
        rotation_angle: f64,
        is_large_arc: bool,
        sweep_direction: SweepDirection,
    ) {
        EllipticalArc::build_arc_between(
            stream_geometry_context_impl,
            current_point,
            point,
            size,
            rotation_angle * (PI / 180.0),
            is_large_arc,
            sweep_direction == SweepDirection::Clockwise,
        );
    }
}

/// This type represents an elliptical arc on a 2D plane.
///
/// It is adapted for use with a [`StreamGeometryContext`], and needs to be created explicitly
/// for each particular arc.
///
/// It can handle ellipses which are not aligned with the x and y reference axes of the plane,
/// as well as their parts.
///
/// Another improvement is that this type can handle degenerated cases like for example very
/// flat ellipses (semi-minor axis much smaller than semi-major axis) and drawing of very small
/// parts of such ellipses at very high magnification scales. This imply monitoring the drawing
/// approximation error for extremely small values. Such cases occur for example while drawing
/// orbits of comets near the perihelion.
///
/// When the arc does not cover the complete ellipse, the lines joining the center of the
/// ellipse to the endpoints can optionally be included or not in the outline, hence allowing
/// to use it for pie-charts rendering. If these lines are not included, the curve is not
/// naturally closed.
// The upstream type exposes more members than the arc drawing path uses.
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct EllipticalArc {
    /// Abscissa of the center of the ellipse
    pub(crate) cx: f64,
    /// Ordinate of the center of the ellipse
    pub(crate) cy: f64,
    /// Semi-major axis
    pub(crate) a: f64,
    /// Semi-minor axis
    pub(crate) b: f64,
    /// Orientation of the major axis with respect to the x axis
    pub(crate) theta: f64,
    /// Pre-calculated cosine value for the major-axis-to-X orientation (theta)
    cos_theta: f64,
    /// Pre-calculated sine value for the major-axis-to-X orientation (theta)
    sin_theta: f64,
    /// Start angle of the arc
    pub(crate) eta1: f64,
    /// End angle of the arc
    pub(crate) eta2: f64,
    /// Abscissa of the start point
    pub(crate) x1: f64,
    /// Ordinate of the start point
    pub(crate) y1: f64,
    /// Abscissa of the end point
    pub(crate) x2: f64,
    /// Ordinate of the end point
    pub(crate) y2: f64,
    /// Abscissa of the first focus
    pub(crate) first_focus_x: f64,
    /// Ordinate of the first focus
    pub(crate) first_focus_y: f64,
    /// Abscissa of the second focus
    pub(crate) second_focus_x: f64,
    /// Ordinate of the second focus
    pub(crate) second_focus_y: f64,
    /// Abscissa of the leftmost point of the arc
    x_left: f64,
    /// Ordinate of the highest point of the arc
    y_up: f64,
    /// Horizontal width of the arc
    width: f64,
    /// Vertical height of the arc
    height: f64,
    /// Indicator for center to endpoints line inclusion
    pub(crate) is_pie_slice: bool,
    /// Maximal degree for Bezier curve approximation
    max_degree: i32,
    /// Default flatness for Bezier curve approximation
    default_flatness: f64,
    /// Indicator for semi-major axis significance (compared to semi-minor one).
    /// Computed by dividing the (a-b) difference by the value of a.
    /// This indicator is used for an early escape in intersection test
    pub(crate) f: f64,
    /// Indicator used for an early escape in intersection test
    pub(crate) e2: f64,
    /// Indicator used for an early escape in intersection test
    pub(crate) g: f64,
    /// Indicator used for an early escape in intersection test
    pub(crate) g2: f64,
    draw_in_opposite_direction: bool,
}

/// Simple matrix used for rotate transforms.
#[derive(Clone, Copy)]
struct SimpleMatrix {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
}

impl SimpleMatrix {
    fn new(a: f64, b: f64, c: f64, d: f64) -> Self {
        Self { a, b, c, d }
    }
}

impl Mul<Point> for SimpleMatrix {
    type Output = Point;

    fn mul(self, p: Point) -> Point {
        Point::new(self.a * p.x + self.b * p.y, self.c * p.x + self.d * p.y)
    }
}

// The upstream type exposes more members than the arc drawing path uses.
#[allow(dead_code)]
impl EllipticalArc {
    /// Field initialisation shared by every constructor; the derived values are computed by
    /// the constructors afterwards.
    #[allow(clippy::too_many_arguments)]
    fn with_fields(
        cx: f64,
        cy: f64,
        a: f64,
        b: f64,
        theta: f64,
        eta1: f64,
        eta2: f64,
        cos_theta: f64,
        sin_theta: f64,
        is_pie_slice: bool,
    ) -> Self {
        Self {
            cx,
            cy,
            a,
            b,
            theta,
            cos_theta,
            sin_theta,
            eta1,
            eta2,
            x1: 0.0,
            y1: 0.0,
            x2: 0.0,
            y2: 0.0,
            first_focus_x: 0.0,
            first_focus_y: 0.0,
            second_focus_x: 0.0,
            second_focus_y: 0.0,
            x_left: 0.0,
            y_up: 0.0,
            width: 0.0,
            height: 0.0,
            is_pie_slice,
            max_degree: 3,
            default_flatness: 0.5, // half a pixel
            f: 0.0,
            e2: 0.0,
            g: 0.0,
            g2: 0.0,
            draw_in_opposite_direction: false,
        }
    }

    fn compute_all(&mut self) {
        self.compute_focii();
        self.compute_end_points();
        self.compute_bounds();
        self.compute_derived_flatness_parameters();
    }

    /// Builds an elliptical arc composed of the full unit circle around (0,0)
    pub(crate) fn new() -> Self {
        let mut arc = Self::with_fields(0.0, 0.0, 1.0, 1.0, 0.0, 0.0, TWO_PI, 1.0, 0.0, false);
        arc.compute_all();
        arc
    }

    /// Builds an elliptical arc from its canonical geometrical elements
    ///
    /// * `center` - Center of the ellipse
    /// * `a` - Semi-major axis
    /// * `b` - Semi-minor axis
    /// * `theta` - Orientation of the major axis with respect to the x axis
    /// * `lambda1` - Start angle of the arc
    /// * `lambda2` - End angle of the arc
    /// * `is_pie_slice` - If true, the lines between the center of the ellipse
    ///   and the endpoints are part of the shape (it is pie slice like)
    pub(crate) fn from_center_and_angles(
        center: Point,
        a: f64,
        b: f64,
        theta: f64,
        lambda1: f64,
        lambda2: f64,
        is_pie_slice: bool,
    ) -> Self {
        Self::from_angles(center.x, center.y, a, b, theta, lambda1, lambda2, is_pie_slice)
    }

    /// Builds an elliptical arc from its canonical geometrical elements
    ///
    /// * `cx` - Abscissa of the center of the ellipse
    /// * `cy` - Ordinate of the center of the ellipse
    /// * `a` - Semi-major axis
    /// * `b` - Semi-minor axis
    /// * `theta` - Orientation of the major axis with respect to the x axis
    /// * `lambda1` - Start angle of the arc
    /// * `lambda2` - End angle of the arc
    /// * `is_pie_slice` - If true, the lines between the center of the ellipse
    ///   and the endpoints are part of the shape (it is pie slice like)
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_angles(
        cx: f64,
        cy: f64,
        a: f64,
        b: f64,
        theta: f64,
        lambda1: f64,
        lambda2: f64,
        is_pie_slice: bool,
    ) -> Self {
        let eta1 = (lambda1.sin() / b).atan2(lambda1.cos() / a);
        let eta2 = (lambda2.sin() / b).atan2(lambda2.cos() / a);
        let mut arc = Self::with_fields(cx, cy, a, b, theta, eta1, eta2, theta.cos(), theta.sin(), is_pie_slice);
        // make sure we have eta1 <= eta2 <= eta1 + 2 PI
        arc.eta2 -= TWO_PI * ((arc.eta2 - arc.eta1) / TWO_PI).floor();
        // the preceding correction fails if we have exactly eta2-eta1 == 2*PI
        // it reduces the interval to zero length
        if lambda2 - lambda1 > PI && arc.eta2 - arc.eta1 < PI {
            arc.eta2 += TWO_PI;
        }
        arc.compute_all();
        arc
    }

    /// Build a full ellipse from its canonical geometrical elements
    ///
    /// * `center` - Center of the ellipse
    /// * `a` - Semi-major axis
    /// * `b` - Semi-minor axis
    /// * `theta` - Orientation of the major axis with respect to the x axis
    pub(crate) fn from_center(center: Point, a: f64, b: f64, theta: f64) -> Self {
        Self::full(center.x, center.y, a, b, theta)
    }

    /// Build a full ellipse from its canonical geometrical elements
    ///
    /// * `cx` - Abscissa of the center of the ellipse
    /// * `cy` - Ordinate of the center of the ellipse
    /// * `a` - Semi-major axis
    /// * `b` - Semi-minor axis
    /// * `theta` - Orientation of the major axis with respect to the x axis
    pub(crate) fn full(cx: f64, cy: f64, a: f64, b: f64, theta: f64) -> Self {
        let mut arc = Self::with_fields(cx, cy, a, b, theta, 0.0, TWO_PI, theta.cos(), theta.sin(), false);
        arc.compute_all();
        arc
    }

    pub(crate) fn draw_in_opposite_direction(&self) -> bool {
        self.draw_in_opposite_direction
    }

    pub(crate) fn set_draw_in_opposite_direction(&mut self, value: bool) {
        self.draw_in_opposite_direction = value;
    }

    /// Sets the maximal degree allowed for Bezier curve approximation.
    ///
    /// # Panics
    ///
    /// Panics if `max_degree` is not between 1 and 3.
    pub(crate) fn set_max_degree(&mut self, max_degree: i32) {
        if !(1..=3).contains(&max_degree) {
            panic!("maxDegree must be between 1 and 3");
        }
        self.max_degree = max_degree;
    }

    /// Sets the default flatness for Bezier curve approximation
    ///
    /// # Panics
    ///
    /// Panics if `default_flatness` is lower than 1e-10.
    pub(crate) fn set_default_flatness(&mut self, default_flatness: f64) {
        if default_flatness < 1.0E-10 {
            panic!("defaultFlatness must be greater than 1.0e-10");
        }
        self.default_flatness = default_flatness;
    }

    /// Computes the locations of the focii
    fn compute_focii(&mut self) {
        let d = (self.a * self.a - self.b * self.b).sqrt();
        let dx = d * self.cos_theta;
        let dy = d * self.sin_theta;
        self.first_focus_x = self.cx - dx;
        self.first_focus_y = self.cy - dy;
        self.second_focus_x = self.cx + dx;
        self.second_focus_y = self.cy + dy;
    }

    /// Computes the locations of the endpoints
    fn compute_end_points(&mut self) {
        let a_cos_eta1 = self.a * self.eta1.cos();
        let b_sin_eta1 = self.b * self.eta1.sin();
        self.x1 = self.cx + a_cos_eta1 * self.cos_theta - b_sin_eta1 * self.sin_theta;
        self.y1 = self.cy + a_cos_eta1 * self.sin_theta + b_sin_eta1 * self.cos_theta;
        let a_cos_eta2 = self.a * self.eta2.cos();
        let b_sin_eta2 = self.b * self.eta2.sin();
        self.x2 = self.cx + a_cos_eta2 * self.cos_theta - b_sin_eta2 * self.sin_theta;
        self.y2 = self.cy + a_cos_eta2 * self.sin_theta + b_sin_eta2 * self.cos_theta;
    }

    /// Computes the bounding box
    fn compute_bounds(&mut self) {
        let (a, b, cx, cy) = (self.a, self.b, self.cx, self.cy);
        let (cos_theta, sin_theta) = (self.cos_theta, self.sin_theta);
        let b_on_a = b / a;
        let mut eta_x_min;
        let mut eta_x_max;
        let mut eta_y_min;
        let mut eta_y_max;
        if sin_theta.abs() < 0.1 {
            let tan_theta = sin_theta / cos_theta;
            if cos_theta < 0.0 {
                eta_x_min = -(tan_theta * b_on_a).atan();
                eta_x_max = eta_x_min + PI;
                eta_y_min = 0.5 * PI - (tan_theta / b_on_a).atan();
                eta_y_max = eta_y_min + PI;
            } else {
                eta_x_max = -(tan_theta * b_on_a).atan();
                eta_x_min = eta_x_max - PI;
                eta_y_max = 0.5 * PI - (tan_theta / b_on_a).atan();
                eta_y_min = eta_y_max - PI;
            }
        } else {
            let inv_tan_theta = cos_theta / sin_theta;
            if sin_theta < 0.0 {
                eta_x_max = 0.5 * PI + (inv_tan_theta / b_on_a).atan();
                eta_x_min = eta_x_max - PI;
                eta_y_min = (inv_tan_theta * b_on_a).atan();
                eta_y_max = eta_y_min + PI;
            } else {
                eta_x_min = 0.5 * PI + (inv_tan_theta / b_on_a).atan();
                eta_x_max = eta_x_min + PI;
                eta_y_max = (inv_tan_theta * b_on_a).atan();
                eta_y_min = eta_y_max - PI;
            }
        }
        eta_x_min -= TWO_PI * ((eta_x_min - self.eta1) / TWO_PI).floor();
        eta_y_min -= TWO_PI * ((eta_y_min - self.eta1) / TWO_PI).floor();
        eta_x_max -= TWO_PI * ((eta_x_max - self.eta1) / TWO_PI).floor();
        eta_y_max -= TWO_PI * ((eta_y_max - self.eta1) / TWO_PI).floor();
        self.x_left = if eta_x_min <= self.eta2 {
            cx + a * eta_x_min.cos() * cos_theta - b * eta_x_min.sin() * sin_theta
        } else {
            math_utilities::min(self.x1, self.x2)
        };
        self.y_up = if eta_y_min <= self.eta2 {
            cy + a * eta_y_min.cos() * sin_theta + b * eta_y_min.sin() * cos_theta
        } else {
            math_utilities::min(self.y1, self.y2)
        };
        self.width = (if eta_x_max <= self.eta2 {
            cx + a * eta_x_max.cos() * cos_theta - b * eta_x_max.sin() * sin_theta
        } else {
            math_utilities::max(self.x1, self.x2)
        }) - self.x_left;
        self.height = (if eta_y_max <= self.eta2 {
            cy + a * eta_y_max.cos() * sin_theta + b * eta_y_max.sin() * cos_theta
        } else {
            math_utilities::max(self.y1, self.y2)
        }) - self.y_up;
    }

    /// Computes the flatness parameters used in intersection tests
    fn compute_derived_flatness_parameters(&mut self) {
        self.f = (self.a - self.b) / self.a;
        self.e2 = self.f * (2.0 - self.f);
        self.g = 1.0 - self.f;
        self.g2 = self.g * self.g;
    }

    /// Computes the value of a rational function.
    /// This method handles rational functions where the numerator is quadratic
    /// and the denominator is linear
    ///
    /// * `x` - Abscissa for which the value should be computed
    /// * `c` - Coefficients array of the rational function
    fn rational_function(x: f64, c: &[f64; 4]) -> f64 {
        (x * (x * c[0] + c[1]) + c[2]) / (x + c[3])
    }

    /// Estimate the approximation error for a sub-arc of the instance
    ///
    /// * `degree` - Degree of the Bezier curve to use (1, 2 or 3)
    /// * `eta_a` - Start angle of the sub-arc
    /// * `eta_b` - End angle of the sub-arc
    ///
    /// Returns the upper bound of the approximation error between the Bezier curve and the
    /// real ellipse.
    ///
    /// # Panics
    ///
    /// Panics if `degree` is not between 1 and the maximal degree.
    pub(crate) fn estimate_error(&self, degree: i32, eta_a: f64, eta_b: f64) -> f64 {
        if degree < 1 || degree > self.max_degree {
            panic!("degree should be between {} and {}", 1, self.max_degree);
        }
        let (a, b, cx, cy) = (self.a, self.b, self.cx, self.cy);
        let (cos_theta, sin_theta) = (self.cos_theta, self.sin_theta);
        let eta = 0.5 * (eta_a + eta_b);
        if degree < 2 {
            // start point
            let a_cos_eta_a = a * eta_a.cos();
            let b_sin_eta_a = b * eta_a.sin();
            let x_a = cx + a_cos_eta_a * cos_theta - b_sin_eta_a * sin_theta;
            let y_a = cy + a_cos_eta_a * sin_theta + b_sin_eta_a * cos_theta;

            // end point
            let a_cos_eta_b = a * eta_b.cos();
            let b_sin_eta_b = b * eta_b.sin();
            let x_b = cx + a_cos_eta_b * cos_theta - b_sin_eta_b * sin_theta;
            let y_b = cy + a_cos_eta_b * sin_theta + b_sin_eta_b * cos_theta;

            // maximal error point
            let a_cos_eta = a * eta.cos();
            let b_sin_eta = b * eta.sin();
            let x = cx + a_cos_eta * cos_theta - b_sin_eta * sin_theta;
            let y = cy + a_cos_eta * sin_theta + b_sin_eta * cos_theta;

            let dx = x_b - x_a;
            let dy = y_b - y_a;

            (x * dy - y * dx + x_b * y_a - x_a * y_b).abs() / (dx * dx + dy * dy).sqrt()
        } else {
            let x = b / a;
            let d_eta = eta_b - eta_a;
            let cos2 = (2.0 * eta).cos();
            let cos4 = (4.0 * eta).cos();
            let cos6 = (6.0 * eta).cos();

            // select the right coefficients set according to degree and b/a
            let (coeffs, safety) = if degree == 2 {
                (if x < 0.25 { &COEFFS_2_LOW } else { &COEFFS_2_HIGH }, &SAFETY_2)
            } else {
                (if x < 0.25 { &COEFFS_3_LOW } else { &COEFFS_3_HIGH }, &SAFETY_3)
            };
            let c0 = Self::rational_function(x, &coeffs[0][0])
                + cos2 * Self::rational_function(x, &coeffs[0][1])
                + cos4 * Self::rational_function(x, &coeffs[0][2])
                + cos6 * Self::rational_function(x, &coeffs[0][3]);
            let c1 = Self::rational_function(x, &coeffs[1][0])
                + cos2 * Self::rational_function(x, &coeffs[1][1])
                + cos4 * Self::rational_function(x, &coeffs[1][2])
                + cos6 * Self::rational_function(x, &coeffs[1][3]);
            Self::rational_function(x, safety) * a * (c0 + c1 * d_eta).exp()
        }
    }

    /// Get the elliptical arc point for a given angular parameter
    ///
    /// * `lambda` - Angular parameter for which point is desired
    pub(crate) fn point_at(&self, lambda: f64) -> Point {
        let eta = (lambda.sin() / self.b).atan2(lambda.cos() / self.a);
        let a_cos_eta = self.a * eta.cos();
        let b_sin_eta = self.b * eta.sin();
        Point::new(
            self.cx + a_cos_eta * self.cos_theta - b_sin_eta * self.sin_theta,
            self.cy + a_cos_eta * self.sin_theta + b_sin_eta * self.cos_theta,
        )
    }

    /// Tests if the specified coordinates are inside the closed shape formed by this arc.
    /// If this is not a pie, then a shape derived by adding a closing chord is considered.
    ///
    /// * `x` - Abscissa of the test point
    /// * `y` - Ordinate of the test point
    pub(crate) fn contains(&self, x: f64, y: f64) -> bool {
        // position relative to the focii
        let dx1 = x - self.first_focus_x;
        let dy1 = y - self.first_focus_y;
        let dx2 = x - self.second_focus_x;
        let dy2 = y - self.second_focus_y;
        if dx1 * dx1 + dy1 * dy1 + dx2 * dx2 + dy2 * dy2 > 4.0 * self.a * self.a {
            // the point is outside of the ellipse
            return false;
        }
        if self.is_pie_slice {
            // check the location of the test point with respect to the
            // angular sector counted from the center of the ellipse
            let dx_c = x - self.cx;
            let dy_c = y - self.cy;
            let u = dx_c * self.cos_theta + dy_c * self.sin_theta;
            let v = dy_c * self.cos_theta - dx_c * self.sin_theta;
            let mut eta = (v / self.b).atan2(u / self.a);
            eta -= TWO_PI * ((eta - self.eta1) / TWO_PI).floor();
            return eta <= self.eta2;
        }
        // check the location of the test point with respect to the
        // chord joining the start and end points
        let dx = self.x2 - self.x1;
        let dy = self.y2 - self.y1;
        x * dy - y * dx + self.x2 * self.y1 - self.x1 * self.y2 >= 0.0
    }

    /// Tests if a line segment intersects the arc
    ///
    /// * `x_a` - abscissa of the first point of the line segment
    /// * `y_a` - ordinate of the first point of the line segment
    /// * `x_b` - abscissa of the second point of the line segment
    /// * `y_b` - ordinate of the second point of the line segment
    fn intersect_arc(&self, x_a: f64, y_a: f64, x_b: f64, y_b: f64) -> bool {
        let mut dx = x_a - x_b;
        let mut dy = y_a - y_b;
        let l = (dx * dx + dy * dy).sqrt();
        if l < 1.0E-10 * self.a {
            // too small line segment, we consider it doesn't intersect anything
            return false;
        }
        let cz = (dx * self.cos_theta + dy * self.sin_theta) / l;
        let sz = (dy * self.cos_theta - dx * self.sin_theta) / l;

        // express position of the first point in canonical frame
        dx = x_a - self.cx;
        dy = y_a - self.cy;
        let u = dx * self.cos_theta + dy * self.sin_theta;
        let v = dy * self.cos_theta - dx * self.sin_theta;
        let u2 = u * u;
        let v2 = v * v;
        let g2_u2_ma2 = self.g2 * (u2 - self.a * self.a);
        let g2_u2_ma2_pv2 = g2_u2_ma2 + v2;

        // compute intersections with the ellipse along the line
        // as the roots of a 2nd degree polynom : c0 k^2 - 2 c1 k + c2 = 0
        let c0 = 1.0 - self.e2 * cz * cz;
        let c1 = self.g2 * u * cz + v * sz;
        let c2 = g2_u2_ma2_pv2;
        let c12 = c1 * c1;
        let c0_c2 = c0 * c2;
        if c12 < c0_c2 {
            // the line does not intersect the ellipse at all
            return false;
        }
        let mut k = if c1 >= 0.0 { (c1 + (c12 - c0_c2).sqrt()) / c0 } else { c2 / (c1 - (c12 - c0_c2).sqrt()) };
        if k >= 0.0 && k <= l {
            let u_intersect = u - k * cz;
            let v_intersect = v - k * sz;
            let mut eta = (v_intersect / self.b).atan2(u_intersect / self.a);
            eta -= TWO_PI * ((eta - self.eta1) / TWO_PI).floor();
            if eta <= self.eta2 {
                return true;
            }
        }
        k = c2 / (k * c0);
        if k >= 0.0 && k <= l {
            let u_intersect = u - k * cz;
            let v_intersect = v - k * sz;
            let mut eta = (v_intersect / self.b).atan2(u_intersect / self.a);
            eta -= TWO_PI * ((eta - self.eta1) / TWO_PI).floor();
            if eta <= self.eta2 {
                return true;
            }
        }
        false
    }

    /// Tests if two line segments intersect
    ///
    /// * `x1`, `y1` - first point of the first line segment
    /// * `x2`, `y2` - second point of the first line segment
    /// * `x_a`, `y_a` - first point of the second line segment
    /// * `x_b`, `y_b` - second point of the second line segment
    #[allow(clippy::too_many_arguments)]
    fn intersect(x1: f64, y1: f64, x2: f64, y2: f64, x_a: f64, y_a: f64, x_b: f64, y_b: f64) -> bool {
        // elements of the equation of the (1, 2) line segment
        let dx12 = x2 - x1;
        let dy12 = y2 - y1;
        let k12 = x2 * y1 - x1 * y2;
        // elements of the equation of the (A, B) line segment
        let dx_ab = x_b - x_a;
        let dy_ab = y_b - y_a;
        let k_ab = x_b * y_a - x_a * y_b;
        // compute relative positions of endpoints versus line segments
        let p_a_vs_12 = x_a * dy12 - y_a * dx12 + k12;
        let p_b_vs_12 = x_b * dy12 - y_b * dx12 + k12;
        let p1_vs_ab = x1 * dy_ab - y1 * dx_ab + k_ab;
        let p2_vs_ab = x2 * dy_ab - y2 * dx_ab + k_ab;

        p_a_vs_12 * p_b_vs_12 <= 0.0 && p1_vs_ab * p2_vs_ab <= 0.0
    }

    /// Tests if a line segment intersects the outline
    ///
    /// * `x_a`, `y_a` - first point of the line segment
    /// * `x_b`, `y_b` - second point of the line segment
    fn intersect_outline(&self, x_a: f64, y_a: f64, x_b: f64, y_b: f64) -> bool {
        if self.intersect_arc(x_a, y_a, x_b, y_b) {
            return true;
        }
        if self.is_pie_slice {
            return Self::intersect(self.cx, self.cy, self.x1, self.y1, x_a, y_a, x_b, y_b)
                || Self::intersect(self.cx, self.cy, self.x2, self.y2, x_a, y_a, x_b, y_b);
        }
        Self::intersect(self.x1, self.y1, self.x2, self.y2, x_a, y_a, x_b, y_b)
    }

    /// Tests if the interior of a closed path derived from this arc entirely contains the
    /// specified rectangular area. The closed path is derived with respect to the
    /// `is_pie_slice` value.
    ///
    /// * `x` - Abscissa of the upper-left corner of the test rectangle
    /// * `y` - Ordinate of the upper-left corner of the test rectangle
    /// * `w` - Width of the test rectangle
    /// * `h` - Height of the test rectangle
    pub(crate) fn contains_area(&self, x: f64, y: f64, w: f64, h: f64) -> bool {
        let x_plus_w = x + w;
        let y_plus_h = y + h;
        self.contains(x, y)
            && self.contains(x_plus_w, y)
            && self.contains(x, y_plus_h)
            && self.contains(x_plus_w, y_plus_h)
            && !self.intersect_outline(x, y, x_plus_w, y)
            && !self.intersect_outline(x_plus_w, y, x_plus_w, y_plus_h)
            && !self.intersect_outline(x_plus_w, y_plus_h, x, y_plus_h)
            && !self.intersect_outline(x, y_plus_h, x, y)
    }

    /// Tests if a specified point is inside the boundary of a closed path derived from this
    /// arc. The closed path is derived with respect to the `is_pie_slice` value.
    pub(crate) fn contains_point(&self, p: Point) -> bool {
        self.contains(p.x, p.y)
    }

    /// Tests if the interior of a closed path derived from this arc entirely contains the
    /// specified rectangle. The closed path is derived with respect to the `is_pie_slice`
    /// value.
    pub(crate) fn contains_rect(&self, r: Rect) -> bool {
        self.contains_area(r.x, r.y, r.width, r.height)
    }

    /// Returns a rectangle that completely encloses the closed path derived from this arc.
    /// The closed path is derived with respect to the `is_pie_slice` value.
    pub(crate) fn get_bounds(&self) -> Rect {
        Rect::new(self.x_left, self.y_up, self.width, self.height)
    }

    /// Builds the arc outline using given context and default (max) Bezier curve degree and
    /// acceptable error of half a pixel (0.5)
    ///
    /// * `path` - A context to output the path commands to
    pub(crate) fn build_arc(&self, path: &mut StreamGeometryContext) {
        self.build_arc_with(path, self.max_degree, self.default_flatness, true);
    }

    /// Builds the arc outline using given context
    ///
    /// * `path` - A context to output the path commands to
    /// * `degree` - degree of the Bezier curve to use
    /// * `threshold` - acceptable error
    /// * `open_new_figure` - if true, a new figure will be started in the specified context
    ///
    /// # Panics
    ///
    /// Panics if `degree` is not between 1 and the maximal degree.
    pub(crate) fn build_arc_with(
        &self,
        path: &mut StreamGeometryContext,
        degree: i32,
        threshold: f64,
        open_new_figure: bool,
    ) {
        // Upstream only consults this flag in code that is commented out (pie drawing).
        let _ = open_new_figure;

        if degree < 1 || degree > self.max_degree {
            panic!("degree should be between {} and {}", 1, self.max_degree);
        }

        let (a, b, cx, cy) = (self.a, self.b, self.cx, self.cy);
        let (cos_theta, sin_theta) = (self.cos_theta, self.sin_theta);

        // find the number of Bezier curves needed
        let mut found = false;
        let mut n: i32 = 1;
        let mut d_eta;
        let mut eta_b;
        while !found && n < 1024 {
            d_eta = (self.eta2 - self.eta1) / n as f64;
            if d_eta <= 0.5 * PI {
                eta_b = self.eta1;
                found = true;
                let mut i = 0;
                while found && i < n {
                    let eta_a = eta_b;
                    eta_b += d_eta;
                    found = self.estimate_error(degree, eta_a, eta_b) <= threshold;
                    i += 1;
                }
            }
            n <<= 1;
        }
        if !self.draw_in_opposite_direction {
            d_eta = (self.eta2 - self.eta1) / n as f64;
            eta_b = self.eta1;
        } else {
            d_eta = (self.eta1 - self.eta2) / n as f64;
            eta_b = self.eta2;
        }

        let mut cos_eta_b = eta_b.cos();
        let mut sin_eta_b = eta_b.sin();
        let mut a_cos_eta_b = a * cos_eta_b;
        let mut b_sin_eta_b = b * sin_eta_b;
        let mut a_sin_eta_b = a * sin_eta_b;
        let mut b_cos_eta_b = b * cos_eta_b;
        let mut x_b = cx + a_cos_eta_b * cos_theta - b_sin_eta_b * sin_theta;
        let mut y_b = cy + a_cos_eta_b * sin_theta + b_sin_eta_b * cos_theta;
        let mut x_b_dot = -a_sin_eta_b * cos_theta - b_cos_eta_b * sin_theta;
        let mut y_b_dot = -a_sin_eta_b * sin_theta + b_cos_eta_b * cos_theta;

        // we're supposed to be already at the (x_b, y_b)

        let t = (0.5 * d_eta).tan();
        let alpha = d_eta.sin() * ((4.0 + 3.0 * t * t).sqrt() - 1.0) / 3.0;
        for _ in 0..n {
            let x_a = x_b;
            let y_a = y_b;
            let x_a_dot = x_b_dot;
            let y_a_dot = y_b_dot;
            eta_b += d_eta;
            cos_eta_b = eta_b.cos();
            sin_eta_b = eta_b.sin();
            a_cos_eta_b = a * cos_eta_b;
            b_sin_eta_b = b * sin_eta_b;
            a_sin_eta_b = a * sin_eta_b;
            b_cos_eta_b = b * cos_eta_b;
            x_b = cx + a_cos_eta_b * cos_theta - b_sin_eta_b * sin_theta;
            y_b = cy + a_cos_eta_b * sin_theta + b_sin_eta_b * cos_theta;
            x_b_dot = -a_sin_eta_b * cos_theta - b_cos_eta_b * sin_theta;
            y_b_dot = -a_sin_eta_b * sin_theta + b_cos_eta_b * cos_theta;
            if degree == 1 {
                path.line_to(Point::new(x_b, y_b), true);
            } else if degree == 2 {
                let k = (y_b_dot * (x_b - x_a) - x_b_dot * (y_b - y_a)) / (x_a_dot * y_b_dot - y_a_dot * x_b_dot);
                path.quadratic_bezier_to(
                    Point::new(x_a + k * x_a_dot, y_a + k * y_a_dot),
                    Point::new(x_b, y_b),
                    true,
                );
            } else {
                path.cubic_bezier_to(
                    Point::new(x_a + alpha * x_a_dot, y_a + alpha * y_a_dot),
                    Point::new(x_b - alpha * x_b_dot, y_b - alpha * y_b_dot),
                    Point::new(x_b, y_b),
                    true,
                );
            }
        }
        if self.is_pie_slice {
            path.line_to(Point::new(cx, cy), true);
        }
    }

    /// Calculates the angle between two vectors
    ///
    /// Returns the signed angle between `v2` and `v1`.
    fn get_angle(v1: Vector, v2: Vector) -> f64 {
        let scalar = v1 * v2;
        (v1.x * v2.y - v2.x * v1.y).atan2(scalar)
    }

    /// ArcTo helper for a [`StreamGeometryContext`]
    ///
    /// * `path` - Target path
    /// * `p1` - Start point
    /// * `p2` - End point
    /// * `size` - Ellipse radii
    /// * `theta` - Ellipse theta (angle measured from the abscissa)
    /// * `is_large_arc` - Large Arc Indicator
    /// * `clockwise` - Clockwise direction flag
    pub(crate) fn build_arc_between(
        path: &mut StreamGeometryContext,
        p1: Point,
        p2: Point,
        size: Size,
        theta: f64,
        is_large_arc: bool,
        clockwise: bool,
    ) {
        let orth = SimpleMatrix::new(theta.cos(), theta.sin(), -theta.sin(), theta.cos());
        let rest = SimpleMatrix::new(theta.cos(), -theta.sin(), theta.sin(), theta.cos());

        let p1_s = orth * Point::new((p1.x - p2.x) / 2.0, (p1.y - p2.y) / 2.0);

        let mut rx = size.width;
        let mut ry = size.height;
        let mut rx2 = rx * rx;
        let mut ry2 = ry * ry;
        let y1_s2 = p1_s.y * p1_s.y;
        let x1_s2 = p1_s.x * p1_s.x;

        let mut numerator = rx2 * ry2 - rx2 * y1_s2 - ry2 * x1_s2;
        let mut denominator = rx2 * y1_s2 + ry2 * x1_s2;

        if denominator.abs() < 1e-8 {
            path.line_to(p2, true);
            return;
        }
        if (numerator / denominator) < 0.0 {
            let lambda = x1_s2 / rx2 + y1_s2 / ry2;
            let lambda_sqrt = lambda.sqrt();
            if lambda > 1.0 {
                rx *= lambda_sqrt;
                ry *= lambda_sqrt;
                rx2 = rx * rx;
                ry2 = ry * ry;
                numerator = rx2 * ry2 - rx2 * y1_s2 - ry2 * x1_s2;
                if numerator < 0.0 {
                    numerator = 0.0;
                }

                denominator = rx2 * y1_s2 + ry2 * x1_s2;
            }
        }

        let multiplier = (numerator / denominator).abs().sqrt();
        let mul_vec = Point::new(rx * p1_s.y / ry, -ry * p1_s.x / rx);

        let sign: f64 = if clockwise != is_large_arc { 1.0 } else { -1.0 };

        let cs = Point::new(mul_vec.x * multiplier * sign, mul_vec.y * multiplier * sign);

        let translation = Vector::new((p1.x + p2.x) / 2.0, (p1.y + p2.y) / 2.0);

        let c = rest * cs + translation;

        // See "https://www.w3.org/TR/SVG/implnote.html#ArcConversionEndpointToCenter" to understand
        // how the ellipse center is calculated

        // From here we are on our own with our task of finding out lambda1 and lambda2
        // matching our points p1 and p2.

        // We eliminate the offset, making our ellipse zero-centered, then we eliminate the theta,
        // making its Y and X axes the same as global axes. Then we can easily get our angles using
        // good old school formula for angles between vectors.

        // We should remember that this type expects true angles, and not the t-values for ellipse
        // equation. To understand how t-values are obtained, one should see etas calculation in
        // the constructor code.

        let p1_no_offset = orth * (p1 - c);
        let p2_no_offset = orth * (p2 - c);

        // if the arc is drawn clockwise, we swap start and end points
        let revised_p1 = if clockwise { p1_no_offset } else { p2_no_offset };
        let revised_p2 = if clockwise { p2_no_offset } else { p1_no_offset };

        let theta_start = Self::get_angle(Vector::new(1.0, 0.0), revised_p1.into());
        let theta_end = Self::get_angle(Vector::new(1.0, 0.0), revised_p2.into());

        let mut arc = EllipticalArc::from_angles(c.x, c.y, rx, ry, theta, theta_start, theta_end, false);

        let manhattan_distance = |p1: Point, p2: Point| (p1.x - p2.x).abs() + (p1.y - p2.y).abs();
        if manhattan_distance(p2, Point::new(arc.x2, arc.y2)) > manhattan_distance(p2, Point::new(arc.x1, arc.y1)) {
            arc.draw_in_opposite_direction = true;
        }

        arc.build_arc_with(path, arc.max_degree, arc.default_flatness, false);
    }

    /// Tests if the interior of the closed path derived from this arc intersects the interior
    /// of a specified rectangular area. The closed path is derived with respect to the
    /// `is_pie_slice` value.
    pub(crate) fn intersects(&self, x: f64, y: f64, w: f64, h: f64) -> bool {
        let x_plus_w = x + w;
        let y_plus_h = y + h;
        self.contains(x, y)
            || self.contains(x_plus_w, y)
            || self.contains(x, y_plus_h)
            || self.contains(x_plus_w, y_plus_h)
            || self.intersect_outline(x, y, x_plus_w, y)
            || self.intersect_outline(x_plus_w, y, x_plus_w, y_plus_h)
            || self.intersect_outline(x_plus_w, y_plus_h, x, y_plus_h)
            || self.intersect_outline(x, y_plus_h, x, y)
    }

    /// Tests if the interior of the closed path derived from this arc intersects the interior
    /// of a specified rectangular area. The closed path is derived with respect to the
    /// `is_pie_slice` value.
    pub(crate) fn intersects_rect(&self, r: Rect) -> bool {
        self.intersects(r.x, r.y, r.width, r.height)
    }
}

// Upstream has no unit tests for this helper. None of the tests below are upstream tests: the
// expected values were produced by running the upstream C# algorithm with the same inputs.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::FillRule;
    use crate::platform::IStreamGeometryContextImpl;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Clone, Debug, PartialEq)]
    enum Actual {
        Line(Vec<f64>),
        Quad(Vec<f64>),
        Cubic(Vec<f64>),
    }

    #[derive(Clone, Copy, Debug)]
    enum Cmd {
        Line(&'static [f64]),
        Quad(&'static [f64]),
        Cubic(&'static [f64]),
    }

    struct Recorder {
        log: Rc<RefCell<Vec<Actual>>>,
    }

    impl IGeometryContext for Recorder {
        fn arc_to(&mut self, _: Point, _: Size, _: f64, _: bool, _: SweepDirection, _: bool) {
            panic!("the precise arc must not use the platform arc");
        }
        fn begin_figure(&mut self, _: Point, _: bool) {}
        fn cubic_bezier_to(&mut self, p1: Point, p2: Point, p3: Point, is_stroked: bool) {
            assert!(is_stroked);
            self.log.borrow_mut().push(Actual::Cubic(vec![p1.x, p1.y, p2.x, p2.y, p3.x, p3.y]));
        }
        fn quadratic_bezier_to(&mut self, p1: Point, p2: Point, is_stroked: bool) {
            assert!(is_stroked);
            self.log.borrow_mut().push(Actual::Quad(vec![p1.x, p1.y, p2.x, p2.y]));
        }
        fn line_to(&mut self, point: Point, is_stroked: bool) {
            assert!(is_stroked);
            self.log.borrow_mut().push(Actual::Line(vec![point.x, point.y]));
        }
        fn end_figure(&mut self, _: bool) {}
        fn set_fill_rule(&mut self, _: FillRule) {}
        fn dispose(&mut self) {}
    }

    impl IStreamGeometryContextImpl for Recorder {}

    fn record(start: Point, draw: impl FnOnce(&mut StreamGeometryContext)) -> Vec<Actual> {
        let log = Rc::new(RefCell::new(Vec::new()));
        let mut context = StreamGeometryContext::new(Box::new(Recorder { log: log.clone() }));
        context.begin_figure(start, true);
        draw(&mut context);
        drop(context);
        let result = log.borrow().clone();
        result
    }

    fn assert_close(expected: f64, actual: f64) {
        let tolerance = 1e-9 * (1.0 + expected.abs());
        assert!((expected - actual).abs() <= tolerance, "expected {expected}, actual {actual}");
    }

    fn assert_close_all(expected: &[f64], actual: &[f64]) {
        assert_eq!(expected.len(), actual.len());
        for (expected, actual) in expected.iter().zip(actual) {
            assert_close(*expected, *actual);
        }
    }

    fn assert_commands(expected: &[Cmd], actual: &[Actual]) {
        assert_eq!(expected.len(), actual.len(), "{actual:?}");
        for (expected, actual) in expected.iter().zip(actual) {
            match (expected, actual) {
                (Cmd::Line(e), Actual::Line(a)) | (Cmd::Quad(e), Actual::Quad(a)) | (Cmd::Cubic(e), Actual::Cubic(a)) => {
                    assert_close_all(e, a)
                }
                _ => panic!("expected {expected:?}, actual {actual:?}"),
            }
        }
    }

    fn full_ellipse() -> EllipticalArc {
        EllipticalArc::full(10.0, 20.0, 5.0, 3.0, 0.5)
    }

    fn pie_slice() -> EllipticalArc {
        EllipticalArc::from_center_and_angles(Point::new(10.0, 20.0), 8.0, 1.0, -2.0, 0.25, 2.5, true)
    }

    fn chord() -> EllipticalArc {
        EllipticalArc::from_angles(0.0, 0.0, 10.0, 5.0, 0.0, 0.0, PI / 2.0, false)
    }

    #[test]
    fn arc_to_quarter_circle_clockwise() {
        let actual = record(Point::new(10.0, 0.0), |context| {
            context.precise_arc_to(Point::new(0.0, 10.0), Size::new(10.0, 10.0), 0.0, false, SweepDirection::Clockwise);
        });
        assert_commands(
            &[
                Cmd::Cubic(&[10.0, 1.3131879680738558, 9.741330604075909, 2.613606837607377, 9.238795325112868, 3.826834323650898]),
                Cmd::Cubic(&[8.736260046149827, 5.040061809694419, 7.999631929063082, 6.142503694667868, 7.0710678118654755, 7.071067811865475]),
                Cmd::Cubic(&[6.142503694667869, 7.9996319290630815, 5.040061809694419, 8.736260046149827, 3.8268343236508984, 9.238795325112868]),
                Cmd::Cubic(&[2.613606837607378, 9.741330604075909, 1.3131879680738565, 10.0, 6.123233995736766e-16, 10.0]),
            ],
            &actual,
        );
    }

    #[test]
    fn arc_to_quarter_circle_counter_clockwise() {
        let actual = record(Point::new(10.0, 0.0), |context| {
            context.precise_arc_to(Point::new(0.0, 10.0), Size::new(10.0, 10.0), 0.0, false, SweepDirection::CounterClockwise);
        });
        assert_commands(
            &[
                Cmd::Cubic(&[8.686812031926141, 2.4122871626706963e-16, 7.386393162392617, 0.2586693959240939, 6.173165676349097, 0.7612046748871357]),
                Cmd::Cubic(&[4.959938190305577, 1.2637399538501777, 3.8574963053321274, 2.0003680709369216, 2.928932188134521, 2.928932188134529]),
                Cmd::Cubic(&[2.0003680709369145, 3.8574963053321363, 1.2637399538501708, 4.959938190305587, 0.7612046748871304, 6.173165676349107]),
                Cmd::Cubic(&[0.25866939592409, 7.386393162392628, -7.4399175130457435e-16, 8.686812031926149, 0.0, 10.000000000000005]),
            ],
            &actual,
        );
    }

    #[test]
    fn arc_to_large_arc_clockwise() {
        let actual = record(Point::new(10.0, 0.0), |context| {
            context.precise_arc_to(Point::new(0.0, 10.0), Size::new(10.0, 10.0), 0.0, true, SweepDirection::Clockwise);
        });
        assert_commands(
            &[
                Cmd::Cubic(&[11.977576428669655, 1.2109163217197738e-16, 13.911407623751414, 0.5866212796844741, 15.555702330196024, 1.6853038769745474]),
                Cmd::Cubic(&[17.199997036640635, 2.783986474264621, 18.482009589625267, 4.34612328992444, 19.238795325112868, 6.173165676349102]),
                Cmd::Cubic(&[19.99558106060047, 8.000208062773764, 20.193658826312973, 10.011325368049697, 19.807852804032304, 11.950903220161283]),
                Cmd::Cubic(&[19.422046781751636, 13.89048107227287, 18.469425514892464, 15.672710108838487, 17.071067811865476, 17.071067811865476]),
                Cmd::Cubic(&[15.672710108838487, 18.469425514892464, 13.89048107227287, 19.422046781751636, 11.950903220161283, 19.807852804032304]),
                Cmd::Cubic(&[10.011325368049697, 20.193658826312973, 8.000208062773764, 19.99558106060047, 6.173165676349103, 19.238795325112868]),
                Cmd::Cubic(&[4.346123289924442, 18.482009589625267, 2.7839864742646205, 17.19999703664063, 1.6853038769745474, 15.555702330196022]),
                Cmd::Cubic(&[0.5866212796844743, 13.911407623751412, 2.4218326434395475e-16, 11.977576428669657, 0.0, 10.000000000000002]),
            ],
            &actual,
        );
    }

    #[test]
    fn arc_to_rotated_ellipse_large_arc_counter_clockwise() {
        let actual = record(Point::new(80.0, 200.0), |context| {
            context.precise_arc_to(Point::new(100.0, 50.0), Size::new(100.0, 50.0), 45.0, true, SweepDirection::CounterClockwise);
        });
        assert_commands(
            &[
                Cmd::Cubic(&[93.29602817674778, 209.02816728050777, 106.72143862669877, 216.02852653912004, 119.50790220185246, 220.60045091344642]),
                Cmd::Cubic(&[132.29436577700613, 225.1723752877728, 144.19321797944906, 227.2269523688147, 154.52349378327244, 226.6465997955662]),
                Cmd::Cubic(&[164.85376958709583, 226.0662472223177, 173.41457115757257, 222.8622513899882, 179.71596834311688, 217.21797528753268]),
                Cmd::Cubic(&[186.0173655286612, 211.57369918507717, 189.93681203192614, 203.5989097605539, 191.25, 193.75]),
                Cmd::Cubic(&[192.56318796807386, 183.9010902394461, 191.24457920387596, 172.36959662096655, 187.36963699041868, 159.81546043276924]),
                Cmd::Cubic(&[183.4946947769614, 147.26132424457194, 177.1387769764316, 133.9286918022997, 168.66562940700342, 120.5805826175841]),
                Cmd::Cubic(&[160.19248183757526, 107.2324734328685, 149.76688586930584, 94.12847459552546, 137.98549285207824, 82.01852103675344]),
                Cmd::Cubic(&[126.20409983485064, 69.90856747798142, 113.29602817674783, 59.02816728050778, 100.00000000000003, 50.00000000000002]),
            ],
            &actual,
        );
    }

    #[test]
    fn arc_to_scales_up_radii_that_are_too_small() {
        let actual = record(Point::new(0.0, 0.0), |context| {
            context.precise_arc_to(Point::new(100.0, 0.0), Size::new(10.0, 5.0), 0.0, false, SweepDirection::Clockwise);
        });
        assert_commands(
            &[
                Cmd::Cubic(&[-8.040957208902322e-16, -3.2829699201846365, 1.293346979620456, -6.5340170940184406, 3.806023374435661, -9.567085809127242]),
                Cmd::Cubic(&[6.318699769250866, -12.600154524236043, 10.001840354684582, -15.356259236669667, 14.644660940672615, -17.677669529663685]),
                Cmd::Cubic(&[19.28748152666065, -19.999079822657702, 24.799690951527882, -21.840650115374558, 30.865828381745484, -23.096988312782162]),
                Cmd::Cubic(&[36.93196581196308, -24.353326510189767, 43.43406015963071, -25.0, 49.99999999999999, -25.0]),
                Cmd::Cubic(&[56.565939840369275, -25.0, 63.0680341880369, -24.35332651018977, 69.1341716182545, -23.096988312782166]),
                Cmd::Cubic(&[75.2003090484721, -21.84065011537456, 80.71251847333937, -19.999079822657695, 85.3553390593274, -17.677669529663675]),
                Cmd::Cubic(&[89.99815964531544, -15.356259236669656, 93.68130023074916, -12.60015452423602, 96.19397662556436, -9.567085809127217]),
                Cmd::Cubic(&[98.70665302037956, -6.534017094018415, 100.00000000000001, -3.2829699201846014, 100.0, 3.828568698926949e-14]),
            ],
            &actual,
        );
    }

    #[test]
    fn arc_to_between_identical_points_is_a_line() {
        let actual = record(Point::new(5.0, 5.0), |context| {
            context.precise_arc_to(Point::new(5.0, 5.0), Size::new(10.0, 10.0), 0.0, false, SweepDirection::Clockwise);
        });
        assert_commands(
            &[
                Cmd::Line(&[5.0, 5.0]),
            ],
            &actual,
        );
    }

    #[test]
    fn arc_to_very_flat_ellipse() {
        let actual = record(Point::new(0.0, 0.0), |context| {
            context.precise_arc_to(Point::new(200.0, 1.0), Size::new(1000.0, 1.0), 0.0, false, SweepDirection::Clockwise);
        });
        assert_commands(
            &[
                Cmd::Cubic(&[59.72291294658482, 0.0664053505668793, 110.28144517538155, 0.14051331078930307, 150.32512875401846, 0.2203443844397902]),
                Cmd::Cubic(&[190.36881233265538, 0.30017545809027735, 219.5379089325161, 0.3850124709620576, 237.0532834039467, 0.47258934331921054]),
                Cmd::Cubic(&[254.56865787537728, 0.5601662156763635, 260.27295827234445, 0.6496961877887224, 254.01381714225158, 0.7387878263809555]),
                Cmd::Cubic(&[247.75467601215874, 0.8278794649731887, 229.58832327112475, 0.9157324021895785, 200.0000000000001, 0.9999999999999999]),
            ],
            &actual,
        );
    }

    #[test]
    fn unit_circle_arc() {
        let arc = EllipticalArc::new();
        assert_close_all(&[0.0, 6.283185307179586, 1.0, 0.0, 1.0, -2.4492935982947064e-16], &[arc.eta1, arc.eta2, arc.x1, arc.y1, arc.x2, arc.y2]);
        assert_close_all(
            &[0.0, 0.0, 0.0, 0.0],
            &[arc.first_focus_x, arc.first_focus_y, arc.second_focus_x, arc.second_focus_y],
        );
        let bounds = arc.get_bounds();
        assert_close_all(
            &[-1.0, -1.0, 2.0, 2.0],
            &[bounds.x, bounds.y, bounds.width, bounds.height],
        );
        assert_close_all(&[0.0, 0.0, 1.0, 1.0], &[arc.f, arc.e2, arc.g, arc.g2]);
        assert_close_all(
            &[0.031087578289355124, 0.0005336189805860216, 2.1428327180340756e-06],
            &[
                arc.estimate_error(1, arc.eta1, arc.eta1 + 0.5),
                arc.estimate_error(2, arc.eta1, arc.eta1 + 0.5),
                arc.estimate_error(3, arc.eta1, arc.eta1 + 0.5),
            ],
        );
        let point = arc.point_at(1.0);
        assert_close_all(&[0.5403023058681398, 0.8414709848078965], &[point.x, point.y]);
    }

    #[test]
    fn full_ellipse_arc() {
        let arc = full_ellipse();
        assert_close_all(&[0.0, 6.283185307179586, 14.387912809451864, 22.397127693021016, 14.387912809451864, 22.397127693021016], &[arc.eta1, arc.eta2, arc.x1, arc.y1, arc.x2, arc.y2]);
        assert_close_all(
            &[6.489669752438509, 18.082297845583188, 13.510330247561491, 21.917702154416812],
            &[arc.first_focus_x, arc.first_focus_y, arc.second_focus_x, arc.second_focus_y],
        );
        let bounds = arc.get_bounds();
        assert_close_all(
            &[5.382379568766494, 16.439440837023646, 9.235240862467014, 7.121118325952711],
            &[bounds.x, bounds.y, bounds.width, bounds.height],
        );
        assert_close_all(&[0.4, 0.6400000000000001, 0.6, 0.36], &[arc.f, arc.e2, arc.g, arc.g2]);
        assert_close_all(
            &[0.14761389915578074, 0.002582276306270396, 1.0397979896769884e-05],
            &[
                arc.estimate_error(1, arc.eta1, arc.eta1 + 0.5),
                arc.estimate_error(2, arc.eta1, arc.eta1 + 0.5),
                arc.estimate_error(3, arc.eta1, arc.eta1 + 0.5),
            ],
        );
        let point = arc.point_at(1.0);
        assert_close_all(&[10.235331065119212, 23.31850217586119], &[point.x, point.y]);
    }

    #[test]
    fn pie_slice_arc() {
        let arc = pie_slice();
        assert_close_all(&[1.1155519445638467, 1.7365913284724346, 9.352911174641779, 16.427823840412337, 11.446263935349636, 20.790097588852785], &[arc.eta1, arc.eta2, arc.x1, arc.y1, arc.x2, arc.y2]);
        assert_close_all(
            &[13.303063115169952, 27.217324577515118, 6.696936884830048, 12.782675422484882],
            &[arc.first_focus_x, arc.first_focus_y, arc.second_focus_x, arc.second_focus_y],
        );
        let bounds = arc.get_bounds();
        assert_close_all(
            &[9.352911174641779, 16.427823840412337, 2.093352760707857, 4.3622737484404475],
            &[bounds.x, bounds.y, bounds.width, bounds.height],
        );
        assert_close_all(&[0.875, 0.984375, 0.125, 0.015625], &[arc.f, arc.e2, arc.g, arc.g2]);
        assert_close_all(
            &[0.03174331206651061, 0.0006351350838111361, 2.574939876538463e-06],
            &[
                arc.estimate_error(1, arc.eta1, arc.eta1 + 0.5),
                arc.estimate_error(2, arc.eta1, arc.eta1 + 0.5),
                arc.estimate_error(3, arc.eta1, arc.eta1 + 0.5),
            ],
        );
        let point = arc.point_at(1.0);
        assert_close_all(&[10.640034399891451, 19.003205481564187], &[point.x, point.y]);
    }

    #[test]
    fn chord_arc() {
        let arc = chord();
        assert_close_all(&[0.0, 1.5707963267948966, 10.0, 0.0, 6.123233995736766e-16, 5.0], &[arc.eta1, arc.eta2, arc.x1, arc.y1, arc.x2, arc.y2]);
        assert_close_all(
            &[-8.660254037844387, 0.0, 8.660254037844387, 0.0],
            &[arc.first_focus_x, arc.first_focus_y, arc.second_focus_x, arc.second_focus_y],
        );
        let bounds = arc.get_bounds();
        assert_close_all(
            &[6.123233995736766e-16, 0.0, 10.0, 5.0],
            &[bounds.x, bounds.y, bounds.width, bounds.height],
        );
        assert_close_all(&[0.5, 0.75, 0.5, 0.25], &[arc.f, arc.e2, arc.g, arc.g2]);
        assert_close_all(
            &[0.28574564157922205, 0.005067178276239396, 2.041027165756168e-05],
            &[
                arc.estimate_error(1, arc.eta1, arc.eta1 + 0.5),
                arc.estimate_error(2, arc.eta1, arc.eta1 + 0.5),
                arc.estimate_error(3, arc.eta1, arc.eta1 + 0.5),
            ],
        );
        let point = arc.point_at(1.0);
        assert_close_all(&[3.056792781862688, 4.760672691142297], &[point.x, point.y]);
    }

    #[test]
    fn build_arc_with_each_degree() {
        let actual = record(Point::new(10.0, 0.0), |context| chord().build_arc_with(context, 1, 0.5, true));
        assert_commands(
            &[
                Cmd::Line(&[9.807852804032304, 0.9754516100806412]),
                Cmd::Line(&[9.238795325112868, 1.913417161825449]),
                Cmd::Line(&[8.314696123025453, 2.777851165098011]),
                Cmd::Line(&[7.0710678118654755, 3.5355339059327373]),
                Cmd::Line(&[5.555702330196023, 4.157348061512726]),
                Cmd::Line(&[3.8268343236508984, 4.619397662556434]),
                Cmd::Line(&[1.9509032201612833, 4.903926402016152]),
                Cmd::Line(&[6.123233995736766e-16, 5.0]),
            ],
            &actual,
        );

        let actual = record(Point::new(10.0, 0.0), |context| chord().build_arc_with(context, 2, 0.5, true));
        assert_commands(
            &[
                Cmd::Quad(&[10.0, 0.9945618368982905, 9.238795325112868, 1.913417161825449]),
                Cmd::Quad(&[8.477590650225736, 2.832272486752607, 7.0710678118654755, 3.5355339059327373]),
                Cmd::Quad(&[5.6645449735052145, 4.238795325112868, 3.8268343236508984, 4.619397662556434]),
                Cmd::Quad(&[1.989123673796582, 5.0, 6.123233995736766e-16, 5.0]),
            ],
            &actual,
        );

        // A pie slice ends with a line to the center.
        let actual = record(Point::new(10.0, 0.0), |context| pie_slice().build_arc(context));
        assert_commands(
            &[
                Cmd::Cubic(&[9.704492636583561, 17.08646380812493, 10.06438431231443, 17.798749290519094, 10.419657725690477, 18.539092683079986]),
                Cmd::Cubic(&[10.774931139066524, 19.279436075640877, 11.121280540193471, 20.038864757946502, 11.446263935349636, 20.790097588852788]),
                Cmd::Line(&[10.0, 20.0]),
            ],
            &actual,
        );
    }

    #[test]
    fn full_ellipse_from_center_matches_coordinates() {
        let a = EllipticalArc::from_center(Point::new(10.0, 20.0), 5.0, 3.0, 0.5);
        let b = full_ellipse();
        assert_eq!((a.x1, a.y1, a.x2, a.y2, a.get_bounds()), (b.x1, b.y1, b.x2, b.y2, b.get_bounds()));
        assert_eq!(0.5, a.theta);
        assert!(!a.is_pie_slice);
    }

    #[test]
    fn contains_and_intersects() {
        let check = |arc: &EllipticalArc| {
            [
                arc.contains(10.0, 20.0),
                arc.contains_point(Point::new(4.0, 4.0)),
                arc.contains_point(Point::new(11.0, 18.0)),
                arc.contains_area(9.0, 19.0, 2.0, 2.0),
                arc.contains_rect(Rect::new(3.0, 1.0, 2.0, 1.0)),
                arc.intersects(0.0, 0.0, 12.0, 21.0),
                arc.intersects_rect(Rect::new(100.0, 100.0, 5.0, 5.0)),
                arc.intersects_rect(Rect::new(4.0, 2.0, 10.0, 10.0)),
            ]
        };

        assert_eq!([true, false, true, true, false, true, false, false], check(&full_ellipse()));
        assert_eq!([false, false, true, false, false, true, false, false], check(&pie_slice()));
        assert_eq!([false, true, false, false, false, true, false, true], check(&chord()));
    }

    #[test]
    fn opposite_direction_reverses_the_arc() {
        let mut arc = chord();
        assert!(!arc.draw_in_opposite_direction());
        arc.set_draw_in_opposite_direction(true);
        assert!(arc.draw_in_opposite_direction());

        let actual = record(Point::new(0.0, 5.0), |context| arc.build_arc_with(context, 1, 0.5, false));
        let Some(Actual::Line(last)) = actual.last() else { panic!("{actual:?}") };
        assert_close_all(&[10.0, 0.0], last);
        assert_eq!(8, actual.len());
    }

    #[test]
    fn max_degree_limits_the_degree() {
        let mut arc = chord();
        arc.set_max_degree(1);
        let actual = record(Point::new(10.0, 0.0), |context| arc.build_arc(context));
        assert!(actual.iter().all(|command| matches!(command, Actual::Line(_))));

        arc.set_default_flatness(0.01);
        let finer = record(Point::new(10.0, 0.0), |context| arc.build_arc(context));
        assert!(finer.len() > actual.len());
    }

    #[test]
    #[should_panic(expected = "maxDegree must be between 1 and 3")]
    fn set_max_degree_rejects_out_of_range_values() {
        chord().set_max_degree(4);
    }

    #[test]
    #[should_panic(expected = "defaultFlatness must be greater than 1.0e-10")]
    fn set_default_flatness_rejects_tiny_values() {
        chord().set_default_flatness(1.0e-11);
    }

    #[test]
    #[should_panic(expected = "degree should be between 1 and 3")]
    fn estimate_error_rejects_out_of_range_degree() {
        chord().estimate_error(0, 0.0, 1.0);
    }
}
