//! The model of a Lottie document, read once: layers, shapes, transforms and markers.

use crate::content::{text_documents, DocumentFonts, ImageAsset, TextDocument};
use crate::path::{BezierShape, MergeMode, PathData};
use ferroui_base::media::imaging::BitmapBlendingMode;
use crate::value::{keyframes, parse_eases, Animated, Components, Ease};
use ferroui_base::Matrix;
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

/// How far before the out point the last frame is taken.
const LAST_FRAME_MARGIN: f64 = 1e-6;

/// Why a document could not be read.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum AnimatedVisualError {
    /// It is not JSON.
    Json(String),
    /// It is JSON, but not a Lottie document: what is missing.
    Invalid(&'static str),
}

impl fmt::Display for AnimatedVisualError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(error) => write!(f, "the animated visual is not JSON: {error}"),
            Self::Invalid(what) => write!(f, "the animated visual has no valid {what}"),
        }
    }
}

impl std::error::Error for AnimatedVisualError {}

/// A named position of the animation.
#[derive(Clone, Debug, PartialEq)]
pub struct Marker {
    pub name: String,
    /// The frame it starts at.
    pub frame: f64,
    /// Its length in frames.
    pub duration: f64,
}

/// How a layer is cut by the layer that is its matte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatteMode {
    Alpha,
    AlphaInverted,
    Luma,
    LumaInverted,
}

/// A transform at a frame: anchor, position, scale as a factor and rotation in degrees.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TransformParts {
    pub(crate) anchor: [f64; 2],
    pub(crate) position: [f64; 2],
    pub(crate) scale: [f64; 2],
    pub(crate) rotation: f64,
}

/// The transform of a layer, a group or a repeater.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Transform {
    anchor: Animated,
    position: Option<Animated>,
    position_x: Option<Animated>,
    position_y: Option<Animated>,
    scale: Animated,
    rotation: Animated,
    opacity: Animated,
    skew: Animated,
    skew_axis: Animated,
}

impl Transform {
    pub(crate) fn parse(value: Option<&Value>) -> Self {
        let empty = Value::Null;
        let value = value.unwrap_or(&empty);
        let position = value.get("p");
        let split = position.and_then(|p| p.get("s")).and_then(Value::as_bool).unwrap_or(false);
        Self {
            anchor: Animated::of(value, "a", [0.0; 4]),
            position: if split { None } else { position.and_then(Animated::parse) },
            position_x: if split { position.and_then(|p| p.get("x")).and_then(Animated::parse) } else { None },
            position_y: if split { position.and_then(|p| p.get("y")).and_then(Animated::parse) } else { None },
            scale: Animated::of(value, "s", [100.0, 100.0, 100.0, 0.0]),
            rotation: Animated::of(value, "r", [0.0; 4]),
            opacity: Animated::of(value, "o", [100.0, 0.0, 0.0, 0.0]),
            skew: Animated::of(value, "sk", [0.0; 4]),
            skew_axis: Animated::of(value, "sa", [0.0; 4]),
        }
    }

    /// The matrix at `frame`: anchor, scale, skew, rotation, position.
    pub(crate) fn matrix_at(&self, frame: f64) -> Matrix {
        let anchor = self.anchor.at(frame);
        let position = match (&self.position, &self.position_x, &self.position_y) {
            (Some(position), _, _) => position.at(frame),
            (None, x, y) => [
                x.as_ref().map_or(0.0, |x| x.at(frame)[0]),
                y.as_ref().map_or(0.0, |y| y.at(frame)[0]),
                0.0,
                0.0,
            ],
        };
        let scale = self.scale.at(frame);
        let rotation = self.rotation.at(frame)[0].to_radians();
        let skew = self.skew.at(frame)[0].to_radians();
        let mut matrix = Matrix::create_translation(-anchor[0], -anchor[1])
            * Matrix::create_scale(scale[0] / 100.0, scale[1] / 100.0);
        if skew != 0.0 {
            let axis = self.skew_axis.at(frame)[0].to_radians();
            let (sin, cos) = axis.sin_cos();
            matrix = matrix
                * Matrix::new(cos, sin, -sin, cos, 0.0, 0.0)
                * Matrix::new(1.0, 0.0, (-skew).tan(), 1.0, 0.0, 0.0)
                * Matrix::new(cos, -sin, sin, cos, 0.0, 0.0);
        }
        matrix * Matrix::create_rotation(rotation) * Matrix::create_translation(position[0], position[1])
    }

