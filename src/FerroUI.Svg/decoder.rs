//! From the XML of a document to its tree of shapes.

use crate::color::parse_color;
use crate::document::{Gradient, GradientKind, Group, Item, Paint, Shape, Stroke, SvgDocument, SvgError, SvgLimits};
use crate::numbers::{length, number_list, opacity};
use crate::path::{self, OutOfSegments};
use crate::transform::parse_transform;
use ferroui_base::media::immutable::ImmutableGradientStop;
use ferroui_base::media::{Color, Colors, FillRule, GradientSpreadMethod, PathCommand, PenLineCap, PenLineJoin};
use ferroui_base::{Matrix, Point, Rect, Size};
use roxmltree::{Document, Node, NodeId, ParsingOptions};
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use std::collections::{BTreeSet, HashMap};
use std::rc::Rc;
use std::sync::Mutex;

const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";
const XLINK_NAMESPACE: &str = "http://www.w3.org/1999/xlink";

/// The size of an image that does not say: the default of a replaced element in CSS.
const DEFAULT_SIZE: Size = Size::new(300.0, 150.0);

/// How many gradients one `href` chain may pass through.
const MOST_GRADIENT_LINKS: usize = 16;

/// The most stops of a gradient.
const MOST_GRADIENT_STOPS: usize = 256;

/// The most unsupported features one document lists.
const MOST_UNSUPPORTED: usize = 64;

/// The most features this process remembers having reported.
const MOST_REPORTED: usize = 256;

/// The elements the decoder reads, and those that draw nothing by their nature.
const KNOWN_ELEMENTS: &[&str] = &[
    "svg", "g", "a", "switch", "use", "symbol", "defs", "path", "rect", "circle", "ellipse", "line", "polyline", "polygon",
    "linearGradient", "radialGradient", "stop", "clipPath", "title", "desc", "metadata",
];

/// The properties the decoder reads in a `style` attribute.
const KNOWN_PROPERTIES: &[&str] = &[
    "color", "fill", "fill-opacity", "fill-rule", "clip-rule", "clip-path", "stroke", "stroke-opacity", "stroke-width",
    "stroke-linecap", "stroke-linejoin", "stroke-miterlimit", "stroke-dasharray", "stroke-dashoffset", "visibility",
    "display", "opacity", "stop-color", "stop-opacity",
];

/// The presentation attributes that change the picture and that the decoder does not apply, with
/// the value of each that changes nothing.
const UNSUPPORTED_ATTRIBUTES: &[(&str, &str)] = &[
    ("mask", "none"),
    ("filter", "none"),
    ("marker", "none"),
    ("marker-start", "none"),
    ("marker-mid", "none"),
    ("marker-end", "none"),
    ("mix-blend-mode", "normal"),
    ("isolation", "auto"),
    ("paint-order", "normal"),
    ("vector-effect", "none"),
    ("transform-origin", "0 0"),
    ("transform-box", "view-box"),
    ("overflow", "hidden"),
    ("color-interpolation", "sRGB"),
    ("shape-rendering", "auto"),
];

pub(crate) fn decode(bytes: &[u8], limits: &SvgLimits) -> Result<SvgDocument, SvgError> {
    if bytes.len() > limits.max_bytes {
        return Err(SvgError::TooLarge { bytes: bytes.len(), limit: limits.max_bytes });
    }
    let bytes = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(bytes);
    let text = std::str::from_utf8(bytes).map_err(|_| SvgError::InvalidEncoding)?;
    // A document type definition is refused: no entity is expanded.
    let options = ParsingOptions { allow_dtd: false, ..ParsingOptions::default() };
    let document = Document::parse_with_options(text, options).map_err(|error| SvgError::InvalidXml(error.to_string()))?;
    let root = document.root_element();
    if root.tag_name().name() != "svg" || !is_svg(root) {
        return Err(SvgError::NotSvg);
    }

    let view_box = view_box(root);
    let fallback = view_box.map_or(DEFAULT_SIZE, |view_box| Size::new(view_box.width, view_box.height));
    let width = root.attribute("width").and_then(|text| length(text, fallback.width)).unwrap_or(fallback.width);
    let height = root.attribute("height").and_then(|text| length(text, fallback.height)).unwrap_or(fallback.height);
    let is_valid = |side: f64| side.is_finite() && side > 0.0 && side <= limits.max_dimension;
    if !is_valid(width) || !is_valid(height) {
        return Err(SvgError::InvalidSize);
    }

    let mut ids = HashMap::new();
    for node in document.descendants().filter(Node::is_element) {
        if let Some(id) = node.attribute("id") {
            // The first element of an id is the one a reference finds.
            ids.entry(id).or_insert(node);
        }
    }
    let mut unsupported = unsupported_features(&document);
    let mut decoder = Decoder {
        ids,
        limits,
        elements: 0,
        segments: limits.max_path_segments,
        use_stack: Vec::new(),
        gradients: HashMap::new(),
        unsupported: &mut unsupported,
        viewport: view_box.map_or(Size::new(width, height), |view_box| Size::new(view_box.width, view_box.height)),
    };

    let mut style = Style::default();
    style.apply(root);
    decoder.count(1)?;
    let children = decoder.children(root, &style, 1)?;
    let mut group = Group::new(children);
    if let Some(view_box) = view_box {
        let aspect = root.attribute("preserveAspectRatio").unwrap_or("");
        group.transform = view_transform(view_box, aspect, Rect::new(0.0, 0.0, width, height));
    }
    if let Some(value) = property(root, "opacity").and_then(opacity) {
        group.opacity = value;
    }
    let unsupported: Vec<String> = unsupported.into_iter().collect();
    report_once(&unsupported);
    Ok(SvgDocument { size: Size::new(width, height), root: group, unsupported })
}

