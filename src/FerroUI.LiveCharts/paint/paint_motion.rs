//! Per-paint property timelines that produce FerroUI brush and pen snapshots.

use super::{BrushSpace, snapshot_brush};
use crate::{
    PaintStyle,
    motion::{Animatable, Animation, PropertyError, PropertyValue},
};
use ferroui_base::{
    Point, RelativePoint, RelativeScalar, RelativeUnit,
    media::immutable::{
        ImmutableConicGradientBrush, ImmutableGradientStop, ImmutableLinearGradientBrush,
        ImmutableRadialGradientBrush, ImmutableSolidColorBrush, ImmutableTransform,
    },
    media::{Color, GradientSpreadMethod, IBrush, PenLineCap, PenLineJoin},
};
use std::{any::type_name, fmt, rc::Rc};

#[derive(Clone, Debug, PartialEq)]
pub enum PaintError {
    Property(PropertyError),
    InvalidValue {
        property: String,
        reason: &'static str,
    },
}
impl From<PropertyError> for PaintError {
    fn from(value: PropertyError) -> Self {
        Self::Property(value)
    }
}
impl fmt::Display for PaintError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Property(error) => fmt::Display::fmt(error, f),
            Self::InvalidValue { property, reason } => {
                write!(f, "Invalid paint property {property}: {reason}.")
            }
        }
    }
}
impl std::error::Error for PaintError {}

/// A sampled paint uses the same brush/pen objects as every other FerroUI control.
#[derive(Clone, Debug)]
pub struct PaintSnapshot {
    pub brush: Rc<dyn IBrush>,
    pub brush_space: BrushSpace,
    pub stroke_width: f64,
    pub stroke_miter: f64,
    pub stroke_cap: PenLineCap,
    pub stroke_join: PenLineJoin,
    pub dashes: Vec<f64>,
    pub dash_offset: f64,
    pub is_antialias: bool,
    pub is_paused: bool,
    pub z_index: f64,
}
impl PartialEq for PaintSnapshot {
    fn eq(&self, other: &Self) -> bool {
        same_brush(&self.brush, &other.brush)
            && self.brush_space == other.brush_space
            && self.stroke_width == other.stroke_width
            && self.stroke_miter == other.stroke_miter
            && self.stroke_cap == other.stroke_cap
            && self.stroke_join == other.stroke_join
            && self.dashes == other.dashes
            && self.dash_offset == other.dash_offset
            && self.is_antialias == other.is_antialias
            && self.is_paused == other.is_paused
            && self.z_index == other.z_index
    }
}
impl PaintSnapshot {
    pub fn apply_fill(&self, style: &mut PaintStyle) {
        style.fill = None;
        style.fill_brush = (!self.is_paused).then(|| self.brush.clone());
        style.brush_space = self.brush_space;
    }
    pub fn apply_stroke(&self, style: &mut PaintStyle) {
        style.stroke = None;
        style.stroke_brush = (!self.is_paused).then(|| self.brush.clone());
        style.brush_space = self.brush_space;
        style.stroke_width = self.stroke_width;
        style.stroke_miter = self.stroke_miter;
        style.stroke_cap = self.stroke_cap;
        style.stroke_join = self.stroke_join;
        style.dashes = self.dashes.clone();
        style.dash_offset = self.dash_offset;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaintKind {
    Solid,
    LinearGradient,
    RadialGradient,
    ConicGradient,
    Native,
}

#[derive(Clone, Copy, Debug)]
enum BrushMode {
    Solid,
    Linear {
        start: RelativeUnit,
        end: RelativeUnit,
    },
    Radial {
        center: RelativeUnit,
        origin: RelativeUnit,
        radius_x: RelativeUnit,
        radius_y: RelativeUnit,
        source_circle: bool,
    },
    Conic {
        center: RelativeUnit,
    },
    Native,
}
impl BrushMode {
    fn kind(self) -> PaintKind {
        match self {
            Self::Solid => PaintKind::Solid,
            Self::Linear { .. } => PaintKind::LinearGradient,
            Self::Radial { .. } => PaintKind::RadialGradient,
            Self::Conic { .. } => PaintKind::ConicGradient,
            Self::Native => PaintKind::Native,
        }
    }
}

/// A paint's timeline is independent of geometries referring to its brush.
/// Read it once per host frame and reuse its snapshot for all those geometries.
///
/// Source names (`Color`, `GradientStops`, `ColorPos`, `StartPoint`, `EndPoint`,
/// `Center`, `Radius`, `StrokeThickness`, `StrokeMiter`) are registered directly.
/// `ColorPos` is `Option<Vec<f64>>`; `None` retains source evenly spaced stops.
pub struct PaintMotion {
    values: Animatable,
    mode: BrushMode,
    transform: Option<Rc<ImmutableTransform>>,
    transform_origin: RelativePoint,
    relative_transform: Option<Rc<ImmutableTransform>>,
    cached_brush: Option<Rc<dyn IBrush>>,
    pub brush_space: BrushSpace,
}
impl fmt::Debug for PaintMotion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PaintMotion")
            .field("kind", &self.kind())
            .field("values", &self.values)
            .field("brush_space", &self.brush_space)
            .finish_non_exhaustive()
    }
}
impl Default for PaintMotion {
    fn default() -> Self {
        Self::solid(Color::default())
    }
}

