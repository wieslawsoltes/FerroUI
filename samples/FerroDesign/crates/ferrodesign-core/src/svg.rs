use crate::*;
use std::collections::BTreeSet;
use std::fmt::Write;

#[path = "svg_regions.rs"]
mod regions;

#[derive(Clone, Debug)]
pub struct SvgExport {
    pub source: String,
    pub warnings: Vec<String>,
}

pub fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn number(value: f64) -> String {
    if !value.is_finite() {
        return "0".into();
    }
    let s = format!("{value:.4}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" || s.is_empty() {
        "0".into()
    } else {
        s.into()
    }
}

fn matrix(t: Transform) -> String {
    format!(
        "matrix({} {} {} {} {} {})",
        number(t.a),
        number(t.b),
        number(t.c),
        number(t.d),
        number(t.e),
        number(t.f)
    )
}

fn blend(mode: BlendMode) -> &'static str {
    match mode {
        BlendMode::Normal | BlendMode::PassThrough => "normal",
        BlendMode::Multiply => "multiply",
        BlendMode::Screen => "screen",
        BlendMode::Overlay => "overlay",
        BlendMode::Darken => "darken",
        BlendMode::Lighten => "lighten",
        BlendMode::ColorDodge => "color-dodge",
        BlendMode::ColorBurn => "color-burn",
        BlendMode::HardLight => "hard-light",
        BlendMode::SoftLight => "soft-light",
        BlendMode::Difference => "difference",
        BlendMode::Exclusion => "exclusion",
        BlendMode::Hue => "hue",
        BlendMode::Saturation => "saturation",
        BlendMode::Color => "color",
        BlendMode::Luminosity => "luminosity",
    }
}

fn paint_color(paint: &Paint, id: &str) -> String {
    match paint {
        Paint::Solid(c) => format!("#{:02X}{:02X}{:02X}", c.r, c.g, c.b),
        _ => format!("url(#{id})"),
    }
}
fn paint_opacity(paint: &Paint, opacity: f64) -> f64 {
    opacity.clamp(0.0, 1.0)
        * match paint {
            Paint::Solid(c) => c.a as f64 / 255.0,
            _ => 1.0,
        }
}

fn paint_definition(paint: &Paint, id: &str, node: &Node, stroke: Option<&Stroke>, out: &mut String) {
    paint_definition_in_bounds(paint, id, node, stroke, None, out);
}

/// `bounds` pins brush coordinates to the original region, before replacement
/// clips remove overlaps. Ordinary SVG paths retain their existing paint API.
fn paint_definition_in_bounds(
    paint: &Paint,
    id: &str,
    node: &Node,
    stroke: Option<&Stroke>,
    bounds: Option<Rect>,
    out: &mut String,
) {
    match paint {
        Paint::Solid(_) => {}
        Paint::LinearGradient { start, end, stops } => {
            let _ = write!(
                out,
                r#"<linearGradient id="{id}" x1="{}" y1="{}" x2="{}" y2="{}">"#,
                number(start.x),
                number(start.y),
                number(end.x),
                number(end.y)
            );
            stops_definition(stops, out);
            out.push_str("</linearGradient>");
        }
        Paint::RadialGradient {
            center,
            radius,
            stops,
        } => {
            let _ = write!(
                out,
                r#"<radialGradient id="{id}" cx="{}" cy="{}" r="{}">"#,
                number(center.x),
                number(center.y),
                number(*radius)
            );
            stops_definition(stops, out);
            out.push_str("</radialGradient>");
        }
        Paint::AngularGradient { center, angle, stops } => {
            let (paint_bounds, tile_bounds) = bounds.map_or_else(
                || angular_paint_bounds(node, stroke),
                |bounds| (bounds, bounds),
            );
            angular_gradient_definition(*center, *angle, stops, id, paint_bounds, tile_bounds, out);
        }
        Paint::Image { href, scale, crop } => {
            let mode = match scale {
                ImageScale::Fill => "xMidYMid slice",
                ImageScale::Fit => "xMidYMid meet",
                _ => "none",
            };
            let target = bounds.unwrap_or_else(|| node.local_bounds());
            let size = crate::embedded_image_size(href)
                .unwrap_or(Point::new(target.width.max(0.001), target.height.max(0.001)));
            let crop = crop.unwrap_or(Rect::new(0.0, 0.0, 1.0, 1.0));
            let source = Rect::new(
                crop.x * size.x,
                crop.y * size.y,
                crop.width * size.x,
                crop.height * size.y,
            );
            let tiled = *scale == ImageScale::Tile;
            let (width, height) = if tiled {
                (source.width, source.height)
            } else {
                (target.width.max(0.001), target.height.max(0.001))
            };
            let viewport = Rect::new(target.x, target.y, width, height);
            let _ = write!(
                out,
                r#"<pattern id="{id}" x="{}" y="{}" width="{}" height="{}" patternUnits="userSpaceOnUse" viewBox="{} {} {} {}" preserveAspectRatio="none"><svg x="{}" y="{}" width="{}" height="{}" viewBox="{} {} {} {}" preserveAspectRatio="{mode}" overflow="hidden"><image href="{}" width="{}" height="{}" preserveAspectRatio="none"/></svg></pattern>"#,
                number(viewport.x),
                number(viewport.y),
                number(width),
                number(height),
                number(viewport.x),
                number(viewport.y),
                number(width),
                number(height),
                number(viewport.x),
                number(viewport.y),
                number(viewport.width),
                number(viewport.height),
                number(source.x),
                number(source.y),
                number(source.width),
                number(source.height),
                escape_xml(&safe_image_href(href)),
                number(size.x),
                number(size.y),
            );
        }
    }
}

