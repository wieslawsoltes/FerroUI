//! A frame of an animated visual as a list of drawing commands, in the space of the composition.

use crate::content::{document_at, lines_of, Glyph, ImageAsset, TextDocument};
use crate::model::{AnimatedVisual, GradientModel, Layer, LayerKind, MatteMode, ShapeItem, StrokeModel};
use crate::path::{CombinedPath, MergeMode, PathData, Polystar};
use ferroui_base::media::imaging::BitmapBlendingMode;
use crate::value::{Animated, Components};
use ferroui_base::media::{Color, GeometryCombineMode};
use ferroui_base::{Matrix, Point};
use std::collections::HashMap;
use std::rc::Rc;

/// How deep precompositions nest: a document that refers to itself stops here.
pub(crate) const MAX_DEPTH: usize = 32;

/// A kind of gradient.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GradientKind {
    Linear,
    Radial,
}

/// What a shape is painted with.
#[derive(Clone, Debug, PartialEq)]
pub enum Paint {
    Solid(Color),
    Gradient { kind: GradientKind, start: Point, end: Point, stops: Vec<(f64, Color)> },
}

/// How a shape is stroked. `cap` and `join` are the Lottie values: 1 butt or miter, 2 round, 3
/// square or bevel.
#[derive(Clone, Debug, PartialEq)]
pub struct StrokeStyle {
    pub width: f64,
    pub cap: u8,
    pub join: u8,
    pub miter: f64,
    pub dashes: Vec<f64>,
    pub dash_offset: f64,
}

/// A path, in the space of its layer, filled or stroked.
#[derive(Clone, Debug, PartialEq)]
pub struct ShapeDraw {
    pub path: PathData,
    pub paint: Paint,
    pub stroke: Option<StrokeStyle>,
    pub even_odd: bool,
    pub opacity: f64,
    /// The colour property a solid paint takes its colour from, when one is set.
    pub binding: Option<Rc<str>>,
}

/// Text of a font the document does not carry the glyphs of, drawn by the text stack of the
/// backend: `lines` from the baseline of the first at the origin of the layer.
#[derive(Clone, Debug, PartialEq)]
pub struct TextDraw {
    pub lines: Vec<String>,
    pub family: String,
    pub size: f64,
    pub fill: Option<Color>,
    pub stroke: Option<(Color, f64)>,
    pub stroke_over_fill: bool,
    /// 0 left, 1 right, 2 centre.
    pub justify: u8,
    pub line_height: f64,
    /// Added after each character, in the units of the layer.
    pub tracking: f64,
    pub baseline_shift: f64,
    pub opacity: f64,
}

/// How a mask combines with the masks before it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaskMode {
    Add,
    Subtract,
    Intersect,
}

/// A command of a frame. Every `Push` and `BeginMatte` ends with a `Pop`.
#[derive(Clone, Debug, PartialEq)]
pub enum FrameCommand {
    PushTransform(Matrix),
    PushOpacity(f64),
    /// What follows up to its `Pop` is one group, mixed with what is under it.
    PushBlend(BitmapBlendingMode),
    /// Clips what follows to a path of the current space.
    PushMask { path: PathData, mode: MaskMode, inverted: bool, opacity: f64 },
    /// Keeps what the union of these paths covers, each with its opacity.
    PushCoverage(Vec<(PathData, f64)>),
    /// What follows up to `EndMatteSource` is the matte of what follows after it, up to `Pop`.
    BeginMatte(MatteMode),
    EndMatteSource,
    Draw(ShapeDraw),
    /// An image at the origin of the current space, at the size of the asset.
    DrawImage(Rc<ImageAsset>),
    DrawText(TextDraw),
    Pop,
}

/// The commands of a frame. Kept between frames, so its storage is reused.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FrameCommands {
    pub commands: Vec<FrameCommand>,
}

fn channel(value: f64) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn color(components: Components, opacity: f64) -> Color {
    // Older documents write channels from 0 to 255.
    let scale = if components[..3].iter().any(|value| *value > 1.0) { 1.0 / 255.0 } else { 1.0 };
    Color::from_argb(
        channel(opacity),
        channel(components[0] * scale),
        channel(components[1] * scale),
        channel(components[2] * scale),
    )
}

/// The colour of the components of a solid paint, opaque.
pub(crate) fn solid_color(components: Components) -> Color {
    color(components, 1.0)
}

fn scale_of(matrix: Matrix) -> f64 {
    (matrix.m11 * matrix.m22 - matrix.m12 * matrix.m21).abs().sqrt()
}

