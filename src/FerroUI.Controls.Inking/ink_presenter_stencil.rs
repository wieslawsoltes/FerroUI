use super::ink_drawing_attributes::validate_affine;
use super::{InkError, InkPoint, InkPresenter};
use ferroui_base::media::Color;
use ferroui_base::{Matrix, Point, Size};
use std::cell::Cell;
use std::rc::Weak;

pub(crate) const MAX_STENCIL_SIZE: f64 = 100_000.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum InkPresenterStencilKind {
    #[default]
    Other = 0,
    Ruler = 1,
    Protractor = 2,
}
ferroui_base::ferro_markup_enum!(InkPresenterStencilKind {
    Other,
    Ruler,
    Protractor
});

/// Shared programmatic contract of the two canvas stencil services.
pub trait IInkPresenterStencil {
    fn kind(&self) -> InkPresenterStencilKind;
    fn is_visible(&self) -> bool;
    fn set_is_visible(&self, value: bool);
    fn transform(&self) -> Matrix;
    fn set_transform(&self, value: Matrix) -> Result<(), InkError>;
    fn background_color(&self) -> Color;
    fn set_background_color(&self, value: Color);
    fn foreground_color(&self) -> Color;
    fn set_foreground_color(&self, value: Color);
}

fn validate_dimension(value: f64) -> Result<(), InkError> {
    if !value.is_finite() || value <= 0.0 || value > MAX_STENCIL_SIZE {
        Err(InkError::InvalidInput(
            "stencil dimensions must be positive and at most 100000 DIPs",
        ))
    } else {
        Ok(())
    }
}
fn validate_transform(value: Matrix, size: Size) -> Result<(), InkError> {
    validate_affine(value)?;
    if value.try_invert().is_none()
        || (value.m11 * value.m22 - value.m12 * value.m21).abs() < 1.0e-12
        || [
            Point::new(0.0, 0.0),
            Point::new(size.width, 0.0),
            Point::new(0.0, size.height),
            Point::new(size.width, size.height),
        ]
        .into_iter()
        .map(|p| p * value)
        .any(|p| !p.x.is_finite() || !p.y.is_finite() || p.x.abs() > 1.0e12 || p.y.abs() > 1.0e12)
    {
        return Err(InkError::InvalidTransform);
    }
    Ok(())
}
fn valid_canvas_size(size: Size) -> bool {
    size.width.is_finite()
        && size.height.is_finite()
        && size.width > 0.0
        && size.height > 0.0
        && size.width <= 1.0e12
        && size.height <= 1.0e12
}
fn notify(presenter: &Weak<InkPresenter>) {
    if let Some(p) = presenter.upgrade() {
        p.invalidate();
    }
}
macro_rules! stencil_contract {
    ($type:ty, $kind:ident) => {
        impl IInkPresenterStencil for $type {
            fn kind(&self) -> InkPresenterStencilKind {
                InkPresenterStencilKind::$kind
            }
            fn is_visible(&self) -> bool {
                self.is_visible()
            }
            fn set_is_visible(&self, value: bool) {
                self.set_is_visible(value);
            }
            fn transform(&self) -> Matrix {
                self.transform()
            }
            fn set_transform(&self, value: Matrix) -> Result<(), InkError> {
                self.set_transform(value)
            }
            fn background_color(&self) -> Color {
                self.background_color()
            }
            fn set_background_color(&self, value: Color) {
                self.set_background_color(value);
            }
            fn foreground_color(&self) -> Color {
                self.foreground_color()
            }
            fn set_foreground_color(&self, value: Color) {
                self.set_foreground_color(value);
            }
        }
        impl PartialEq for $type {
            fn eq(&self, other: &Self) -> bool {
                std::ptr::eq(self, other)
            }
        }
    };
}