fn angular_paint_bounds(node: &Node, stroke: Option<&Stroke>) -> (Rect, Rect) {
    let path = if stroke.is_some() {
        node.vector_network.as_ref().map(|network| network.stroke_path())
            .unwrap_or_else(|| outline_path(node))
    } else {
        outline_path(node)
    };
    let geometry = path.tight_bounds().unwrap_or_else(|| node.local_bounds());
    let Some(stroke) = stroke else { return (geometry, geometry); };
    let alignment = if node.has_closed_stroke_contours() { stroke.align } else { StrokeAlign::Center };
    // FerroUI draws center strokes with relative brush coordinates against the
    // geometry bounds inflated by half the pen width. Aligned strokes are filled
    // outlines: ordinary inside/outside contours use the corresponding bounds.
    let paint_bounds = geometry.inflate(match alignment {
        StrokeAlign::Center => stroke.width / 2.0,
        StrokeAlign::Inside => 0.0,
        StrokeAlign::Outside => stroke.width,
    });
    // A paint server repeats outside its tile. Keep its tile independent of the
    // center's reference bounds and large enough for the renderer's miter limit
    // of ten, caps, and doubled-width alignment. This also gives horizontal and
    // vertical paths a nonzero user-space tile, unlike objectBoundingBox paints.
    let tile_bounds = geometry.inflate(stroke.width * 10.0 + 1.0);
    (paint_bounds, tile_bounds)
}

/// SVG has no portable conic paint server. Export a bounded, explicitly reported
/// vector approximation instead of replacing the paint with a representative
/// color. Shared hard-stop cuts retain their exact angular boundary. Crisp
/// internal edges prevent antialiasing gaps without overlapping translucent
/// sectors; the outer layer geometry still supplies normal antialiasing.
fn angular_gradient_definition(
    center: Point, angle: f64, stops: &[GradientStop], id: &str,
    paint_bounds: Rect, tile_bounds: Rect, out: &mut String,
) {
    const MAX_SECTORS: usize = 1024;
    let mut stops: Vec<_> = stops.iter().filter(|stop| stop.offset.is_finite()).cloned().collect();
    stops.sort_by(|a, b| a.offset.total_cmp(&b.offset));
    if stops.is_empty() { return; }
    let divisions = if stops.len() <= MAX_SECTORS / 2 { MAX_SECTORS / 2 } else { MAX_SECTORS };
    let mut cuts: Vec<_> = (0..=divisions).map(|i| i as f64 / divisions as f64).collect();
    if stops.len() <= MAX_SECTORS / 2 {
        cuts.extend(stops.iter().map(|stop| stop.offset.clamp(0.0, 1.0)));
        cuts.sort_by(f64::total_cmp);
        cuts.dedup();
    }
    let center = Point::new(paint_bounds.x + center.x * paint_bounds.width,
        paint_bounds.y + center.y * paint_bounds.height);
    let radius = tile_bounds.corners()
        .iter().map(|corner| (corner.x - center.x).hypot(corner.y - center.y))
        .fold(0.0, f64::max) * 1.01;
    let _ = write!(out,
        r#"<pattern id="{id}" x="{}" y="{}" width="{}" height="{}" patternUnits="userSpaceOnUse" viewBox="{} {} {} {}" preserveAspectRatio="none" data-ferrodesign-approximation="angular-gradient"><g shape-rendering="crispEdges">"#,
        number(tile_bounds.x), number(tile_bounds.y), number(tile_bounds.width.max(0.001)), number(tile_bounds.height.max(0.001)),
        number(tile_bounds.x), number(tile_bounds.y), number(tile_bounds.width.max(0.001)), number(tile_bounds.height.max(0.001)));
    let angle = angle.rem_euclid(360.0).to_radians() - std::f64::consts::FRAC_PI_2;
    let mut next_stop = 0;
    for interval in cuts.windows(2) {
        let position = (interval[0] + interval[1]) * 0.5;
        while next_stop < stops.len() && stops[next_stop].offset <= position { next_stop += 1; }
        let color = if next_stop == 0 {
            stops[0].color
        } else if next_stop == stops.len() {
            stops[next_stop - 1].color
        } else {
            let a = &stops[next_stop - 1];
            let b = &stops[next_stop];
            a.color.lerp(b.color, (position - a.offset) / (b.offset - a.offset))
        };
        let point = |offset: f64| {
            let radians = angle + std::f64::consts::TAU * offset;
            Point::new(center.x + radius * radians.cos(), center.y + radius * radians.sin())
        };
        let a = point(interval[0]);
        let b = point(interval[1]);
        let _ = write!(out,
            r##"<path d="M{} {} L{} {} L{} {} Z" fill="#{:02X}{:02X}{:02X}" fill-opacity="{}"/>"##,
            number(center.x), number(center.y), number(a.x), number(a.y), number(b.x), number(b.y),
            color.r, color.g, color.b, number(color.a as f64 / 255.0));
    }
    out.push_str("</g></pattern>");
}

fn safe_image_href(href: &str) -> String {
    let lower = href.trim().to_ascii_lowercase();
    if lower.starts_with("https://")
        || lower.starts_with("http://")
        || lower.starts_with("data:image/png;")
        || lower.starts_with("data:image/jpeg;")
        || lower.starts_with("data:image/webp;")
        || lower.starts_with("data:image/gif;")
    {
        href.trim().into()
    } else {
        String::new()
    }
}
fn stops_definition(stops: &[GradientStop], out: &mut String) {
    let mut sorted = stops.to_vec();
    sorted.sort_by(|a, b| a.offset.total_cmp(&b.offset));
    for stop in sorted {
        let c = stop.color;
        let _ = write!(
            out,
            r##"<stop offset="{}" stop-color="#{:02X}{:02X}{:02X}" stop-opacity="{}"/>"##,
            number(stop.offset.clamp(0.0, 1.0)),
            c.r,
            c.g,
            c.b,
            number(c.a as f64 / 255.0)
        );
    }
}

fn path_data(path: &VectorPath, t: Transform) -> String {
    let mut out = String::new();
    for segment in &path.segments {
        match *segment {
            PathSegment::MoveTo { point } => {
                let p = t.apply(point);
                let _ = write!(out, "M{} {} ", number(p.x), number(p.y));
            }
            PathSegment::LineTo { point } => {
                let p = t.apply(point);
                let _ = write!(out, "L{} {} ", number(p.x), number(p.y));
            }
            PathSegment::QuadraticTo { control, point } => {
                let c = t.apply(control);
                let p = t.apply(point);
                let _ = write!(
                    out,
                    "Q{} {} {} {} ",
                    number(c.x),
                    number(c.y),
                    number(p.x),
                    number(p.y)
                );
            }
            PathSegment::CubicTo {
                control1,
                control2,
                point,
            } => {
                let a = t.apply(control1);
                let b = t.apply(control2);
                let p = t.apply(point);
                let _ = write!(
                    out,
                    "C{} {} {} {} {} {} ",
                    number(a.x),
                    number(a.y),
                    number(b.x),
                    number(b.y),
                    number(p.x),
                    number(p.y)
                );
            }
            PathSegment::Close => out.push_str("Z "),
        }
    }
    out
}