/// What `document` asks for that is not drawn, each named once: elements of SVG the decoder does
/// not read, properties it does not apply and references it does not follow.
fn unsupported_features(document: &Document<'_>) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for node in document.descendants().filter(|node| node.is_element() && is_svg(*node)) {
        if found.len() >= MOST_UNSUPPORTED {
            break;
        }
        let name = node.tag_name().name();
        if !KNOWN_ELEMENTS.contains(&name) {
            found.insert(format!("element <{name}>"));
        }
        for (attribute, neutral) in UNSUPPORTED_ATTRIBUTES {
            if property(node, attribute).is_some_and(|value| value != *neutral) {
                found.insert(format!("property {attribute}"));
            }
        }
        for declaration in node.attribute("style").unwrap_or("").split(';') {
            if let Some((key, _)) = declaration.split_once(':') {
                let key = key.trim().to_ascii_lowercase();
                if !key.is_empty() && !KNOWN_PROPERTIES.contains(&key.as_str()) {
                    found.insert(format!("property {}", key.chars().take(64).collect::<String>()));
                }
            }
        }
        let reference = node.attribute((XLINK_NAMESPACE, "href")).or_else(|| node.attribute("href"));
        if reference.is_some_and(|reference| !reference.trim_start().starts_with('#')) {
            found.insert("external reference".to_owned());
        }
    }
    found
}

/// The features of `features` this process has not reported yet, which are now reported.
pub(crate) fn newly_reported(features: &[String]) -> Vec<&String> {
    static REPORTED: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());
    let mut reported = REPORTED.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut new = Vec::new();
    for feature in features {
        if !reported.contains(feature) {
            if reported.len() < MOST_REPORTED {
                reported.insert(feature.clone());
            }
            new.push(feature);
        }
    }
    new
}

/// Logs each feature that is not drawn, once in the life of the process.
fn report_once(features: &[String]) {
    for feature in newly_reported(features) {
        if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::VISUAL) {
            logger.log(None, &format!("SVG: the {feature} is not drawn; the picture differs from the document."));
        }
    }
}

fn is_svg(node: Node<'_, '_>) -> bool {
    matches!(node.tag_name().namespace(), None | Some(SVG_NAMESPACE))
}

fn view_box(node: Node<'_, '_>) -> Option<Rect> {
    match number_list(node.attribute("viewBox")?)[..] {
        [x, y, width, height] if width > 0.0 && height > 0.0 => Some(Rect::new(x, y, width, height)),
        _ => None,
    }
}