    /// Whether a skew takes part in the matrix.
    pub(crate) fn has_skew(&self) -> bool {
        self.skew.is_animated() || self.skew.at(0.0)[0] != 0.0
    }

    /// Whether any part of the transform changes.
    pub(crate) fn is_animated(&self) -> bool {
        self.anchor.is_animated()
            || self.position.as_ref().is_some_and(Animated::is_animated)
            || self.position_x.as_ref().is_some_and(Animated::is_animated)
            || self.position_y.as_ref().is_some_and(Animated::is_animated)
            || self.scale.is_animated()
            || self.rotation.is_animated()
            || self.skew.is_animated()
            || self.skew_axis.is_animated()
    }

    /// The parts of the transform at `frame`.
    pub(crate) fn parts_at(&self, frame: f64) -> TransformParts {
        let anchor = self.anchor.at(frame);
        let position = match (&self.position, &self.position_x, &self.position_y) {
            (Some(position), _, _) => position.at(frame),
            (None, x, y) => [
                x.as_ref().map_or(0.0, |x| x.at(frame)[0]),
                y.as_ref().map_or(0.0, |y| y.at(frame)[0]),
                0.0,
                0.0,
            ],
        };
        let scale = self.scale.at(frame);
        TransformParts {
            anchor: [anchor[0], anchor[1]],
            position: [position[0], position[1]],
            scale: [scale[0] / 100.0, scale[1] / 100.0],
            rotation: self.rotation.at(frame)[0],
        }
    }

    /// The opacity at `frame`, from 0 to 1.
    pub(crate) fn opacity_at(&self, frame: f64) -> f64 {
        (self.opacity.at(frame)[0] / 100.0).clamp(0.0, 1.0)
    }

    /// The matrix of copy `index` of a repeater: its anchor, the scale, rotation and position each
    /// applied `index` times.
    pub(crate) fn repeated_at(&self, frame: f64, index: f64) -> Matrix {
        let anchor = self.anchor.at(frame);
        let position = self.position.as_ref().map_or([0.0; 4], |position| position.at(frame));
        let scale = self.scale.at(frame);
        let rotation = self.rotation.at(frame)[0].to_radians() * index;
        Matrix::create_translation(-anchor[0], -anchor[1])
            * Matrix::create_scale((scale[0] / 100.0).powf(index), (scale[1] / 100.0).powf(index))
            * Matrix::create_rotation(rotation)
            * Matrix::create_translation(anchor[0] + position[0] * index, anchor[1] + position[1] * index)
    }
}

#[derive(Clone, Debug, PartialEq)]
struct ShapeKey {
    time: f64,
    start: BezierShape,
    end: BezierShape,
    ease: Ease,
    hold: bool,
}

/// A Bezier shape that may change over the frames.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AnimatedShape {
    value: BezierShape,
    keys: Vec<ShapeKey>,
}

impl AnimatedShape {
    pub(crate) fn parse(property: &Value) -> Option<Self> {
        let Some(items) = keyframes(property) else {
            return Some(Self { value: BezierShape::parse(property.get("k")?)?, keys: Vec::new() });
        };
        let mut keys = Vec::with_capacity(items.len());
        let mut previous_end: Option<BezierShape> = None;
        for (index, key) in items.iter().enumerate() {
            let time = key.get("t")?.as_f64()?;
            let start = key.get("s").and_then(BezierShape::parse).or_else(|| previous_end.clone())?;
            let end = key
                .get("e")
                .and_then(BezierShape::parse)
                .or_else(|| items.get(index + 1).and_then(|next| next.get("s")).and_then(BezierShape::parse))
                .unwrap_or_else(|| start.clone());
            let hold = key.get("h").and_then(Value::as_i64) == Some(1);
            previous_end = Some(end.clone());
            keys.push(ShapeKey { time, start, end, ease: parse_eases(key)[0], hold });
        }
        Some(Self { value: keys.first()?.start.clone(), keys })
    }