pub fn outline_path(node: &Node) -> VectorPath {
    if node.kind == NodeKind::Vector {
        return node
            .vector_network
            .as_ref()
            .map(|network| network.fill_path())
            .unwrap_or_else(|| node.path.clone());
    }
    if node.kind == NodeKind::Polygon || node.kind == NodeKind::Star {
        return crate::polygon_corner_path(node);
    }
    if node.kind == NodeKind::Line {
        return VectorPath::polygon(
            &[Point::default(), Point::new(node.width, node.height)],
            false,
        );
    }
    let w = node.width;
    let h = node.height;
    let k = 0.552_284_749_830_793_6;
    if node.kind == NodeKind::Ellipse {
        let rx = w / 2.0;
        let ry = h / 2.0;
        return VectorPath {
            fill_rule: FillRule::NonZero,
            anchor_modes: Default::default(),
            segments: vec![
                PathSegment::MoveTo {
                    point: Point::new(w, ry),
                },
                PathSegment::CubicTo {
                    control1: Point::new(w, ry + k * ry),
                    control2: Point::new(rx + k * rx, h),
                    point: Point::new(rx, h),
                },
                PathSegment::CubicTo {
                    control1: Point::new(rx - k * rx, h),
                    control2: Point::new(0.0, ry + k * ry),
                    point: Point::new(0.0, ry),
                },
                PathSegment::CubicTo {
                    control1: Point::new(0.0, ry - k * ry),
                    control2: Point::new(rx - k * rx, 0.0),
                    point: Point::new(rx, 0.0),
                },
                PathSegment::CubicTo {
                    control1: Point::new(rx + k * rx, 0.0),
                    control2: Point::new(w, ry - k * ry),
                    point: Point::new(w, ry),
                },
                PathSegment::Close,
            ],
        };
    }
    crate::corner_path(node)
}

fn node_shape(node: &Node, attributes: &str) -> String {
    match node.kind {
        NodeKind::Text => text_shape(node, attributes),
        _ => format!(
            r#"<path d="{}" fill-rule="{}" {attributes}/>"#,
            path_data(&outline_path(node), Transform::IDENTITY),
            if node
                .vector_network
                .as_ref()
                .map(|g| g.fill_rule)
                .unwrap_or(node.path.fill_rule)
                == FillRule::EvenOdd
            {
                "evenodd"
            } else {
                "nonzero"
            }
        ),
    }
}

fn stroke_shape(node: &Node, attributes: &str) -> String {
    if let Some(network) = &node.vector_network {
        format!(
            r#"<path d="{}" {attributes}/>"#,
            path_data(&network.stroke_path(), Transform::IDENTITY)
        )
    } else {
        node_shape(node, attributes)
    }
}