/// The transform from `view_box` to `target` under a `preserveAspectRatio` value.
fn view_transform(view_box: Rect, aspect: &str, target: Rect) -> Matrix {
    let mut words = aspect.split_ascii_whitespace();
    let align = words.next().unwrap_or("xMidYMid");
    let is_slice = words.next() == Some("slice");
    let (mut scale_x, mut scale_y) = (target.width / view_box.width, target.height / view_box.height);
    let (mut along_x, mut along_y) = (0.0, 0.0);
    if align != "none" {
        let scale = if is_slice { scale_x.max(scale_y) } else { scale_x.min(scale_y) };
        scale_x = scale;
        scale_y = scale;
        let place = |name: &str, default: f64| match name {
            "Min" => 0.0,
            "Mid" => 0.5,
            "Max" => 1.0,
            _ => default,
        };
        along_x = align.get(1..4).map_or(0.5, |name| place(name, 0.5));
        along_y = align.get(5..8).map_or(0.5, |name| place(name, 0.5));
    }
    let x = target.x - view_box.x * scale_x + (target.width - view_box.width * scale_x) * along_x;
    let y = target.y - view_box.y * scale_y + (target.height - view_box.height * scale_y) * along_y;
    Matrix::new(scale_x, 0.0, 0.0, scale_y, x, y)
}

/// A property of an element: from its `style` attribute, else its presentation attribute.
fn property<'a>(node: Node<'a, '_>, name: &str) -> Option<&'a str> {
    if let Some(style) = node.attribute("style") {
        for declaration in style.split(';') {
            if let Some((key, value)) = declaration.split_once(':') {
                if key.trim().eq_ignore_ascii_case(name) {
                    let value = value.trim();
                    return Some(value.strip_suffix("!important").map_or(value, str::trim));
                }
            }
        }
    }
    node.attribute(name).map(str::trim)
}

/// A paint as it is written.
#[derive(Clone)]
enum PaintValue {
    None,
    Color(Color),
    CurrentColor,
    Reference(String, Option<Box<PaintValue>>),
}

fn parse_paint(text: &str) -> Option<PaintValue> {
    let text = text.trim();
    if text == "none" {
        return Some(PaintValue::None);
    }
    if text == "currentColor" || text == "currentcolor" {
        return Some(PaintValue::CurrentColor);
    }
    if let Some(rest) = text.strip_prefix("url(") {
        let (reference, after) = rest.split_once(')')?;
        let id = reference.trim().trim_matches(|c| c == '"' || c == '\'').strip_prefix('#')?;
        let after = after.trim();
        let fallback = if after.is_empty() { None } else { parse_paint(after).map(Box::new) };
        return Some(PaintValue::Reference(id.to_owned(), fallback));
    }
    parse_color(text).map(PaintValue::Color)
}

/// The inherited properties.
#[derive(Clone)]
struct Style {
    color: Color,
    fill: PaintValue,
    fill_opacity: f64,
    fill_rule: FillRule,
    clip_rule: FillRule,
    stroke: PaintValue,
    stroke_opacity: f64,
    stroke_width: String,
    cap: PenLineCap,
    join: PenLineJoin,
    miter_limit: f64,
    dashes: Option<String>,
    dash_offset: String,
    is_visible: bool,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            color: Colors::BLACK,
            fill: PaintValue::Color(Colors::BLACK),
            fill_opacity: 1.0,
            fill_rule: FillRule::NonZero,
            clip_rule: FillRule::NonZero,
            stroke: PaintValue::None,
            stroke_opacity: 1.0,
            stroke_width: "1".to_owned(),
            cap: PenLineCap::Flat,
            join: PenLineJoin::Miter,
            miter_limit: 4.0,
            dashes: None,
            dash_offset: "0".to_owned(),
            is_visible: true,
        }
    }
}

fn parse_fill_rule(text: &str) -> Option<FillRule> {
    match text {
        "nonzero" => Some(FillRule::NonZero),
        "evenodd" => Some(FillRule::EvenOdd),
        _ => None,
    }
}