impl PaintMotion {
    pub const DEFAULT_START_POINT: Point = Point::new(0.0, 0.5);
    pub const DEFAULT_END_POINT: Point = Point::new(1.0, 0.5);

    fn new(mode: BrushMode, opacity: f64) -> Self {
        let mut values = Animatable::new();
        values.register_motion("StrokeThickness", 1.0_f64).unwrap();
        values.register_motion("StrokeMiter", 0.0_f64).unwrap();
        values.register_motion("Opacity", opacity).unwrap();
        values.register_motion("Dashes", Vec::<f64>::new()).unwrap();
        values.register_motion("DashOffset", 0.0_f64).unwrap();
        values
            .register_state("StrokeCap", PenLineCap::Flat)
            .unwrap();
        values
            .register_state("StrokeJoin", PenLineJoin::Miter)
            .unwrap();
        values
            .register_state("SpreadMethod", GradientSpreadMethod::Pad)
            .unwrap();
        values.register_state("IsAntialias", true).unwrap();
        values.register_state("IsPaused", false).unwrap();
        values.register_state("ZIndex", 0.0_f64).unwrap();
        Self {
            values,
            mode,
            transform: None,
            transform_origin: RelativePoint::TOP_LEFT,
            relative_transform: None,
            cached_brush: None,
            brush_space: BrushSpace::Clip,
        }
    }
    pub fn solid(color: Color) -> Self {
        let mut result = Self::new(BrushMode::Solid, 1.0);
        result.values.register_motion("Color", color).unwrap();
        result
    }
    pub fn solid_with_stroke(color: Color, stroke_width: f64) -> Result<Self, PaintError> {
        nonnegative("StrokeThickness", stroke_width)?;
        let mut result = Self::solid(color);
        result.set("StrokeThickness", stroke_width, 0.0)?;
        result.values = result.values.clone_snapshot(0.0)?;
        Ok(result)
    }
    pub fn linear(gradient_stops: Vec<Color>) -> Self {
        Self::linear_with_points(
            gradient_stops,
            Self::DEFAULT_START_POINT,
            Self::DEFAULT_END_POINT,
            None,
        )
    }
    pub fn linear_with_points(
        gradient_stops: Vec<Color>,
        start: Point,
        end: Point,
        color_positions: Option<Vec<f64>>,
    ) -> Self {
        let mut result = Self::new(
            BrushMode::Linear {
                start: RelativeUnit::Relative,
                end: RelativeUnit::Relative,
            },
            1.0,
        );
        result.register_gradient(gradient_stops, color_positions);
        result.values.register_motion("StartPoint", start).unwrap();
        result.values.register_motion("EndPoint", end).unwrap();
        result
    }
    pub fn radial(gradient_stops: Vec<Color>) -> Self {
        Self::radial_with_center(gradient_stops, Point::new(0.5, 0.5), 0.5, None)
    }
    pub fn radial_with_center(
        gradient_stops: Vec<Color>,
        center: Point,
        radius: f64,
        color_positions: Option<Vec<f64>>,
    ) -> Self {
        let mut result = Self::new(
            BrushMode::Radial {
                center: RelativeUnit::Relative,
                origin: RelativeUnit::Relative,
                radius_x: RelativeUnit::Relative,
                radius_y: RelativeUnit::Relative,
                source_circle: true,
            },
            1.0,
        );
        result.register_gradient(gradient_stops, color_positions);
        result.values.register_motion("Center", center).unwrap();
        result.values.register_motion("Radius", radius).unwrap();
        result
    }
    fn register_gradient(&mut self, stops: Vec<Color>, positions: Option<Vec<f64>>) {
        self.values.register_motion("GradientStops", stops).unwrap();
        self.values.register_motion("ColorPos", positions).unwrap();
    }