/// A movable straightedge. Its transform uses the local top-left corner.
pub struct InkPresenterRuler {
    presenter: Weak<InkPresenter>,
    pub(crate) visible: Cell<bool>,
    transform: Cell<Matrix>,
    transform_explicit: Cell<bool>,
    length: Cell<f64>,
    length_explicit: Cell<bool>,
    width: Cell<f64>,
    width_explicit: Cell<bool>,
    tick_marks: Cell<bool>,
    compass: Cell<bool>,
    background: Cell<Color>,
    foreground: Cell<Color>,
}
stencil_contract!(InkPresenterRuler, Ruler);
impl InkPresenterRuler {
    pub(crate) fn new(presenter: Weak<InkPresenter>) -> Self {
        Self {
            presenter,
            visible: Cell::new(false),
            transform: Cell::new(Matrix::IDENTITY),
            transform_explicit: Cell::new(false),
            length: Cell::new(1600.0),
            length_explicit: Cell::new(false),
            width: Cell::new(120.0),
            width_explicit: Cell::new(false),
            tick_marks: Cell::new(true),
            compass: Cell::new(true),
            background: Cell::new(Color::from_uint32(0xffe6e6e6)),
            foreground: Cell::new(Color::from_uint32(0xff404040)),
        }
    }
    pub fn kind(&self) -> InkPresenterStencilKind {
        InkPresenterStencilKind::Ruler
    }
    pub fn is_visible(&self) -> bool {
        self.visible.get()
    }
    pub fn set_is_visible(&self, value: bool) {
        if let Some(p) = self.presenter.upgrade() {
            p.set_ruler_visible(value);
        } else {
            self.visible.set(value);
        }
    }
    pub fn transform(&self) -> Matrix {
        self.transform.get()
    }
    pub fn set_transform(&self, value: Matrix) -> Result<(), InkError> {
        validate_transform(value, Size::new(self.length(), self.width()))?;
        self.transform_explicit.set(true);
        if self.transform.replace(value) != value {
            notify(&self.presenter);
        }
        Ok(())
    }
    pub fn length(&self) -> f64 {
        self.length.get()
    }
    pub fn set_length(&self, value: f64) -> Result<(), InkError> {
        validate_dimension(value)?;
        if value < 300.0 {
            return Err(InkError::InvalidInput(
                "ruler length must be at least 300 DIPs",
            ));
        }
        validate_transform(self.transform(), Size::new(value, self.width()))?;
        self.length_explicit.set(true);
        if self.length.replace(value) != value {
            notify(&self.presenter);
        }
        Ok(())
    }
    pub fn width(&self) -> f64 {
        self.width.get()
    }
    pub fn set_width(&self, value: f64) -> Result<(), InkError> {
        validate_dimension(value)?;
        if value < 120.0 {
            return Err(InkError::InvalidInput(
                "ruler width must be at least 120 DIPs",
            ));
        }
        validate_transform(self.transform(), Size::new(self.length(), value))?;
        self.width_explicit.set(true);
        if self.width.replace(value) != value {
            notify(&self.presenter);
        }
        Ok(())
    }
    pub fn are_tick_marks_visible(&self) -> bool {
        self.tick_marks.get()
    }
    pub fn set_are_tick_marks_visible(&self, value: bool) {
        if self.tick_marks.replace(value) != value {
            notify(&self.presenter);
        }
    }
    pub fn is_compass_visible(&self) -> bool {
        self.compass.get()
    }
    pub fn set_is_compass_visible(&self, value: bool) {
        if self.compass.replace(value) != value {
            notify(&self.presenter);
        }
    }
    pub fn background_color(&self) -> Color {
        self.background.get()
    }
    pub fn set_background_color(&self, value: Color) {
        if self.background.replace(value) != value {
            notify(&self.presenter);
        }
    }
    pub fn foreground_color(&self) -> Color {
        self.foreground.get()
    }
    pub fn set_foreground_color(&self, value: Color) {
        if self.foreground.replace(value) != value {
            notify(&self.presenter);
        }
    }
    /// Clockwise orientation of the ruler's horizontal edge, in degrees.
    pub fn angle(&self) -> f64 {
        self.transform()
            .m12
            .atan2(self.transform().m11)
            .to_degrees()
            .rem_euclid(360.0)
    }
    pub(crate) fn center(&self) -> Point {
        Point::new(self.length() * 0.5, self.width() * 0.5)
    }
    pub(crate) fn initialize_position(&self, size: Size) {
        if self.transform_explicit.get() || !valid_canvas_size(size) {
            return;
        }
        let (scale_x, scale_y) = if self.length_explicit.get() || self.width_explicit.get() {
            (1.0, 1.0)
        } else {
            (
                (size.width * 0.8 / self.length()).min(1.0),
                (size.height * 0.25 / self.width()).min(1.0),
            )
        };
        let value = Matrix::create_scale(scale_x, scale_y)
            * Matrix::create_translation(
                (size.width - self.length() * scale_x) * 0.5,
                (size.height - self.width() * scale_y) * 0.5,
            );
        if validate_transform(value, Size::new(self.length(), self.width())).is_ok() {
            self.transform.set(value);
        }
    }
    pub(crate) fn hit_test(&self, point: Point) -> bool {
        self.is_visible()
            && self.transform().try_invert().is_some_and(|inverse| {
                let p = point * inverse;
                p.x >= 0.0 && p.x <= self.length() && p.y >= 0.0 && p.y <= self.width()
            })
    }
    pub(crate) fn blocks_ink(&self, point: Point) -> bool {
        self.is_visible()
            && self.transform().try_invert().is_some_and(|inverse| {
                let p = point * inverse;
                p.x > 0.0 && p.x < self.length() && p.y > 1.0e-8 && p.y < self.width() - 1.0e-8
            })
    }
    pub(crate) fn constrain(&self, point: Point, tolerance: f64) -> Point {
        if !self.is_visible() {
            return point;
        }
        let transform = self.transform();
        let nearest = |y| {
            let a = Point::new(0.0, y) * transform;
            let b = Point::new(self.length(), y) * transform;
            let x = b.x - a.x;
            let y = b.y - a.y;
            if x * x + y * y == 0.0 {
                return a;
            }
            let t = (((point.x - a.x) * x + (point.y - a.y) * y) / (x * x + y * y)).clamp(0.0, 1.0);
            Point::new(a.x + x * t, a.y + y * t)
        };
        let a = nearest(0.0);
        let b = nearest(self.width());
        let distance = |p: Point| (p.x - point.x).hypot(p.y - point.y);
        let candidate = if distance(a) < distance(b) { a } else { b };
        if distance(candidate) <= tolerance {
            candidate
        } else {
            point
        }
    }
}