impl Style {
    /// Takes the properties `node` sets. A value that is not valid, or `inherit`, leaves what the
    /// parent gave.
    fn apply(&mut self, node: Node<'_, '_>) {
        let get = |name: &str| property(node, name).filter(|value| *value != "inherit");
        if let Some(color) = get("color").and_then(parse_color) {
            self.color = color;
        }
        if let Some(fill) = get("fill").and_then(parse_paint) {
            self.fill = fill;
        }
        if let Some(value) = get("fill-opacity").and_then(opacity) {
            self.fill_opacity = value;
        }
        if let Some(rule) = get("fill-rule").and_then(parse_fill_rule) {
            self.fill_rule = rule;
        }
        if let Some(rule) = get("clip-rule").and_then(parse_fill_rule) {
            self.clip_rule = rule;
        }
        if let Some(stroke) = get("stroke").and_then(parse_paint) {
            self.stroke = stroke;
        }
        if let Some(value) = get("stroke-opacity").and_then(opacity) {
            self.stroke_opacity = value;
        }
        if let Some(width) = get("stroke-width") {
            self.stroke_width = width.to_owned();
        }
        match get("stroke-linecap") {
            Some("butt") => self.cap = PenLineCap::Flat,
            Some("round") => self.cap = PenLineCap::Round,
            Some("square") => self.cap = PenLineCap::Square,
            _ => {}
        }
        match get("stroke-linejoin") {
            Some("miter" | "miter-clip" | "arcs") => self.join = PenLineJoin::Miter,
            Some("round") => self.join = PenLineJoin::Round,
            Some("bevel") => self.join = PenLineJoin::Bevel,
            _ => {}
        }
        if let Some(limit) = get("stroke-miterlimit").and_then(|text| text.parse::<f64>().ok()) {
            if limit.is_finite() && limit >= 1.0 {
                self.miter_limit = limit;
            }
        }
        if let Some(dashes) = get("stroke-dasharray") {
            self.dashes = (dashes != "none").then(|| dashes.to_owned());
        }
        if let Some(offset) = get("stroke-dashoffset") {
            self.dash_offset = offset.to_owned();
        }
        match get("visibility") {
            Some("visible") => self.is_visible = true,
            Some("hidden" | "collapse") => self.is_visible = false,
            _ => {}
        }
    }
}

struct Decoder<'a, 'input> {
    ids: HashMap<&'a str, Node<'a, 'input>>,
    limits: &'a SvgLimits,
    elements: usize,
    segments: usize,
    use_stack: Vec<NodeId>,
    gradients: HashMap<NodeId, Option<Rc<Gradient>>>,
    unsupported: &'a mut BTreeSet<String>,
    viewport: Size,
}

impl From<OutOfSegments> for SvgError {
    fn from(_: OutOfSegments) -> Self {
        SvgError::LimitExceeded("path segments")
    }
}

impl<'a, 'input> Decoder<'a, 'input> {
    fn count(&mut self, depth: usize) -> Result<(), SvgError> {
        self.elements += 1;
        if self.elements > self.limits.max_elements {
            return Err(SvgError::LimitExceeded("elements"));
        }
        if depth > self.limits.max_depth {
            return Err(SvgError::LimitExceeded("depth"));
        }
        Ok(())
    }

    /// The length of an attribute against the width of the viewport.
    fn x(&self, node: Node<'_, '_>, name: &str) -> Option<f64> {
        node.attribute(name).and_then(|text| length(text, self.viewport.width))
    }

    /// The length of an attribute against the height of the viewport.
    fn y(&self, node: Node<'_, '_>, name: &str) -> Option<f64> {
        node.attribute(name).and_then(|text| length(text, self.viewport.height))
    }

    /// The length of an attribute against the diagonal of the viewport, as for a radius.
    fn diagonal(&self) -> f64 {
        ((self.viewport.width.powi(2) + self.viewport.height.powi(2)) / 2.0).sqrt()
    }

    fn children(&mut self, node: Node<'a, 'input>, style: &Style, depth: usize) -> Result<Vec<Item>, SvgError> {
        let mut items = Vec::new();
        for child in node.children().filter(Node::is_element) {
            if let Some(item) = self.item(child, style, depth)? {
                match item {
                    // A group that changes nothing is its children.
                    Item::Group(group) if group.is_plain() => items.extend(group.children),
                    item => items.push(item),
                }
            }
        }
        Ok(items)
    }

    fn item(&mut self, node: Node<'a, 'input>, parent: &Style, depth: usize) -> Result<Option<Item>, SvgError> {
        self.count(depth)?;
        if !is_svg(node) || property(node, "display") == Some("none") {
            return Ok(None);
        }
        let mut style = parent.clone();
        style.apply(node);
        let item = match node.tag_name().name() {
            "g" | "a" => Some(Item::Group(Group::new(self.children(node, &style, depth + 1)?))),
            "switch" => match node.children().find(|child| child.is_element() && is_svg(*child)) {
                Some(first) => self.item(first, &style, depth + 1)?,
                None => None,
            },
            "svg" => self.nested_viewport(node, node, &style, depth)?,
            "use" => self.use_element(node, &style, depth)?,
            "path" | "rect" | "circle" | "ellipse" | "line" | "polyline" | "polygon" => {
                self.shape_item(node, &style)?
            }
            _ => None,
        };
        let Some(item) = item else { return Ok(None) };
        self.placed(node, item)
    }