    /// Whether the shape changes over the frames.
    pub(crate) fn is_animated(&self) -> bool {
        !self.keys.is_empty()
    }

    pub(crate) fn path_at(&self, frame: f64) -> PathData {
        let (Some(first), Some(last)) = (self.keys.first(), self.keys.last()) else {
            return self.value.to_path();
        };
        if frame <= first.time {
            return first.start.to_path();
        }
        if frame >= last.time {
            return last.start.to_path();
        }
        let index = self.keys.partition_point(|key| key.time <= frame) - 1;
        let (key, next) = (&self.keys[index], &self.keys[index + 1]);
        if key.hold || next.time <= key.time {
            return key.start.to_path();
        }
        let progress = key.ease.at((frame - key.time) / (next.time - key.time));
        key.start.lerp(&key.end, progress).to_path()
    }
}

/// The colour stops of a gradient (`g`): offset, red, green, blue for each of `count` stops, then
/// optionally offset and alpha pairs.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GradientStops {
    pub(crate) count: usize,
    keys: Vec<(f64, Vec<f64>, Vec<f64>, Ease, bool)>,
    value: Vec<f64>,
}

fn numbers(value: Option<&Value>) -> Option<Vec<f64>> {
    value?.as_array()?.iter().map(Value::as_f64).collect()
}

impl GradientStops {
    fn parse(value: &Value) -> Option<Self> {
        let count = value.get("p")?.as_u64()? as usize;
        let property = value.get("k")?;
        let Some(items) = keyframes(property) else {
            return Some(Self { count, keys: Vec::new(), value: numbers(property.get("k"))? });
        };
        let mut keys = Vec::with_capacity(items.len());
        let mut previous: Option<Vec<f64>> = None;
        for (index, key) in items.iter().enumerate() {
            let time = key.get("t")?.as_f64()?;
            let start = numbers(key.get("s")).or_else(|| previous.clone())?;
            let end = numbers(key.get("e"))
                .or_else(|| items.get(index + 1).and_then(|next| numbers(next.get("s"))))
                .unwrap_or_else(|| start.clone());
            previous = Some(end.clone());
            keys.push((time, start, end, parse_eases(key)[0], key.get("h").and_then(Value::as_i64) == Some(1)));
        }
        let value = keys.first()?.1.clone();
        Some(Self { count, keys, value })
    }

    /// Whether the stops change over the frames.
    pub(crate) fn is_animated(&self) -> bool {
        !self.keys.is_empty()
    }

    pub(crate) fn at(&self, frame: f64) -> Vec<f64> {
        let (Some(first), Some(last)) = (self.keys.first(), self.keys.last()) else {
            return self.value.clone();
        };
        if frame <= first.0 {
            return first.1.clone();
        }
        if frame >= last.0 {
            return last.1.clone();
        }
        let index = self.keys.partition_point(|key| key.0 <= frame) - 1;
        let (key, next) = (&self.keys[index], &self.keys[index + 1]);
        if key.4 || next.0 <= key.0 || key.1.len() != key.2.len() {
            return key.1.clone();
        }
        let t = key.3.at((frame - key.0) / (next.0 - key.0));
        key.1.iter().zip(&key.2).map(|(a, b)| a + (b - a) * t).collect()
    }
}