fn variable_stroke_shape(node: &Node, index: usize, stroke: &Stroke, defs: &mut String,
    body: &mut String, warnings: &mut Vec<String>)
{
    let path = match crate::variable_width_outline(node, stroke) {
        Ok(path) => path,
        Err(error) => {
            warnings.push(format!("Layer {}: variable-width stroke {} could not be outlined: {error}", node.name, index + 1));
            return;
        }
    };
    let Some(bounds) = path.tight_bounds() else { return; };
    let id = node.id;
    let paint_id = format!("stroke-{id}-{index}");
    // The native renderer clips the band after assigning its brush, so all
    // exported paint coordinates use the same uncut band bounds.
    paint_definition_in_bounds(&stroke.paint, &paint_id, node, Some(stroke), Some(bounds), defs);
    let painted = format!(r#"<path d="{}" fill-rule="nonzero" fill="{}" fill-opacity="{}" stroke="none"/>"#,
        path_data(&path, Transform::IDENTITY), paint_color(&stroke.paint, &paint_id),
        number(paint_opacity(&stroke.paint, stroke.opacity)));
    let alignment = if node.has_closed_stroke_contours() { stroke.align } else { StrokeAlign::Center };
    match alignment {
        StrokeAlign::Center => body.push_str(&painted),
        StrokeAlign::Inside => {
            let _ = write!(defs, r#"<clipPath id="width-clip-{id}-{index}">{}</clipPath>"#,
                node_shape(node, r#"fill="black" stroke="none""#));
            let _ = write!(body, r#"<g clip-path="url(#width-clip-{id}-{index})">{painted}</g>"#);
        }
        StrokeAlign::Outside => {
            let b = bounds.inflate(1.0);
            let _ = write!(defs,
                r#"<mask id="width-mask-{id}-{index}" maskUnits="userSpaceOnUse" x="{}" y="{}" width="{}" height="{}"><rect x="{}" y="{}" width="{}" height="{}" fill="white"/>{}</mask>"#,
                number(b.x), number(b.y), number(b.width), number(b.height),
                number(b.x), number(b.y), number(b.width), number(b.height),
                node_shape(node, r#"fill="black" stroke="none""#));
            let _ = write!(body, r#"<g mask="url(#width-mask-{id}-{index})">{painted}</g>"#);
        }
    }
}

/// Bounds of the same filled bands written by variable_stroke_shape. A base
/// width or even a fixed miter multiplier cannot bound arbitrary custom width
/// multipliers. Keep an aligned band's uncut bounds, as the native renderer
/// does; clipping can reduce its ink but must never introduce a new crop.
fn variable_stroke_bounds(node: &Node) -> Option<Rect> {
    node.strokes.iter()
        .filter(|stroke| stroke.visible && stroke.opacity > 0.0 && stroke.width > 0.0
            && !stroke.width_profile.is_uniform())
        .filter_map(|stroke| crate::variable_width_outline(node, stroke).ok()?.tight_bounds())
        .reduce(Rect::union)
}

fn text_shape(node: &Node, attributes: &str) -> String {
    // This renderer-independent export retains editable hard paragraphs. Native
    // wrapping, glyph baselines and Auto's logical indentation require the real
    // TextLayout; export_svg_report advertises that fidelity boundary.
    let paragraphs = crate::text_paragraph_ranges(&node.text);
    let styles: std::collections::BTreeMap<_, _> = node.text_paragraphs.iter().map(|style| (style.start, style)).collect();
    let metrics = |start| {
        let style = styles.get(&start).copied();
        let line_height = node.font_size * style.and_then(|style| style.line_height).unwrap_or(node.line_height);
        let before = style.and_then(|style| style.space_before).unwrap_or(0.0);
        let after = style.and_then(|style| style.space_after).unwrap_or(0.0);
        (style, line_height, before, after)
    };
    let text_height: f64 = paragraphs.iter().map(|range| {
        let (_, line, before, after) = metrics(range.start); line + before + after
    }).sum();
    let top = match node.text_vertical_align {
        TextVerticalAlign::Top => 0.0,
        TextVerticalAlign::Center => (node.height - text_height).max(0.0) / 2.0,
        TextVerticalAlign::Bottom => (node.height - text_height).max(0.0),
    };
    let decoration = match node.text_decoration {
        TextDecoration::None => "none",
        TextDecoration::Underline => "underline",
        TextDecoration::Strikethrough => "line-through",
    };
    let mut out = format!(
        r#"<text font-family="{}" font-size="{}" font-weight="{}" font-style="{}" letter-spacing="{}" text-decoration="{decoration}" xml:space="preserve" {attributes}>"#,
        escape_xml(&node.font_family), number(node.font_size), node.font_weight,
        if node.italic { "italic" } else { "normal" }, number(node.letter_spacing)
    );
    let mut y = top;
    for paragraph in paragraphs {
        let start = paragraph.start;
        let (style, line_height, before, after) = metrics(start);
        let alignment = style.and_then(|style| style.alignment).unwrap_or(node.text_align);
        let direction = style.and_then(|style| style.direction).or(node.text_direction).unwrap_or(TextDirection::LeftToRight);
        let rtl = direction == TextDirection::RightToLeft;
        let indent = style.and_then(|style| style.indent).unwrap_or(0.0).min(node.width.max(0.0));
        let (left, right) = if rtl { (0.0, node.width - indent) } else { (indent, node.width) };
        let (x, anchor) = match alignment {
            TextAlign::Center => ((left + right) / 2.0, "middle"),
            TextAlign::Right => (right, if rtl { "start" } else { "end" }),
            TextAlign::Justify if rtl => (right, "start"),
            _ => (left, if rtl { "end" } else { "start" }),
        };
        let content = node.text[paragraph.clone()].trim_end_matches(['\r','\n','\u{0085}','\u{2028}','\u{2029}']);
        let end = start + content.len();
        y += before;
        let _ = write!(out,
            r#"<tspan x="{}" y="{}" text-anchor="{anchor}" direction="{}" unicode-bidi="{}" letter-spacing="{}" data-paragraph-start="{start}" data-paragraph-end="{}" data-paragraph-direction="{}" data-line-height="{}" data-space-before="{}" data-space-after="{}" data-first-line-indent="{}""#,
            number(x), number(y + node.font_size), if rtl { "rtl" } else { "ltr" },
            if direction == TextDirection::Auto { "plaintext" } else { "embed" },
            number(style.and_then(|style| style.letter_spacing).unwrap_or(node.letter_spacing)), paragraph.end,
            match direction { TextDirection::Auto => "auto", TextDirection::LeftToRight => "ltr", TextDirection::RightToLeft => "rtl" },
            number(line_height), number(before), number(after), number(indent));
        if let Some(style) = style {
            let _ = write!(out, r#" data-ferrodesign-paragraph="{}""#,
                escape_xml(&serde_json::to_string(style).expect("validated paragraph style")));
        }
        out.push('>');
        let mut boundaries = vec![start, end];
        for run in &node.text_runs {
            if run.start < end && run.end > start {
                boundaries.push(run.start.max(start));
                boundaries.push(run.end.min(end));
            }
        }
        boundaries.sort_unstable();
        boundaries.dedup();
        if boundaries.len() == 1 {
            out.push_str("&#8203;")
        }
        for pair in boundaries.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            if !node.text.is_char_boundary(a) || !node.text.is_char_boundary(b) {
                continue;
            }
            let run = node
                .text_runs
                .iter()
                .rev()
                .find(|run| run.start <= a && run.end >= b);
            if let Some(run) = run {
                out.push_str("<tspan");
                if let Some(family) = &run.font_family {
                    let _ = write!(out, r#" font-family="{}""#, escape_xml(family));
                }
                if let Some(size) = run.font_size {
                    let _ = write!(out, r#" font-size="{}""#, number(size));
                }
                if let Some(weight) = run.font_weight {
                    let _ = write!(out, r#" font-weight="{weight}""#);
                }
                if let Some(c) = run.color {
                    let _ = write!(
                        out,
                        r##" fill="#{:02X}{:02X}{:02X}" fill-opacity="{}""##,
                        c.r,
                        c.g,
                        c.b,
                        number(c.a as f64 / 255.0)
                    );
                }
                if let Some(italic) = run.italic {
                    let _ = write!(
                        out,
                        r#" font-style="{}""#,
                        if italic { "italic" } else { "normal" }
                    );
                }
                if let Some(decoration) = run.decoration {
                    let _ = write!(
                        out,
                        r#" text-decoration="{}""#,
                        match decoration {
                            TextDecoration::None => "none",
                            TextDecoration::Underline => "underline",
                            TextDecoration::Strikethrough => "line-through",
                        }
                    );
                }
                out.push('>');
                out.push_str(&escape_xml(&node.text[a..b]));
                out.push_str("</tspan>");
            } else {
                out.push_str(&escape_xml(&node.text[a..b]));
            }
        }
        out.push_str("</tspan>");
        y += line_height + after;
    }
    out.push_str("</text>");
    out
}

#[cfg(test)]
#[path = "svg_paragraph_tests.rs"]
mod paragraph_tests;
#[cfg(test)]
#[path = "stroke_export_tests.rs"]
mod stroke_export_tests;

/// Retain selected paint subtrees, their ancestor transforms/clips, and every
/// preceding mask needed by those subtrees. This also keeps document paint order
/// when selection order differs from layer order.
fn selection_projection(
    document: &Document,
    page: NodeId,
    selected: &BTreeSet<NodeId>,
) -> Document {
    fn retain(
        document: &Document,
        children: &[NodeId],
        selected: &BTreeSet<NodeId>,
        kept: &mut BTreeSet<NodeId>,
        painted: &mut BTreeSet<NodeId>,
        whole: bool,
    ) -> bool {
        let mut masks = Vec::new();
        let mut any = false;
        for id in children {
            let Some(node) = document.nodes.get(id) else {
                continue;
            };
            let whole = whole || selected.contains(id);
            let descendant = retain(document, &node.children, selected, kept, painted, whole);
            if node.is_mask && node.visible {
                masks.push(*id);
            }
            if whole || descendant {
                kept.insert(*id);
                if whole {
                    painted.insert(*id);
                }
                for mask in &masks {
                    let descendants = document.descendants(*mask);
                    kept.extend(descendants.iter().copied());
                    painted.extend(descendants);
                }
                any = true;
            }
        }
        any
    }
    let mut kept = BTreeSet::new();
    let mut painted = BTreeSet::new();
    retain(
        document,
        document.children(page).unwrap_or_default(),
        selected,
        &mut kept,
        &mut painted,
        false,
    );
    let mut projected = document.clone();
    projected.nodes.retain(|id, _| kept.contains(id));
    for node in projected.nodes.values_mut() {
        node.children.retain(|id| kept.contains(id));
        if !painted.contains(&node.id) {
            node.fills.clear();
            node.strokes.clear();
        }
    }
    for page in &mut projected.pages {
        page.children.retain(|id| kept.contains(id));
    }
    projected
}

impl Document {
    pub fn export_svg(&self, page: NodeId, selection: &[NodeId]) -> String {
        self.export_svg_report(page, selection).source
    }
    pub fn export_svg_report(&self, page: NodeId, selection: &[NodeId]) -> SvgExport {
        let selection_set: BTreeSet<_> = selection.iter().copied().collect();
        let mut roots: Vec<_> = if selection.is_empty() {
            self.page(page)
                .map(|p| p.children.clone())
                .unwrap_or_default()
        } else {
            selection
                .iter()
                .filter(|id| {
                    self.nodes.contains_key(id)
                        && !selection_set
                            .iter()
                            .any(|other| other != *id && self.is_ancestor(*other, **id))
                })
                .copied()
                .collect()
        };
        roots.sort_by_key(|id| {
            let mut order = Vec::new();
            let mut current = *id;
            for _ in 0..MAX_DOCUMENT_DEPTH {
                let Some(parent) = self.nodes.get(&current).and_then(|n| n.parent) else {
                    break;
                };
                order.push(
                    self.children(parent)
                        .unwrap_or_default()
                        .iter()
                        .position(|id| *id == current)
                        .unwrap_or(0),
                );
                current = parent;
            }
            order.reverse();
            order
        });
        roots.dedup();
        let bounds = roots
            .iter()
            .filter_map(|id| self.world_transform(*id)
                .map(|transform| transform.transform_rect(self.svg_export_bounds(*id, 0))))
            .reduce(|a, b| a.union(b))
            .unwrap_or(Rect::new(0.0, 0.0, 1.0, 1.0));
        let mut defs = String::new();
        let mut body = String::new();
        let mut warnings = Vec::new();
        let needs_mask_context = roots.iter().any(|id| {
            let mut current = *id;
            for _ in 0..MAX_DOCUMENT_DEPTH {
                let Some(parent) = self.nodes.get(&current).and_then(|n| n.parent) else {
                    break;
                };
                if self
                    .children(parent)
                    .unwrap_or_default()
                    .iter()
                    .take_while(|id| **id != current)
                    .any(|id| self.nodes.get(id).is_some_and(|n| n.visible && n.is_mask))
                {
                    return true;
                }
                current = parent;
            }
            false
        });
        let projected;
        let document = if selection.is_empty() || !needs_mask_context {
            self
        } else {
            projected = selection_projection(self, page, &selection_set);
            &projected
        };
        document.export_svg_siblings(
            if selection.is_empty() || needs_mask_context {
                document.children(page).unwrap_or_default()
            } else {
                &roots
            },
            true,
            &mut defs,
            &mut body,
            &mut warnings,
            0,
            false,
        );
        let source = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="{}" height="{}" viewBox="{} {} {} {}" role="img" aria-label="{}"><title>{}</title><defs>{}</defs>{}</svg>"#,
            number(bounds.width.max(1.0)),
            number(bounds.height.max(1.0)),
            number(bounds.x),
            number(bounds.y),
            number(bounds.width.max(1.0)),
            number(bounds.height.max(1.0)),
            escape_xml(&self.name),
            escape_xml(&self.name),
            defs,
            body
        );
        SvgExport { source, warnings }
    }

    /// Preserve the authored viewport for ordinary contained artwork, while
    /// including actual variable-width bands and unclipped child overflow.
    /// The broader uniform-pen/effect margins used for mask allocation below
    /// must not add five stroke widths of padding to every exported frame.
    fn svg_export_bounds(&self, id: NodeId, depth: usize) -> Rect {
        let Some(node) = self.nodes.get(&id) else { return Rect::default(); };
        let mut bounds = node.local_bounds();
        if depth >= MAX_DOCUMENT_DEPTH { return bounds; }
        if let Some(band) = variable_stroke_bounds(node) { bounds = bounds.union(band); }
        for child in &node.children {
            if let Some(child_node) = self.nodes.get(child).filter(|node| node.visible) {
                let child = child_node.local_transform()
                    .transform_rect(self.svg_export_bounds(*child, depth + 1));
                let child = if node.clip_content { child.intersection(node.local_bounds()) } else { Some(child) };
                if let Some(child) = child { bounds = bounds.union(child); }
            }
        }
        bounds
    }

    /// Conservative ink bounds in the node's own coordinates, including
    /// overflowing descendants. A mask region must not impose a new crop.
    fn svg_ink_bounds(&self, id: NodeId, depth: usize) -> Rect {
        let Some(node) = self.nodes.get(&id) else {
            return Rect::default();
        };
        if depth >= MAX_DOCUMENT_DEPTH {
            return node.local_bounds();
        }
        let stroke = node
            .strokes
            .iter()
            .filter(|s| s.visible && s.width_profile.is_uniform())
            .map(|s| s.width * 5.0)
            .fold(0.0, f64::max);
        let effect = node
            .effects
            .iter()
            .filter(|e| e.visible)
            .map(|e| e.radius * 3.0 + e.spread.abs() + e.offset.x.abs().max(e.offset.y.abs()))
            .sum::<f64>();
        let mut bounds = node.local_bounds().inflate(stroke);
        if let Some(band) = variable_stroke_bounds(node) { bounds = bounds.union(band); }
        for child in &node.children {
            if let Some(child_node) = self.nodes.get(child).filter(|n| n.visible) {
                let child = child_node.local_transform()
                    .transform_rect(self.svg_ink_bounds(*child, depth + 1));
                let child = if node.clip_content { child.intersection(node.local_bounds()) } else { Some(child) };
                if let Some(child) = child { bounds = bounds.union(child); }
            }
        }
        bounds.inflate(effect)
    }

    fn svg_blends_with_parent(&self, id: NodeId, depth: usize) -> bool {
        let Some(node) = self.nodes.get(&id).filter(|n| n.visible) else {
            return false;
        };
        if depth >= MAX_DOCUMENT_DEPTH {
            return false;
        }
        match node.blend_mode {
            BlendMode::Normal => false,
            BlendMode::PassThrough => node
                .children
                .iter()
                .any(|id| self.svg_blends_with_parent(*id, depth + 1)),
            _ => true,
        }
    }

    fn svg_has_region_paint_overrides(&self, id: NodeId, depth: usize) -> bool {
        let Some(node) = self.nodes.get(&id).filter(|node| node.visible) else { return false; };
        if depth >= MAX_DOCUMENT_DEPTH { return true; }
        node.vector_network.as_ref().is_some_and(|network| network.regions.values().any(|region| region.fills.is_some()))
            || node.children.iter().any(|child| self.svg_has_region_paint_overrides(*child, depth + 1))
    }

    fn export_svg_siblings(
        &self,
        children: &[NodeId],
        root: bool,
        defs: &mut String,
        body: &mut String,
        warnings: &mut Vec<String>,
        depth: usize,
        vector_mask: bool,
    ) {
        // A list without a visible mask never uses this allocation region.
        // In particular, do not retessellate every descendant's variable-width
        // bands at each ordinary frame level just to discard the bounds.
        let region = if children.iter().any(|id| self.nodes.get(id)
            .is_some_and(|node| node.visible && node.is_mask))
        {
            children.iter().filter_map(|id| {
                self.nodes.get(id).filter(|n| n.visible).map(|n| {
                    let transform = if root {
                        self.world_transform(*id).unwrap_or_default()
                    } else {
                        n.local_transform()
                    };
                    transform.transform_rect(self.svg_ink_bounds(*id, depth))
                })
            }).reduce(|a, b| a.union(b))
                .unwrap_or(Rect::new(0.0, 0.0, 1.0, 1.0)).inflate(1.0)
        } else {
            Rect::default()
        };
        let mut opened = 0;
        for id in children {
            let Some(node) = self.nodes.get(id).filter(|n| n.visible) else {
                continue;
            };
            if node.is_mask {
                let geometry_only = vector_mask || node.mask_type == MaskType::Vector;
                let kind = if geometry_only || node.mask_type == MaskType::Alpha {
                    "alpha"
                } else {
                    "luminance"
                };
                let mut source = String::new();
                self.export_svg_node(*id, root, defs, &mut source, warnings, depth, geometry_only);
                let _ = write!(
                    defs,
                    r#"<mask id="layer-mask-{id}" maskUnits="userSpaceOnUse" maskContentUnits="userSpaceOnUse" x="{}" y="{}" width="{}" height="{}" mask-type="{kind}" color-interpolation="sRGB"{}>{source}</mask>"#,
                    number(region.x),
                    number(region.y),
                    number(region.width.max(0.0)),
                    number(region.height.max(0.0)),
                    // Standard alpha coverage preserves internal replacement
                    // masks on reimport. A geometry-only role would treat their
                    // black cutters as positive vector geometry.
                    if geometry_only && !self.svg_has_region_paint_overrides(*id, depth) {
                        r#" data-ferro-mask-type="vector""#
                    } else {
                        ""
                    }
                );
                let _ = write!(body, r#"<g mask="url(#layer-mask-{id})">"#);
                opened += 1;
            } else {
                if opened > 0 && !vector_mask && self.svg_blends_with_parent(*id, depth) {
                    warnings.push(format!("Layer {}: SVG masks isolate the masked sibling group; blending through that scope with an external backdrop cannot be preserved", node.name));
                }
                self.export_svg_node(*id, root, defs, body, warnings, depth, vector_mask);
            }
        }
        for _ in 0..opened {
            body.push_str("</g>");
        }
    }

    fn export_svg_node(
        &self,
        id: NodeId,
        root: bool,
        defs: &mut String,
        body: &mut String,
        warnings: &mut Vec<String>,
        depth: usize,
        vector_mask: bool,
    ) {
        if depth >= MAX_DOCUMENT_DEPTH {
            return;
        }
        let Some(node) = self.nodes.get(&id) else {
            return;
        };
        if !node.visible || (vector_mask && node.opacity <= 0.0) {
            return;
        }
        let mut geometry;
        let node = if vector_mask {
            geometry = node.clone();
            geometry.opacity = 1.0;
            geometry.blend_mode = BlendMode::PassThrough;
            geometry.effects.clear();
            for fill in &mut geometry.fills {
                regions::whiten_vector_fill(fill);
            }
            if let Some(network) = &mut geometry.vector_network {
                for fills in network.regions.values_mut().filter_map(|region| region.fills.as_mut()) {
                    for fill in fills {
                        regions::whiten_vector_fill(fill);
                    }
                }
            }
            for stroke in &mut geometry.strokes {
                stroke.visible &= stroke.opacity > 0.0 && regions::paint_has_alpha(&stroke.paint);
                stroke.paint = Paint::Solid(Color::WHITE);
                stroke.opacity = 1.0;
            }
            for run in &mut geometry.text_runs {
                run.color = Some(Color::WHITE);
            }
            &geometry
        } else {
            node
        };
        if node.fills.iter().chain(node.vector_network.iter().flat_map(|network| {
            network.regions.values().filter_map(|region| region.fills.as_ref()).flatten()
        }))
            .any(|fill| fill.visible && matches!(fill.paint, Paint::AngularGradient { .. }))
            || node.strokes.iter().any(|stroke| stroke.visible && matches!(stroke.paint, Paint::AngularGradient { .. }))
        {
            warnings.push(format!("Layer {}: SVG has no portable angular-gradient primitive; the gradient is approximated by at most 1024 vector sectors per paint. JSON and PNG retain the native angular paint. Complex aligned strokes, arrows, and text may differ in paint bounds.", node.name));
        }
        if node.kind == NodeKind::Text && (!node.text_paragraphs.is_empty()
            || node.text_direction.is_some() || node.text_auto_resize != TextAutoResize::WidthAndHeight) {
            warnings.push(format!("Layer {}: editable SVG preserves hard paragraph breaks, direction and authored spacing, but does not reproduce native wrapping, font baselines, justification or Auto-direction logical indentation exactly. Use the native PNG or glyph outlines for pixel-faithful output; JSON retains every paragraph field.", node.name));
        }
        let transform = if root {
            self.world_transform(id).unwrap_or_default()
        } else {
            node.local_transform()
        };
        let _ = write!(
            body,
            r#"<g id="node-{id}" data-name="{}" transform="{}" opacity="{}" style="mix-blend-mode:{};isolation:{}">"#,
            escape_xml(&node.name),
            matrix(transform),
            number(node.opacity),
            blend(node.blend_mode),
            if node.blend_mode == BlendMode::PassThrough {
                "auto"
            } else {
                "isolate"
            }
        );
        let _ = write!(body, "<title>{}</title>", escape_xml(&node.name));
        if node.blend_mode == BlendMode::PassThrough
            && node.opacity < 1.0
            && node
                .children
                .iter()
                .any(|id| self.svg_blends_with_parent(*id, depth + 1))
        {
            warnings.push(format!("Layer {}: SVG opacity isolates this pass-through group; external backdrop blending may differ", node.name));
        }
        let effects: Vec<_> = node.effects.iter().filter(|e| e.visible).collect();
        let has_filter = effects.iter().any(|e| e.kind != EffectKind::BackgroundBlur);
        if has_filter {
            let _ = write!(
                defs,
                r#"<filter id="effects-{id}" x="-100%" y="-100%" width="300%" height="300%" color-interpolation-filters="sRGB">"#
            );
            for (i, effect) in effects.iter().enumerate() {
                let c = effect.color;
                match effect.kind {
                    EffectKind::DropShadow => {
                        let _ = write!(
                            defs,
                            r##"<feDropShadow dx="{}" dy="{}" stdDeviation="{}" flood-color="#{:02X}{:02X}{:02X}" flood-opacity="{}"/>"##,
                            number(effect.offset.x),
                            number(effect.offset.y),
                            number(effect.radius / 2.0),
                            c.r,
                            c.g,
                            c.b,
                            number(c.a as f64 / 255.0)
                        );
                    }
                    EffectKind::InnerShadow => {
                        let _ = write!(
                            defs,
                            r##"<feGaussianBlur in="SourceAlpha" stdDeviation="{}" result="blur-{id}-{i}"/><feOffset dx="{}" dy="{}" result="offset-{id}-{i}"/><feComposite in="SourceAlpha" in2="offset-{id}-{i}" operator="out" result="inner-{id}-{i}"/><feFlood flood-color="#{:02X}{:02X}{:02X}" flood-opacity="{}"/><feComposite in2="inner-{id}-{i}" operator="in"/><feComposite in2="SourceGraphic" operator="over"/>"##,
                            number(effect.radius / 2.0),
                            number(effect.offset.x),
                            number(effect.offset.y),
                            c.r,
                            c.g,
                            c.b,
                            number(c.a as f64 / 255.0)
                        );
                    }
                    EffectKind::LayerBlur => {
                        let _ = write!(
                            defs,
                            r#"<feGaussianBlur stdDeviation="{}"/>"#,
                            number(effect.radius / 2.0)
                        );
                    }
                    EffectKind::BackgroundBlur => {}
                }
            }
            defs.push_str("</filter>");
            let _ = write!(body, r#"<g filter="url(#effects-{id})">"#);
        }
        if effects.iter().any(|e| e.kind == EffectKind::BackgroundBlur) {
            warnings.push(format!("Layer {}: background blur needs a composited backdrop; exported without backdrop blur",node.name));
        }
        let shape = node_shape(node, "");
        if node.clip_content {
            let _ = write!(defs, r#"<clipPath id="clip-{id}">{shape}</clipPath>"#);
        }
        if node.kind == NodeKind::Boolean {
            // A Boolean's operands form its own derived geometry, which the
            // native renderer intersects with the node's clip before painting.
            if node.clip_content { let _ = write!(body, r#"<g clip-path="url(#clip-{id})">"#); }
            self.export_boolean(node, defs, body, warnings);
            if node.clip_content { body.push_str("</g>"); }
        } else {
            if let Some(network) = &node.vector_network {
                regions::export_network_fills(node, network, defs, body);
            } else {
                for (i, fill) in node.fills.iter().enumerate().rev().filter(|(_, f)| f.visible) {
                    let paint_id = format!("fill-{id}-{i}");
                    paint_definition(&fill.paint, &paint_id, node, None, defs);
                    body.push_str(&node_shape(
                        node,
                        &format!(
                            r#"fill="{}" fill-opacity="{}" stroke="none""#,
                            paint_color(&fill.paint, &paint_id),
                            number(paint_opacity(&fill.paint, fill.opacity))
                        ),
                    ));
                }
            }
            if node.kind == NodeKind::Text && node.fills.is_empty() {
                body.push_str(&node_shape(node, r#"fill="none""#));
            }
            // Match native push_content_clip: descendants are clipped, while
            // this node's own outside/center stroke remains outside the clip.
            if node.clip_content { let _ = write!(body, r#"<g clip-path="url(#clip-{id})">"#); }
            self.export_svg_siblings(
                &node.children,
                false,
                defs,
                body,
                warnings,
                depth + 1,
                vector_mask,
            );
            if node.clip_content { body.push_str("</g>"); }
            for (i, stroke) in node
                .strokes
                .iter()
                .enumerate()
                .rev()
                .filter(|(_, s)| s.visible && s.width > 0.0)
            {
                let paint_id = format!("stroke-{id}-{i}");
                if !stroke.width_profile.is_uniform() {
                    variable_stroke_shape(node, i, stroke, defs, body, warnings);
                    continue;
                }
                paint_definition(&stroke.paint, &paint_id, node, Some(stroke), defs);
                let cap = match stroke.cap {
                    StrokeCap::Round => "round",
                    StrokeCap::Square => "square",
                    _ => "butt",
                };
                let join = match stroke.join {
                    StrokeJoin::Miter => "miter",
                    StrokeJoin::Round => "round",
                    StrokeJoin::Bevel => "bevel",
                };
                let alignment = if node.has_closed_stroke_contours() {
                    stroke.align
                } else {
                    StrokeAlign::Center
                };
                let mut attributes = format!(
                    r#"fill="none" stroke="{}" stroke-opacity="{}" stroke-width="{}" stroke-linecap="{cap}" stroke-linejoin="{join}""#,
                    paint_color(&stroke.paint, &paint_id),
                    number(paint_opacity(&stroke.paint, stroke.opacity)),
                    number(
                        stroke.width
                            * if alignment == StrokeAlign::Center {
                                1.0
                            } else {
                                2.0
                            }
                    )
                );
                if !stroke.dash.is_empty() {
                    let _ = write!(
                        attributes,
                        r#" stroke-dasharray="{}""#,
                        stroke
                            .dash
                            .iter()
                            .map(|v| number(*v))
                            .collect::<Vec<_>>()
                            .join(" ")
                    );
                }
                if stroke.cap == StrokeCap::Arrow {
                    let _ = write!(
                        defs,
                        r#"<marker id="arrow-{id}-{i}" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="5" markerHeight="5" orient="auto-start-reverse"><path d="M0 0 L10 5 L0 10 Z" fill="{}"/></marker>"#,
                        paint_color(&stroke.paint, &paint_id)
                    );
                    let _ = write!(attributes, r#" marker-end="url(#arrow-{id}-{i})""#);
                }
                match alignment {
                    StrokeAlign::Center => body.push_str(&stroke_shape(node, &attributes)),
                    StrokeAlign::Inside => {
                        let _ = write!(
                            defs,
                            r#"<clipPath id="stroke-clip-{id}-{i}">{shape}</clipPath>"#
                        );
                        let _ = write!(
                            body,
                            r#"<g clip-path="url(#stroke-clip-{id}-{i})">{}</g>"#,
                            stroke_shape(node, &attributes)
                        );
                    }
                    StrokeAlign::Outside => {
                        let extent = stroke.width * 3.0 + 2.0;
                        let _ = write!(
                            defs,
                            r#"<mask id="stroke-mask-{id}-{i}" maskUnits="userSpaceOnUse" x="{}" y="{}" width="{}" height="{}"><rect x="{}" y="{}" width="{}" height="{}" fill="white"/>{}</mask>"#,
                            number(-extent),
                            number(-extent),
                            number(node.width + 2.0 * extent),
                            number(node.height + 2.0 * extent),
                            number(-extent),
                            number(-extent),
                            number(node.width + 2.0 * extent),
                            number(node.height + 2.0 * extent),
                            node_shape(node, r#"fill="black""#)
                        );
                        let _ = write!(
                            body,
                            r#"<g mask="url(#stroke-mask-{id}-{i})">{}</g>"#,
                            stroke_shape(node, &attributes)
                        );
                    }
                }
            }
        }
        if has_filter {
            body.push_str("</g>")
        }
        body.push_str("</g>");
    }

    fn export_boolean(
        &self,
        node: &Node,
        defs: &mut String,
        body: &mut String,
        warnings: &mut Vec<String>,
    ) {
        let id = node.id;
        let operands: Vec<_> = node
            .children
            .iter()
            .filter_map(|id| self.nodes.get(id))
            .filter(|n| n.visible)
            .collect();
        let _ = write!(
            defs,
            r#"<mask id="boolean-{id}" maskUnits="userSpaceOnUse" x="-1" y="-1" width="{}" height="{}">"#,
            number(node.width + 2.0),
            number(node.height + 2.0)
        );
        if node.boolean_operation == BooleanOperation::Exclude {
            let paths = operands
                .iter()
                .map(|n| path_data(&outline_path(n), n.local_transform()))
                .collect::<Vec<_>>()
                .join(" ");
            let _ = write!(
                defs,
                r#"<path d="{paths}" fill="white" fill-rule="evenodd"/>"#
            );
        } else if node.boolean_operation == BooleanOperation::Intersect {
            for (i, n) in operands.iter().enumerate() {
                let path = path_data(&outline_path(n), n.local_transform());
                let _ = write!(
                    defs,
                    r#"<clipPath id="operand-{id}-{i}"><path d="{path}"/></clipPath><g clip-path="url(#operand-{id}-{i})">"#
                );
            }
            let _ = write!(
                defs,
                r#"<rect width="{}" height="{}" fill="white"/>"#,
                number(node.width),
                number(node.height)
            );
            for _ in &operands {
                defs.push_str("</g>")
            }
        } else {
            for (i, n) in operands.iter().enumerate() {
                let path = path_data(&outline_path(n), n.local_transform());
                let fill = if node.boolean_operation == BooleanOperation::Subtract && i > 0 {
                    "black"
                } else {
                    "white"
                };
                let _ = write!(defs, r#"<path d="{path}" fill="{fill}"/>"#);
            }
        }
        defs.push_str("</mask>");
        for (i, fill) in node.fills.iter().enumerate().rev().filter(|(_, f)| f.visible) {
            let paint_id = format!("fill-{id}-{i}");
            paint_definition(&fill.paint, &paint_id, node, None, defs);
            let _ = write!(
                body,
                r#"<rect width="{}" height="{}" fill="{}" fill-opacity="{}" mask="url(#boolean-{id})"/>"#,
                number(node.width),
                number(node.height),
                paint_color(&fill.paint, &paint_id),
                number(paint_opacity(&fill.paint, fill.opacity))
            );
        }
        if operands.iter().any(|n| n.kind == NodeKind::Text) {
            warnings.push(format!("Boolean group {}: text operands use their layout bounds; convert text to glyph outlines for exact typography booleans",node.name));
        }
    }
}