    /// `item` under the transform, the opacity and the clip of `node`.
    fn placed(&mut self, node: Node<'a, 'input>, item: Item) -> Result<Option<Item>, SvgError> {
        let transform = node.attribute("transform").and_then(parse_transform).unwrap_or(Matrix::IDENTITY);
        let element_opacity = property(node, "opacity").and_then(opacity).unwrap_or(1.0);
        let clip = match clip_reference(node) {
            Some(id) => {
                let bounds = item.bounds();
                match self.clip_commands(id, bounds)? {
                    // A clip without content hides the element.
                    Some(commands) if commands.is_empty() => return Ok(None),
                    other => other,
                }
            }
            None => None,
        };
        let mut item = item;
        let mut opacity_left = element_opacity;
        // The opacity of a shape with one paint is the opacity of that paint, with no layer.
        if let Item::Shape(shape) = &mut item {
            if opacity_left < 1.0 && (shape.fill.is_none() || shape.stroke.is_none()) {
                shape.fill = shape.fill.take().map(|paint| paint.with_opacity(opacity_left));
                if let Some(stroke) = &mut shape.stroke {
                    stroke.paint = stroke.paint.clone().with_opacity(opacity_left);
                }
                opacity_left = 1.0;
            }
        }
        if transform == Matrix::IDENTITY && opacity_left >= 1.0 && clip.is_none() {
            return Ok(Some(item));
        }
        // A group of this element alone takes them directly.
        let mut group = match item {
            Item::Group(group) if group.is_plain() => group,
            item => Group::new(vec![item]),
        };
        group.transform = transform;
        group.opacity = opacity_left;
        group.clip = clip;
        Ok(Some(Item::Group(group)))
    }

    /// The content of `container` (an `svg` or a `symbol`) in the viewport `node` gives it.
    fn nested_viewport(
        &mut self,
        node: Node<'a, 'input>,
        container: Node<'a, 'input>,
        style: &Style,
        depth: usize,
    ) -> Result<Option<Item>, SvgError> {
        let target = Rect::new(
            self.x(node, "x").unwrap_or(0.0),
            self.y(node, "y").unwrap_or(0.0),
            self.x(node, "width").unwrap_or(self.viewport.width),
            self.y(node, "height").unwrap_or(self.viewport.height),
        );
        if target.width <= 0.0 || target.height <= 0.0 {
            return Ok(None);
        }
        let view = view_box(container);
        let saved = self.viewport;
        self.viewport = view.map_or(Size::new(target.width, target.height), |view| Size::new(view.width, view.height));
        let children = self.children(container, style, depth + 1);
        self.viewport = saved;
        let mut group = Group::new(children?);
        group.transform = match view {
            Some(view) => view_transform(view, container.attribute("preserveAspectRatio").unwrap_or(""), target),
            None => Matrix::create_translation(target.x, target.y),
        };
        // The viewport clips its content.
        group.clip = Some(path::rectangle(
            view.unwrap_or(Rect::new(0.0, 0.0, target.width, target.height)),
            (0.0, 0.0),
            FillRule::NonZero,
        ));
        Ok(Some(Item::Group(group)))
    }

    /// A `use`: only an element of this document, and never one it is already inside.
    fn use_element(&mut self, node: Node<'a, 'input>, style: &Style, depth: usize) -> Result<Option<Item>, SvgError> {
        let Some(target) = href(node).and_then(|id| self.ids.get(id).copied()) else {
            return Ok(None);
        };
        if self.use_stack.contains(&target.id()) || node.ancestors().any(|ancestor| ancestor == target) {
            return Ok(None);
        }
        self.use_stack.push(target.id());
        let result = if target.tag_name().name() == "symbol" {
            let mut style = style.clone();
            style.apply(target);
            self.nested_viewport(node, target, &style, depth)
        } else {
            self.item(target, style, depth + 1)
        };
        self.use_stack.pop();
        let Some(item) = result? else { return Ok(None) };
        let (x, y) = (self.x(node, "x").unwrap_or(0.0), self.y(node, "y").unwrap_or(0.0));
        if x == 0.0 && y == 0.0 || target.tag_name().name() == "symbol" {
            return Ok(Some(item));
        }
        let mut group = Group::new(vec![item]);
        group.transform = Matrix::create_translation(x, y);
        Ok(Some(Item::Group(group)))
    }

