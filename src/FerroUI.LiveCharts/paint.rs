//! LiveCharts paint semantics carried by FerroUI's native brush and pen types.
//!
//! Source gradients use the active clip as their reference rectangle and a
//! circular radial radius. `Geometry` retains FerroUI's usual shape-relative
//! brush coordinates. Source color transitions interpolate encoded ARGB bytes;
//! FerroUI's gamma-correct color animator therefore cannot be used unchanged.

use crate::motion::Interpolate;
use ferroui_base::animation::animators::GradientBrushAnimator;
use ferroui_base::media::immutable::{
    ImmutableConicGradientBrush, ImmutableGradientStop, ImmutableLinearGradientBrush,
    ImmutableRadialGradientBrush, ImmutableSolidColorBrush, ImmutableTransform,
};
use ferroui_base::media::{Color, IBrush, IGradientBrush};
use ferroui_base::{Matrix, Rect, RelativePoint, RelativeScalar, RelativeUnit, Size};
use std::rc::Rc;

/// The rectangle against which relative gradient coordinates are evaluated.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BrushSpace {
    /// Native FerroUI semantics: each drawn geometry supplies its bounds.
    Geometry,
    /// The chart's measured data rectangle.
    Plot,
    /// LiveCharts semantics: the currently active chart clip.
    #[default]
    Clip,
}
ferroui_base::ferro_markup_enum!(BrushSpace {
    Geometry,
    Plot,
    Clip
});

impl BrushSpace {
    pub fn reference_rect(self, size: Size, plot: Rect, clip: Option<Rect>) -> Option<Rect> {
        match self {
            Self::Geometry => None,
            Self::Plot => Some(plot),
            Self::Clip => {
                let canvas = Rect::new(0.0, 0.0, size.width, size.height);
                Some(clip.map_or(canvas, |clip| canvas.intersect(clip)))
            }
        }
    }
}

fn stops(gradient: &dyn IGradientBrush) -> Vec<ImmutableGradientStop> {
    gradient
        .gradient_stops()
        .iter()
        .map(|stop| ImmutableGradientStop::new(stop.offset(), stop.color()))
        .collect()
}

fn transform_snapshot(brush: &dyn IBrush) -> Option<Rc<ImmutableTransform>> {
    brush
        .transform()
        .map(|transform| Rc::new(ImmutableTransform::new(transform.value())))
}

fn relative_transform_snapshot(brush: &dyn IBrush) -> Option<Rc<ImmutableTransform>> {
    brush
        .relative_transform()
        .map(|transform| Rc::new(ImmutableTransform::new(transform.value())))
}

/// Freezes native mutable brush values before a measured scene is retained.
/// Custom non-native brush implementations are responsible for immutability.
pub fn snapshot_brush(brush: &Rc<dyn IBrush>) -> Rc<dyn IBrush> {
    if brush.as_immutable_brush().is_some() {
        return brush.clone();
    }
    if let Some(gradient) = brush.as_gradient_brush() {
        if let Some(result) = copy_gradient(brush.as_ref(), &stops(gradient), brush.opacity(), None)
        {
            return result;
        }
    }
    if let Some(mutable) = brush.as_mutable_brush() {
        return mutable.to_immutable() as Rc<dyn IBrush>;
    }
    brush.clone()
}

/// Returns a solid brush only when no native brush override was supplied.
pub fn brush_or_color(
    brush: &Option<Rc<dyn IBrush>>,
    color: Option<Color>,
) -> Option<Rc<dyn IBrush>> {
    brush.clone().or_else(|| {
        color.map(|color| Rc::new(ImmutableSolidColorBrush::new(color)) as Rc<dyn IBrush>)
    })
}

/// A representative color for legend and tooltip accents.
pub fn representative_color(brush: &dyn IBrush) -> Option<Color> {
    brush
        .as_solid_color_brush()
        .map(|solid| solid.color())
        .or_else(|| {
            brush
                .as_gradient_brush()?
                .gradient_stops()
                .first()
                .map(|stop| stop.color())
        })
}

/// Resolves a native gradient into the requested chart coordinate rectangle.
pub fn resolve_brush(brush: &Rc<dyn IBrush>, reference: Option<Rect>) -> Rc<dyn IBrush> {
    let Some(reference) = reference else {
        return brush.clone();
    };
    let Some(gradient) = brush.as_gradient_brush() else {
        return brush.clone();
    };
    copy_gradient(
        brush.as_ref(),
        &stops(gradient),
        brush.opacity(),
        Some(reference),
    )
    .unwrap_or_else(|| brush.clone())
}