    /// Freezes the supplied native brush, preserving its coordinate units,
    /// transforms, opacity, spread and independent radial focal point/radii.
    /// Custom brush kinds remain a motionless `Brush` state property.
    pub fn from_brush(brush: Rc<dyn IBrush>) -> Self {
        let brush = snapshot_brush(&brush);
        let mut result = if let Some(solid) = brush.as_solid_color_brush() {
            Self::solid(solid.color())
        } else if let Some(linear) = brush.as_linear_gradient_brush() {
            let mut result = Self::linear_with_points(
                Vec::new(),
                linear.start_point().point,
                linear.end_point().point,
                None,
            );
            result.mode = BrushMode::Linear {
                start: linear.start_point().unit,
                end: linear.end_point().unit,
            };
            result
        } else if let Some(radial) = brush.as_radial_gradient_brush() {
            let mut result = Self::radial_with_center(
                Vec::new(),
                radial.center().point,
                radial.radius_x().scalar,
                None,
            );
            result.mode = BrushMode::Radial {
                center: radial.center().unit,
                origin: radial.gradient_origin().unit,
                radius_x: radial.radius_x().unit,
                radius_y: radial.radius_y().unit,
                source_circle: false,
            };
            result
                .values
                .register_motion("GradientOrigin", radial.gradient_origin().point)
                .unwrap();
            result
                .values
                .register_motion("RadiusY", radial.radius_y().scalar)
                .unwrap();
            result
        } else if let Some(conic) = brush.as_conic_gradient_brush() {
            let mut result = Self::new(
                BrushMode::Conic {
                    center: conic.center().unit,
                },
                brush.opacity(),
            );
            result.register_gradient(Vec::new(), None);
            result
                .values
                .register_motion("Center", conic.center().point)
                .unwrap();
            result
                .values
                .register_motion("Angle", conic.angle())
                .unwrap();
            result
        } else {
            let mut result = Self::new(BrushMode::Native, 1.0);
            result
                .values
                .register_state("Brush", brush.clone())
                .unwrap();
            result
        };
        result.transform = brush
            .transform()
            .map(|t| Rc::new(ImmutableTransform::new(t.value())));
        result.relative_transform = brush
            .relative_transform()
            .map(|t| Rc::new(ImmutableTransform::new(t.value())));
        result.transform_origin = brush.transform_origin();
        if result.kind() != PaintKind::Native {
            result.set("Opacity", brush.opacity(), 0.0).unwrap();
        }
        if let Some(gradient) = brush.as_gradient_brush() {
            let stops = gradient.gradient_stops();
            result
                .set(
                    "GradientStops",
                    stops.iter().map(|s| s.color()).collect::<Vec<_>>(),
                    0.0,
                )
                .unwrap();
            result
                .set(
                    "ColorPos",
                    Some(stops.iter().map(|s| s.offset()).collect::<Vec<_>>()),
                    0.0,
                )
                .unwrap();
            result
                .set("SpreadMethod", gradient.spread_method(), 0.0)
                .unwrap();
        }
        result.brush_space = BrushSpace::Geometry;
        result.values = result
            .values
            .clone_snapshot(0.0)
            .expect("fresh native brush property types");
        result
    }