#[derive(Clone, Copy)]
struct ArcMeasurement {
    start: f64,
    last: f64,
    sweep: f64,
}

/// A movable circular guide with a measured arc readout. The transform uses
/// the bounding circle's top-left corner; its local center is `(radius,radius)`.
pub struct InkPresenterProtractor {
    presenter: Weak<InkPresenter>,
    pub(crate) visible: Cell<bool>,
    transform: Cell<Matrix>,
    transform_explicit: Cell<bool>,
    radius: Cell<f64>,
    radius_explicit: Cell<bool>,
    center_marker: Cell<bool>,
    angle_readout: Cell<bool>,
    tick_marks: Cell<bool>,
    rays: Cell<bool>,
    resizable: Cell<bool>,
    background: Cell<Color>,
    foreground: Cell<Color>,
    accent: Cell<Color>,
    measurement: Cell<Option<ArcMeasurement>>,
}
stencil_contract!(InkPresenterProtractor, Protractor);
impl InkPresenterProtractor {
    pub(crate) fn new(presenter: Weak<InkPresenter>) -> Self {
        Self {
            presenter,
            visible: Cell::new(false),
            transform: Cell::new(Matrix::IDENTITY),
            transform_explicit: Cell::new(false),
            radius: Cell::new(280.0),
            radius_explicit: Cell::new(false),
            center_marker: Cell::new(true),
            angle_readout: Cell::new(true),
            tick_marks: Cell::new(true),
            rays: Cell::new(true),
            resizable: Cell::new(true),
            background: Cell::new(Color::from_uint32(0xffe6e6e6)),
            foreground: Cell::new(Color::from_uint32(0xff404040)),
            accent: Cell::new(Color::from_uint32(0xff0078d4)),
            measurement: Cell::new(None),
        }
    }
    pub fn kind(&self) -> InkPresenterStencilKind {
        InkPresenterStencilKind::Protractor
    }
    pub fn is_visible(&self) -> bool {
        self.visible.get()
    }
    pub fn set_is_visible(&self, value: bool) {
        if let Some(p) = self.presenter.upgrade() {
            p.set_protractor_visible(value);
        } else {
            self.visible.set(value);
        }
    }
    pub fn transform(&self) -> Matrix {
        self.transform.get()
    }
    pub fn set_transform(&self, value: Matrix) -> Result<(), InkError> {
        validate_transform(value, Size::new(self.radius() * 2.0, self.radius() * 2.0))?;
        self.transform_explicit.set(true);
        if self.transform.replace(value) != value {
            self.measurement.set(None);
            notify(&self.presenter);
        }
        Ok(())
    }
    pub fn radius(&self) -> f64 {
        self.radius.get()
    }
    pub fn set_radius(&self, value: f64) -> Result<(), InkError> {
        validate_dimension(value)?;
        validate_transform(self.transform(), Size::new(value * 2.0, value * 2.0))?;
        self.radius_explicit.set(true);
        if self.radius.replace(value) != value {
            self.measurement.set(None);
            notify(&self.presenter);
        }
        Ok(())
    }
    pub fn is_center_marker_visible(&self) -> bool {
        self.center_marker.get()
    }
    pub fn set_is_center_marker_visible(&self, value: bool) {
        if self.center_marker.replace(value) != value {
            notify(&self.presenter);
        }
    }
    pub fn is_angle_readout_visible(&self) -> bool {
        self.angle_readout.get()
    }
    pub fn set_is_angle_readout_visible(&self, value: bool) {
        if self.angle_readout.replace(value) != value {
            notify(&self.presenter);
        }
    }
    /// Compatibility spelling for the original portable API.
    pub fn are_angle_readouts_visible(&self) -> bool {
        self.is_angle_readout_visible()
    }
    pub fn set_are_angle_readouts_visible(&self, value: bool) {
        self.set_is_angle_readout_visible(value);
    }
    pub fn are_tick_marks_visible(&self) -> bool {
        self.tick_marks.get()
    }
    pub fn set_are_tick_marks_visible(&self, value: bool) {
        if self.tick_marks.replace(value) != value {
            notify(&self.presenter);
        }
    }
    pub fn are_rays_visible(&self) -> bool {
        self.rays.get()
    }
    pub fn set_are_rays_visible(&self, value: bool) {
        if self.rays.replace(value) != value {
            notify(&self.presenter);
        }
    }
    pub fn is_resizable(&self) -> bool {
        self.resizable.get()
    }
    pub fn set_is_resizable(&self, value: bool) {
        if self.resizable.replace(value) != value {
            notify(&self.presenter);
        }
    }
    pub fn background_color(&self) -> Color {
        self.background.get()
    }
    pub fn set_background_color(&self, value: Color) {
        if self.background.replace(value) != value {
            notify(&self.presenter);
        }
    }
    pub fn foreground_color(&self) -> Color {
        self.foreground.get()
    }
    pub fn set_foreground_color(&self, value: Color) {
        if self.foreground.replace(value) != value {
            notify(&self.presenter);
        }
    }
    pub fn accent_color(&self) -> Color {
        self.accent.get()
    }
    pub fn set_accent_color(&self, value: Color) {
        if self.accent.replace(value) != value {
            notify(&self.presenter);
        }
    }
    /// The absolute swept angle of the latest snapped arc, in degrees.
    pub fn angle(&self) -> Option<f64> {
        self.measurement.get().map(|v| v.sweep.abs().to_degrees())
    }
    pub(crate) fn rays(&self) -> Option<(f64, f64)> {
        self.measurement.get().map(|v| (v.start, v.last))
    }
    pub(crate) fn begin_ink(&self) {
        self.measurement.set(None);
    }
    pub(crate) fn center(&self) -> Point {
        Point::new(self.radius(), self.radius())
    }
    pub(crate) fn initialize_position(&self, size: Size) {
        if self.transform_explicit.get() || !valid_canvas_size(size) {
            return;
        }
        let scale = if self.radius_explicit.get() {
            1.0
        } else {
            (size.width.min(size.height) * 0.35 / self.radius()).min(1.0)
        };
        let value = Matrix::create_scale(scale, scale)
            * Matrix::create_translation(
                size.width * 0.5 - self.radius() * scale,
                size.height * 0.5 - self.radius() * scale,
            );
        if validate_transform(value, Size::new(self.radius() * 2.0, self.radius() * 2.0)).is_ok()
            && self.transform.replace(value) != value
        {
            self.measurement.set(None);
        }
    }
    pub(crate) fn apply_manipulation(
        &self,
        transform: Matrix,
        radius: f64,
    ) -> Result<(), InkError> {
        validate_dimension(radius)?;
        validate_transform(transform, Size::new(radius * 2.0, radius * 2.0))?;
        self.transform_explicit.set(true);
        self.radius_explicit.set(true);
        if self.transform.get() != transform || self.radius.get() != radius {
            self.transform.set(transform);
            self.radius.set(radius);
            self.measurement.set(None);
            notify(&self.presenter);
        }
        Ok(())
    }
    pub(crate) fn hit_test(&self, point: Point) -> bool {
        self.is_visible()
            && self.transform().try_invert().is_some_and(|inverse| {
                let p = point * inverse;
                (p.x - self.radius()).hypot(p.y - self.radius()) <= self.radius()
            })
    }
    pub(crate) fn blocks_ink(&self, point: Point) -> bool {
        self.is_visible()
            && self.transform().try_invert().is_some_and(|inverse| {
                let p = point * inverse;
                (p.x - self.radius()).hypot(p.y - self.radius()) < self.radius() - 1.0e-8
            })
    }
    pub(crate) fn constrain(&self, point: Point, tolerance: f64) -> Point {
        if !self.is_visible() {
            return point;
        }
        let Some(inverse) = self.transform().try_invert() else {
            return point;
        };
        let p = point * inverse;
        let x = p.x - self.radius();
        let y = p.y - self.radius();
        let distance = x.hypot(y);
        if distance <= f64::EPSILON {
            return point;
        }
        let candidate = Point::new(
            self.radius() + x * self.radius() / distance,
            self.radius() + y * self.radius() / distance,
        ) * self.transform();
        if (candidate.x - point.x).hypot(candidate.y - point.y) > tolerance {
            return point;
        }
        let angle = y.atan2(x);
        let measurement = match self.measurement.get() {
            Some(old) => ArcMeasurement {
                start: old.start,
                last: angle,
                sweep: old.sweep
                    + (angle - old.last + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU)
                    - std::f64::consts::PI,
            },
            None => ArcMeasurement {
                start: angle,
                last: angle,
                sweep: 0.0,
            },
        };
        self.measurement.set(Some(measurement));
        candidate
    }
}