    /// The commands of a basic shape or a path, in its own coordinates.
    fn shape_commands(&mut self, node: Node<'a, 'input>, fill_rule: FillRule) -> Result<Option<Vec<PathCommand>>, SvgError> {
        let spend = |decoder: &mut Self, count: usize| -> Result<(), SvgError> {
            decoder.segments = decoder.segments.checked_sub(count).ok_or(OutOfSegments)?;
            Ok(())
        };
        let commands = match node.tag_name().name() {
            "path" => match node.attribute("d") {
                Some(data) => path::parse_path(data, fill_rule, &mut self.segments)?,
                None => return Ok(None),
            },
            "rect" => {
                let (width, height) = (self.x(node, "width").unwrap_or(0.0), self.y(node, "height").unwrap_or(0.0));
                if width <= 0.0 || height <= 0.0 {
                    return Ok(None);
                }
                let rect = Rect::new(self.x(node, "x").unwrap_or(0.0), self.y(node, "y").unwrap_or(0.0), width, height);
                let (rx, ry) = (self.x(node, "rx").filter(|r| *r >= 0.0), self.y(node, "ry").filter(|r| *r >= 0.0));
                let (rx, ry) = (rx.or(ry).unwrap_or(0.0), ry.or(rx).unwrap_or(0.0));
                spend(self, 8)?;
                path::rectangle(rect, (rx.min(width / 2.0), ry.min(height / 2.0)), fill_rule)
            }
            "circle" | "ellipse" => {
                let center = Point::new(self.x(node, "cx").unwrap_or(0.0), self.y(node, "cy").unwrap_or(0.0));
                let radii = if node.tag_name().name() == "circle" {
                    let diagonal = self.diagonal();
                    let radius = node.attribute("r").and_then(|text| length(text, diagonal)).unwrap_or(0.0);
                    (radius, radius)
                } else {
                    let rx = self.x(node, "rx");
                    let ry = self.y(node, "ry");
                    (rx.or(ry).unwrap_or(0.0), ry.or(rx).unwrap_or(0.0))
                };
                if radii.0 <= 0.0 || radii.1 <= 0.0 {
                    return Ok(None);
                }
                spend(self, 4)?;
                path::ellipse(center, radii, fill_rule)
            }
            "line" => {
                let from = [self.x(node, "x1").unwrap_or(0.0), self.y(node, "y1").unwrap_or(0.0)];
                let to = [self.x(node, "x2").unwrap_or(0.0), self.y(node, "y2").unwrap_or(0.0)];
                spend(self, 2)?;
                path::polyline(&[from[0], from[1], to[0], to[1]], false, fill_rule)
            }
            name @ ("polyline" | "polygon") => {
                let points = number_list(node.attribute("points").unwrap_or(""));
                if points.len() < 4 {
                    return Ok(None);
                }
                spend(self, points.len() / 2)?;
                path::polyline(&points, name == "polygon", fill_rule)
            }
            _ => return Ok(None),
        };
        Ok(path::is_finite(&commands).then_some(commands))
    }

    fn shape_item(&mut self, node: Node<'a, 'input>, style: &Style) -> Result<Option<Item>, SvgError> {
        if !style.is_visible {
            return Ok(None);
        }
        let Some(commands) = self.shape_commands(node, style.fill_rule)? else {
            return Ok(None);
        };
        let Some(bounds) = path::bounds(&commands) else {
            return Ok(None);
        };
        let fill = self.resolve(&style.fill, style).map(|paint| paint.with_opacity(style.fill_opacity));
        let stroke = self.stroke(style);
        if fill.is_none() && stroke.is_none() {
            return Ok(None);
        }
        Ok(Some(Item::Shape(Shape { commands, bounds, fill, stroke })))
    }

    fn stroke(&mut self, style: &Style) -> Option<Stroke> {
        let paint = self.resolve(&style.stroke, style)?.with_opacity(style.stroke_opacity);
        let diagonal = self.diagonal();
        let width = length(&style.stroke_width, diagonal).filter(|width| *width > 0.0)?;
        let dashes = style.dashes.as_deref().and_then(|text| parse_dashes(text, diagonal));
        let dash_offset = length(&style.dash_offset, diagonal).unwrap_or(0.0);
        Some(Stroke { paint, width, cap: style.cap, join: style.join, miter_limit: style.miter_limit, dashes, dash_offset })
    }

