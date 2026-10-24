//! Retains native drawing resources between frames. There is no renderer-specific API here.

use ferroui_base::media::immutable::{ImmutableDashStyle, ImmutablePen};
use ferroui_base::media::{
    BoxShadows, DrawingContext, FillRule, FlowDirection, FormattedText, Geometry as NativeGeometry,
    IBrush, IPen, StreamGeometry, Typeface,
};
use ferroui_base::platform::IGeometryContext;
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{Matrix, Point, Rect, Ref};
use ferroui_livecharts::paint::{
    brush_or_color, native_dash_pattern, resolve_brush, snapshot_brush,
};
use ferroui_livecharts::{Geometry, PathCommand, Scene, TextAnchor};
use std::rc::Rc;

enum NativeShape {
    Line(Point, Point),
    Rectangle(Rect, f64),
    Ellipse(Rect),
    Path(Ref<NativeGeometry>),
    Text {
        text: FormattedText,
        origin: Point,
        rotation_center: Point,
        rotation: f64,
    },
}

struct NativePrimitive {
    shape: NativeShape,
    fill: Option<Rc<dyn IBrush>>,
    pen: Option<Rc<dyn IPen>>,
    opacity: f64,
    clip: Option<Rect>,
}

/// A chart scene compiled into the selected FerroUI backend's retained resources.
pub struct NativeScene {
    primitives: Vec<NativePrimitive>,
}

impl NativeScene {
    pub fn compile(scene: &Scene) -> Self {
        let mut primitives = Vec::with_capacity(scene.primitives.len());
        for primitive in &scene.primitives {
            let style = &primitive.style;
            let reference =
                style
                    .brush_space
                    .reference_rect(scene.size, scene.plot_bounds, primitive.clip);
            let fill = brush_or_color(&style.fill_brush, style.fill)
                .map(|brush| resolve_brush(&snapshot_brush(&brush), reference));
            let pen: Option<Rc<dyn IPen>> = brush_or_color(&style.stroke_brush, style.stroke)
                .filter(|_| style.stroke_width.is_finite() && style.stroke_width > 0.0)
                .and_then(|brush| {
                    let brush =
                        resolve_brush(&snapshot_brush(&brush), reference).into_immutable_brush()?;
                    let dash =
                        native_dash_pattern(&style.dashes, style.dash_offset, style.stroke_width)
                            .map(|(dashes, offset)| {
                                Rc::new(ImmutableDashStyle::new(Some(&dashes), offset))
                            });
                    let miter = if style.stroke_miter.is_finite() {
                        style.stroke_miter.max(0.0)
                    } else {
                        0.0
                    };
                    Some(Rc::new(ImmutablePen::new(
                        Some(brush),
                        style.stroke_width,
                        dash,
                        style.stroke_cap,
                        style.stroke_join,
                        miter,
                    )) as Rc<dyn IPen>)
                });
            let shape = match &primitive.geometry {
                Geometry::Line { start, end } => NativeShape::Line(*start, *end),
                Geometry::Rectangle { rect, radius } => NativeShape::Rectangle(
                    *rect,
                    if radius.is_finite() {
                        radius.max(0.0)
                    } else {
                        0.0
                    },
                ),
                Geometry::Ellipse(rect) => NativeShape::Ellipse(*rect),
                Geometry::Polyline(points) | Geometry::Polygon(points) => {
                    let Some(first) = points.first() else {
                        continue;
                    };
                    let closed = matches!(primitive.geometry, Geometry::Polygon(_));
                    let path = StreamGeometry::new();
                    {
                        let mut context = path.open();
                        context.set_fill_rule(FillRule::EvenOdd);
                        context.begin_figure(*first, closed && fill.is_some());
                        for point in &points[1..] {
                            context.line_to(*point, true);
                        }
                        context.end_figure(closed);
                    }
                    NativeShape::Path(path.upcast())
                }
                Geometry::Path(commands) => {
                    let path = StreamGeometry::new();
                    {
                        let mut context = path.open();
                        context.set_fill_rule(FillRule::EvenOdd);
                        let mut open = false;
                        for command in commands {
                            match command {
                                PathCommand::MoveTo(point) => {
                                    if open {
                                        context.end_figure(false);
                                    }
                                    context.begin_figure(*point, fill.is_some());
                                    open = true;
                                }
                                PathCommand::LineTo(point) if open => context.line_to(*point, true),
                                PathCommand::CubicTo {
                                    control1,
                                    control2,
                                    end,
                                } if open => {
                                    context.cubic_bezier_to(*control1, *control2, *end, true)
                                }
                                PathCommand::Close if open => {
                                    context.end_figure(true);
                                    open = false;
                                }
                                _ => {}
                            }
                        }
                        if open {
                            context.end_figure(false);
                        }
                    }
                    NativeShape::Path(path.upcast())
                }
                Geometry::Text {
                    origin,
                    text,
                    size,
                    rotation,
                    anchor,
                } => {
                    if text.is_empty() || !size.is_finite() || *size <= 0.0 {
                        continue;
                    }
                    let color = fill
                        .clone()
                        .or_else(|| pen.as_ref().and_then(|p| p.brush()));
                    let text = FormattedText::new(
                        text,
                        CultureInfo::invariant_culture(),
                        FlowDirection::LeftToRight,
                        Typeface::default(),
                        size.min(4096.0),
                        color,
                    );
                    let offset = match anchor {
                        TextAnchor::Start => 0.0,
                        TextAnchor::Center => text.width() * 0.5,
                        TextAnchor::End => text.width(),
                    };
                    NativeShape::Text {
                        text,
                        origin: Point::new(origin.x - offset, origin.y),
                        rotation_center: *origin,
                        rotation: if rotation.is_finite() { *rotation } else { 0.0 },
                    }
                }
            };
            primitives.push(NativePrimitive {
                shape,
                fill,
                pen,
                opacity: if style.opacity.is_finite() {
                    style.opacity.clamp(0.0, 1.0)
                } else {
                    1.0
                },
                clip: primitive.clip,
            });
        }
        Self { primitives }
    }