/// Clips a sampled centerline against the visible guide, splitting sparse
/// crossings into separate visible polylines. Points already snapped to the
/// circular edge follow a bounded shortest arc instead of its obscured chord.
pub(crate) fn clip_stencil_segment(
    from: InkPoint,
    to: InkPoint,
    ruler: &InkPresenterRuler,
    protractor: &InkPresenterProtractor,
) -> Vec<Vec<InkPoint>> {
    let hidden = if ruler.is_visible() {
        let Some(inverse) = ruler.transform().try_invert() else {
            return vec![vec![from, to]];
        };
        rectangle_interval(
            from.position * inverse,
            to.position * inverse,
            ruler.length(),
            ruler.width(),
        )
    } else if protractor.is_visible() {
        let Some(inverse) = protractor.transform().try_invert() else {
            return vec![vec![from, to]];
        };
        let a = from.position * inverse;
        let b = to.position * inverse;
        let center = protractor.center();
        let radius = protractor.radius();
        let ax = a.x - center.x;
        let ay = a.y - center.y;
        let bx = b.x - center.x;
        let by = b.y - center.y;
        let epsilon = 1.0e-8 * radius.max(1.0);
        if (ax.hypot(ay) - radius).abs() <= epsilon && (bx.hypot(by) - radius).abs() <= epsilon {
            let start = ay.atan2(ax);
            let sweep = (by.atan2(bx) - start + std::f64::consts::PI)
                .rem_euclid(std::f64::consts::TAU)
                - std::f64::consts::PI;
            let transform = protractor.transform();
            let world_radius = radius
                * transform
                    .m11
                    .hypot(transform.m12)
                    .max(transform.m21.hypot(transform.m22));
            let steps = ((sweep.abs() * world_radius / 2.0).ceil() as usize).clamp(1, 256);
            let mut points = Vec::with_capacity(steps + 1);
            points.push(from);
            for step in 1..steps {
                let t = step as f64 / steps as f64;
                let angle = start + sweep * t;
                let mut point = InkPoint::interpolate(from, to, t);
                point.position = Point::new(
                    center.x + radius * angle.cos(),
                    center.y + radius * angle.sin(),
                ) * transform;
                points.push(point);
            }
            points.push(to);
            return vec![points];
        }
        circle_interval(Point::new(ax, ay), Point::new(bx, by), radius)
    } else {
        None
    };
    let Some((entry, exit)) = hidden else {
        return vec![vec![from, to]];
    };
    let mut pieces = Vec::with_capacity(2);
    if entry > 0.0 {
        pieces.push(vec![from, InkPoint::interpolate(from, to, entry)]);
    }
    if exit < 1.0 {
        pieces.push(vec![InkPoint::interpolate(from, to, exit), to]);
    }
    pieces
}