pub(crate) fn stroke_style(model: &StrokeModel, frame: f64, scale: f64) -> StrokeStyle {
    let mut dashes = Vec::with_capacity(model.dashes.len());
    let mut dash_offset = 0.0;
    for dash in &model.dashes {
        let value = dash.value.at(frame)[0] * scale;
        if dash.kind == b'o' {
            dash_offset = value;
        } else {
            dashes.push(value.max(0.0));
        }
    }
    StrokeStyle {
        width: model.width.at(frame)[0] * scale,
        cap: model.cap,
        join: model.join,
        miter: model.miter,
        dashes,
        dash_offset,
    }
}

/// The stops at `frame`: the colours, with the opacity stops when the document has them.
pub(crate) fn gradient_paint(model: &GradientModel, frame: f64, transform: Matrix) -> Paint {
    let values = model.stops.at(frame);
    let count = model.stops.count;
    let alphas = values.get(count * 4..).unwrap_or(&[]);
    let alpha_at = |offset: f64| -> f64 {
        let pairs: Vec<(f64, f64)> = alphas.chunks_exact(2).map(|pair| (pair[0], pair[1])).collect();
        let (Some(first), Some(last)) = (pairs.first(), pairs.last()) else {
            return 1.0;
        };
        if offset <= first.0 {
            return first.1;
        }
        if offset >= last.0 {
            return last.1;
        }
        for pair in pairs.windows(2) {
            if offset <= pair[1].0 {
                let span = pair[1].0 - pair[0].0;
                let t = if span > 0.0 { (offset - pair[0].0) / span } else { 0.0 };
                return pair[0].1 + (pair[1].1 - pair[0].1) * t;
            }
        }
        last.1
    };
    let stops = values
        .chunks_exact(4)
        .take(count)
        .map(|stop| (stop[0].clamp(0.0, 1.0), color([stop[1], stop[2], stop[3], 1.0], alpha_at(stop[0]))))
        .collect();
    let point = |value: &Animated| {
        let at = value.at(frame);
        transform.transform(Point::new(at[0], at[1]))
    };
    Paint::Gradient {
        kind: if model.radial { GradientKind::Radial } else { GradientKind::Linear },
        start: point(&model.start),
        end: point(&model.end),
        stops,
    }
}

pub(crate) fn merged(geometry: &[PathData]) -> PathData {
    let mut path = PathData::default();
    for part in geometry {
        path.append(part);
    }
    path
}