    pub fn kind(&self) -> PaintKind {
        self.mode.kind()
    }
    pub fn animatable(&self) -> &Animatable {
        &self.values
    }
    pub fn animatable_mut(&mut self) -> &mut Animatable {
        &mut self.values
    }
    pub fn is_valid(&self) -> bool {
        self.values.is_valid
    }
    pub fn set_transition(
        &mut self,
        animation: Option<Animation>,
        properties: &[&str],
    ) -> Result<(), PropertyError> {
        self.values.set_transition(animation, properties)
    }
    pub fn complete_transition(&mut self, properties: &[&str]) -> Result<(), PropertyError> {
        self.values.complete_transition(properties)
    }
    pub fn remove_transition(&mut self, properties: &[&str]) -> Result<(), PropertyError> {
        self.values.remove_transition(properties)
    }
    pub fn set<T: 'static>(
        &mut self,
        property: &str,
        value: T,
        now_ms: f64,
    ) -> Result<(), PropertyError> {
        self.values
            .set_value(property, PropertyValue::new(value), now_ms)
    }
    pub fn get<T: Clone + 'static>(
        &mut self,
        property: &str,
        now_ms: f64,
    ) -> Result<T, PropertyError> {
        let value = self.values.get_value(property, now_ms)?;
        value
            .downcast_ref::<T>()
            .cloned()
            .ok_or_else(|| PropertyError::TypeMismatch {
                property: property.to_owned(),
                expected: type_name::<T>(),
                actual: value.type_name(),
            })
    }

    /// Samples each registered property once and reuses an unchanged immutable
    /// brush. Invalid paint parameters are reported before a renderer sees them.
    pub fn sample(&mut self, now_ms: f64) -> Result<PaintSnapshot, PaintError> {
        self.values.begin_frame();
        let opacity: f64 = self.get("Opacity", now_ms)?;
        finite("Opacity", opacity)?;
        let opacity = opacity.clamp(0.0, 1.0);
        let brush: Rc<dyn IBrush> = match self.mode {
            BrushMode::Solid => Rc::new(ImmutableSolidColorBrush::with_transforms(
                self.get("Color", now_ms)?,
                opacity,
                self.transform.clone(),
                self.relative_transform.clone(),
            )),
            BrushMode::Native => {
                if opacity != 1.0 {
                    return Err(PaintError::InvalidValue {
                        property: "Opacity".to_owned(),
                        reason: "an opaque custom brush manages its own opacity",
                    });
                }
                self.get("Brush", now_ms)?
            }
            mode => {
                let colors: Vec<Color> = self.get("GradientStops", now_ms)?;
                let positions: Option<Vec<f64>> = self.get("ColorPos", now_ms)?;
                let stops = gradient_stops(&colors, positions.as_deref())?;
                let spread: GradientSpreadMethod = self.get("SpreadMethod", now_ms)?;
                match mode {
                    BrushMode::Linear { start, end } => {
                        let a = checked_point("StartPoint", self.get("StartPoint", now_ms)?)?;
                        let b = checked_point("EndPoint", self.get("EndPoint", now_ms)?)?;
                        Rc::new(ImmutableLinearGradientBrush::new(
                            &stops,
                            opacity,
                            self.transform.clone(),
                            Some(self.transform_origin),
                            spread,
                            Some(RelativePoint::from_point(a, start)),
                            Some(RelativePoint::from_point(b, end)),
                            self.relative_transform.clone(),
                        ))
                    }
                    BrushMode::Radial {
                        center,
                        origin,
                        radius_x,
                        radius_y,
                        source_circle,
                    } => {
                        let c = checked_point("Center", self.get("Center", now_ms)?)?;
                        let x: f64 = self.get("Radius", now_ms)?;
                        nonnegative("Radius", x)?;
                        let (o, y) = if source_circle {
                            (c, x)
                        } else {
                            (
                                checked_point(
                                    "GradientOrigin",
                                    self.get("GradientOrigin", now_ms)?,
                                )?,
                                self.get("RadiusY", now_ms)?,
                            )
                        };
                        nonnegative("RadiusY", y)?;
                        Rc::new(ImmutableRadialGradientBrush::new(
                            &stops,
                            opacity,
                            self.transform.clone(),
                            Some(self.transform_origin),
                            spread,
                            Some(RelativePoint::from_point(c, center)),
                            Some(RelativePoint::from_point(o, origin)),
                            Some(RelativeScalar::new(x, radius_x)),
                            Some(RelativeScalar::new(y, radius_y)),
                            self.relative_transform.clone(),
                        ))
                    }
                    BrushMode::Conic { center } => {
                        let c = checked_point("Center", self.get("Center", now_ms)?)?;
                        let angle: f64 = self.get("Angle", now_ms)?;
                        finite("Angle", angle)?;
                        Rc::new(ImmutableConicGradientBrush::new(
                            &stops,
                            opacity,
                            self.transform.clone(),
                            Some(self.transform_origin),
                            spread,
                            Some(RelativePoint::from_point(c, center)),
                            angle,
                            self.relative_transform.clone(),
                        ))
                    }
                    BrushMode::Solid | BrushMode::Native => unreachable!(),
                }
            }
        };
        let brush = match &self.cached_brush {
            Some(cached) if same_brush(cached, &brush) => cached.clone(),
            _ => {
                self.cached_brush = Some(brush.clone());
                brush
            }
        };
        let stroke_width = self.get("StrokeThickness", now_ms)?;
        let stroke_miter = self.get("StrokeMiter", now_ms)?;
        nonnegative("StrokeThickness", stroke_width)?;
        nonnegative("StrokeMiter", stroke_miter)?;
        let dashes: Vec<f64> = self.get("Dashes", now_ms)?;
        for value in &dashes {
            nonnegative("Dashes", *value)?;
        }
        let dash_offset = self.get("DashOffset", now_ms)?;
        finite("DashOffset", dash_offset)?;
        let z_index = self.get("ZIndex", now_ms)?;
        finite("ZIndex", z_index)?;
        Ok(PaintSnapshot {
            brush,
            brush_space: self.brush_space,
            stroke_width,
            stroke_miter,
            stroke_cap: self.get("StrokeCap", now_ms)?,
            stroke_join: self.get("StrokeJoin", now_ms)?,
            dashes,
            dash_offset,
            is_antialias: self.get("IsAntialias", now_ms)?,
            is_paused: self.get("IsPaused", now_ms)?,
            z_index,
        })
    }

    /// Creates an independent paint seeded from the current values. Registered
    /// motion defaults/from/to all start at those values; animations, state
    /// trackers and callbacks are intentionally instance-local.
    pub fn clone_snapshot(&mut self, now_ms: f64) -> Result<Self, PaintError> {
        let mut result = Self::new(self.mode, 1.0);
        result.transform = self.transform.clone();
        result.transform_origin = self.transform_origin;
        result.relative_transform = self.relative_transform.clone();
        result.brush_space = self.brush_space;
        result.values = self.values.clone_snapshot(now_ms)?;
        // Validate the new, settled values without resampling the source.
        result.sample(now_ms)?;
        Ok(result)
    }
    /// Releases cached native brush handles; reference-counted render snapshots
    /// remain valid for frames already retained by the host.
    pub fn clear_cache(&mut self) {
        self.cached_brush = None;
    }
}