fn rectangle_interval(from: Point, to: Point, width: f64, height: f64) -> Option<(f64, f64)> {
    let mut entry = 0.0_f64;
    let mut exit = 1.0_f64;
    for (origin, delta, maximum) in [
        (from.x, to.x - from.x, width),
        (from.y, to.y - from.y, height),
    ] {
        if delta == 0.0 {
            // A line on the boundary is visible, so it can trace the guide.
            if origin <= 0.0 || origin >= maximum {
                return None;
            }
        } else {
            let first = -origin / delta;
            let second = (maximum - origin) / delta;
            entry = entry.max(first.min(second));
            exit = exit.min(first.max(second));
            if exit <= entry {
                return None;
            }
        }
    }
    Some((entry, exit))
}

fn circle_interval(from: Point, to: Point, radius: f64) -> Option<(f64, f64)> {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let distance = dx.hypot(dy);
    if distance <= f64::EPSILON {
        return (from.x.hypot(from.y) < radius).then_some((0.0, 1.0));
    }
    let ux = dx / distance;
    let uy = dy / distance;
    let perpendicular = (from.x * uy - from.y * ux).abs();
    if perpendicular >= radius {
        return None;
    }
    let projection = -(from.x * ux + from.y * uy);
    let half = ((radius - perpendicular) * (radius + perpendicular)).sqrt();
    let entry = ((projection - half) / distance).max(0.0);
    let exit = ((projection + half) / distance).min(1.0);
    (exit > entry).then_some((entry, exit))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dimensions_and_transforms_are_bounded_and_reject_perspective() {
        let p = InkPresenter::new();
        assert_eq!(InkPresenterStencilKind::Other as i32, 0);
        assert_eq!(p.ruler().kind() as i32, 1);
        assert_eq!(p.protractor().kind() as i32, 2);
        for invalid in [0.0, -1.0, f64::INFINITY, f64::NAN, MAX_STENCIL_SIZE + 1.0] {
            assert!(p.ruler().set_length(invalid).is_err());
            assert!(p.ruler().set_width(invalid).is_err());
            assert!(p.protractor().set_radius(invalid).is_err());
        }
        let mut perspective = Matrix::IDENTITY;
        perspective.m13 = 0.01;
        assert!(p.ruler().set_transform(perspective).is_err());
        assert!(p.protractor().set_transform(Matrix::default()).is_err());
        assert_eq!(p.ruler().transform(), Matrix::IDENTITY);
    }
    #[test]
    fn default_guides_fit_canvas_but_explicit_transforms_remain_authoritative() {
        let p = InkPresenter::new();
        p.ruler().initialize_position(Size::new(300.0, 200.0));
        p.protractor().initialize_position(Size::new(300.0, 200.0));
        assert_eq!(
            p.ruler().center() * p.ruler().transform(),
            Point::new(150.0, 100.0)
        );
        assert_eq!(
            p.protractor().center() * p.protractor().transform(),
            Point::new(150.0, 100.0)
        );
        assert_eq!(p.ruler().length(), 1600.0);
        assert_eq!(p.ruler().width(), 120.0);
        assert_eq!(p.protractor().radius(), 280.0);
        p.ruler().set_transform(Matrix::IDENTITY).unwrap();
        p.ruler().initialize_position(Size::new(900.0, 800.0));
        assert_eq!(p.ruler().transform(), Matrix::IDENTITY);
    }
    #[test]
    fn protractor_tracks_arc_across_angle_wrap_and_resets_after_manipulation() {
        let p = InkPresenter::new();
        let guide = p.protractor();
        guide.set_radius(100.0).unwrap();
        guide.set_is_visible(true);
        guide.begin_ink();
        for degrees in [170.0_f64, 179.0, -179.0, -160.0] {
            let radians = degrees.to_radians();
            guide.constrain(
                Point::new(100.0 + 103.0 * radians.cos(), 100.0 + 103.0 * radians.sin()),
                12.0,
            );
        }
        assert!((guide.angle().unwrap() - 30.0).abs() < 1e-9);
        assert!(guide.rays().is_some());
        guide
            .set_transform(Matrix::create_translation(10.0, 20.0))
            .unwrap();
        assert_eq!(guide.angle(), None);
    }

    #[test]
    fn sparse_ruler_crossing_splits_outside_fragments_and_preserves_boundary_tracing() {
        let p = InkPresenter::new();
        p.ruler().set_length(300.0).unwrap();
        p.ruler().set_is_visible(true);
        let mut from = InkPoint::new(Point::new(-20.0, 60.0), 0.2).unwrap();
        from.timestamp = 100;
        let mut to = InkPoint::new(Point::new(320.0, 60.0), 0.8).unwrap();
        to.timestamp = 440;
        let clipped = clip_stencil_segment(from, to, &p.ruler(), &p.protractor());
        assert_eq!(clipped.len(), 2);
        assert_eq!(clipped[0][1].position, Point::new(0.0, 60.0));
        assert_eq!(clipped[1][0].position, Point::new(300.0, 60.0));
        assert_eq!(clipped[0][1].timestamp, 120);
        assert_eq!(clipped[1][0].timestamp, 420);
        assert!(clipped[0][1].pressure > from.pressure);
        from.position.y = 0.0;
        to.position.y = 0.0;
        assert_eq!(
            clip_stencil_segment(from, to, &p.ruler(), &p.protractor()),
            vec![vec![from, to]]
        );
        from.position = Point::new(50.0, 60.0);
        to.position = Point::new(250.0, 60.0);
        assert!(clip_stencil_segment(from, to, &p.ruler(), &p.protractor()).is_empty());
    }

    #[test]
    fn sparse_circle_crossing_clips_but_snapped_points_follow_the_arc() {
        let p = InkPresenter::new();
        p.protractor().set_radius(100.0).unwrap();
        p.protractor().set_is_visible(true);
        let point = |x, y| InkPoint::new(Point::new(x, y), 0.5).unwrap();
        let clipped = clip_stencil_segment(
            point(-10.0, 100.0),
            point(210.0, 100.0),
            &p.ruler(),
            &p.protractor(),
        );
        assert_eq!(clipped.len(), 2);
        assert_eq!(clipped[0][1].position, Point::new(0.0, 100.0));
        assert_eq!(clipped[1][0].position, Point::new(200.0, 100.0));
        let arc = clip_stencil_segment(
            point(200.0, 100.0),
            point(100.0, 200.0),
            &p.ruler(),
            &p.protractor(),
        );
        assert_eq!(arc.len(), 1);
        assert!(arc[0].len() > 2);
        assert!(arc[0].len() <= 257);
        for point in &arc[0] {
            assert!(
                ((point.position.x - 100.0).hypot(point.position.y - 100.0) - 100.0).abs() < 1e-9
            );
        }
    }
}