/// Evaluates the items of a group: geometry gathers in `geometry`, and each paint draws what was
/// gathered before it. `draws` runs from the top of the stack to the bottom, as the items do.
pub(crate) fn evaluate_items(
    items: &[ShapeItem],
    frame: f64,
    transform: Matrix,
    opacity: f64,
    geometry: &mut Vec<PathData>,
    draws: &mut Vec<ShapeDraw>,
) {
    let first_draw = draws.len();
    for (index, item) in items.iter().enumerate() {
        match item {
            ShapeItem::Group { items, transform: own } => {
                let matrix = own.matrix_at(frame) * transform;
                let mut inner = Vec::new();
                evaluate_items(items, frame, matrix, opacity * own.opacity_at(frame), &mut inner, draws);
                geometry.append(&mut inner);
            }
            ShapeItem::Rectangle { position, size, radius } => {
                let (p, s) = (position.at(frame), size.at(frame));
                let path = PathData::rectangle(Point::new(p[0], p[1]), s[0], s[1], radius.at(frame)[0]);
                geometry.push(path.transformed(transform));
            }
            ShapeItem::Ellipse { position, size } => {
                let (p, s) = (position.at(frame), size.at(frame));
                geometry.push(PathData::ellipse(Point::new(p[0], p[1]), s[0], s[1]).transformed(transform));
            }
            ShapeItem::Path(shape) => geometry.push(shape.path_at(frame).transformed(transform)),
            ShapeItem::Polystar { star, position, points, rotation, outer_radius, outer_roundness, inner_radius, inner_roundness } => {
                let center = position.at(frame);
                let shape = Polystar {
                    star: *star,
                    center: Point::new(center[0], center[1]),
                    points: points.at(frame)[0],
                    rotation: rotation.at(frame)[0],
                    outer_radius: outer_radius.at(frame)[0],
                    outer_roundness: outer_roundness.at(frame)[0],
                    inner_radius: inner_radius.at(frame)[0],
                    inner_roundness: inner_roundness.at(frame)[0],
                };
                geometry.push(PathData::polystar(&shape).transformed(transform));
            }
            ShapeItem::Fill { color: paint, opacity: own, even_odd, binding } => {
                let alpha = (own.at(frame)[0] / 100.0).clamp(0.0, 1.0);
                draws.push(ShapeDraw {
                    path: merged(geometry),
                    paint: Paint::Solid(color(paint.at(frame), 1.0)),
                    stroke: None,
                    even_odd: *even_odd,
                    opacity: opacity * alpha,
                    binding: binding.clone(),
                });
            }
            ShapeItem::Stroke { color: paint, opacity: own, stroke, binding } => {
                let alpha = (own.at(frame)[0] / 100.0).clamp(0.0, 1.0);
                draws.push(ShapeDraw {
                    path: merged(geometry),
                    paint: Paint::Solid(color(paint.at(frame), 1.0)),
                    stroke: Some(stroke_style(stroke, frame, scale_of(transform))),
                    even_odd: false,
                    opacity: opacity * alpha,
                    binding: binding.clone(),
                });
            }
            ShapeItem::GradientFill { gradient, opacity: own, even_odd } => {
                let alpha = (own.at(frame)[0] / 100.0).clamp(0.0, 1.0);
                draws.push(ShapeDraw {
                    path: merged(geometry),
                    paint: gradient_paint(gradient, frame, transform),
                    stroke: None,
                    even_odd: *even_odd,
                    opacity: opacity * alpha,
                    binding: None,
                });
            }
            ShapeItem::GradientStroke { gradient, opacity: own, stroke } => {
                let alpha = (own.at(frame)[0] / 100.0).clamp(0.0, 1.0);
                draws.push(ShapeDraw {
                    path: merged(geometry),
                    paint: gradient_paint(gradient, frame, transform),
                    stroke: Some(stroke_style(stroke, frame, scale_of(transform))),
                    even_odd: false,
                    opacity: opacity * alpha,
                    binding: None,
                });
            }
            ShapeItem::Trim { start, end, offset } => {
                let (s, e) = (start.at(frame)[0] / 100.0, end.at(frame)[0] / 100.0);
                let (low, high) = (s.min(e).clamp(0.0, 1.0), s.max(e).clamp(0.0, 1.0));
                if high - low >= 1.0 {
                    continue;
                }
                if high <= low {
                    geometry.clear();
                    continue;
                }
                let turn = offset.at(frame)[0] / 360.0;
                let (from, to) = ((low + turn).rem_euclid(1.0), (high + turn).rem_euclid(1.0));
                for path in geometry.iter_mut() {
                    *path = path.trimmed(from, to);
                }
            }
            ShapeItem::Repeater { copies, offset, above, transform: own, start_opacity, end_opacity } => {
                // A repeater draws everything before it, once per copy.
                draws.truncate(first_draw);
                geometry.clear();
                let count = copies.at(frame)[0].floor().clamp(0.0, 4096.0) as usize;
                let first = offset.at(frame)[0];
                let (from, to) = (start_opacity.at(frame)[0] / 100.0, end_opacity.at(frame)[0] / 100.0);
                let copy = |copy: usize, geometry: &mut Vec<PathData>, draws: &mut Vec<ShapeDraw>| {
                    let t = if count > 1 { copy as f64 / (count - 1) as f64 } else { 0.0 };
                    let alpha = (from + (to - from) * t).clamp(0.0, 1.0);
                    let matrix = own.repeated_at(frame, first + copy as f64) * transform;
                    // Each copy paints its own geometry; a paint after the repeater sees them all.
                    let mut own_geometry = Vec::new();
                    evaluate_items(&items[..index], frame, matrix, opacity * alpha, &mut own_geometry, draws);
                    geometry.append(&mut own_geometry);
                };
                // `draws` runs from the top: the copy on top comes first.
                if *above {
                    for index in (0..count).rev() {
                        copy(index, geometry, draws);
                    }
                } else {
                    for index in 0..count {
                        copy(index, geometry, draws);
                    }
                }
            }
            ShapeItem::Merge(mode) => {
                let parts = std::mem::take(geometry);
                let mut path = merged(&parts);
                let combine = match mode {
                    MergeMode::Merge => None,
                    MergeMode::Add => Some(GeometryCombineMode::Union),
                    MergeMode::Subtract => Some(GeometryCombineMode::Exclude),
                    MergeMode::Intersect => Some(GeometryCombineMode::Intersect),
                    MergeMode::Exclude => Some(GeometryCombineMode::Xor),
                };
                if let (Some(mode), true) = (combine, parts.len() > 1) {
                    path.combined = Some(Rc::new(CombinedPath { mode, parts }));
                }
                geometry.push(path);
            }
        }
    }
}

