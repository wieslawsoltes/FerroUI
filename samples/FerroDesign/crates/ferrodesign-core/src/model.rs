use crate::{
    distance_to_segment, flatten_cubic, point_in_polygon, Point, Rect, Transform, GEOMETRY_EPSILON,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
pub type NodeId = u64;
pub type StyleId = u64;
pub type VariableId = u64;
pub const DOCUMENT_VERSION: u32 = 1;
pub const MAX_DOCUMENT_NODES: usize = 250_000;
pub const MAX_DOCUMENT_DEPTH: usize = 256;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}
impl Color {
    pub const TRANSPARENT: Self = Self::rgba(0, 0, 0, 0);
    pub const WHITE: Self = Self::rgb(255, 255, 255);
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::rgba(r, g, b, 255)
    }
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
    pub fn from_hex(value: &str) -> Option<Self> {
        let s = value.trim().trim_start_matches('#');
        if !s.is_ascii() {
            return None;
        }
        let byte = |a: usize, b: usize| u8::from_str_radix(&s[a..b], 16).ok();
        match s.len() {
            3 | 4 => Some(Self::rgba(
                byte(0, 1)? * 17,
                byte(1, 2)? * 17,
                byte(2, 3)? * 17,
                if s.len() == 4 { byte(3, 4)? * 17 } else { 255 },
            )),
            6 => Some(Self::rgb(byte(0, 2)?, byte(2, 4)?, byte(4, 6)?)),
            8 => Some(Self::rgba(
                byte(0, 2)?,
                byte(2, 4)?,
                byte(4, 6)?,
                byte(6, 8)?,
            )),
            _ => None,
        }
    }
    pub fn hex(self) -> String {
        if self.a == 255 {
            format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
        } else {
            format!("#{:02X}{:02X}{:02X}{:02X}", self.r, self.g, self.b, self.a)
        }
    }
    pub fn css(self) -> String {
        if self.a == 255 {
            self.hex()
        } else {
            format!(
                "rgba({},{},{},{:.4})",
                self.r,
                self.g,
                self.b,
                self.a as f64 / 255.0
            )
        }
    }
    pub fn lerp(self, other: Self, t: f64) -> Self {
        let t = t.clamp(0.0, 1.0);
        let c = |a: u8, b: u8| (a as f64 + (b as f64 - a as f64) * t).round() as u8;
        Self::rgba(
            c(self.r, other.r),
            c(self.g, other.g),
            c(self.b, other.b),
            c(self.a, other.a),
        )
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GradientStop {
    pub offset: f64,
    pub color: Color,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageScale {
    #[default]
    Fill,
    Fit,
    Stretch,
    Tile,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Paint {
    Solid(Color),
    LinearGradient {
        start: Point,
        end: Point,
        stops: Vec<GradientStop>,
    },
    RadialGradient {
        center: Point,
        radius: f64,
        stops: Vec<GradientStop>,
    },
    /// A clockwise sweep around a relative center. Zero degrees points up.
    AngularGradient {
        center: Point,
        angle: f64,
        stops: Vec<GradientStop>,
    },
    Image {
        href: String,
        scale: ImageScale,
        /// Normalized source-image window. Direct crop resolves the visible
        /// cover region and preserves Fill/Stretch while retaining pixel positions.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        crop: Option<Rect>,
    },
}
impl Paint {
    pub fn representative_color(&self) -> Color {
        match self {
            Self::Solid(c) => *c,
            Self::LinearGradient { stops, .. } | Self::RadialGradient { stops, .. }
            | Self::AngularGradient { stops, .. } => {
                stops.first().map(|s| s.color).unwrap_or(Color::TRANSPARENT)
            }
            Self::Image { .. } => Color::rgb(224, 226, 230),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Fill {
    pub paint: Paint,
    pub opacity: f64,
    pub visible: bool,
}
impl Fill {
    pub fn solid(color: Color) -> Self {
        Self {
            paint: Paint::Solid(color),
            opacity: 1.0,
            visible: true,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StrokeAlign {
    Inside,
    #[default]
    Center,
    Outside,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StrokeCap {
    #[default]
    Butt,
    Round,
    Square,
    Arrow,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StrokeJoin {
    #[default]
    Miter,
    Round,
    Bevel,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Stroke {
    pub paint: Paint,
    pub width: f64,
    pub opacity: f64,
    pub visible: bool,
    pub align: StrokeAlign,
    pub cap: StrokeCap,
    pub join: StrokeJoin,
    pub dash: Vec<f64>,
    #[serde(default, skip_serializing_if = "crate::StrokeWidthProfile::is_uniform")]
    pub width_profile: crate::StrokeWidthProfile,
}
impl Stroke {
    pub fn new(color: Color, width: f64) -> Self {
        Self {
            paint: Paint::Solid(color),
            width,
            opacity: 1.0,
            visible: true,
            align: StrokeAlign::Center,
            cap: StrokeCap::Butt,
            join: StrokeJoin::Miter,
            dash: Vec::new(),
            width_profile: crate::StrokeWidthProfile::Uniform,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectKind {
    #[default]
    DropShadow,
    InnerShadow,
    LayerBlur,
    BackgroundBlur,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Effect {
    pub kind: EffectKind,
    pub color: Color,
    pub offset: Point,
    pub radius: f64,
    pub spread: f64,
    pub visible: bool,
}
impl Effect {
    pub fn shadow(color: Color, offset: Point, radius: f64) -> Self {
        Self {
            kind: EffectKind::DropShadow,
            color,
            offset,
            radius,
            spread: 0.0,
            visible: true,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlendMode {
    #[default]
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
    Hue,
    Saturation,
    Color,
    Luminosity,
    /// Children can blend with artwork below this layer outside the group.
    PassThrough,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    Frame,
    Group,
    #[default]
    Rectangle,
    Ellipse,
    Line,
    Polygon,
    Star,
    Text,
    Vector,
    Component,
    Instance,
    Section,
    Boolean,
}
impl NodeKind {
    pub fn is_container(self) -> bool {
        matches!(
            self,
            Self::Frame
                | Self::Group
                | Self::Component
                | Self::Instance
                | Self::Section
                | Self::Boolean
        )
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Frame => "Frame",
            Self::Group => "Group",
            Self::Rectangle => "Rectangle",
            Self::Ellipse => "Ellipse",
            Self::Line => "Line",
            Self::Polygon => "Polygon",
            Self::Star => "Star",
            Self::Text => "Text",
            Self::Vector => "Vector",
            Self::Component => "Component",
            Self::Instance => "Instance",
            Self::Section => "Section",
            Self::Boolean => "Boolean group",
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BooleanOperation {
    #[default]
    Union,
    Subtract,
    Intersect,
    Exclude,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
    Justify,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextVerticalAlign {
    #[default]
    Top,
    Center,
    Bottom,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextDecoration {
    #[default]
    None,
    Underline,
    Strikethrough,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextAutoResize {
    #[default]
    Fixed,
    Height,
    WidthAndHeight,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TextRun {
    pub start: usize,
    pub end: usize,
    pub font_family: Option<String>,
    pub font_size: Option<f64>,
    pub font_weight: Option<u16>,
    pub color: Option<Color>,
    pub italic: Option<bool>,
    pub decoration: Option<TextDecoration>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Constraint {
    #[default]
    Start,
    End,
    Center,
    Stretch,
    Scale,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Constraints {
    pub horizontal: Constraint,
    pub vertical: Constraint,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayoutAxis {
    #[default]
    Row,
    Column,
    Grid,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayoutAlign {
    #[default]
    Start,
    Center,
    End,
    SpaceBetween,
    Stretch,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SizingMode {
    #[default]
    Fixed,
    Hug,
    Fill,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Padding {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}
impl Padding {
    pub const fn all(v: f64) -> Self {
        Self {
            top: v,
            right: v,
            bottom: v,
            left: v,
        }
    }
    pub const fn symmetric(horizontal: f64, vertical: f64) -> Self {
        Self {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AutoLayout {
    pub axis: LayoutAxis,
    pub gap: f64,
    pub cross_gap: f64,
    pub padding: Padding,
    pub main_align: LayoutAlign,
    pub cross_align: LayoutAlign,
    pub wrap: bool,
    pub columns: usize,
    /// Empty lists preserve the original uniform grid. Omitted columns use
    /// equal fractional tracks; additional rows use intrinsic content sizing.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub column_tracks: Vec<crate::GridTrack>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub row_tracks: Vec<crate::GridTrack>,
    pub width: SizingMode,
    pub height: SizingMode,
    pub reverse: bool,
}
impl Default for AutoLayout {
    fn default() -> Self {
        Self {
            axis: LayoutAxis::Row,
            gap: 12.0,
            cross_gap: 12.0,
            padding: Padding::all(16.0),
            main_align: LayoutAlign::Start,
            cross_align: LayoutAlign::Start,
            wrap: false,
            columns: 3,
            column_tracks: Vec::new(),
            row_tracks: Vec::new(),
            width: SizingMode::Fixed,
            height: SizingMode::Hug,
            reverse: false,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GridPlacement {
    /// Zero-based track indices. A missing index uses auto placement.
    pub row: Option<usize>,
    pub column: Option<usize>,
    pub row_span: usize,
    pub column_span: usize,
}
pub const MAX_GRID_COLUMNS: usize = 4096;
pub const MAX_GRID_ROWS: usize = 65_536;
pub const MAX_GRID_CELLS: usize = 1_048_576;
impl Default for GridPlacement {
    fn default() -> Self {
        Self { row: None, column: None, row_span: 1, column_span: 1 }
    }
}
impl GridPlacement {
    pub fn validate(self) -> Result<(), DocumentError> {
        if self.row_span == 0 || self.column_span == 0
            || self.row_span > MAX_GRID_ROWS || self.column_span > MAX_GRID_COLUMNS
            || self.row.is_some_and(|row| row.checked_add(self.row_span).is_none_or(|end| end > MAX_GRID_ROWS))
            || self.column.is_some_and(|column| column.checked_add(self.column_span).is_none_or(|end| end > MAX_GRID_COLUMNS))
        {
            return Err(DocumentError::Invalid("Grid placement requires positive spans within the supported track bounds".into()));
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LayoutChild {
    pub width: SizingMode,
    pub height: SizingMode,
    pub min_width: f64,
    pub max_width: Option<f64>,
    pub min_height: f64,
    pub max_height: Option<f64>,
    pub absolute: bool,
    pub grow: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grid: Option<GridPlacement>,
}
impl Default for LayoutChild {
    fn default() -> Self {
        Self {
            width: SizingMode::Fixed,
            height: SizingMode::Fixed,
            min_width: 0.0,
            max_width: None,
            min_height: 0.0,
            max_height: None,
            absolute: false,
            grow: 1.0,
            grid: None,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FillRule {
    #[default]
    NonZero,
    EvenOdd,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PathSegment {
    MoveTo {
        point: Point,
    },
    LineTo {
        point: Point,
    },
    CubicTo {
        control1: Point,
        control2: Point,
        point: Point,
    },
    QuadraticTo {
        control: Point,
        point: Point,
    },
    Close,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnchorMode {
    #[default]
    Corner,
    Smooth,
    Symmetric,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct VectorPath {
    pub segments: Vec<PathSegment>,
    pub fill_rule: FillRule,
    /// Persistent constraints keyed by the canonical anchor command. Legacy paths
    /// omit this map and retain independent handles. Structural edits remap keys.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub anchor_modes: BTreeMap<usize, AnchorMode>,
}
impl VectorPath {
    pub fn transformed(&self, transform: Transform) -> Self {
        Self {
            fill_rule: self.fill_rule,
            anchor_modes: self.anchor_modes.clone(),
            segments: self
                .segments
                .iter()
                .map(|segment| match *segment {
                    PathSegment::MoveTo { point } => PathSegment::MoveTo {
                        point: transform.apply(point),
                    },
                    PathSegment::LineTo { point } => PathSegment::LineTo {
                        point: transform.apply(point),
                    },
                    PathSegment::QuadraticTo { control, point } => PathSegment::QuadraticTo {
                        control: transform.apply(control),
                        point: transform.apply(point),
                    },
                    PathSegment::CubicTo {
                        control1,
                        control2,
                        point,
                    } => PathSegment::CubicTo {
                        control1: transform.apply(control1),
                        control2: transform.apply(control2),
                        point: transform.apply(point),
                    },
                    PathSegment::Close => PathSegment::Close,
                })
                .collect(),
        }
    }
    pub fn polygon(points: &[Point], closed: bool) -> Self {
        let mut path = Self::default();
        if let Some(first) = points.first() {
            path.segments.push(PathSegment::MoveTo { point: *first });
            for p in &points[1..] {
                path.segments.push(PathSegment::LineTo { point: *p })
            }
            if closed {
                path.segments.push(PathSegment::Close)
            }
        }
        path
    }
    pub fn flattened(&self, tolerance: f64) -> Vec<Vec<Point>> {
        let mut result = Vec::new();
        let mut points = Vec::new();
        let mut current = Point::default();
        let mut start = current;
        for segment in &self.segments {
            match *segment {
                PathSegment::MoveTo { point } => {
                    if !points.is_empty() {
                        result.push(std::mem::take(&mut points))
                    }
                    points.push(point);
                    current = point;
                    start = point
                }
                PathSegment::LineTo { point } => {
                    points.push(point);
                    current = point
                }
                PathSegment::CubicTo {
                    control1,
                    control2,
                    point,
                } => {
                    flatten_cubic(current, control1, control2, point, tolerance, &mut points);
                    current = point
                }
                PathSegment::QuadraticTo { control, point } => {
                    let c1 = current + (control - current) * (2.0 / 3.0);
                    let c2 = point + (control - point) * (2.0 / 3.0);
                    flatten_cubic(current, c1, c2, point, tolerance, &mut points);
                    current = point
                }
                PathSegment::Close => {
                    if points.last() != Some(&start) {
                        points.push(start)
                    }
                    current = start
                }
            }
        }
        if !points.is_empty() {
            result.push(points)
        }
        result
    }
    pub fn bounds(&self) -> Option<Rect> {
        self.flattened(0.05)
            .iter()
            .flatten()
            .copied()
            .map(|p| Rect::new(p.x, p.y, 0.0, 0.0))
            .reduce(|a, b| a.union(b))
    }
    pub fn contains(&self, p: Point) -> bool {
        crate::geometry::path_contours_contain(self.fill_rule, &self.flattened(0.35), p)
    }
}
/// Editor-only layout guide. Its effective values travel with the frame,
/// so detaching or deleting a local grid style preserves the authored grid.
/// Grid overlays do not enter artwork exports or prototype presentations.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LayoutGrid {
    pub size: f64,
    pub color: Color,
    pub visible: bool,
    #[serde(default, skip_serializing_if = "crate::LayoutGridPattern::is_square")]
    pub pattern: crate::LayoutGridPattern,
}
impl Default for LayoutGrid {
    fn default() -> Self {
        Self { size: 8.0, color: Color::rgba(255, 0, 0, 26), visible: true, pattern: crate::LayoutGridPattern::Square }
    }
}
impl LayoutGrid {
    pub fn supports(kind: NodeKind) -> bool {
        matches!(kind, NodeKind::Frame | NodeKind::Component | NodeKind::Instance)
    }
    pub fn validate(self) -> Result<(), DocumentError> {
        if !self.size.is_finite() || self.size <= 0.0 {
            return Err(DocumentError::Invalid("Layout grid size must be positive and finite.".into()));
        }
        self.pattern.validate()?;
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Node {
    #[serde(skip)]
    pub revision: u64,
    pub id: NodeId,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub kind: NodeKind,
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub rotation: f64,
    pub flip_x: bool,
    pub flip_y: bool,
    /// Residual local affine transform for imported scale/shear geometry. Normal
    /// editor layers use `None`; the matrix composes after position/rotation.
    pub affine_transform: Option<Transform>,
    pub visible: bool,
    pub locked: bool,
    pub opacity: f64,
    pub blend_mode: BlendMode,
    /// A mask affects subsequent siblings within this parent; the source is
    /// retained as an independently editable layer and is not ordinary paint.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_mask: bool,
    #[serde(default, skip_serializing_if = "crate::MaskType::is_alpha")]
    pub mask_type: crate::MaskType,
    pub clip_content: bool,
    pub prototype_overflow: PrototypeOverflow,
    pub fills: Vec<Fill>,
    pub strokes: Vec<Stroke>,
    pub effects: Vec<Effect>,
    pub corner_radius: f64,
    pub corner_radii: Option<[f64; 4]>,
    /// Continuous cubic transitions; zero retains ordinary circular corners.
    #[serde(default, skip_serializing_if = "crate::corners::is_zero")]
    pub corner_smoothing: f64,
    pub text: String,
    pub font_family: String,
    pub font_size: f64,
    pub font_weight: u16,
    pub italic: bool,
    pub line_height: f64,
    pub letter_spacing: f64,
    pub text_align: TextAlign,
    pub text_vertical_align: TextVerticalAlign,
    pub text_decoration: TextDecoration,
    pub text_auto_resize: TextAutoResize,
    pub text_runs: Vec<TextRun>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub text_paragraphs: Vec<crate::TextParagraph>,
    /// Omitted legacy documents retain the original left-to-right paragraph.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_direction: Option<crate::TextDirection>,
    /// Explicit whole-layer OpenType settings; absent tags use font defaults.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub font_features: Vec<crate::FontFeatureSetting>,
    pub polygon_sides: usize,
    pub star_inner_radius: f64,
    pub path: VectorPath,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vector_network: Option<crate::VectorNetwork>,
    pub boolean_operation: BooleanOperation,
    pub constraints: Constraints,
    pub layout: Option<AutoLayout>,
    pub layout_child: LayoutChild,
    pub component_id: Option<NodeId>,
    pub component_source: Option<NodeId>,
    pub overridden_properties: BTreeSet<String>,
    pub variant_properties: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub component_property_definitions: BTreeMap<String, crate::ComponentPropertyDefinition>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub component_property_values: BTreeMap<String, crate::ComponentPropertyValue>,
    pub fill_style: Option<StyleId>,
    pub text_style: Option<StyleId>,
    pub effect_style: Option<StyleId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grid_style: Option<StyleId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout_grid: Option<LayoutGrid>,
    /// Additional guides, in paint order after the legacy primary guide. Keeping
    /// the original field preserves existing square-grid documents verbatim.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layout_grid_overlays: Vec<LayoutGrid>,
    pub variable_bindings: BTreeMap<String, VariableId>,
    pub interactions: Vec<Interaction>,
    pub export_settings: Vec<ExportSetting>,
    pub metadata: BTreeMap<String, String>,
}
impl Default for Node {
    fn default() -> Self {
        Self {
            revision: 0,
            id: 0,
            parent: None,
            children: Vec::new(),
            kind: NodeKind::Rectangle,
            name: "Rectangle".into(),
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
            rotation: 0.0,
            flip_x: false,
            flip_y: false,
            affine_transform: None,
            visible: true,
            locked: false,
            opacity: 1.0,
            blend_mode: BlendMode::Normal,
            is_mask: false,
            mask_type: crate::MaskType::Alpha,
            clip_content: false,
            prototype_overflow: PrototypeOverflow::None,
            fills: vec![Fill::solid(Color::rgb(217, 217, 217))],
            strokes: Vec::new(),
            effects: Vec::new(),
            corner_radius: 0.0,
            corner_radii: None,
            corner_smoothing: 0.0,
            text: String::new(),
            font_family: "Inter".into(),
            font_size: 14.0,
            font_weight: 400,
            italic: false,
            line_height: 1.4,
            letter_spacing: 0.0,
            text_align: TextAlign::Left,
            text_vertical_align: TextVerticalAlign::Top,
            text_decoration: TextDecoration::None,
            text_auto_resize: TextAutoResize::Fixed,
            text_runs: Vec::new(),
            text_paragraphs: Vec::new(),
            text_direction: None,
            font_features: Vec::new(),
            polygon_sides: 3,
            star_inner_radius: 0.45,
            path: VectorPath::default(),
            vector_network: None,
            boolean_operation: BooleanOperation::Union,
            constraints: Constraints::default(),
            layout: None,
            layout_child: LayoutChild::default(),
            component_id: None,
            component_source: None,
            overridden_properties: BTreeSet::new(),
            variant_properties: BTreeMap::new(),
            component_property_definitions: BTreeMap::new(),
            component_property_values: BTreeMap::new(),
            fill_style: None,
            text_style: None,
            effect_style: None,
            grid_style: None,
            layout_grid: None,
            layout_grid_overlays: Vec::new(),
            variable_bindings: BTreeMap::new(),
            interactions: Vec::new(),
            export_settings: Vec::new(),
            metadata: BTreeMap::new(),
        }
    }
}
impl Node {
    pub fn new(
        kind: NodeKind,
        name: impl Into<String>,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> Self {
        let mut n = Self {
            kind,
            name: name.into(),
            x,
            y,
            width: width.max(0.0),
            height: height.max(0.0),
            ..Self::default()
        };
        if matches!(
            kind,
            NodeKind::Group | NodeKind::Vector | NodeKind::Line | NodeKind::Boolean
        ) {
            n.fills.clear()
        }
        if kind == NodeKind::Text {
            n.fills = vec![Fill::solid(Color::BLACK)];
            n.text = "Text".into()
        }
        if kind == NodeKind::Line {
            n.strokes = vec![Stroke::new(Color::BLACK, 1.0)]
        }
        // New groups pass child blends through to their backdrop. Keep the
        // deserialization/default value Normal so existing documents retain
        // the isolation behavior they were authored with.
        if kind == NodeKind::Group {
            n.blend_mode = BlendMode::PassThrough;
        }
        if kind == NodeKind::Frame {
            n.fills = vec![Fill::solid(Color::WHITE)];
            n.clip_content = true
        }
        n
    }
    pub fn with_fill(mut self, color: Color) -> Self {
        self.fills = vec![Fill::solid(color)];
        self
    }
    /// Inside/outside alignment needs closed stroke boundaries. Open paths and
    /// branching graphs retain a centered stroke in both native and SVG output.
    pub fn has_closed_stroke_contours(&self) -> bool {
        if let Some(network) = &self.vector_network {
            let mut degrees = BTreeMap::new();
            for edge in network.edges.values().filter(|edge| edge.stroke) {
                *degrees.entry(edge.start).or_insert(0usize) += 1;
                *degrees.entry(edge.end).or_insert(0usize) += 1;
            }
            return !network.regions.is_empty() && !degrees.is_empty() && degrees.values().all(|degree| *degree == 2);
        }
        if self.kind == NodeKind::Line { return false; }
        if self.kind != NodeKind::Vector { return true; }
        let mut open = false;
        for segment in &self.path.segments {
            match segment {
                PathSegment::MoveTo { .. } => { if open { return false; } open = true; }
                PathSegment::Close => open = false,
                _ => {}
            }
        }
        !open && !self.path.segments.is_empty()
    }
    pub fn with_text(mut self, text: impl Into<String>, font_size: f64, color: Color) -> Self {
        self.text = text.into();
        self.font_size = font_size;
        self.fills = vec![Fill::solid(color)];
        self
    }
    pub fn with_radius(mut self, radius: f64) -> Self {
        self.corner_radius = radius;
        self
    }
    pub fn with_weight(mut self, weight: u16) -> Self {
        self.font_weight = weight;
        self
    }
    pub fn bounds(&self) -> Rect {
        Rect::new(self.x, self.y, self.width, self.height)
    }
    pub fn local_bounds(&self) -> Rect {
        Rect::new(0.0, 0.0, self.width, self.height)
    }
    pub fn local_transform(&self) -> Transform {
        let cx = self.width / 2.0;
        let cy = self.height / 2.0;
        Transform::translation(self.x + cx, self.y + cy)
            * Transform::rotation(self.rotation)
            * Transform::scale(
                if self.flip_x { -1.0 } else { 1.0 },
                if self.flip_y { -1.0 } else { 1.0 },
            )
            * Transform::translation(-cx, -cy)
            * self.affine_transform.unwrap_or(Transform::IDENTITY)
    }
    /// Assign an arbitrary affine transform without flattening child geometry.
    /// Rotation and reflection remain editable; scale and shear are retained in
    /// the residual matrix. Translation is represented by the usual x/y fields.
    pub fn set_local_transform(&mut self, transform: Transform) {
        self.rotation = if transform.a.hypot(transform.b) > GEOMETRY_EPSILON {
            transform.b.atan2(transform.a).to_degrees()
        } else {
            (-transform.c).atan2(transform.d).to_degrees()
        };
        self.flip_x = false;
        self.flip_y = transform.a * transform.d - transform.b * transform.c < 0.0;
        let rigid = Transform::rotation(self.rotation)
            * Transform::scale(1.0, if self.flip_y { -1.0 } else { 1.0 });
        let mut linear = transform;
        linear.e = 0.0;
        linear.f = 0.0;
        let residual = rigid.inverse().unwrap_or(Transform::IDENTITY) * linear;
        self.affine_transform = if (residual.a - 1.0).abs() < 1e-12
            && residual.b.abs() < 1e-12
            && residual.c.abs() < 1e-12
            && (residual.d - 1.0).abs() < 1e-12
        {
            None
        } else {
            Some(residual)
        };
        let center = Point::new(self.width / 2.0, self.height / 2.0);
        let rotated = rigid.apply(center);
        self.x = transform.e - center.x + rotated.x;
        self.y = transform.f - center.y + rotated.y;
    }
    pub fn fill_color(&self) -> Color {
        self.fills
            .iter()
            .find(|f| f.visible)
            .map(|f| f.paint.representative_color())
            .unwrap_or(Color::TRANSPARENT)
    }
    pub fn polygon_points(&self) -> Vec<Point> {
        let sides = self.polygon_sides.clamp(3, 128);
        let count = if self.kind == NodeKind::Star {
            sides * 2
        } else {
            sides
        };
        (0..count)
            .map(|i| {
                let a =
                    -std::f64::consts::FRAC_PI_2 + i as f64 * std::f64::consts::TAU / count as f64;
                let r = if self.kind == NodeKind::Star && i % 2 == 1 {
                    self.star_inner_radius.clamp(0.0, 1.0)
                } else {
                    1.0
                };
                Point::new(
                    self.width / 2.0 + a.cos() * self.width / 2.0 * r,
                    self.height / 2.0 + a.sin() * self.height / 2.0 * r,
                )
            })
            .collect()
    }
    pub fn hit_test_local(&self, p: Point, tolerance: f64) -> bool {
        let widths = crate::stroke_hit::VariableStrokeHits::prepare(self);
        self.hit_test_local_prepared(p, tolerance, widths.as_ref())
    }
    pub(crate) fn hit_test_local_prepared(&self, p: Point, tolerance: f64,
        widths: Option<&crate::stroke_hit::VariableStrokeHits>) -> bool
    {
        if widths.is_some_and(|widths| widths.hit(p, tolerance)) { return true; }
        let stroke = self
            .strokes
            .iter()
            .filter(|s| s.visible && s.width_profile.is_uniform())
            .map(|s| s.width)
            .fold(0.0, f64::max)
            / 2.0
            + tolerance;
        if !self.local_bounds().inflate(stroke).contains(p) {
            return false;
        }
        match self.kind {
            NodeKind::Ellipse => {
                let rx = self.width / 2.0 + stroke;
                let ry = self.height / 2.0 + stroke;
                if rx <= 0.0 || ry <= 0.0 {
                    return false;
                }
                ((p.x - self.width / 2.0) / rx).powi(2) + ((p.y - self.height / 2.0) / ry).powi(2)
                    <= 1.0
            }
            NodeKind::Line => {
                distance_to_segment(p, Point::default(), Point::new(self.width, self.height))
                    <= stroke.max(tolerance)
            }
            NodeKind::Polygon | NodeKind::Star => {
                if self.corner_radius > 0.0 {
                    let path = crate::polygon_corner_path(self);
                    let contours = path.flattened(0.1);
                    return crate::geometry::path_contours_contain(path.fill_rule, &contours, p)
                        || contours.iter().any(|contour| contour.windows(2)
                            .any(|edge| distance_to_segment(p, edge[0], edge[1]) <= stroke));
                }
                let ps = self.polygon_points();
                point_in_polygon(p, &ps)
                    || ps
                        .iter()
                        .enumerate()
                        .any(|(i, a)| distance_to_segment(p, *a, ps[(i + 1) % ps.len()]) <= stroke)
            }
            NodeKind::Vector => {
                if let Some(network) = &self.vector_network {
                    return network.hit_test_with_fills(p, stroke, &self.fills);
                }
                self.path.contains(p)
                    || self.path.flattened(0.35).iter().any(|c| {
                        c.windows(2)
                            .any(|s| distance_to_segment(p, s[0], s[1]) <= stroke)
                    })
            }
            _ => {
                if !self.local_bounds().inflate(tolerance).contains(p) {
                    return false;
                }
                if self.corner_smoothing > 0.0 {
                    let path = crate::corner_path(self);
                    return path.contains(p) || (tolerance > 0.0 && path.flattened(0.1).iter().any(|c|
                        c.windows(2).any(|s| distance_to_segment(p, s[0], s[1]) <= tolerance)));
                }
                let rs = crate::normalized_corner_radii(self.width, self.height,
                    self.corner_radii.unwrap_or([self.corner_radius; 4]));
                let corners = [
                    (rs[0], Point::new(rs[0], rs[0]), p.x < rs[0] && p.y < rs[0]),
                    (
                        rs[1],
                        Point::new(self.width - rs[1], rs[1]),
                        p.x > self.width - rs[1] && p.y < rs[1],
                    ),
                    (
                        rs[2],
                        Point::new(self.width - rs[2], self.height - rs[2]),
                        p.x > self.width - rs[2] && p.y > self.height - rs[2],
                    ),
                    (
                        rs[3],
                        Point::new(rs[3], self.height - rs[3]),
                        p.x < rs[3] && p.y > self.height - rs[3],
                    ),
                ];
                for (r, c, test) in corners {
                    if r > 0.0 && test && p.distance(c) > r + tolerance {
                        return false;
                    }
                }
                true
            }
        }
    }
    pub(crate) fn contains_clip_point(&self, point: Point) -> bool {
        self.local_bounds().contains(point) && crate::corner_path(self).contains(point)
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Page {
    pub id: NodeId,
    pub name: String,
    pub children: Vec<NodeId>,
    pub background: Color,
    pub prototype_start: Option<NodeId>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StyleValue {
    Fill {
        fills: Vec<Fill>,
    },
    Text {
        family: String,
        size: f64,
        weight: u16,
        line_height: f64,
        letter_spacing: f64,
    },
    Effect {
        effects: Vec<Effect>,
    },
    Grid {
        size: f64,
        color: Color,
    },
    GridStack {
        grids: Vec<LayoutGrid>,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Style {
    pub id: StyleId,
    pub name: String,
    pub description: String,
    pub value: StyleValue,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum VariableValue {
    Color(Color),
    Number(f64),
    String(String),
    Boolean(bool),
    Alias(VariableId),
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Variable {
    pub id: VariableId,
    pub name: String,
    pub collection: String,
    pub values: BTreeMap<String, VariableValue>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrototypeOverflow {
    #[default]
    None,
    Horizontal,
    Vertical,
    Both,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Trigger {
    #[default]
    Click,
    Hover,
    Press,
    Drag,
    AfterDelay,
    Key,
    WhileHover,
    WhilePress,
    MouseLeave,
    Release,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitionKind {
    #[default]
    Instant,
    Dissolve,
    SmartAnimate,
    SlideLeft,
    SlideRight,
    SlideUp,
    SlideDown,
    Push,
    MoveIn,
    MoveOut,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Easing {
    Linear,
    EaseIn,
    EaseOut,
    #[default]
    EaseInOut,
    Spring,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Transition {
    pub kind: TransitionKind,
    pub duration_ms: u32,
    pub easing: Easing,
}
impl Default for Transition {
    fn default() -> Self {
        Self {
            kind: TransitionKind::Dissolve,
            duration_ms: 300,
            easing: Easing::EaseInOut,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PrototypeAction {
    NavigateTo {
        target: NodeId,
    },
    OpenOverlay {
        target: NodeId,
        position: Point,
    },
    CloseOverlay,
    Back,
    ScrollTo {
        target: NodeId,
    },
    SetVariable {
        variable: VariableId,
        value: VariableValue,
    },
    OpenUrl {
        url: String,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Interaction {
    pub trigger: Trigger,
    pub action: PrototypeAction,
    pub transition: Transition,
    pub delay_ms: u32,
    pub key: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExportSetting {
    pub format: String,
    pub scale: f64,
    pub suffix: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommentMessage {
    pub author: String,
    pub body: String,
    pub created_at: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommentThread {
    pub id: u64,
    pub node: Option<NodeId>,
    pub position: Point,
    pub resolved: bool,
    pub messages: Vec<CommentMessage>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Guide {
    pub horizontal: bool,
    pub position: f64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Document {
    #[serde(skip)]
    pub revision: u64,
    pub schema_version: u32,
    pub name: String,
    pub pages: Vec<Page>,
    pub nodes: BTreeMap<NodeId, Node>,
    pub styles: BTreeMap<StyleId, Style>,
    pub variables: BTreeMap<VariableId, Variable>,
    pub variable_mode: String,
    pub comments: Vec<CommentThread>,
    pub guides: BTreeMap<NodeId, Vec<Guide>>,
    pub next_id: u64,
}
impl Default for Document {
    fn default() -> Self {
        Self::new("Untitled")
    }
}
impl Document {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            revision: 0,
            schema_version: DOCUMENT_VERSION,
            name: name.into(),
            pages: vec![Page {
                id: 1,
                name: "Page 1".into(),
                children: Vec::new(),
                background: Color::rgb(30, 30, 30),
                prototype_start: None,
            }],
            nodes: BTreeMap::new(),
            styles: BTreeMap::new(),
            variables: BTreeMap::new(),
            variable_mode: "Default".into(),
            comments: Vec::new(),
            guides: BTreeMap::new(),
            next_id: 2,
        }
    }
    pub fn allocate_id(&mut self) -> NodeId {
        while self.nodes.contains_key(&self.next_id)
            || self.pages.iter().any(|p| p.id == self.next_id)
            || self.styles.contains_key(&self.next_id)
            || self.variables.contains_key(&self.next_id)
            || self.comments.iter().any(|c| c.id == self.next_id)
        {
            self.next_id = self.next_id.checked_add(1).expect("ID space exhausted")
        }
        let id = self.next_id;
        self.next_id = self.next_id.checked_add(1).expect("ID space exhausted");
        id
    }
    pub fn page(&self, id: NodeId) -> Option<&Page> {
        self.pages.iter().find(|p| p.id == id)
    }
    pub fn page_mut(&mut self, id: NodeId) -> Option<&mut Page> {
        self.pages.iter_mut().find(|p| p.id == id)
    }
    pub fn add_page(&mut self, name: impl Into<String>) -> NodeId {
        let id = self.allocate_id();
        self.pages.push(Page {
            id,
            name: name.into(),
            children: Vec::new(),
            background: Color::rgb(30, 30, 30),
            prototype_start: None,
        });
        id
    }
    pub fn children(&self, parent: NodeId) -> Option<&[NodeId]> {
        self.nodes
            .get(&parent)
            .map(|n| n.children.as_slice())
            .or_else(|| self.page(parent).map(|p| p.children.as_slice()))
    }
    pub fn children_mut(&mut self, parent: NodeId) -> Option<&mut Vec<NodeId>> {
        if self.nodes.contains_key(&parent) {
            self.nodes.get_mut(&parent).map(|n| &mut n.children)
        } else {
            self.page_mut(parent).map(|p| &mut p.children)
        }
    }
    pub fn insert_node(&mut self, parent: NodeId, mut node: Node) -> Result<NodeId, DocumentError> {
        if self.is_component_set(parent) && node.kind != NodeKind::Component {
            return Err(DocumentError::InvalidParent(parent));
        }
        if self.children(parent).is_none()
            || self
                .nodes
                .get(&parent)
                .is_some_and(|n| !n.kind.is_container())
        {
            return Err(DocumentError::InvalidParent(parent));
        }
        if !node.children.is_empty() {
            return Err(DocumentError::Invalid(
                "new nodes must not contain existing children".into(),
            ));
        }
        if self.nodes.len() >= MAX_DOCUMENT_NODES {
            return Err(DocumentError::Invalid("node limit exceeded".into()));
        }
        let id = self.allocate_id();
        node.id = id;
        node.parent = Some(parent);
        self.nodes.insert(id, node);
        self.children_mut(parent).unwrap().push(id);
        Ok(id)
    }
    pub fn descendants(&self, id: NodeId) -> Vec<NodeId> {
        let mut out = Vec::new();
        let mut stack = vec![id];
        let mut seen = BTreeSet::new();
        while let Some(id) = stack.pop() {
            if !seen.insert(id) {
                continue;
            }
            if let Some(n) = self.nodes.get(&id) {
                out.push(id);
                stack.extend(n.children.iter().rev().copied())
            }
        }
        out
    }
    pub fn draw_order(&self, page: NodeId) -> Vec<NodeId> {
        let mut out = Vec::new();
        if let Some(p) = self.page(page) {
            for id in &p.children {
                out.extend(self.descendants(*id))
            }
        }
        out
    }
    pub fn is_ancestor(&self, ancestor: NodeId, mut id: NodeId) -> bool {
        for _ in 0..MAX_DOCUMENT_DEPTH {
            let Some(parent) = self.nodes.get(&id).and_then(|n| n.parent) else {
                return false;
            };
            if parent == ancestor {
                return true;
            }
            id = parent
        }
        false
    }
    pub fn page_of(&self, mut id: NodeId) -> Option<NodeId> {
        for _ in 0..MAX_DOCUMENT_DEPTH {
            if self.page(id).is_some() {
                return Some(id);
            }
            id = self.nodes.get(&id)?.parent?
        }
        None
    }
    /// Resolve a comment's page from its current layer or page anchor. Legacy
    /// unanchored comments belong to the first page; missing anchors stay hidden.
    pub fn comment_page(&self, comment: &CommentThread) -> Option<NodeId> {
        match comment.node {
            Some(anchor) => self.page_of(anchor),
            None => self.pages.first().map(|page| page.id),
        }
    }
    pub fn world_transform(&self, id: NodeId) -> Option<Transform> {
        let mut current = Some(id);
        let mut chain = Vec::new();
        while let Some(id) = current {
            if chain.len() >= MAX_DOCUMENT_DEPTH {
                return None;
            }
            if self.page(id).is_some() {
                break;
            }
            let node = self.nodes.get(&id)?;
            chain.push(node.local_transform());
            current = node.parent
        }
        Some(chain.iter().rev().fold(Transform::IDENTITY, |p, l| p * *l))
    }
    pub fn world_bounds(&self, id: NodeId) -> Option<Rect> {
        let n = self.nodes.get(&id)?;
        Some(self.world_transform(id)?.transform_rect(n.local_bounds()))
    }
    pub fn world_origin(&self, id: NodeId) -> Option<Point> {
        Some(self.world_transform(id)?.apply(Point::default()))
    }
    pub fn effective_visible(&self, mut id: NodeId) -> bool {
        for _ in 0..MAX_DOCUMENT_DEPTH {
            if self.page(id).is_some() {
                return true;
            }
            let Some(n) = self.nodes.get(&id) else {
                return false;
            };
            if !n.visible {
                return false;
            }
            let Some(parent) = n.parent else { return true };
            id = parent
        }
        false
    }
    pub fn effective_locked(&self, mut id: NodeId) -> bool {
        for _ in 0..MAX_DOCUMENT_DEPTH {
            let Some(n) = self.nodes.get(&id) else {
                return false;
            };
            if n.locked {
                return true;
            }
            let Some(parent) = n.parent else { return false };
            id = parent
        }
        true
    }
    pub fn hit_test_node(&self, id: NodeId, world: Point, tolerance: f64) -> bool {
        fn hit(d: &Document, id: NodeId, p: Point, t: f64, depth: usize) -> bool {
            if depth >= MAX_DOCUMENT_DEPTH {
                return false;
            }
            let Some(n) = d.nodes.get(&id) else {
                return false;
            };
            if !n.visible {
                return false;
            }
            if n.kind == NodeKind::Boolean {
                let values: Vec<_> = n
                    .children
                    .iter()
                    .filter(|id| d.nodes.get(id).is_some_and(|n| n.visible))
                    .map(|id| hit(d, *id, p, t, depth + 1))
                    .collect();
                return match n.boolean_operation {
                    BooleanOperation::Union => values.iter().any(|v| *v),
                    BooleanOperation::Subtract => {
                        values.first().copied().unwrap_or(false)
                            && !values.iter().skip(1).any(|v| *v)
                    }
                    BooleanOperation::Intersect => !values.is_empty() && values.iter().all(|v| *v),
                    BooleanOperation::Exclude => values.iter().filter(|v| **v).count() % 2 == 1,
                };
            }
            let Some(inv) = d.world_transform(id).and_then(|t| t.inverse()) else {
                return false;
            };
            n.hit_test_local(inv.apply(p), t)
        }
        hit(self, id, world, tolerance, 0)
    }
    pub fn remove_subtree(&mut self, id: NodeId) -> Vec<Node> {
        let ids = self.descendants(id);
        if let Some(parent) = self.nodes.get(&id).and_then(|n| n.parent) {
            if let Some(children) = self.children_mut(parent) {
                children.retain(|child| *child != id)
            }
        }
        let set: BTreeSet<_> = ids.iter().copied().collect();
        let removed: Vec<_> = ids.iter().filter_map(|id| self.nodes.remove(id)).collect();
        for n in self.nodes.values_mut() {
            n.interactions.retain(|i| match i.action {
                PrototypeAction::NavigateTo { target }
                | PrototypeAction::OpenOverlay { target, .. }
                | PrototypeAction::ScrollTo { target } => !set.contains(&target),
                _ => true,
            });
            if n.component_id.is_some_and(|id| set.contains(&id)) {
                n.component_id = None;
                n.component_source = None;
                if n.kind == NodeKind::Instance {
                    n.kind = NodeKind::Frame
                }
            }
        }
        for p in &mut self.pages {
            if p.prototype_start.is_some_and(|id| set.contains(&id)) {
                p.prototype_start = None
            }
        }
        self.comments
            .retain(|c| !c.node.is_some_and(|id| set.contains(&id)));
        self.repair_component_references();
        removed
    }
    pub fn reparent(
        &mut self,
        id: NodeId,
        parent: NodeId,
        index: usize,
    ) -> Result<(), DocumentError> {
        if id == parent || self.is_ancestor(id, parent) {
            return Err(DocumentError::Cycle(id));
        }
        if self.is_component_set(parent) && !self.nodes.get(&id).is_some_and(|node| node.kind == NodeKind::Component) {
            return Err(DocumentError::InvalidParent(parent));
        }
        if self.children(parent).is_none()
            || self
                .nodes
                .get(&parent)
                .is_some_and(|n| !n.kind.is_container())
        {
            return Err(DocumentError::InvalidParent(parent));
        }
        let old = self
            .nodes
            .get(&id)
            .ok_or(DocumentError::MissingNode(id))?
            .parent;
        if let Some(old) = old {
            if let Some(children) = self.children_mut(old) {
                children.retain(|v| *v != id)
            }
        }
        self.nodes.get_mut(&id).unwrap().parent = Some(parent);
        let children = self.children_mut(parent).unwrap();
        children.insert(index.min(children.len()), id);
        self.repair_component_references();
        Ok(())
    }
    pub fn resolve_variable(&self, id: VariableId, mode: &str) -> Option<&VariableValue> {
        let mut id = id;
        let mut seen = BTreeSet::new();
        while seen.insert(id) {
            let v = self.variables.get(&id)?;
            let value = v
                .values
                .get(mode)
                .or_else(|| v.values.get("Default"))
                .or_else(|| v.values.values().next())?;
            match value {
                VariableValue::Alias(next) => id = *next,
                _ => return Some(value),
            }
        }
        None
    }
    pub fn apply_variables(&mut self) {
        let mut changes = Vec::new();
        for n in self.nodes.values() {
            for (property, id) in &n.variable_bindings {
                if let Some(value) = self.resolve_variable(*id, &self.variable_mode) {
                    changes.push((n.id, property.clone(), value.clone()))
                }
            }
        }
        for (id, property, value) in changes {
            let Some(n) = self.nodes.get_mut(&id) else {
                continue;
            };
            if crate::apply_layout_guide_variable(n, &property, &value) { continue; }
            if crate::layout_variables::apply_layout_variable(n, &property, &value) { continue; }
            match (property.as_str(), value) {
                ("fill", VariableValue::Color(c)) => n.fills = vec![Fill::solid(c)],
                ("stroke", VariableValue::Color(c)) => {
                    if let Some(s) = n.strokes.first_mut() {
                        s.paint = Paint::Solid(c)
                    }
                }
                ("opacity", VariableValue::Number(v)) => n.opacity = v.clamp(0.0, 1.0),
                ("width", VariableValue::Number(v)) => n.width = v.max(0.0),
                ("height", VariableValue::Number(v)) => n.height = v.max(0.0),
                ("corner_radius", VariableValue::Number(v)) => n.corner_radius = v.max(0.0),
                ("font_size", VariableValue::Number(v)) => n.font_size = v.max(1.0),
                ("visible", VariableValue::Boolean(v)) => n.visible = v,
                ("text", VariableValue::String(v)) => {
                    n.replace_text_preserving_runs(v);
                }
                _ => {}
            }
        }
    }
    pub fn to_json(&self) -> Result<String, DocumentError> {
        serde_json::to_string_pretty(self).map_err(|e| DocumentError::Json(e.to_string()))
    }
    pub fn from_json(value: &str) -> Result<Self, DocumentError> {
        if value.len() > 64 * 1024 * 1024 {
            return Err(DocumentError::Invalid("document exceeds 64 MiB".into()));
        }
        let mut d: Self =
            serde_json::from_str(value).map_err(|e| DocumentError::Json(e.to_string()))?;
        d.validate()?;
        let max = d
            .nodes
            .keys()
            .chain(d.styles.keys())
            .chain(d.variables.keys())
            .copied()
            .chain(d.pages.iter().map(|p| p.id))
            .chain(d.comments.iter().map(|c| c.id))
            .max()
            .unwrap_or(1);
        d.next_id = d.next_id.max(
            max.checked_add(1)
                .ok_or_else(|| DocumentError::Invalid("ID space exhausted".into()))?,
        );
        Ok(d)
    }
    pub fn validate(&self) -> Result<(), DocumentError> {
        if self.schema_version != DOCUMENT_VERSION {
            return Err(DocumentError::UnsupportedVersion(self.schema_version));
        }
        if self.pages.is_empty() {
            return Err(DocumentError::Invalid("document has no pages".into()));
        }
        if self.nodes.len() > MAX_DOCUMENT_NODES {
            return Err(DocumentError::Invalid("node limit exceeded".into()));
        }
        let mut seen = BTreeSet::new();
        let mut page_ids = BTreeSet::new();
        let mut stack = Vec::new();
        for p in &self.pages {
            if p.id == 0 || !page_ids.insert(p.id) || self.nodes.contains_key(&p.id) {
                return Err(DocumentError::Invalid(
                    "duplicate or invalid page ID".into(),
                ));
            }
            for child in &p.children {
                stack.push((*child, p.id, 0usize))
            }
            if p.prototype_start
                .is_some_and(|id| !self.nodes.contains_key(&id))
            {
                return Err(DocumentError::Invalid("prototype start is missing".into()));
            }
        }
        while let Some((id, parent, depth)) = stack.pop() {
            if depth >= MAX_DOCUMENT_DEPTH {
                return Err(DocumentError::Invalid(
                    "hierarchy depth limit exceeded".into(),
                ));
            }
            if !seen.insert(id) {
                return Err(DocumentError::Cycle(id));
            }
            let n = self.nodes.get(&id).ok_or(DocumentError::MissingNode(id))?;
            if n.id != id || id == 0 {
                return Err(DocumentError::Invalid(
                    "node ID does not match map key".into(),
                ));
            }
            if n.parent != Some(parent) {
                return Err(DocumentError::InvalidParent(id));
            }
            if !n.bounds().is_finite()
                || n.width < 0.0
                || n.height < 0.0
                || !n.rotation.is_finite()
                || n.affine_transform
                    .is_some_and(|t| ![t.a, t.b, t.c, t.d, t.e, t.f].iter().all(|v| v.is_finite()))
                || !n.opacity.is_finite()
                || !(0.0..=1.0).contains(&n.opacity)
            {
                return Err(DocumentError::Invalid(format!(
                    "invalid geometry on node {id}"
                )));
            }
            if n.layout_grid.is_some() || !n.layout_grid_overlays.is_empty() {
                if !LayoutGrid::supports(n.kind) {
                    return Err(DocumentError::Invalid(format!("Layout grids require a frame, component or instance on node {id}")));
                }
                if n.layout_guide_count() > crate::MAX_LAYOUT_GUIDES {
                    return Err(DocumentError::Invalid(format!("Too many layout guides on node {id}")));
                }
                for grid in n.layout_guides() {
                    grid.validate().map_err(|error| DocumentError::Invalid(format!("Node {id}: {error}")))?;
                }
                if n.layout_grid.is_none() {
                    return Err(DocumentError::Invalid(format!("Additional layout guides require a primary guide on node {id}")));
                }
            }
            if let Some(grid) = n.layout_child.grid {
                grid.validate().map_err(|error| DocumentError::Invalid(format!("Node {id}: {error}")))?;
            }
            if let Some(network) = &n.vector_network {
                if n.kind != NodeKind::Vector || !n.path.segments.is_empty() || !n.path.anchor_modes.is_empty() {
                    return Err(DocumentError::Invalid(format!("vector network must be the only geometry on vector node {id}")));
                }
                network.validate().map_err(|error| DocumentError::Invalid(format!("invalid network on node {id}: {error}")))?;
            }
            if !n.path.validate_anchor_modes() {
                return Err(DocumentError::Invalid(format!("invalid vector anchor constraints on node {id}")));
            }
            for paint in n.fills.iter().map(|fill| &fill.paint).chain(n.strokes.iter().map(|stroke| &stroke.paint)) {
                if let Paint::AngularGradient { center, angle, stops } = paint {
                    if !center.x.is_finite() || !center.y.is_finite() || !angle.is_finite()
                        || stops.is_empty() || stops.iter().any(|stop| {
                            !stop.offset.is_finite() || !(0.0..=1.0).contains(&stop.offset)
                        })
                    {
                        return Err(DocumentError::Invalid(format!("invalid angular gradient on node {id}")));
                    }
                }
                if let Paint::Image { crop: Some(crop), .. } = paint {
                    if !crate::valid_image_crop(*crop) {
                        return Err(DocumentError::Invalid(format!("invalid image source crop on node {id}")));
                    }
                }
            }
            for stroke in &n.strokes {
                crate::validate_variable_width_support(n, stroke)?;
            }
            if !n.font_size.is_finite()
                || n.font_size <= 0.0
                || !n.corner_radius.is_finite()
                || n.corner_radius < 0.0
                || !n.corner_smoothing.is_finite()
                || !(0.0..=1.0).contains(&n.corner_smoothing)
                || n.corner_radii.is_some_and(|radii| radii.iter().any(|r| !r.is_finite() || *r < 0.0))
            {
                return Err(DocumentError::Invalid(format!(
                    "invalid appearance on node {id}"
                )));
            }
            if !n.kind.is_container() && !n.children.is_empty() {
                return Err(DocumentError::Invalid(format!(
                    "non-container node {id} has children"
                )));
            }
            for child in &n.children {
                stack.push((*child, id, depth + 1))
            }
            if let Some(source) = n.component_id {
                if !self
                    .nodes
                    .get(&source)
                    .is_some_and(|n| n.kind == NodeKind::Component)
                {
                    return Err(DocumentError::Invalid(format!(
                        "missing component source for node {id}"
                    )));
                }
            }
            for run in &n.text_runs {
                if run.start > run.end
                    || run.end > n.text.len()
                    || !n.text.is_char_boundary(run.start)
                    || !n.text.is_char_boundary(run.end)
                {
                    return Err(DocumentError::Invalid(format!(
                        "invalid text-run range on node {id}"
                    )));
                }
            }
            crate::validate_font_features(&n.font_features)?;
            crate::validate_text_paragraphs(n)?;
        }
        if seen.len() != self.nodes.len() {
            return Err(DocumentError::Invalid(
                "unreachable nodes in document".into(),
            ));
        }
        self.validate_component_properties()?;
        self.validate_grid_placements()?;
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DocumentError {
    MissingNode(NodeId),
    InvalidParent(NodeId),
    Cycle(NodeId),
    UnsupportedVersion(u32),
    Invalid(String),
    Json(String),
}
impl std::fmt::Display for DocumentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingNode(id) => write!(f, "Node {id} does not exist"),
            Self::InvalidParent(id) => write!(f, "Invalid parent relationship for node {id}"),
            Self::Cycle(id) => write!(f, "Cyclic or duplicate node reference: {id}"),
            Self::UnsupportedVersion(v) => write!(f, "Unsupported document version {v}"),
            Self::Invalid(s) | Self::Json(s) => f.write_str(s),
        }
    }
}
impl std::error::Error for DocumentError {}