fn reference_transform(brush: &dyn IBrush, rect: Rect) -> Option<Rc<ImmutableTransform>> {
    let relative = brush.relative_transform().and_then(|transform| {
        (rect.width > 0.0 && rect.height > 0.0).then(|| {
            Matrix::create_translation(-rect.x, -rect.y)
                * Matrix::create_scale(1.0 / rect.width, 1.0 / rect.height)
                * transform.value()
                * Matrix::create_scale(rect.width, rect.height)
                * Matrix::create_translation(rect.x, rect.y)
        })
    });
    let absolute = brush.transform().map(|transform| {
        let origin = brush.transform_origin().to_pixels_rect(rect);
        Matrix::create_translation(-origin.x, -origin.y)
            * transform.value()
            * Matrix::create_translation(origin.x, origin.y)
    });
    match (relative, absolute) {
        (Some(a), Some(b)) => Some(a * b),
        (a, b) => a.or(b),
    }
    .map(|matrix| Rc::new(ImmutableTransform::new(matrix)))
}

fn copy_gradient(
    brush: &dyn IBrush,
    stops: &[ImmutableGradientStop],
    opacity: f64,
    reference: Option<Rect>,
) -> Option<Rc<dyn IBrush>> {
    let gradient = brush.as_gradient_brush()?;
    let point = |point: RelativePoint| {
        reference.map_or(point, |rect| {
            RelativePoint::from_point(point.to_pixels_rect(rect), RelativeUnit::Absolute)
        })
    };
    let radius = |radius: RelativeScalar| {
        reference.map_or(radius, |rect| {
            RelativeScalar::new(
                radius.to_value(rect.width.min(rect.height)),
                RelativeUnit::Absolute,
            )
        })
    };
    let transform = reference.map_or_else(
        || transform_snapshot(brush),
        |rect| reference_transform(brush, rect),
    );
    let origin = Some(if reference.is_some() {
        RelativePoint::new(0.0, 0.0, RelativeUnit::Absolute)
    } else {
        brush.transform_origin()
    });
    let relative_transform = if reference.is_some() {
        None
    } else {
        relative_transform_snapshot(brush)
    };
    if let Some(linear) = brush.as_linear_gradient_brush() {
        return Some(Rc::new(ImmutableLinearGradientBrush::new(
            stops,
            opacity,
            transform,
            origin,
            gradient.spread_method(),
            Some(point(linear.start_point())),
            Some(point(linear.end_point())),
            relative_transform,
        )));
    }
    if let Some(radial) = brush.as_radial_gradient_brush() {
        return Some(Rc::new(ImmutableRadialGradientBrush::new(
            stops,
            opacity,
            transform,
            origin,
            gradient.spread_method(),
            Some(point(radial.center())),
            Some(point(radial.gradient_origin())),
            Some(radius(radial.radius_x())),
            Some(radius(radial.radius_y())),
            relative_transform,
        )));
    }
    if let Some(conic) = brush.as_conic_gradient_brush() {
        return Some(Rc::new(ImmutableConicGradientBrush::new(
            stops,
            opacity,
            transform,
            origin,
            gradient.spread_method(),
            Some(point(conic.center())),
            conic.angle(),
            relative_transform,
        )));
    }
    None
}