/// The matrix of a layer with its chain of parents.
fn world_matrix(layers: &[Layer], layer: &Layer, frame: f64) -> Matrix {
    let local = |layer: &Layer| layer.transform.matrix_at((frame - layer.start_time) / layer.stretch);
    let mut matrix = local(layer);
    let mut parent = layer.parent;
    for _ in 0..layers.len() {
        let Some(index) = parent else {
            break;
        };
        let Some(next) = layers.iter().find(|candidate| candidate.index == Some(index)) else {
            break;
        };
        matrix = matrix * local(next);
        parent = next.parent;
    }
    matrix
}

impl AnimatedVisual {
    /// The commands of `frame`. `out` is cleared first and keeps its storage.
    pub fn evaluate(&self, frame: f64, out: &mut FrameCommands) {
        out.commands.clear();
        self.evaluate_layers(&self.layers, frame, 0, &mut out.commands);
    }

    fn evaluate_layers(&self, layers: &[Layer], frame: f64, depth: usize, out: &mut Vec<FrameCommand>) {
        if depth >= MAX_DEPTH {
            return;
        }
        // The first layer of a document is on top: the last is drawn first.
        for (index, layer) in layers.iter().enumerate().rev() {
            if layer.is_matte || !visible(layer, frame) {
                continue;
            }
            let matte = layer.matte.and_then(|mode| {
                let source = match layer.matte_parent {
                    Some(parent) => layers.iter().find(|candidate| candidate.index == Some(parent)),
                    None => index.checked_sub(1).map(|before| &layers[before]),
                };
                Some((mode, source?))
            });
            if let Some((mode, source)) = matte {
                out.push(FrameCommand::BeginMatte(mode));
                if visible(source, frame) {
                    self.evaluate_layer(layers, source, frame, depth, out);
                }
                out.push(FrameCommand::EndMatteSource);
                self.evaluate_layer(layers, layer, frame, depth, out);
                out.push(FrameCommand::Pop);
            } else {
                self.evaluate_layer(layers, layer, frame, depth, out);
            }
        }
    }

    fn evaluate_layer(&self, layers: &[Layer], layer: &Layer, frame: f64, depth: usize, out: &mut Vec<FrameCommand>) {
        let local = (frame - layer.start_time) / layer.stretch;
        let opacity = layer.transform.opacity_at(local);
        if opacity <= 0.0 || matches!(layer.kind, LayerKind::Null | LayerKind::NotDrawn) {
            return;
        }
        let mut pushed = 1;
        if let Some(blend) = layer.blend {
            out.push(FrameCommand::PushBlend(blend));
            pushed += 1;
        }
        out.push(FrameCommand::PushTransform(world_matrix(layers, layer, frame)));
        if opacity < 1.0 {
            out.push(FrameCommand::PushOpacity(opacity));
            pushed += 1;
        }
        // Added masks are a union, which nested clips are not: one path, or at partial opacity
        // a coverage each path adds to.
        let mut added: Vec<(PathData, f64)> = Vec::new();
        for mask in &layer.masks {
            let mode = match mask.mode {
                b'a' => MaskMode::Add,
                b's' => MaskMode::Subtract,
                b'i' => MaskMode::Intersect,
                _ => continue,
            };
            let path = mask.path.path_at(local);
            let opacity = (mask.opacity.at(local)[0] / 100.0).clamp(0.0, 1.0);
            if mode == MaskMode::Add && !mask.inverted {
                added.push((path, opacity));
                continue;
            }
            out.push(FrameCommand::PushMask { path, mode, inverted: mask.inverted, opacity });
            pushed += 1;
        }
        if added.iter().any(|(path, _)| !path.is_empty()) {
            if added.iter().all(|(_, opacity)| *opacity >= 1.0) {
                let mut union = PathData::default();
                for (path, _) in &added {
                    union.append(path);
                }
                out.push(FrameCommand::PushMask { path: union, mode: MaskMode::Add, inverted: false, opacity: 1.0 });
            } else {
                out.push(FrameCommand::PushCoverage(added));
            }
            pushed += 1;
        }
        match &layer.kind {
            LayerKind::Shape(items) => {
                let (mut geometry, mut draws) = (Vec::new(), Vec::new());
                evaluate_items(items, local, Matrix::IDENTITY, 1.0, &mut geometry, &mut draws);
                out.extend(draws.into_iter().rev().filter(|draw| !draw.path.is_empty()).map(FrameCommand::Draw));
            }
            LayerKind::Solid { color: solid, width, height } => {
                let path = PathData::rectangle(Point::new(width * 0.5, height * 0.5), *width, *height, 0.0);
                out.push(FrameCommand::Draw(ShapeDraw {
                    path,
                    paint: Paint::Solid(color(*solid, 1.0)),
                    stroke: None,
                    even_odd: false,
                    opacity: 1.0,
                    binding: None,
                }));
            }
            LayerKind::Precomposition { reference, width, height, time_remap } => {
                if let Some(inner) = self.precompositions.get(reference) {
                    let inner_frame = match time_remap {
                        Some(remap) => remap.at(local)[0] * self.frame_rate,
                        None => local,
                    };
                    if *width > 0.0 && *height > 0.0 {
                        let path = PathData::rectangle(Point::new(width * 0.5, height * 0.5), *width, *height, 0.0);
                        out.push(FrameCommand::PushMask { path, mode: MaskMode::Add, inverted: false, opacity: 1.0 });
                        pushed += 1;
                    }
                    self.evaluate_layers(inner, inner_frame, depth + 1, out);
                }
            }
            LayerKind::Image { reference } => {
                if let Some(image) = self.images.get(reference) {
                    out.push(FrameCommand::DrawImage(image.clone()));
                }
            }
            LayerKind::Text(documents) => {
                if let Some(document) = document_at(documents, local) {
                    match self.fonts.glyphs_for(&document.font, &document.text) {
                        Some(glyphs) => glyph_draws(document, glyphs, local, out),
                        None => out.push(FrameCommand::DrawText(text_draw(document, self.fonts.family_of(&document.font)))),
                    }
                }
            }
            LayerKind::Null | LayerKind::NotDrawn => {}
        }
        for _ in 0..pushed {
            out.push(FrameCommand::Pop);
        }
    }
}