fn finite(property: &str, value: f64) -> Result<(), PaintError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(PaintError::InvalidValue {
            property: property.to_owned(),
            reason: "must be finite",
        })
    }
}
fn nonnegative(property: &str, value: f64) -> Result<(), PaintError> {
    finite(property, value)?;
    if value >= 0.0 {
        Ok(())
    } else {
        Err(PaintError::InvalidValue {
            property: property.to_owned(),
            reason: "must be nonnegative",
        })
    }
}
fn checked_point(property: &str, value: Point) -> Result<Point, PaintError> {
    finite(property, value.x)?;
    finite(property, value.y)?;
    Ok(value)
}
fn gradient_stops(
    colors: &[Color],
    positions: Option<&[f64]>,
) -> Result<Vec<ImmutableGradientStop>, PaintError> {
    if colors.is_empty() {
        return Err(PaintError::InvalidValue {
            property: "GradientStops".to_owned(),
            reason: "must contain at least one color",
        });
    }
    if let Some(positions) = positions {
        if positions.len() != colors.len()
            || positions
                .iter()
                .any(|p| !p.is_finite() || !(0.0..=1.0).contains(p))
            || positions.windows(2).any(|pair| pair[0] > pair[1])
        {
            return Err(PaintError::InvalidValue {
                property: "ColorPos".to_owned(),
                reason: "must contain one finite, ordered [0,1] offset per color",
            });
        }
    }
    Ok(colors
        .iter()
        .enumerate()
        .map(|(index, color)| {
            ImmutableGradientStop::new(
                positions.map_or_else(
                    || {
                        if colors.len() == 1 {
                            0.0
                        } else {
                            index as f64 / (colors.len() - 1) as f64
                        }
                    },
                    |p| p[index],
                ),
                *color,
            )
        })
        .collect())
}