/// Interpolates native paint values with the source encoded-channel arithmetic.
/// A different gradient stop count snaps that property to its target while
/// compatible spatial properties continue moving. Unsupported kind changes
/// return `None`, allowing the scene host to crossfade the two primitives.
pub fn interpolate_brush(
    from: &Rc<dyn IBrush>,
    to: &Rc<dyn IBrush>,
    progress: f64,
) -> Option<Rc<dyn IBrush>> {
    if Rc::ptr_eq(from, to) || progress == 0.0 {
        return Some(from.clone());
    }
    if progress == 1.0 {
        return Some(to.clone());
    }
    if let (Some(a), Some(b)) = (from.as_solid_color_brush(), to.as_solid_color_brush()) {
        return Some(Rc::new(ImmutableSolidColorBrush::with_transforms(
            Color::interpolate(&a.color(), &b.color(), progress),
            f64::interpolate(&a.opacity(), &b.opacity(), progress).clamp(0.0, 1.0),
            transform_snapshot(if progress < 0.5 {
                from.as_ref()
            } else {
                to.as_ref()
            }),
            relative_transform_snapshot(if progress < 0.5 {
                from.as_ref()
            } else {
                to.as_ref()
            }),
        )));
    }
    let (Some(a), Some(b)) = (from.as_gradient_brush(), to.as_gradient_brush()) else {
        return None;
    };
    let compatible = (from.as_linear_gradient_brush().is_some()
        && to.as_linear_gradient_brush().is_some())
        || (from.as_radial_gradient_brush().is_some() && to.as_radial_gradient_brush().is_some())
        || (from.as_conic_gradient_brush().is_some() && to.as_conic_gradient_brush().is_some());
    if !compatible {
        return None;
    }
    let (a_stops, b_stops) = (stops(a), stops(b));
    if a_stops.is_empty() || b_stops.is_empty() {
        return Some(to.clone());
    }
    let interpolated_stops = if a_stops.len() == b_stops.len() {
        a_stops
            .iter()
            .zip(&b_stops)
            .map(|(a, b)| {
                ImmutableGradientStop::new(
                    f64::interpolate(&a.offset(), &b.offset(), progress),
                    Color::interpolate(&a.color(), &b.color(), progress),
                )
            })
            .collect()
    } else {
        b_stops
    };
    // Reuse FerroUI's relative-point, radius and transform interpolation. Only
    // the stop array is replaced: native gamma-correct colors and padded stop
    // counts intentionally differ from the source motion-property contracts.
    let spatial =
        GradientBrushAnimator::interpolate_core(progress, &Some(from.clone()), &Some(to.clone()))?;
    copy_gradient(
        spatial.as_ref(),
        &interpolated_stops,
        spatial.opacity().clamp(0.0, 1.0),
        None,
    )
}

/// Changes the native paint opacity without altering its gradient or colors.
pub fn fade_brush(brush: &Rc<dyn IBrush>, amount: f64) -> Option<Rc<dyn IBrush>> {
    let opacity = (brush.opacity() * amount).clamp(0.0, 1.0);
    if let Some(solid) = brush.as_solid_color_brush() {
        return Some(Rc::new(ImmutableSolidColorBrush::with_transforms(
            solid.color(),
            opacity,
            transform_snapshot(brush.as_ref()),
            relative_transform_snapshot(brush.as_ref()),
        )));
    }
    copy_gradient(
        brush.as_ref(),
        &stops(brush.as_gradient_brush()?),
        opacity,
        None,
    )
}