fn text_color(components: Option<crate::value::Components>) -> Option<Color> {
    components.map(|value| color(value, value[3].clamp(0.0, 1.0)))
}

fn text_draw(document: &TextDocument, family: &str) -> TextDraw {
    TextDraw {
        lines: lines_of(&document.text).map(str::to_owned).collect(),
        family: family.to_owned(),
        size: document.size,
        fill: text_color(document.fill),
        stroke: text_color(document.stroke).filter(|_| document.stroke_width > 0.0).map(|paint| (paint, document.stroke_width)),
        stroke_over_fill: document.stroke_over_fill,
        justify: document.justify,
        line_height: document.line_height,
        tracking: document.tracking * document.size / 1000.0,
        baseline_shift: document.baseline_shift,
        opacity: 1.0,
    }
}

/// The text from the shapes the document carries: one path per paint, placed line by line.
fn glyph_draws(document: &TextDocument, glyphs: &HashMap<char, Glyph>, frame: f64, out: &mut Vec<FrameCommand>) {
    let scale = document.size / 100.0;
    let tracking = document.tracking * document.size / 1000.0;
    let mut path = PathData::default();
    for (index, line) in lines_of(&document.text).enumerate() {
        let width: f64 = line.chars().map(|character| glyphs[&character].advance * scale + tracking).sum();
        let mut x = match document.justify {
            1 => -width,
            2 => -width * 0.5,
            _ => 0.0,
        };
        let y = index as f64 * document.line_height - document.baseline_shift;
        for character in line.chars() {
            let glyph = &glyphs[&character];
            let matrix = Matrix::create_scale(scale, scale) * Matrix::create_translation(x, y);
            let (mut geometry, mut ignored) = (Vec::new(), Vec::new());
            evaluate_items(&glyph.shapes, frame, matrix, 1.0, &mut geometry, &mut ignored);
            for part in &geometry {
                path.append(part);
            }
            x += glyph.advance * scale + tracking;
        }
    }
    if path.is_empty() {
        return;
    }
    let fill = text_color(document.fill).map(|paint| {
        FrameCommand::Draw(ShapeDraw {
            path: path.clone(),
            paint: Paint::Solid(Color::from_rgb(paint.r, paint.g, paint.b)),
            stroke: None,
            even_odd: false,
            opacity: f64::from(paint.a) / 255.0,
            binding: None,
        })
    });
    let stroke = text_color(document.stroke).filter(|_| document.stroke_width > 0.0).map(|paint| {
        FrameCommand::Draw(ShapeDraw {
            path: path.clone(),
            paint: Paint::Solid(Color::from_rgb(paint.r, paint.g, paint.b)),
            stroke: Some(StrokeStyle {
                width: document.stroke_width,
                cap: 1,
                join: 2,
                miter: 4.0,
                dashes: Vec::new(),
                dash_offset: 0.0,
            }),
            even_odd: false,
            opacity: f64::from(paint.a) / 255.0,
            binding: None,
        })
    });
    let ordered = if document.stroke_over_fill { [fill, stroke] } else { [stroke, fill] };
    out.extend(ordered.into_iter().flatten());
}

