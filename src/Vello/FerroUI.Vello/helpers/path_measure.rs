use kurbo::{BezPath, ParamCurve, ParamCurveArclen, ParamCurveDeriv, PathEl, PathSeg, Point, Vec2};

/// The accuracy of the lengths of curves.
const ARCLEN_ACCURACY: f64 = 1e-4;

/// Measures a contour of a path: its length, the point and the tangent at a
/// distance along it, and parts of it.
///
/// Skia has this as `SkPathMeasure`, which the Skia backend uses; kurbo has
/// the pieces (the arc length of a segment and its inverse). As the measure
/// of Skia created from a path, this one measures the first contour of the
/// path that has a length, and none of the following.
pub struct PathMeasure {
    segments: Vec<PathSeg>,
    /// The distance along the contour at which each segment ends.
    ends: Vec<f64>,
}

impl PathMeasure {
    /// Creates the measure of the first contour of `path` that has a length.
    pub fn new(path: &BezPath) -> Self {
        let mut segments = Vec::new();
        let mut ends = Vec::new();
        let mut length = 0.0;

        // A close element yields the closing line, a move ends the contour.
        let mut elements = path.elements().iter();
        let mut contour: Vec<PathEl> = Vec::new();

        loop {
            let element = elements.next();
            let contour_ends = matches!(element, None | Some(PathEl::MoveTo(_)));

            if contour_ends && !contour.is_empty() {
                for segment in kurbo::segments(contour.drain(..)) {
                    let segment_length = segment.arclen(ARCLEN_ACCURACY);
                    if segment_length > 0.0 && segment_length.is_finite() {
                        length += segment_length;
                        segments.push(segment);
                        ends.push(length);
                    }
                }
                if !segments.is_empty() {
                    break;
                }
            }

            match element {
                Some(element) => contour.push(*element),
                None => break,
            }
        }

        Self { segments, ends }
    }

    /// The length of the contour.
    pub fn length(&self) -> f64 {
        self.ends.last().copied().unwrap_or(0.0)
    }

    /// The segment that holds `distance` and the parameter of the point at
    /// that distance on it.
    fn locate(&self, distance: f64) -> Option<(usize, f64)> {
        if self.segments.is_empty() {
            return None;
        }

        let distance = distance.clamp(0.0, self.length());
        let index = self.ends.partition_point(|end| *end < distance).min(self.segments.len() - 1);
        let start = if index == 0 { 0.0 } else { self.ends[index - 1] };
        let segment_length = self.ends[index] - start;
        let along = (distance - start).clamp(0.0, segment_length);

        let t = if along <= 0.0 {
            0.0
        } else if along >= segment_length {
            1.0
        } else {
            self.segments[index].inv_arclen(along, ARCLEN_ACCURACY)
        };

        Some((index, t))
    }

    /// The point at `distance` along the contour and the unit tangent
    /// there. The distance is pinned to the contour. `None` for a contour
    /// without a length.
    pub fn pos_tan(&self, distance: f64) -> Option<(Point, Vec2)> {
        if !distance.is_finite() {
            return None;
        }

        let (index, t) = self.locate(distance)?;
        let segment = self.segments[index];
        let position = segment.eval(t);

        let cubic = segment.to_cubic();
        let mut tangent = cubic.deriv().eval(t).to_vec2();
        if tangent.hypot2() == 0.0 {
            // A control point on an end point: the direction of the chord
            // towards the nearest point that differs.
            let other = if t < 0.5 { segment.eval((t + 0.01).min(1.0)) } else { segment.eval((t - 0.01).max(0.0)) };
            tangent = if t < 0.5 { other - position } else { position - other };
        }
        let length = tangent.hypot();
        let tangent = if length > 0.0 { tangent / length } else { Vec2::new(1.0, 0.0) };

        Some((position, tangent))
    }

    /// The part of the contour between two distances, which are pinned to
    /// the contour. `None` when the start is past the stop or the contour
    /// has no length.
    ///
    /// The part begins with a move when `start_with_move_to` is set, else
    /// with a line to its first point (it continues a figure).
    pub fn get_segment(&self, start_distance: f64, stop_distance: f64, start_with_move_to: bool) -> Option<BezPath> {
        let start_distance = start_distance.max(0.0);
        let stop_distance = stop_distance.min(self.length());

        // Written this way to reject NaN as well.
        #[allow(clippy::neg_cmp_op_on_partial_ord)]
        if !(start_distance <= stop_distance) || self.segments.is_empty() {
            return None;
        }

        let (start_index, start_t) = self.locate(start_distance)?;
        let (stop_index, stop_t) = self.locate(stop_distance)?;

        let mut path = BezPath::new();
        let first = self.segments[start_index].eval(start_t);
        if start_with_move_to {
            path.move_to(first);
        } else {
            path.line_to(first);
        }

        for index in start_index..=stop_index {
            let from = if index == start_index { start_t } else { 0.0 };
            let to = if index == stop_index { stop_t } else { 1.0 };
            if to > from {
                path.push(self.segments[index].subsegment(from..to).as_path_el());
            }
        }

        // Nothing but the first point: a part without a length.
        if path.elements().len() == 1 {
            path.line_to(first);
        }

        Some(path)
    }
}