/// Converts source absolute dash lengths to FerroUI's pen-thickness units.
pub fn native_dash_pattern(
    dashes: &[f64],
    phase: f64,
    stroke_width: f64,
) -> Option<(Vec<f64>, f64)> {
    if !stroke_width.is_finite()
        || stroke_width <= 0.0
        || !phase.is_finite()
        || dashes.is_empty()
        || dashes.iter().any(|v| !v.is_finite() || *v < 0.0)
        || !dashes.iter().any(|v| *v > 0.0)
    {
        return None;
    }
    Some((
        dashes.iter().map(|value| value / stroke_width).collect(),
        phase / stroke_width,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::media::{GradientSpreadMethod, PenLineCap, PenLineJoin};

    fn linear(colors: &[Color], offsets: Option<&[f64]>, start: RelativePoint) -> Rc<dyn IBrush> {
        let stops: Vec<_> = colors
            .iter()
            .enumerate()
            .map(|(i, color)| {
                ImmutableGradientStop::new(
                    offsets.map_or(i as f64 / (colors.len() - 1) as f64, |v| v[i]),
                    *color,
                )
            })
            .collect();
        Rc::new(ImmutableLinearGradientBrush::new(
            &stops,
            1.0,
            None,
            None,
            GradientSpreadMethod::Pad,
            Some(start),
            Some(RelativePoint::BOTTOM_RIGHT),
            None,
        ))
    }

    #[test]
    fn source_gradient_channels_positions_and_points_interpolate() {
        let a = linear(
            &[Color::from_rgb(0, 0, 0), Color::from_rgb(0, 0, 0)],
            Some(&[0.0, 1.0]),
            RelativePoint::TOP_LEFT,
        );
        let b = linear(
            &[
                Color::from_rgb(200, 100, 50),
                Color::from_rgb(100, 200, 150),
            ],
            Some(&[0.2, 0.8]),
            RelativePoint::BOTTOM_RIGHT,
        );
        let result = interpolate_brush(&a, &b, 0.5).unwrap();
        let gradient = result.as_linear_gradient_brush().unwrap();
        let stops = gradient.gradient_stops();
        assert_eq!(stops[0].color(), Color::from_rgb(100, 50, 25));
        assert_eq!(stops[1].color(), Color::from_rgb(50, 100, 75));
        assert!((stops[0].offset() - 0.1).abs() < 1e-12);
        assert!((stops[1].offset() - 0.9).abs() < 1e-12);
        assert_eq!(gradient.start_point(), RelativePoint::CENTER);
        assert!(Rc::ptr_eq(&interpolate_brush(&a, &b, 1.0).unwrap(), &b));
    }

    #[test]
    fn source_changed_stop_count_snaps_without_padding() {
        let black = Color::from_rgb(0, 0, 0);
        let colors = [
            Color::from_rgb(255, 0, 0),
            Color::from_rgb(0, 128, 0),
            Color::from_rgb(0, 0, 255),
        ];
        let a = linear(&[black, black], None, RelativePoint::TOP_LEFT);
        let b = linear(&colors, None, RelativePoint::BOTTOM_RIGHT);
        let result = interpolate_brush(&a, &b, 0.5).unwrap();
        let gradient = result.as_linear_gradient_brush().unwrap();
        assert_eq!(
            gradient
                .gradient_stops()
                .iter()
                .map(|s| s.color())
                .collect::<Vec<_>>(),
            colors
        );
        assert_eq!(gradient.start_point(), RelativePoint::CENTER);
    }

    #[test]
    fn clip_reference_resolves_global_linear_coordinates() {
        let brush = linear(
            &[Color::from_rgb(255, 0, 0), Color::from_rgb(0, 0, 255)],
            None,
            RelativePoint::CENTER,
        );
        let reference = BrushSpace::Clip.reference_rect(
            Size::new(400.0, 300.0),
            Rect::default(),
            Some(Rect::new(50.0, 20.0, 200.0, 100.0)),
        );
        let result = resolve_brush(&brush, reference);
        let linear = result.as_linear_gradient_brush().unwrap();
        assert_eq!(
            linear.start_point(),
            RelativePoint::new(150.0, 70.0, RelativeUnit::Absolute)
        );
        assert_eq!(
            linear.end_point(),
            RelativePoint::new(250.0, 120.0, RelativeUnit::Absolute)
        );
        assert!(Rc::ptr_eq(&resolve_brush(&brush, None), &brush));
    }

    #[test]
    fn radial_center_animates_and_reference_radius_remains_circular() {
        let stops = [
            ImmutableGradientStop::new(0.0, Color::from_rgb(255, 0, 0)),
            ImmutableGradientStop::new(1.0, Color::from_rgb(0, 0, 255)),
        ];
        let make = |center| {
            Rc::new(ImmutableRadialGradientBrush::with_radius(
                &stops,
                1.0,
                None,
                None,
                GradientSpreadMethod::Reflect,
                Some(center),
                Some(center),
                0.5,
            )) as Rc<dyn IBrush>
        };
        let result = interpolate_brush(
            &make(RelativePoint::TOP_LEFT),
            &make(RelativePoint::BOTTOM_RIGHT),
            0.5,
        )
        .unwrap();
        assert_eq!(
            result.as_radial_gradient_brush().unwrap().center(),
            RelativePoint::CENTER
        );
        let result = resolve_brush(&result, Some(Rect::new(50.0, 20.0, 200.0, 100.0)));
        let radial = result.as_radial_gradient_brush().unwrap();
        assert_eq!(
            radial.center(),
            RelativePoint::new(150.0, 70.0, RelativeUnit::Absolute)
        );
        assert_eq!(
            radial.radius_x(),
            RelativeScalar::new(50.0, RelativeUnit::Absolute)
        );
        assert_eq!(
            radial.radius_y(),
            RelativeScalar::new(50.0, RelativeUnit::Absolute)
        );
        assert_eq!(radial.spread_method(), GradientSpreadMethod::Reflect);
    }

    #[test]
    fn source_stroke_defaults_and_absolute_dash_units_are_preserved() {
        let style = crate::PaintStyle::default();
        assert_eq!(
            (style.stroke_cap, style.stroke_join, style.stroke_miter),
            (PenLineCap::Flat, PenLineJoin::Miter, 0.0)
        );
        let (dashes, offset) = native_dash_pattern(&[6.0, 3.0], 4.0, 2.0).unwrap();
        assert_eq!(dashes, [3.0, 1.5]);
        assert_eq!(offset, 2.0);
        assert!(native_dash_pattern(&[0.0, 0.0], 0.0, 2.0).is_none());
        assert!(native_dash_pattern(&[1.0, f64::NAN], 0.0, 2.0).is_none());
    }
}