fn same_brush(a: &Rc<dyn IBrush>, b: &Rc<dyn IBrush>) -> bool {
    if Rc::ptr_eq(a, b) || a.equals(b.as_ref()) {
        return true;
    }
    if a.opacity() != b.opacity()
        || a.transform_origin() != b.transform_origin()
        || a.transform().map(|t| t.value()) != b.transform().map(|t| t.value())
        || a.relative_transform().map(|t| t.value()) != b.relative_transform().map(|t| t.value())
    {
        return false;
    }
    if let (Some(a), Some(b)) = (a.as_solid_color_brush(), b.as_solid_color_brush()) {
        return a.color() == b.color();
    }
    let geometry_matches = if let (Some(a), Some(b)) =
        (a.as_linear_gradient_brush(), b.as_linear_gradient_brush())
    {
        a.start_point() == b.start_point() && a.end_point() == b.end_point()
    } else if let (Some(a), Some(b)) = (a.as_radial_gradient_brush(), b.as_radial_gradient_brush())
    {
        a.center() == b.center()
            && a.gradient_origin() == b.gradient_origin()
            && a.radius_x() == b.radius_x()
            && a.radius_y() == b.radius_y()
    } else if let (Some(a), Some(b)) = (a.as_conic_gradient_brush(), b.as_conic_gradient_brush()) {
        a.center() == b.center() && a.angle() == b.angle()
    } else {
        false
    };
    if !geometry_matches {
        return false;
    }
    let (Some(a), Some(b)) = (a.as_gradient_brush(), b.as_gradient_brush()) else {
        return false;
    };
    let (a_stops, b_stops) = (a.gradient_stops(), b.gradient_stops());
    a.spread_method() == b.spread_method()
        && a_stops.len() == b_stops.len()
        && a_stops
            .iter()
            .zip(b_stops)
            .all(|(a, b)| a.color() == b.color() && a.offset() == b.offset())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        EasingFunctions,
        visual_states::{DrawnPropertiesDictionary, DrawnPropertySetter, VisualStatesDictionary},
    };
    use ferroui_base::{Matrix, Rect, Size};

    fn animation() -> Animation {
        Animation::new(1000.0, EasingFunctions::lineal)
    }
    fn colors() -> Vec<Color> {
        vec![Color::from_rgb(255, 0, 0), Color::from_rgb(0, 0, 255)]
    }

    #[test]
    fn source_solid_color_interpolates_channels_and_completes() {
        let from = Color::from_rgb(0, 0, 0);
        let to = Color::from_rgb(200, 100, 50);
        let mut paint = PaintMotion::default();
        paint.set_transition(Some(animation()), &["Color"]).unwrap();
        paint.set("Color", from, 0.0).unwrap();
        paint.complete_transition(&["Color"]).unwrap();
        paint.set("Color", to, 0.0).unwrap();
        for time in [250.0, 500.0, 750.0] {
            let snapshot = paint.sample(time).unwrap();
            let color = snapshot.brush.as_solid_color_brush().unwrap().color();
            let p = time / 1000.0;
            assert_eq!(
                color,
                Color::from_rgb((200.0 * p) as u8, (100.0 * p) as u8, (50.0 * p) as u8)
            );
            assert_eq!(color.a, 255);
            assert!(!paint.is_valid());
        }
        assert_eq!(
            paint
                .sample(2000.0)
                .unwrap()
                .brush
                .as_solid_color_brush()
                .unwrap()
                .color(),
            to
        );
        assert!(paint.is_valid());
        assert!(
            paint
                .animatable()
                .motion_info("Color")
                .unwrap()
                .unwrap()
                .is_completed
        );
    }

    #[test]
    fn source_stroke_thickness_interpolates_and_completes() {
        let mut paint = PaintMotion::default();
        paint
            .set_transition(Some(animation()), &["StrokeThickness"])
            .unwrap();
        paint.set("StrokeThickness", 2.0_f64, 0.0).unwrap();
        paint.complete_transition(&["StrokeThickness"]).unwrap();
        paint.set("StrokeThickness", 10.0_f64, 0.0).unwrap();
        for time in [250.0, 500.0, 750.0] {
            assert_eq!(
                paint.sample(time).unwrap().stroke_width,
                2.0 + time / 1000.0 * 8.0
            );
            assert!(!paint.is_valid());
        }
        assert_eq!(paint.sample(2000.0).unwrap().stroke_width, 10.0);
        assert!(
            paint
                .animatable()
                .motion_info("StrokeThickness")
                .unwrap()
                .unwrap()
                .is_completed
        );
    }

    #[test]
    fn source_constructor_and_clone_seed_color_baselines() {
        let red = Color::from_rgb(255, 0, 0);
        let mut paint = PaintMotion::solid(red);
        let clone = paint.clone_snapshot(0.0).unwrap();
        for value in [&paint, &clone] {
            let motion = value.animatable().motion_info("Color").unwrap().unwrap();
            assert_eq!(*motion.from_value.downcast_ref::<Color>().unwrap(), red);
            assert_eq!(*motion.to_value.downcast_ref::<Color>().unwrap(), red);
        }
    }

    #[test]
    fn source_gradient_interpolation_completion_and_mismatched_length_snap() {
        let mut paint = PaintMotion::linear(vec![Color::from_rgb(0, 0, 0); 2]);
        paint
            .set_transition(Some(animation()), &["GradientStops"])
            .unwrap();
        let to = vec![
            Color::from_rgb(200, 100, 50),
            Color::from_rgb(100, 200, 150),
        ];
        paint.set("GradientStops", to.clone(), 0.0).unwrap();
        let mid = paint.sample(500.0).unwrap();
        let stops = mid.brush.as_gradient_brush().unwrap().gradient_stops();
        assert_eq!(stops[0].color(), Color::from_rgb(100, 50, 25));
        assert_eq!(stops[1].color(), Color::from_rgb(50, 100, 75));
        assert!(!paint.is_valid());
        let end = paint.sample(2000.0).unwrap();
        assert_eq!(
            end.brush
                .as_gradient_brush()
                .unwrap()
                .gradient_stops()
                .iter()
                .map(|s| s.color())
                .collect::<Vec<_>>(),
            to
        );
        assert!(paint.is_valid());
        let three = vec![
            Color::from_rgb(255, 0, 0),
            Color::from_rgb(0, 128, 0),
            Color::from_rgb(0, 0, 255),
        ];
        paint.set("GradientStops", three.clone(), 2000.0).unwrap();
        let snapped = paint.sample(2500.0).unwrap();
        assert_eq!(
            snapped
                .brush
                .as_gradient_brush()
                .unwrap()
                .gradient_stops()
                .iter()
                .map(|s| s.color())
                .collect::<Vec<_>>(),
            three
        );
        assert!(paint.is_valid());
    }

    #[test]
    fn source_gradient_points_positions_and_null_restore_use_native_brushes() {
        let mut paint = PaintMotion::linear_with_points(
            colors(),
            Point::new(0.0, 0.0),
            Point::new(1.0, 1.0),
            Some(vec![0.0, 1.0]),
        );
        paint
            .set_transition(Some(animation()), &["StartPoint", "ColorPos"])
            .unwrap();
        paint.set("StartPoint", Point::new(1.0, 1.0), 0.0).unwrap();
        paint
            .set("ColorPos", Some(vec![0.2_f64, 0.8]), 0.0)
            .unwrap();
        let mid = paint.sample(500.0).unwrap();
        let gradient = mid.brush.as_linear_gradient_brush().unwrap();
        assert_eq!(gradient.start_point(), RelativePoint::CENTER);
        let positions = gradient.gradient_stops();
        assert!((positions[0].offset() - 0.1).abs() < 1e-9);
        assert!((positions[1].offset() - 0.9).abs() < 1e-9);
        paint.set("ColorPos", None::<Vec<f64>>, 500.0).unwrap();
        let uniform = paint.sample(600.0).unwrap();
        assert_eq!(
            uniform
                .brush
                .as_gradient_brush()
                .unwrap()
                .gradient_stops()
                .iter()
                .map(|s| s.offset())
                .collect::<Vec<_>>(),
            [0.0, 1.0]
        );
        assert!(
            paint
                .get::<Option<Vec<f64>>>("ColorPos", 600.0)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn source_radial_center_motion_keeps_circular_clip_radius() {
        let mut paint = PaintMotion::radial_with_center(colors(), Point::new(0.0, 0.0), 0.5, None);
        paint
            .set_transition(Some(animation()), &["Center"])
            .unwrap();
        paint.set("Center", Point::new(1.0, 1.0), 0.0).unwrap();
        let snapshot = paint.sample(500.0).unwrap();
        let relative = snapshot.brush.as_radial_gradient_brush().unwrap();
        assert_eq!(relative.center(), RelativePoint::CENTER);
        assert_eq!(relative.gradient_origin(), RelativePoint::CENTER);
        let reference = snapshot.brush_space.reference_rect(
            Size::new(400.0, 300.0),
            Rect::default(),
            Some(Rect::new(50.0, 20.0, 200.0, 100.0)),
        );
        let native = super::super::resolve_brush(&snapshot.brush, reference);
        assert_eq!(
            native.as_radial_gradient_brush().unwrap().radius_x(),
            RelativeScalar::new(50.0, RelativeUnit::Absolute)
        );
    }

    #[test]
    fn native_import_preserves_relative_units_focal_point_elliptical_radii_and_transforms() {
        let stops = [
            ImmutableGradientStop::new(0.0, colors()[0]),
            ImmutableGradientStop::new(1.0, colors()[1]),
        ];
        let transform = Rc::new(ImmutableTransform::new(Matrix::create_translation(
            5.0, 7.0,
        )));
        let source: Rc<dyn IBrush> = Rc::new(ImmutableRadialGradientBrush::new(
            &stops,
            0.6,
            Some(transform),
            Some(RelativePoint::CENTER),
            GradientSpreadMethod::Reflect,
            Some(RelativePoint::new(80.0, 60.0, RelativeUnit::Absolute)),
            Some(RelativePoint::new(60.0, 50.0, RelativeUnit::Absolute)),
            Some(RelativeScalar::new(40.0, RelativeUnit::Absolute)),
            Some(RelativeScalar::new(20.0, RelativeUnit::Absolute)),
            None,
        ));
        let mut paint = PaintMotion::from_brush(source.clone());
        let snapshot = paint.sample(0.0).unwrap();
        assert_eq!(snapshot.brush_space, BrushSpace::Geometry);
        assert!(same_brush(&source, &snapshot.brush));
        assert!(paint.is_valid());
        let motion = paint
            .animatable()
            .motion_info("GradientStops")
            .unwrap()
            .unwrap();
        assert_eq!(
            motion.from_value.downcast_ref::<Vec<Color>>().unwrap(),
            &colors()
        );
    }

    #[test]
    fn sampled_clone_retargets_independently_and_preserves_null_positions() {
        let mut paint = PaintMotion::linear(colors());
        paint
            .set_transition(Some(animation()), &["StartPoint"])
            .unwrap();
        paint.set("StartPoint", Point::new(1.0, 1.0), 0.0).unwrap();
        let mut clone = paint.clone_snapshot(500.0).unwrap();
        let motion = clone
            .animatable()
            .motion_info("StartPoint")
            .unwrap()
            .unwrap();
        assert_eq!(
            *motion.from_value.downcast_ref::<Point>().unwrap(),
            Point::new(0.5, 0.75)
        );
        assert_eq!(
            *motion.to_value.downcast_ref::<Point>().unwrap(),
            Point::new(0.5, 0.75)
        );
        assert!(motion.animation.is_none());
        assert!(
            clone
                .get::<Option<Vec<f64>>>("ColorPos", 500.0)
                .unwrap()
                .is_none()
        );
        clone
            .set("StartPoint", Point::new(0.0, 0.0), 500.0)
            .unwrap();
        assert_eq!(
            paint.get::<Point>("StartPoint", 500.0).unwrap(),
            Point::new(0.5, 0.75)
        );
    }

    #[test]
    fn native_brush_cache_reuses_settled_values_and_releases_ownership() {
        let mut paint = PaintMotion::linear(colors());
        let first = paint.sample(0.0).unwrap();
        let second = paint.sample(10.0).unwrap();
        assert!(Rc::ptr_eq(&first.brush, &second.brush));
        paint.set("Opacity", 0.5_f64, 10.0).unwrap();
        let half = paint.sample(10.0).unwrap();
        assert!(!Rc::ptr_eq(&first.brush, &half.brush));
        assert!(Rc::ptr_eq(&half.brush, &paint.sample(20.0).unwrap().brush));
        let weak = Rc::downgrade(&half.brush);
        drop(half);
        paint.clear_cache();
        assert!(weak.upgrade().is_none());
        assert_eq!(first.brush.opacity(), 1.0);
    }

    #[test]
    fn paint_states_apply_restore_and_validation_prevents_invalid_native_stops() {
        let mut paint = PaintMotion::solid(Color::from_rgb(255, 0, 0));
        let mut states = VisualStatesDictionary::new();
        states.insert(
            "Hover",
            DrawnPropertiesDictionary::new([
                DrawnPropertySetter::new("Color", Color::from_rgb(0, 0, 255)),
                DrawnPropertySetter::new("StrokeThickness", 10.0_f64),
            ]),
        );
        states
            .set_state("Hover", paint.animatable_mut(), 0.0)
            .unwrap();
        let snapshot = paint.sample(0.0).unwrap();
        assert_eq!(snapshot.stroke_width, 10.0);
        assert_eq!(
            snapshot.brush.as_solid_color_brush().unwrap().color(),
            Color::from_rgb(0, 0, 255)
        );
        states.clear_states(paint.animatable_mut(), 0.0).unwrap();
        assert_eq!(paint.sample(0.0).unwrap().stroke_width, 1.0);
        let mut gradient = PaintMotion::linear(colors());
        gradient
            .set("ColorPos", Some(vec![0.7_f64, 0.3]), 0.0)
            .unwrap();
        assert!(matches!(
            gradient.sample(0.0),
            Err(PaintError::InvalidValue { .. })
        ));
    }
}