/// The dashes of a stroke: dash, gap and offset properties in order.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Dash {
    pub(crate) kind: u8,
    pub(crate) value: Animated,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct StrokeModel {
    pub(crate) width: Animated,
    pub(crate) cap: u8,
    pub(crate) join: u8,
    pub(crate) miter: f64,
    pub(crate) dashes: Vec<Dash>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GradientModel {
    pub(crate) radial: bool,
    pub(crate) start: Animated,
    pub(crate) end: Animated,
    pub(crate) stops: GradientStops,
}

/// An item of a shape layer or a group.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ShapeItem {
    Group { items: Vec<ShapeItem>, transform: Transform },
    Rectangle { position: Animated, size: Animated, radius: Animated },
    Ellipse { position: Animated, size: Animated },
    Path(AnimatedShape),
    Polystar {
        star: bool,
        position: Animated,
        points: Animated,
        rotation: Animated,
        outer_radius: Animated,
        outer_roundness: Animated,
        inner_radius: Animated,
        inner_roundness: Animated,
    },
    Fill { color: Animated, opacity: Animated, even_odd: bool, binding: Option<Rc<str>> },
    Stroke { color: Animated, opacity: Animated, stroke: StrokeModel, binding: Option<Rc<str>> },
    GradientFill { gradient: GradientModel, opacity: Animated, even_odd: bool },
    GradientStroke { gradient: GradientModel, opacity: Animated, stroke: StrokeModel },
    Trim { start: Animated, end: Animated, offset: Animated },
    Repeater { copies: Animated, offset: Animated, above: bool, transform: Transform, start_opacity: Animated, end_opacity: Animated },
    Merge(MergeMode),
}

impl GradientModel {
    /// Whether the points or the stops change over the frames.
    pub(crate) fn is_animated(&self) -> bool {
        self.start.is_animated() || self.end.is_animated() || self.stops.is_animated()
    }
}

fn stroke_model(item: &Value) -> StrokeModel {
    let dashes = item
        .get("d")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|dash| {
                    let kind = dash.get("n")?.as_str()?.bytes().next()?;
                    Some(Dash { kind, value: Animated::parse(dash.get("v")?)? })
                })
                .collect()
        })
        .unwrap_or_default();
    StrokeModel {
        width: Animated::of(item, "w", [1.0, 0.0, 0.0, 0.0]),
        cap: item.get("lc").and_then(Value::as_u64).unwrap_or(2) as u8,
        join: item.get("lj").and_then(Value::as_u64).unwrap_or(2) as u8,
        miter: item.get("ml").and_then(Value::as_f64).unwrap_or(4.0),
        dashes,
    }
}

fn gradient_model(item: &Value) -> Option<GradientModel> {
    Some(GradientModel {
        radial: item.get("t").and_then(Value::as_u64) == Some(2),
        start: Animated::of(item, "s", [0.0; 4]),
        end: Animated::of(item, "e", [0.0; 4]),
        stops: GradientStops::parse(item.get("g")?)?,
    })
}

const FULL: Components = [100.0, 0.0, 0.0, 0.0];

pub(crate) fn shape_items(items: Option<&Value>) -> Vec<ShapeItem> {
    let Some(items) = items.and_then(Value::as_array) else {
        return Vec::new();
    };
    items.iter().filter_map(shape_item).collect()
}

/// The colour property a paint is bound to: `{color:Name}` in its name.
fn color_binding(item: &Value) -> Option<Rc<str>> {
    let (_, rest) = item.get("nm")?.as_str()?.split_once("{color:")?;
    let (name, _) = rest.split_once('}')?;
    Some(Rc::from(name.trim()))
}

