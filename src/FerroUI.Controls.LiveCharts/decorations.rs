use crate::{ChartPointerEventArgs, LegendPosition, TooltipPosition};
use ferroui_base::media::{Color, FlowDirection, FormattedText, Typeface};
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{Point, Rect, Size};
use ferroui_livecharts::{Geometry, PaintStyle, Primitive, Scene, TextAnchor};

pub(crate) fn draw_margin(size: Size, legend: LegendPosition, title: bool) -> Option<Rect> {
    if legend == LegendPosition::Hidden && !title {
        return None;
    }
    let left: f64 = 64.0
        + if legend == LegendPosition::Left {
            150.0
        } else {
            0.0
        };
    let right = 24.0
        + if legend == LegendPosition::Right {
            150.0
        } else {
            0.0
        };
    let top: f64 = 24.0
        + if title { 36.0 } else { 0.0 }
        + if legend == LegendPosition::Top {
            32.0
        } else {
            0.0
        };
    let bottom = 44.0
        + if legend == LegendPosition::Bottom {
            32.0
        } else {
            0.0
        };
    Some(Rect::new(
        left.min(size.width),
        top.min(size.height),
        (size.width - left - right).max(0.0),
        (size.height - top - bottom).max(0.0),
    ))
}

fn text(origin: Point, value: String, size: f64, color: Color) -> Primitive {
    Primitive::new(
        Geometry::Text {
            origin,
            text: value,
            size,
            rotation: 0.0,
            anchor: TextAnchor::Start,
        },
        PaintStyle::fill(color),
    )
}

pub(crate) fn text_size(value: &str, size: f64) -> Size {
    if !size.is_finite() || size <= 0.0 {
        return Size::default();
    }
    let text = FormattedText::new(
        value,
        CultureInfo::invariant_culture(),
        FlowDirection::LeftToRight,
        Typeface::default(),
        size.min(4096.0),
        None,
    );
    Size::new(text.width(), text.height())
}

fn fit_text(value: &str, max_width: f64, size: f64) -> String {
    if text_size(value, size).width <= max_width {
        return value.to_owned();
    }
    let mut chars: Vec<_> = value.chars().collect();
    while !chars.is_empty() {
        chars.pop();
        let candidate = format!("{}…", chars.iter().collect::<String>());
        if text_size(&candidate, size).width <= max_width {
            return candidate;
        }
    }
    String::new()
}

pub(crate) fn decorate(
    scene: &mut Scene,
    title: &str,
    title_size: f64,
    color: Color,
    legend: LegendPosition,
) {
    if !title.is_empty() && title_size.is_finite() && title_size > 0.0 {
        let value = fit_text(title, (scene.size.width - 32.0).max(0.0), title_size);
        scene
            .primitives
            .push(text(Point::new(16.0, 10.0), value, title_size, color));
    }
    if legend == LegendPosition::Hidden {
        return;
    }
    let horizontal = matches!(legend, LegendPosition::Top | LegendPosition::Bottom);
    let mut position = match legend {
        LegendPosition::Top => Point::new(16.0, if title.is_empty() { 8.0 } else { 42.0 }),
        LegendPosition::Bottom => Point::new(16.0, (scene.size.height - 26.0).max(0.0)),
        LegendPosition::Left => Point::new(10.0, scene.plot_bounds.y),
        _ => Point::new((scene.size.width - 140.0).max(0.0), scene.plot_bounds.y),
    };
    for item in &scene.legend {
        let label = if item.name.is_empty() {
            format!("Series {}", item.series + 1)
        } else {
            item.name.clone()
        };
        let width = if horizontal {
            (scene.size.width - position.x - 28.0).max(0.0)
        } else {
            112.0_f64.min((scene.size.width - position.x - 24.0).max(0.0))
        };
        if width < 8.0 || position.y + 18.0 > scene.size.height {
            break;
        }
        let label = fit_text(&label, width, 12.0);
        let measured = text_size(&label, 12.0);
        scene.primitives.push(Primitive::new(
            Geometry::Rectangle {
                rect: Rect::new(position.x, position.y + 3.0, 12.0, 12.0),
                radius: 2.0,
            },
            PaintStyle::fill(item.color),
        ));
        scene.primitives.push(text(
            Point::new(position.x + 20.0, position.y),
            label,
            12.0,
            color,
        ));
        if horizontal {
            position.x += measured.width + 42.0;
        } else {
            position.y += 24.0;
        }
    }
}

pub(crate) fn tooltip_scene(
    size: Size,
    args: &ChartPointerEventArgs,
    placement: TooltipPosition,
    color: Color,
) -> Scene {
    let mut scene = Scene::new(size, Rect::from_size(size));
    let lines: Vec<_> = args
        .points
        .iter()
        .map(|point| {
            if point.label.is_empty() {
                ferroui_livecharts::axis::format_number(point.value)
            } else {
                point.label.clone()
            }
        })
        .collect();
    let width = (lines
        .iter()
        .map(|s| text_size(s, 12.0).width)
        .fold(0.0_f64, f64::max)
        + 24.0)
        .min(size.width.max(0.0));
    let height = (lines.len() as f64 * 20.0 + 16.0).min(size.height.max(0.0));
    let p = args.position;
    let (x, y) = match placement {
        TooltipPosition::Bottom => (p.x - width / 2.0, p.y + 14.0),
        TooltipPosition::Left => (p.x - width - 14.0, p.y - height / 2.0),
        TooltipPosition::Right => (p.x + 14.0, p.y - height / 2.0),
        TooltipPosition::Center => (p.x - width / 2.0, p.y - height / 2.0),
        _ => (
            p.x - width / 2.0,
            if p.y - height - 14.0 >= 0.0 {
                p.y - height - 14.0
            } else {
                p.y + 14.0
            },
        ),
    };
    let origin = Point::new(
        x.clamp(0.0, (size.width - width).max(0.0)),
        y.clamp(0.0, (size.height - height).max(0.0)),
    );
    let rect = Rect::new(origin.x, origin.y, width, height);
    scene.primitives.push(Primitive::new(
        Geometry::Rectangle { rect, radius: 6.0 },
        PaintStyle {
            fill: Some(Color::from_argb(248, 255, 255, 255)),
            stroke: Some(Color::from_rgb(203, 213, 225)),
            stroke_width: 1.0,
            ..PaintStyle::default()
        },
    ));
    for (i, line) in lines.iter().enumerate() {
        let y = origin.y + 8.0 + i as f64 * 20.0;
        if y + 16.0 > origin.y + height {
            break;
        }
        scene.primitives.push(
            text(
                Point::new(origin.x + 12.0, y),
                fit_text(line, (width - 24.0).max(0.0), 12.0),
                12.0,
                color,
            )
            .clipped(rect),
        );
    }
    scene
}

pub(crate) fn error_scene(size: Size, error: &str, color: Color) -> Scene {
    let mut scene = Scene::new(size, Rect::from_size(size));
    scene.primitives.push(
        text(
            Point::new(12.0, 12.0),
            format!("Chart data: {error}"),
            12.0,
            color,
        )
        .clipped(Rect::from_size(size)),
    );
    scene
}