    fn resolve(&mut self, value: &PaintValue, style: &Style) -> Option<Paint> {
        match value {
            PaintValue::None => None,
            PaintValue::Color(color) => Some(Paint::Solid(*color, 1.0)),
            PaintValue::CurrentColor => Some(Paint::Solid(style.color, 1.0)),
            PaintValue::Reference(id, fallback) => {
                let gradient = self.ids.get(id.as_str()).copied().and_then(|node| self.gradient(node));
                match gradient {
                    Some(gradient) => Some(gradient),
                    None => fallback.as_ref().and_then(|fallback| self.resolve(fallback, style)),
                }
            }
        }
    }

    /// The paint of a gradient element, kept for its other uses.
    fn gradient(&mut self, node: Node<'a, 'input>) -> Option<Paint> {
        if !matches!(node.tag_name().name(), "linearGradient" | "radialGradient") {
            return None;
        }
        let gradient = match self.gradients.get(&node.id()) {
            Some(known) => known.clone(),
            None => {
                let built = self.build_gradient(node);
                self.gradients.insert(node.id(), built.clone());
                built
            }
        }?;
        // A gradient of one stop paints the colour of the stop.
        if gradient.stops.len() == 1 {
            return Some(Paint::Solid(gradient.stops[0].color(), 1.0));
        }
        Some(Paint::Gradient(gradient, 1.0))
    }

    fn build_gradient(&self, node: Node<'a, 'input>) -> Option<Rc<Gradient>> {
        // The element with the links it inherits from, nearest first.
        let mut chain = vec![node];
        let mut at = node;
        while let Some(next) = href(at).and_then(|id| self.ids.get(id).copied()) {
            let is_gradient = matches!(next.tag_name().name(), "linearGradient" | "radialGradient");
            if !is_gradient || chain.contains(&next) || chain.len() >= MOST_GRADIENT_LINKS {
                break;
            }
            chain.push(next);
            at = next;
        }
        let attribute = |name: &str| chain.iter().find_map(|link| link.attribute(name));

        let stops_node = chain.iter().find(|link| link.children().any(is_stop));
        let mut stops = Vec::new();
        let mut last = 0.0f64;
        if let Some(stops_node) = stops_node {
            for stop in stops_node.children().filter(|child| is_stop(*child)).take(MOST_GRADIENT_STOPS) {
                let offset = stop.attribute("offset").and_then(opacity).unwrap_or(0.0).max(last);
                last = offset;
                let color = match property(stop, "stop-color") {
                    Some("currentColor" | "currentcolor") => property(stop, "color").and_then(parse_color).unwrap_or(Colors::BLACK),
                    Some(text) => parse_color(text).unwrap_or(Colors::BLACK),
                    None => Colors::BLACK,
                };
                let alpha = property(stop, "stop-opacity").and_then(opacity).unwrap_or(1.0);
                let alpha = (f64::from(color.a) * alpha).round().clamp(0.0, 255.0) as u8;
                stops.push(ImmutableGradientStop::new(offset, Color::from_argb(alpha, color.r, color.g, color.b)));
            }
        }
        if stops.is_empty() {
            return None;
        }

        let in_bounding_box = attribute("gradientUnits") != Some("userSpaceOnUse");
        let transform = attribute("gradientTransform").and_then(parse_transform).unwrap_or(Matrix::IDENTITY);
        let spread = match attribute("spreadMethod") {
            Some("reflect") => GradientSpreadMethod::Reflect,
            Some("repeat") => GradientSpreadMethod::Repeat,
            _ => GradientSpreadMethod::Pad,
        };
        // A coordinate in a bounding box is a fraction of it; a percentage of the viewport
        // otherwise.
        let (width, height, diagonal) =
            if in_bounding_box { (1.0, 1.0, 1.0) } else { (self.viewport.width, self.viewport.height, self.diagonal()) };
        let value = |name: &str, reference: f64, default: f64| {
            attribute(name).and_then(|text| length(text, reference)).unwrap_or(default * reference)
        };
        let kind = if node.tag_name().name() == "linearGradient" {
            GradientKind::Linear {
                start: Point::new(value("x1", width, 0.0), value("y1", height, 0.0)),
                end: Point::new(value("x2", width, 1.0), value("y2", height, 0.0)),
            }
        } else {
            let center = Point::new(value("cx", width, 0.5), value("cy", height, 0.5));
            let radius = value("r", diagonal, 0.5);
            if radius <= 0.0 {
                // A radius of zero paints the last colour.
                stops.drain(..stops.len() - 1);
                GradientKind::Radial { center, radius: 1.0, focus: center }
            } else {
                let focus = Point::new(
                    attribute("fx").and_then(|text| length(text, width)).unwrap_or(center.x),
                    attribute("fy").and_then(|text| length(text, height)).unwrap_or(center.y),
                );
                GradientKind::Radial { center, radius, focus }
            }
        };
        Some(Rc::new(Gradient { kind, stops, transform, spread, in_bounding_box }))
    }