fn visible(layer: &Layer, frame: f64) -> bool {
    frame >= layer.in_point && frame < layer.out_point
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path::PathCommand;
    use serde_json::{json, Value};

    fn document(layers: Value) -> AnimatedVisual {
        AnimatedVisual::parse(&json!({"v": "5.7", "fr": 30, "ip": 0, "op": 60, "w": 100, "h": 100, "layers": layers}).to_string()).unwrap()
    }

    fn shape_layer(shapes: Value) -> Value {
        json!({"ty": 4, "ind": 1, "ip": 0, "op": 60, "st": 0, "ks": {}, "shapes": shapes})
    }

    fn fill(r: f64, g: f64, b: f64) -> Value {
        json!({"ty": "fl", "c": {"k": [r, g, b, 1]}, "o": {"k": 100}})
    }

    fn rectangle(x: f64, width: f64) -> Value {
        json!({"ty": "rc", "p": {"k": [x, 10]}, "s": {"k": [width, 20]}, "r": {"k": 0}})
    }

    fn draws(visual: &AnimatedVisual, frame: f64) -> Vec<ShapeDraw> {
        let mut out = FrameCommands::default();
        visual.evaluate(frame, &mut out);
        out.commands
            .into_iter()
            .filter_map(|command| match command {
                FrameCommand::Draw(draw) => Some(draw),
                _ => None,
            })
            .collect()
    }

    fn first_point(path: &PathData) -> Point {
        match path.commands[0] {
            PathCommand::MoveTo(point) => point,
            ref other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_document_needs_its_size_rate_and_layers() {
        assert!(matches!(AnimatedVisual::parse("nope"), Err(crate::AnimatedVisualError::Json(_))));
        assert!(AnimatedVisual::parse(r#"{"w":1,"h":1,"fr":0,"ip":0,"op":1,"layers":[]}"#).is_err());
        assert!(AnimatedVisual::parse(r#"{"w":1,"h":1,"fr":30,"ip":0,"op":1}"#).is_err());
        let visual = AnimatedVisual::parse(
            r#"{"w":10,"h":20,"fr":30,"ip":0,"op":60,"layers":[],"markers":[{"cm":"NormalToPressed","tm":10,"dr":5}]}"#,
        )
        .unwrap();
        assert_eq!(visual.duration_seconds(), 2.0);
        assert_eq!(visual.frame_at(0.5), 30.0);
        assert_eq!(visual.marker("NormalToPressed").map(|marker| marker.frame), Some(10.0));
    }

    #[test]
    fn a_paint_draws_the_geometry_before_it_and_earlier_items_are_on_top() {
        let visual = document(json!([shape_layer(json!([
            {"ty": "gr", "it": [rectangle(10.0, 4.0), fill(1.0, 0.0, 0.0), {"ty": "tr"}]},
            {"ty": "gr", "it": [rectangle(50.0, 4.0), fill(0.0, 0.0, 1.0), {"ty": "tr"}]},
        ]))]));
        let draws = draws(&visual, 0.0);
        assert_eq!(draws.len(), 2);
        // The last group of the document is drawn first, so the first is on top.
        assert_eq!(draws[0].paint, Paint::Solid(Color::from_rgb(0, 0, 255)));
        assert_eq!(draws[1].paint, Paint::Solid(Color::from_rgb(255, 0, 0)));
    }

    #[test]
    fn a_group_transform_moves_its_geometry() {
        let visual = document(json!([shape_layer(json!([{"ty": "gr", "it": [
            rectangle(10.0, 4.0), fill(1.0, 0.0, 0.0),
            {"ty": "tr", "p": {"k": [100, 0]}, "a": {"k": [0, 0]}, "s": {"k": [200, 200]}, "r": {"k": 0}, "o": {"k": 50}}
        ]}])) ]));
        let draws = draws(&visual, 0.0);
        // The right top corner of the rectangle, 12 from the origin, doubled and moved.
        assert_eq!(first_point(&draws[0].path).x, 124.0);
        assert_eq!(draws[0].opacity, 0.5);
    }

    #[test]
    fn a_layer_takes_the_matrices_of_its_parents() {
        let visual = document(json!([
            {"ty": 3, "ind": 2, "ip": 0, "op": 60, "ks": {"p": {"k": [30, 0]}}},
            {"ty": 4, "ind": 1, "parent": 2, "ip": 0, "op": 60, "ks": {"p": {"k": [5, 0]}},
             "shapes": [rectangle(0.0, 0.0), fill(1.0, 1.0, 1.0)]}
        ]));
        let mut out = FrameCommands::default();
        visual.evaluate(0.0, &mut out);
        let FrameCommand::PushTransform(matrix) = &out.commands[0] else { panic!("{:?}", out.commands[0]) };
        assert_eq!(matrix.transform(Point::new(0.0, 0.0)), Point::new(35.0, 0.0));
    }

    #[test]
    fn a_layer_shows_between_its_in_and_out_points_on_its_own_time() {
        let visual = document(json!([{"ty": 4, "ind": 1, "ip": 10, "op": 20, "st": 10, "sr": 2,
            "ks": {"p": {"k": [{"t": 0, "s": [0, 0], "e": [10, 0]}, {"t": 10}]}},
            "shapes": [rectangle(0.0, 2.0), fill(0.0, 0.0, 0.0)]}]));
        assert!(draws(&visual, 9.0).is_empty());
        assert!(draws(&visual, 20.0).is_empty());
        let mut out = FrameCommands::default();
        visual.evaluate(20.0 - 5.0, &mut out);
        // Frame 15 is 5 after the start, at half speed the local frame 2.5.
        let FrameCommand::PushTransform(matrix) = &out.commands[0] else { panic!() };
        assert!((matrix.m31 - 2.5).abs() < 1e-9, "{}", matrix.m31);
    }

    #[test]
    fn trim_paths_cut_the_geometry_before_them() {
        let visual = document(json!([shape_layer(json!([
            rectangle(10.0, 10.0),
            {"ty": "tm", "s": {"k": 0}, "e": {"k": 25}, "o": {"k": 0}},
            {"ty": "st", "c": {"k": [0, 0, 0, 1]}, "o": {"k": 100}, "w": {"k": 2}}
        ]))]));
        let draws = draws(&visual, 0.0);
        // A quarter of the outline of a 10 by 20 rectangle.
        assert_eq!(draws.len(), 1);
        assert_eq!(draws[0].stroke.as_ref().map(|stroke| stroke.width), Some(2.0));
        let curves = draws[0].path.commands.iter().filter(|c| matches!(c, PathCommand::CubicTo(..))).count();
        assert_eq!(curves, 1);
    }

    #[test]
    fn a_repeater_draws_a_copy_per_count_with_its_opacity_ramp() {
        let visual = document(json!([shape_layer(json!([
            rectangle(0.0, 2.0), fill(1.0, 0.0, 0.0),
            {"ty": "rp", "c": {"k": 3}, "o": {"k": 0}, "m": 1,
             "tr": {"p": {"k": [10, 0]}, "a": {"k": [0, 0]}, "s": {"k": [100, 100]}, "r": {"k": 0},
                    "so": {"k": 100}, "eo": {"k": 0}}}
        ]))]));
        let draws = draws(&visual, 0.0);
        assert_eq!(draws.len(), 3);
        // Drawn bottom to top: with "above", the first copy first.
        assert_eq!(first_point(&draws[0].path).x, 1.0);
        assert_eq!(first_point(&draws[2].path).x, 21.0);
        assert_eq!(draws[0].opacity, 1.0);
        assert_eq!(draws[2].opacity, 0.0);
    }

    #[test]
    fn a_precomposition_runs_on_the_time_of_its_layer_and_is_clipped() {
        let visual = AnimatedVisual::parse(&json!({"fr": 30, "ip": 0, "op": 60, "w": 100, "h": 100,
            "assets": [{"id": "inner", "layers": [{"ty": 4, "ind": 1, "ip": 5, "op": 6, "ks": {},
                "shapes": [rectangle(0.0, 2.0), fill(0.0, 1.0, 0.0)]}]}],
            "layers": [{"ty": 0, "ind": 1, "refId": "inner", "w": 50, "h": 50, "ip": 0, "op": 60, "st": 10, "ks": {}}]
        }).to_string()).unwrap();
        assert!(draws(&visual, 5.0).is_empty());
        assert_eq!(draws(&visual, 15.0).len(), 1);
        let mut out = FrameCommands::default();
        visual.evaluate(15.0, &mut out);
        assert!(out.commands.iter().any(|command| matches!(command, FrameCommand::PushMask { .. })));
    }

    #[test]
    fn a_precomposition_that_refers_to_itself_stops() {
        let visual = AnimatedVisual::parse(&json!({"fr": 30, "ip": 0, "op": 60, "w": 100, "h": 100,
            "assets": [{"id": "self", "layers": [{"ty": 0, "ind": 1, "refId": "self", "ip": 0, "op": 60, "ks": {}}]}],
            "layers": [{"ty": 0, "ind": 1, "refId": "self", "ip": 0, "op": 60, "ks": {}}]
        }).to_string()).unwrap();
        assert!(draws(&visual, 0.0).is_empty());
    }

    #[test]
    fn a_track_matte_wraps_its_layer_and_is_not_drawn_alone() {
        let visual = document(json!([
            {"ty": 4, "ind": 1, "td": 1, "ip": 0, "op": 60, "ks": {}, "shapes": [rectangle(0.0, 2.0), fill(1.0, 1.0, 1.0)]},
            {"ty": 4, "ind": 2, "tt": 1, "ip": 0, "op": 60, "ks": {}, "shapes": [rectangle(5.0, 2.0), fill(1.0, 0.0, 0.0)]}
        ]));
        let mut out = FrameCommands::default();
        visual.evaluate(0.0, &mut out);
        assert!(matches!(out.commands[0], FrameCommand::BeginMatte(MatteMode::Alpha)));
        let source = out.commands.iter().position(|c| matches!(c, FrameCommand::EndMatteSource)).unwrap();
        let draws_before = out.commands[..source].iter().filter(|c| matches!(c, FrameCommand::Draw(_))).count();
        let draws_after = out.commands[source..].iter().filter(|c| matches!(c, FrameCommand::Draw(_))).count();
        assert_eq!((draws_before, draws_after), (1, 1));
        assert_eq!(out.commands.last(), Some(&FrameCommand::Pop));
    }

    #[test]
    fn masks_are_pushed_around_the_content_and_every_push_is_popped() {
        let visual = document(json!([{"ty": 4, "ind": 1, "ip": 0, "op": 60, "ks": {"o": {"k": 50}}, "hasMask": true,
            "masksProperties": [{"mode": "a", "inv": false, "o": {"k": 100},
                "pt": {"k": {"c": true, "v": [[0, 0], [10, 0], [10, 10]], "i": [[0, 0], [0, 0], [0, 0]], "o": [[0, 0], [0, 0], [0, 0]]}}}],
            "shapes": [rectangle(0.0, 2.0), fill(0.0, 0.0, 0.0)]}]));
        let mut out = FrameCommands::default();
        visual.evaluate(0.0, &mut out);
        let opened = out.commands.iter().filter(|c| matches!(c, FrameCommand::PushTransform(_) | FrameCommand::PushOpacity(_) | FrameCommand::PushMask { .. })).count();
        let popped = out.commands.iter().filter(|c| matches!(c, FrameCommand::Pop)).count();
        assert_eq!((opened, popped), (3, 3));
    }

    #[test]
    fn a_gradient_has_its_colours_and_opacity_stops() {
        let visual = document(json!([shape_layer(json!([
            rectangle(10.0, 4.0),
            {"ty": "gf", "t": 1, "o": {"k": 100}, "s": {"k": [0, 0]}, "e": {"k": [10, 0]},
             "g": {"p": 2, "k": {"k": [0, 1, 0, 0, 1, 0, 0, 1, 0, 1, 1, 0]}}}
        ]))]));
        let draws = draws(&visual, 0.0);
        let Paint::Gradient { kind, stops, end, .. } = &draws[0].paint else { panic!("{:?}", draws[0].paint) };
        assert_eq!(*kind, GradientKind::Linear);
        assert_eq!(*end, Point::new(10.0, 0.0));
        assert_eq!(stops[0], (0.0, Color::from_argb(255, 255, 0, 0)));
        assert_eq!(stops[1], (1.0, Color::from_argb(0, 0, 0, 255)));
    }

    #[test]
    fn evaluating_a_frame_keeps_the_storage_of_the_commands() {
        let visual = document(json!([shape_layer(json!([rectangle(10.0, 4.0), fill(1.0, 0.0, 0.0)]))]));
        let mut out = FrameCommands::default();
        visual.evaluate(0.0, &mut out);
        let (commands, capacity) = (out.commands.len(), out.commands.capacity());
        visual.evaluate(1.0, &mut out);
        assert_eq!((out.commands.len(), out.commands.capacity()), (commands, capacity));
    }
}