fn shape_item(item: &Value) -> Option<ShapeItem> {
    if item.get("hd").and_then(Value::as_bool) == Some(true) {
        return None;
    }
    let even_odd = item.get("r").and_then(Value::as_u64) == Some(2);
    Some(match item.get("ty")?.as_str()? {
        "gr" => {
            let children = item.get("it")?.as_array()?;
            // The transform of a group is an item of it.
            let transform = Transform::parse(children.iter().find(|child| child.get("ty").and_then(Value::as_str) == Some("tr")));
            ShapeItem::Group { items: children.iter().filter_map(shape_item).collect(), transform }
        }
        "rc" => ShapeItem::Rectangle {
            position: Animated::of(item, "p", [0.0; 4]),
            size: Animated::of(item, "s", [0.0; 4]),
            radius: Animated::of(item, "r", [0.0; 4]),
        },
        "el" => ShapeItem::Ellipse { position: Animated::of(item, "p", [0.0; 4]), size: Animated::of(item, "s", [0.0; 4]) },
        "sh" => ShapeItem::Path(AnimatedShape::parse(item.get("ks")?)?),
        "sr" => ShapeItem::Polystar {
            star: item.get("sy").and_then(Value::as_u64).unwrap_or(1) == 1,
            position: Animated::of(item, "p", [0.0; 4]),
            points: Animated::of(item, "pt", [5.0, 0.0, 0.0, 0.0]),
            rotation: Animated::of(item, "r", [0.0; 4]),
            outer_radius: Animated::of(item, "or", [0.0; 4]),
            outer_roundness: Animated::of(item, "os", [0.0; 4]),
            inner_radius: Animated::of(item, "ir", [0.0; 4]),
            inner_roundness: Animated::of(item, "is", [0.0; 4]),
        },
        "fl" => ShapeItem::Fill {
            color: Animated::of(item, "c", [0.0, 0.0, 0.0, 1.0]),
            opacity: Animated::of(item, "o", FULL),
            even_odd,
            binding: color_binding(item),
        },
        "st" => ShapeItem::Stroke {
            color: Animated::of(item, "c", [0.0, 0.0, 0.0, 1.0]),
            opacity: Animated::of(item, "o", FULL),
            stroke: stroke_model(item),
            binding: color_binding(item),
        },
        "gf" => ShapeItem::GradientFill { gradient: gradient_model(item)?, opacity: Animated::of(item, "o", FULL), even_odd },
        "gs" => ShapeItem::GradientStroke {
            gradient: gradient_model(item)?,
            opacity: Animated::of(item, "o", FULL),
            stroke: stroke_model(item),
        },
        "tm" => ShapeItem::Trim {
            start: Animated::of(item, "s", [0.0; 4]),
            end: Animated::of(item, "e", FULL),
            offset: Animated::of(item, "o", [0.0; 4]),
        },
        "rp" => {
            let transform = item.get("tr");
            ShapeItem::Repeater {
                copies: Animated::of(item, "c", [1.0, 0.0, 0.0, 0.0]),
                offset: Animated::of(item, "o", [0.0; 4]),
                above: item.get("m").and_then(Value::as_u64).unwrap_or(1) == 1,
                transform: Transform::parse(transform),
                start_opacity: transform.map_or_else(|| Animated::constant(FULL), |t| Animated::of(t, "so", FULL)),
                end_opacity: transform.map_or_else(|| Animated::constant(FULL), |t| Animated::of(t, "eo", FULL)),
            }
        }
        "mm" => ShapeItem::Merge(match item.get("mm").and_then(Value::as_u64) {
            Some(2) => MergeMode::Add,
            Some(3) => MergeMode::Subtract,
            Some(4) => MergeMode::Intersect,
            Some(5) => MergeMode::Exclude,
            _ => MergeMode::Merge,
        }),
        _ => return None,
    })
}