    /// The geometry of a `clipPath` for an element with `bounds`. `None` when the reference finds
    /// no clip path, which leaves the element unclipped; empty when it clips everything away.
    fn clip_commands(&mut self, id: &str, bounds: Option<Rect>) -> Result<Option<Vec<PathCommand>>, SvgError> {
        let Some(clip) = self.ids.get(id).copied().filter(|node| node.tag_name().name() == "clipPath") else {
            return Ok(None);
        };
        let mut commands = Vec::new();
        let mut first_rule = None;
        for child in clip.children().filter(Node::is_element) {
            self.count(1)?;
            let (shape, transform) = if child.tag_name().name() == "use" {
                let Some(target) = href(child).and_then(|id| self.ids.get(id).copied()) else { continue };
                let (x, y) = (self.x(child, "x").unwrap_or(0.0), self.y(child, "y").unwrap_or(0.0));
                let inner = target.attribute("transform").and_then(parse_transform).unwrap_or(Matrix::IDENTITY);
                let outer = child.attribute("transform").and_then(parse_transform).unwrap_or(Matrix::IDENTITY);
                (target, inner * Matrix::create_translation(x, y) * outer)
            } else {
                (child, child.attribute("transform").and_then(parse_transform).unwrap_or(Matrix::IDENTITY))
            };
            if property(shape, "display") == Some("none") {
                continue;
            }
            let rule = property(shape, "clip-rule").and_then(parse_fill_rule).unwrap_or(FillRule::NonZero);
            if *first_rule.get_or_insert(rule) != rule {
                self.unsupported.insert("property clip-rule (mixed in one clipPath)".to_owned());
            }
            let Some(mut part) = self.shape_commands(shape, rule)? else { continue };
            path::transform_commands(&mut part, transform);
            // The rule of the first shape is the rule of the geometry.
            if !commands.is_empty() {
                part.retain(|command| !matches!(command, PathCommand::SetFillRule(_)));
            }
            commands.append(&mut part);
        }
        let own = clip.attribute("transform").and_then(parse_transform).unwrap_or(Matrix::IDENTITY);
        let to_user = if clip.attribute("clipPathUnits") == Some("objectBoundingBox") {
            let Some(bounds) = bounds else { return Ok(Some(Vec::new())) };
            own * Matrix::new(bounds.width, 0.0, 0.0, bounds.height, bounds.x, bounds.y)
        } else {
            own
        };
        path::transform_commands(&mut commands, to_user);
        if !path::is_finite(&commands) {
            return Ok(Some(Vec::new()));
        }
        Ok(Some(commands))
    }
}

fn is_stop(node: Node<'_, '_>) -> bool {
    node.is_element() && node.tag_name().name() == "stop" && is_svg(node)
}

/// The id a `clip-path` property refers to.
fn clip_reference<'a>(node: Node<'a, '_>) -> Option<&'a str> {
    let value = property(node, "clip-path")?;
    let inner = value.strip_prefix("url(")?.split_once(')')?.0.trim();
    inner.trim_matches(|c| c == '"' || c == '\'').strip_prefix('#')
}

/// The id of an `href` that points into this document. Any other reference is not followed.
fn href<'a>(node: Node<'a, '_>) -> Option<&'a str> {
    node.attribute((XLINK_NAMESPACE, "href")).or_else(|| node.attribute("href"))?.trim().strip_prefix('#')
}

/// A dash array, repeated when its length is odd; `None` when it draws a solid line.
fn parse_dashes(text: &str, reference: f64) -> Option<Rc<[f64]>> {
    let mut dashes = Vec::new();
    for part in text.split(|c: char| c == ',' || c.is_ascii_whitespace()).filter(|part| !part.is_empty()) {
        let dash = length(part, reference)?;
        if dash < 0.0 {
            return None;
        }
        dashes.push(dash);
    }
    if dashes.iter().sum::<f64>() <= 0.0 {
        return None;
    }
    if dashes.len() % 2 == 1 {
        dashes.extend_from_within(..);
    }
    Some(dashes.into())
}