    pub fn render(&self, context: &mut DrawingContext<'_>) {
        for primitive in &self.primitives {
            if primitive.opacity <= 0.0 {
                continue;
            }
            let clip = primitive.clip.map(|rect| context.push_clip(rect));
            let opacity =
                (primitive.opacity < 1.0).then(|| context.push_opacity(primitive.opacity));
            match &primitive.shape {
                NativeShape::Line(start, end) => {
                    if let Some(pen) = &primitive.pen {
                        context.draw_line(pen, *start, *end);
                    }
                }
                NativeShape::Rectangle(rect, radius) => context.draw_rectangle(
                    primitive.fill.as_ref(),
                    primitive.pen.as_ref(),
                    *rect,
                    *radius,
                    *radius,
                    &BoxShadows::default(),
                ),
                NativeShape::Ellipse(rect) => {
                    context.draw_ellipse(primitive.fill.as_ref(), primitive.pen.as_ref(), *rect)
                }
                NativeShape::Path(path) => {
                    context.draw_geometry(primitive.fill.as_ref(), primitive.pen.as_ref(), path)
                }
                NativeShape::Text {
                    text,
                    origin,
                    rotation_center,
                    rotation,
                } => {
                    let transform = (*rotation != 0.0).then(|| {
                        context.push_transform(Matrix::create_rotation_at(
                            rotation.to_radians(),
                            *rotation_center,
                        ))
                    });
                    context.draw_text(text, *origin);
                    if let Some(transform) = transform {
                        context.pop(transform);
                    }
                }
            }
            if let Some(opacity) = opacity {
                context.pop(opacity);
            }
            if let Some(clip) = clip {
                context.pop(clip);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::media::immutable::{ImmutableGradientStop, ImmutableLinearGradientBrush};
    use ferroui_base::media::{Color, PenLineCap, PenLineJoin};
    use ferroui_base::{RelativePoint, RelativeUnit, Size};
    use ferroui_livecharts::{PaintStyle, Primitive};

    #[test]
    fn native_gradient_and_pen_preserve_reference_and_source_dash_units() {
        let mut scene = Scene::new(Size::new(400.0, 300.0), Rect::new(50.0, 20.0, 200.0, 100.0));
        let brush: Rc<dyn IBrush> = Rc::new(ImmutableLinearGradientBrush::from_stops(&[
            ImmutableGradientStop::new(0.0, Color::from_rgb(255, 0, 0)),
            ImmutableGradientStop::new(1.0, Color::from_rgb(0, 0, 255)),
        ]));
        scene.primitives.push(
            Primitive::new(
                Geometry::Rectangle {
                    rect: Rect::new(60.0, 30.0, 20.0, 20.0),
                    radius: 0.0,
                },
                PaintStyle {
                    fill_brush: Some(brush.clone()),
                    stroke_brush: Some(brush),
                    stroke_width: 2.0,
                    stroke_cap: PenLineCap::Square,
                    stroke_join: PenLineJoin::Bevel,
                    stroke_miter: 7.5,
                    dashes: vec![6.0, 3.0],
                    dash_offset: 4.0,
                    ..PaintStyle::default()
                },
            )
            .clipped(scene.plot_bounds),
        );
        let native = NativeScene::compile(&scene);
        let primitive = &native.primitives[0];
        let fill = primitive
            .fill
            .as_ref()
            .unwrap()
            .as_linear_gradient_brush()
            .unwrap();
        assert_eq!(
            fill.start_point(),
            RelativePoint::new(50.0, 20.0, RelativeUnit::Absolute)
        );
        assert_eq!(
            fill.end_point(),
            RelativePoint::new(250.0, 120.0, RelativeUnit::Absolute)
        );
        let pen = primitive.pen.as_ref().unwrap();
        assert_eq!(
            (pen.line_cap(), pen.line_join(), pen.miter_limit()),
            (PenLineCap::Square, PenLineJoin::Bevel, 7.5)
        );
        let dash = pen.dash_style().unwrap();
        assert_eq!(dash.dashes().unwrap(), [3.0, 1.5]);
        assert_eq!(dash.offset(), 2.0);
        assert!(pen.brush().unwrap().as_linear_gradient_brush().is_some());
    }
}