/// A mask of a layer.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MaskModel {
    pub(crate) mode: u8,
    pub(crate) inverted: bool,
    pub(crate) path: AnimatedShape,
    pub(crate) opacity: Animated,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum LayerKind {
    Shape(Vec<ShapeItem>),
    Solid { color: Components, width: f64, height: f64 },
    Null,
    Precomposition { reference: String, width: f64, height: f64, time_remap: Option<Animated> },
    Image { reference: String },
    Text(Vec<(f64, TextDocument)>),
    /// A kind this runtime does not draw (audio, data and the like): its transform can still be
    /// a parent.
    NotDrawn,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Layer {
    pub(crate) index: Option<i64>,
    pub(crate) parent: Option<i64>,
    pub(crate) in_point: f64,
    pub(crate) out_point: f64,
    pub(crate) start_time: f64,
    pub(crate) stretch: f64,
    pub(crate) transform: Transform,
    pub(crate) matte: Option<MatteMode>,
    pub(crate) is_matte: bool,
    pub(crate) matte_parent: Option<i64>,
    pub(crate) masks: Vec<MaskModel>,
    pub(crate) kind: LayerKind,
    /// How the layer mixes with what is under it, when not normally.
    pub(crate) blend: Option<BitmapBlendingMode>,
}

fn hex_color(text: &str) -> Components {
    let hex = text.trim_start_matches('#');
    let channel = |at: usize| hex.get(at..at + 2).and_then(|part| u8::from_str_radix(part, 16).ok()).unwrap_or(0);
    [f64::from(channel(0)) / 255.0, f64::from(channel(2)) / 255.0, f64::from(channel(4)) / 255.0, 1.0]
}

/// The blend mode of a layer (`bm`).
fn blend_mode(value: Option<u64>) -> Option<BitmapBlendingMode> {
    Some(match value? {
        1 => BitmapBlendingMode::Multiply,
        2 => BitmapBlendingMode::Screen,
        3 => BitmapBlendingMode::Overlay,
        4 => BitmapBlendingMode::Darken,
        5 => BitmapBlendingMode::Lighten,
        6 => BitmapBlendingMode::ColorDodge,
        7 => BitmapBlendingMode::ColorBurn,
        8 => BitmapBlendingMode::HardLight,
        9 => BitmapBlendingMode::SoftLight,
        10 => BitmapBlendingMode::Difference,
        11 => BitmapBlendingMode::Exclusion,
        12 => BitmapBlendingMode::Hue,
        13 => BitmapBlendingMode::Saturation,
        14 => BitmapBlendingMode::Color,
        15 => BitmapBlendingMode::Luminosity,
        16 => BitmapBlendingMode::Plus,
        _ => return None,
    })
}

fn layer(value: &Value) -> Option<Layer> {
    if value.get("hd").and_then(Value::as_bool) == Some(true) {
        return None;
    }
    let float = |name: &str, default: f64| value.get(name).and_then(Value::as_f64).unwrap_or(default);
    let text = |name: &str| value.get(name).and_then(Value::as_str);
    let kind = match value.get("ty")?.as_u64()? {
        0 => LayerKind::Precomposition {
            reference: text("refId")?.to_owned(),
            width: float("w", 0.0),
            height: float("h", 0.0),
            time_remap: value.get("tm").and_then(Animated::parse),
        },
        1 => LayerKind::Solid {
            color: hex_color(text("sc").unwrap_or("#000000")),
            width: float("sw", 0.0),
            height: float("sh", 0.0),
        },
        2 => LayerKind::Image { reference: text("refId")?.to_owned() },
        3 => LayerKind::Null,
        4 => LayerKind::Shape(shape_items(value.get("shapes"))),
        5 => LayerKind::Text(text_documents(value)),
        _ => LayerKind::NotDrawn,
    };
    let matte = match value.get("tt").and_then(Value::as_u64) {
        Some(1) => Some(MatteMode::Alpha),
        Some(2) => Some(MatteMode::AlphaInverted),
        Some(3) => Some(MatteMode::Luma),
        Some(4) => Some(MatteMode::LumaInverted),
        _ => None,
    };
    let masks = value
        .get("masksProperties")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|mask| {
                    Some(MaskModel {
                        mode: mask.get("mode").and_then(Value::as_str).and_then(|mode| mode.bytes().next()).unwrap_or(b'a'),
                        inverted: mask.get("inv").and_then(Value::as_bool).unwrap_or(false),
                        path: AnimatedShape::parse(mask.get("pt")?)?,
                        opacity: Animated::of(mask, "o", FULL),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let stretch = float("sr", 1.0);
    Some(Layer {
        index: value.get("ind").and_then(Value::as_i64),
        parent: value.get("parent").and_then(Value::as_i64),
        in_point: float("ip", f64::NEG_INFINITY),
        out_point: float("op", f64::INFINITY),
        start_time: float("st", 0.0),
        stretch: if stretch > 0.0 { stretch } else { 1.0 },
        transform: Transform::parse(value.get("ks")),
        matte,
        is_matte: value.get("td").and_then(Value::as_u64).unwrap_or(0) != 0,
        matte_parent: value.get("tp").and_then(Value::as_i64),
        masks,
        kind,
        blend: blend_mode(value.get("bm").and_then(Value::as_u64)),
    })
}

fn layers(value: Option<&Value>) -> Vec<Layer> {
    value.and_then(Value::as_array).map(|items| items.iter().filter_map(layer).collect()).unwrap_or_default()
}

/// A Lottie document, read once. Any frame of it is evaluated with
/// [`evaluate`](Self::evaluate).
#[derive(Clone, Debug, PartialEq)]
pub struct AnimatedVisual {
    /// The size of the composition.
    pub width: f64,
    pub height: f64,
    /// Frames per second.
    pub frame_rate: f64,
    /// The first and the last frame.
    pub in_point: f64,
    pub out_point: f64,
    pub markers: Vec<Marker>,
    pub(crate) layers: Vec<Layer>,
    pub(crate) precompositions: HashMap<String, Vec<Layer>>,
    pub(crate) images: HashMap<String, Rc<ImageAsset>>,
    pub(crate) fonts: DocumentFonts,
}

impl AnimatedVisual {
    /// Reads a document.
    pub fn parse(json: &str) -> Result<Self, AnimatedVisualError> {
        let root: Value = serde_json::from_str(json).map_err(|error| AnimatedVisualError::Json(error.to_string()))?;
        let number = |name: &'static str| {
            root.get(name).and_then(Value::as_f64).filter(|value| value.is_finite()).ok_or(AnimatedVisualError::Invalid(name))
        };
        let (width, height, frame_rate) = (number("w")?, number("h")?, number("fr")?);
        let (in_point, out_point) = (number("ip")?, number("op")?);
        if frame_rate <= 0.0 || out_point < in_point || width < 0.0 || height < 0.0 {
            return Err(AnimatedVisualError::Invalid("size, frame rate or frame range"));
        }
        if !root.get("layers").is_some_and(Value::is_array) {
            return Err(AnimatedVisualError::Invalid("layers"));
        }
        let mut precompositions = HashMap::new();
        let mut images = HashMap::new();
        for asset in root.get("assets").and_then(Value::as_array).into_iter().flatten() {
            if let (Some(id), Some(_)) = (asset.get("id").and_then(Value::as_str), asset.get("layers")) {
                precompositions.insert(id.to_owned(), layers(asset.get("layers")));
            } else if let Some(image) = ImageAsset::parse(asset) {
                images.insert(image.id.clone(), Rc::new(image));
            }
        }
        let markers = root
            .get("markers")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|marker| {
                        Some(Marker {
                            name: marker.get("cm")?.as_str()?.to_owned(),
                            frame: marker.get("tm")?.as_f64()?,
                            duration: marker.get("dr").and_then(Value::as_f64).unwrap_or(0.0),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        let fonts = DocumentFonts::parse(&root);
        Ok(Self { width, height, frame_rate, in_point, out_point, markers, layers: layers(root.get("layers")), precompositions, images, fonts })
    }

    /// The images that are in files, by id and path: a loader gives their bytes to
    /// [`set_image_bytes`](Self::set_image_bytes).
    pub fn image_files(&self) -> Vec<(String, String)> {
        self.images
            .values()
            .filter(|image| image.bytes.is_none())
            .filter_map(|image| Some((image.id.clone(), image.path.clone()?)))
            .collect()
    }

    /// Gives an image that is in a file its encoded bytes.
    pub fn set_image_bytes(&mut self, id: &str, bytes: Vec<u8>) {
        if let Some(image) = self.images.get_mut(id) {
            Rc::make_mut(image).bytes = Some(Rc::from(bytes));
        }
    }

    /// The length of the animation in seconds.
    pub fn duration_seconds(&self) -> f64 {
        (self.out_point - self.in_point) / self.frame_rate
    }

    /// The frame at `progress` of the animation (0 to 1).
    pub fn frame_at(&self, progress: f64) -> f64 {
        // The out point is the first frame after the animation: progress 1 shows the last one.
        let frame = self.in_point + (self.out_point - self.in_point) * progress.clamp(0.0, 1.0);
        if self.out_point > self.in_point { frame.min(self.out_point - LAST_FRAME_MARGIN) } else { frame }
    }

    /// The marker of this name.
    pub fn marker(&self, name: &str) -> Option<&Marker> {
        self.markers.iter().find(|marker| marker.name == name)
    }
}
