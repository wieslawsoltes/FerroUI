//! Editable SVG import. Supported: nested viewports/groups, arbitrary affine
//! transforms, paths (including arcs), basic shapes, text/tspan runs, internal
//! use references, solid/linear/centered radial paints, embedded raster images,
//! alpha/sRGB luminance masks, geometry clip paths, and compositing isolation.
//! Mask/clip coordinate systems and reference inheritance follow CSS Masking 1:
//! https://www.w3.org/TR/css-masking-1/ . Unsupported paint servers/effects and
//! linearRGB luminance masks return an error. No network access,
//! executable content, external fonts, or external images are accepted.
use crate::*;
use roxmltree::Node as XmlNode;
use std::collections::{BTreeMap, BTreeSet};

const MAX_SVG_BYTES: usize = 16 * 1024 * 1024;
const MAX_SVG_SEGMENTS: usize = 500_000;
type ImportResult<T> = Result<T, DocumentError>;
type Properties = BTreeMap<String, String>;

#[derive(Clone, Debug)]
pub struct SvgImport {
    pub document: Document,
    pub root: NodeId,
    /// Approximations are also retained in the imported frame's metadata.
    pub warnings: Vec<String>,
}

fn invalid(message: impl Into<String>) -> DocumentError {
    DocumentError::Invalid(format!("SVG import: {}", message.into()))
}

fn finite(value: f64, property: &str) -> ImportResult<f64> {
    if value.is_finite() && value.abs() <= 1e12 {
        Ok(value)
    } else {
        Err(invalid(format!("invalid {property}")))
    }
}

fn number(value: &str, property: &str) -> ImportResult<f64> {
    finite(
        value
            .trim()
            .parse()
            .map_err(|_| invalid(format!("invalid {property}: {value}")))?,
        property,
    )
}

fn unit(value: &str, property: &str) -> ImportResult<f64> {
    let value = value.trim();
    Ok(if let Some(percent) = value.strip_suffix('%') {
        number(percent, property)? / 100.0
    } else {
        number(value, property)?
    }
    .clamp(0.0, 1.0))
}

fn length(value: &str, reference: f64, font_size: f64) -> ImportResult<f64> {
    use svgtypes::LengthUnit::*;
    let value: svgtypes::Length = value
        .trim()
        .parse()
        .map_err(|_| invalid(format!("invalid length: {value}")))?;
    finite(
        value.number
            * match value.unit {
                None | Px => 1.0,
                Em => font_size,
                Ex => font_size * 0.5,
                In => 96.0,
                Cm => 96.0 / 2.54,
                Mm => 96.0 / 25.4,
                Pt => 96.0 / 72.0,
                Pc => 16.0,
                Percent => reference / 100.0,
            },
        "length",
    )
}

fn attribute_length(
    node: XmlNode<'_, '_>,
    key: &str,
    default: f64,
    reference: f64,
    font_size: f64,
) -> ImportResult<f64> {
    node.attribute(key)
        .map(|value| length(value, reference, font_size))
        .unwrap_or(Ok(default))
}

fn numbers(value: &str) -> ImportResult<Vec<f64>> {
    svgtypes::NumberListParser::from(value)
        .map(|n| finite(n.map_err(|e| invalid(e.to_string()))?, "coordinate"))
        .collect()
}

fn transform(value: Option<&str>) -> ImportResult<Transform> {
    let Some(value) = value else {
        return Ok(Transform::IDENTITY);
    };
    let t: svgtypes::Transform = value
        .parse()
        .map_err(|e: svgtypes::Error| invalid(e.to_string()))?;
    for value in [t.a, t.b, t.c, t.d, t.e, t.f] {
        finite(value, "transform")?;
    }
    Ok(Transform {
        a: t.a,
        b: t.b,
        c: t.c,
        d: t.d,
        e: t.e,
        f: t.f,
    })
}

fn color(value: &str, current: Color) -> ImportResult<Color> {
    if value.trim().eq_ignore_ascii_case("currentColor") {
        return Ok(current);
    }
    // svgtypes follows the SVG 1.1/CSS Color 3 name table.
    if value.trim().eq_ignore_ascii_case("rebeccapurple") {
        return Ok(Color::rgb(102, 51, 153));
    }
    let c: svgtypes::Color = value
        .trim()
        .parse()
        .map_err(|_| invalid(format!("unsupported color: {value}")))?;
    Ok(Color::rgba(c.red, c.green, c.blue, c.alpha))
}

fn href<'a, 'input>(node: XmlNode<'a, 'input>) -> Option<&'a str> {
    node.attribute("href")
        .or_else(|| node.attribute(("http://www.w3.org/1999/xlink", "href")))
}

fn raster_data_url(value: &str) -> bool {
    let Some((header, data)) = value.split_once(',') else {
        return false;
    };
    [
        "data:image/png;base64",
        "data:image/jpeg;base64",
        "data:image/jpg;base64",
        "data:image/gif;base64",
        "data:image/bmp;base64",
    ]
    .iter()
    .any(|allowed| header.eq_ignore_ascii_case(allowed))
        && !data.is_empty()
        && data.bytes().all(|b| {
            b.is_ascii_alphanumeric()
                || matches!(b, b'+' | b'/' | b'=' | b'\r' | b'\n' | b'\t' | b' ')
        })
}

fn local_urls(value: &str) -> ImportResult<()> {
    let lowercase = value.to_ascii_lowercase();
    if lowercase.contains("@import")
        || lowercase.contains("@font-face")
        || lowercase.contains("expression(")
    {
        return Err(invalid(
            "external stylesheets, fonts, and executable CSS are unsupported",
        ));
    }
    let mut tail = value;
    while let Some(start) = tail.to_ascii_lowercase().find("url(") {
        tail = &tail[start + 4..];
        let Some(end) = tail.find(')') else {
            return Err(invalid("unterminated URL"));
        };
        let url = tail[..end].trim().trim_matches(['\'', '"']);
        if !url.starts_with('#') || url.len() < 2 {
            return Err(invalid("external resource URLs are not allowed"));
        }
        tail = &tail[end + 1..];
    }
    Ok(())
}

fn preflight(xml: &roxmltree::Document<'_>) -> ImportResult<()> {
    for node in xml.descendants() {
        if node
            .pi()
            .is_some_and(|p| p.target.eq_ignore_ascii_case("xml-stylesheet"))
        {
            return Err(invalid("external XML stylesheets are not allowed"));
        }
        if !node.is_element() {
            continue;
        }
        let tag = node.tag_name().name();
        if [
            "script",
            "foreignobject",
            "animate",
            "animatemotion",
            "animatetransform",
            "set",
            "mpath",
            "iframe",
            "audio",
            "video",
        ]
        .contains(&tag.to_ascii_lowercase().as_str())
        {
            return Err(invalid(format!(
                "executable or embedded element <{tag}> is not allowed"
            )));
        }
        for attr in node.attributes() {
            let key = attr.name().to_ascii_lowercase();
            if key.starts_with("on") {
                return Err(invalid(format!(
                    "event attribute {} is not allowed",
                    attr.name()
                )));
            }
            if key == "href" || key == "src" {
                let value = attr.value().trim();
                if !value.starts_with('#') && !(tag == "image" && raster_data_url(value)) {
                    return Err(invalid(
                        "only local references and embedded raster images are allowed",
                    ));
                }
            }
            local_urls(attr.value())?;
        }
        if tag == "style" {
            for text in node.children().filter(|n| n.is_text()) {
                local_urls(text.text().unwrap_or_default())?;
            }
        }
    }
    Ok(())
}

#[derive(Clone, Debug)]
struct Declaration {
    value: String,
    important: bool,
}

fn declarations(text: &str) -> ImportResult<BTreeMap<String, Declaration>> {
    let mut result = BTreeMap::new();
    for part in text.split(';').filter(|part| !part.trim().is_empty()) {
        let Some((key, value)) = part.split_once(':') else {
            return Err(invalid("invalid CSS declaration"));
        };
        let value = value.trim();
        let important = value.to_ascii_lowercase().ends_with("!important");
        let value = if important {
            value[..value.len() - 10].trim_end()
        } else {
            value
        };
        local_urls(value)?;
        let key = key.trim().to_ascii_lowercase();
        if !result
            .get(&key)
            .is_some_and(|existing: &Declaration| existing.important && !important)
        {
            result.insert(
                key,
                Declaration {
                    value: value.to_string(),
                    important,
                },
            );
        }
    }
    Ok(result)
}

#[derive(Clone, Debug)]
struct Selector {
    tag: Option<String>,
    id: Option<String>,
    classes: Vec<String>,
}

impl Selector {
    fn parse(text: &str) -> ImportResult<Self> {
        let text = text.trim();
        if text.is_empty()
            || text
                .chars()
                .any(|c| c.is_whitespace() || "[]:>+~|\\".contains(c))
        {
            return Err(invalid(format!(
                "unsupported CSS selector: {text}; use type, class or ID selectors"
            )));
        }
        let mut selector = Self {
            tag: None,
            id: None,
            classes: Vec::new(),
        };
        let mut start = 0;
        let mut prefix = ' ';
        for (index, c) in text
            .char_indices()
            .chain(std::iter::once((text.len(), '\0')))
        {
            if c == '.' || c == '#' || c == '\0' {
                if index > start {
                    let token = &text[start..index];
                    match prefix {
                        '#' => {
                            if selector.id.replace(token.to_string()).is_some() {
                                return Err(invalid("multiple IDs in selector"));
                            }
                        }
                        '.' => selector.classes.push(token.to_string()),
                        _ => {
                            if token != "*" {
                                selector.tag = Some(token.to_string())
                            }
                        }
                    }
                } else if prefix != ' ' {
                    return Err(invalid("empty CSS selector part"));
                }
                prefix = c;
                start = index + c.len_utf8();
            }
        }
        Ok(selector)
    }
    fn matches(&self, node: XmlNode<'_, '_>) -> bool {
        self.tag
            .as_ref()
            .is_none_or(|tag| tag == node.tag_name().name())
            && self
                .id
                .as_ref()
                .is_none_or(|id| node.attribute("id") == Some(id.as_str()))
            && self.classes.iter().all(|class| {
                node.attribute("class")
                    .unwrap_or_default()
                    .split_whitespace()
                    .any(|c| c == class)
            })
    }
    fn specificity(&self) -> (u32, u32, u32) {
        (
            self.id.is_some() as u32,
            self.classes.len() as u32,
            self.tag.is_some() as u32,
        )
    }
}

#[derive(Clone, Debug)]
struct CssRule {
    selector: Selector,
    declarations: BTreeMap<String, Declaration>,
}

fn css_rules(xml: &roxmltree::Document<'_>) -> ImportResult<Vec<CssRule>> {
    let mut rules = Vec::new();
    for style in xml
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "style")
    {
        let source: String = style
            .children()
            .filter(|n| n.is_text())
            .filter_map(|n| n.text())
            .collect();
        let mut clean = String::new();
        let mut remaining = source.as_str();
        while let Some(start) = remaining.find("/*") {
            clean.push_str(&remaining[..start]);
            let Some(end) = remaining[start + 2..].find("*/") else {
                return Err(invalid("unterminated CSS comment"));
            };
            remaining = &remaining[start + 2 + end + 2..];
        }
        clean.push_str(remaining);
        let mut remaining = clean.trim();
        while !remaining.is_empty() {
            let Some((selectors, body)) = remaining.split_once('{') else {
                return Err(invalid("invalid CSS rule"));
            };
            let Some((body, rest)) = body.split_once('}') else {
                return Err(invalid("unterminated CSS rule"));
            };
            let properties = declarations(body)?;
            for selector in selectors.split(',') {
                rules.push(CssRule {
                    selector: Selector::parse(selector)?,
                    declarations: properties.clone(),
                });
            }
            remaining = rest.trim();
        }
    }
    Ok(rules)
}

#[derive(Clone, Debug)]
struct SvgStyle {
    fill: String,
    stroke: String,
    fill_opacity: f64,
    stroke_opacity: f64,
    stroke_width: f64,
    cap: StrokeCap,
    join: StrokeJoin,
    dash: Vec<f64>,
    fill_rule: FillRule,
    clip_rule: FillRule,
    linear_rgb: bool,
    color: Color,
    font_family: String,
    font_size: f64,
    font_weight: u16,
    italic: bool,
    letter_spacing: f64,
    decoration: TextDecoration,
    anchor: String,
    visibility: bool,
    // The following properties do not inherit; ancestors apply their effects.
    opacity: f64,
    display: bool,
    blend_mode: BlendMode,
    isolate: bool,
    mask: Option<String>,
    clip_path: Option<String>,
    mask_mode: Option<MaskType>,
    mask_type: MaskType,
}

impl Default for SvgStyle {
    fn default() -> Self {
        Self {
            fill: "black".into(),
            stroke: "none".into(),
            fill_opacity: 1.0,
            stroke_opacity: 1.0,
            stroke_width: 1.0,
            cap: StrokeCap::Butt,
            join: StrokeJoin::Miter,
            dash: Vec::new(),
            fill_rule: FillRule::NonZero,
            clip_rule: FillRule::NonZero,
            linear_rgb: false,
            color: Color::BLACK,
            font_family: "Inter".into(),
            font_size: 16.0,
            font_weight: 400,
            italic: false,
            letter_spacing: 0.0,
            decoration: TextDecoration::None,
            anchor: "start".into(),
            visibility: true,
            opacity: 1.0,
            display: true,
            blend_mode: BlendMode::Normal,
            isolate: false,
            mask: None,
            clip_path: None,
            mask_mode: None,
            mask_type: MaskType::Luminance,
        }
    }
}

impl SvgStyle {
    /// SVG groups pass their backdrop through by default. Opacity and masks
    /// establish an isolated group even when CSS isolation remains `auto`.
    fn compositing_blend(&self) -> BlendMode {
        if self.blend_mode == BlendMode::Normal
            && !self.isolate
            && self.opacity == 1.0
            && self.mask.is_none()
        {
            BlendMode::PassThrough
        } else {
            self.blend_mode
        }
    }
}

fn local_reference(value: &str, property: &str) -> ImportResult<Option<String>> {
    if value == "none" {
        return Ok(None);
    }
    let target = value
        .strip_prefix("url(")
        .and_then(|v| v.strip_suffix(')'))
        .map(str::trim)
        .map(|v| v.trim_matches(['\'', '"']))
        .and_then(|v| v.strip_prefix('#'))
        .filter(|v| {
            !v.is_empty()
                && !v.contains(['(', ')', '#', '\'', '"'])
                && !v.chars().any(char::is_whitespace)
        })
        .ok_or_else(|| invalid(format!("{property} requires one local url(#id) reference")))?;
    Ok(Some(target.into()))
}

fn blend_mode(value: &str) -> ImportResult<BlendMode> {
    Ok(match value {
        "normal" => BlendMode::Normal,
        "multiply" => BlendMode::Multiply,
        "screen" => BlendMode::Screen,
        "overlay" => BlendMode::Overlay,
        "darken" => BlendMode::Darken,
        "lighten" => BlendMode::Lighten,
        "color-dodge" => BlendMode::ColorDodge,
        "color-burn" => BlendMode::ColorBurn,
        "hard-light" => BlendMode::HardLight,
        "soft-light" => BlendMode::SoftLight,
        "difference" => BlendMode::Difference,
        "exclusion" => BlendMode::Exclusion,
        "hue" => BlendMode::Hue,
        "saturation" => BlendMode::Saturation,
        "color" => BlendMode::Color,
        "luminosity" => BlendMode::Luminosity,
        _ => return Err(invalid(format!("unsupported blend mode: {value}"))),
    })
}

struct Importer<'a, 'input> {
    document: Document,
    ids: BTreeMap<String, XmlNode<'a, 'input>>,
    rules: Vec<CssRule>,
    warnings: BTreeSet<String>,
    segments: usize,
    clip_geometry: bool,
}

impl<'a, 'input> Importer<'a, 'input> {
    fn properties(&self, node: XmlNode<'a, 'input>) -> ImportResult<Properties> {
        type Rank = (bool, (u32, u32, u32), usize);
        let mut cascade: BTreeMap<String, (Rank, String)> = BTreeMap::new();
        for attr in node.attributes() {
            if attr.namespace().is_none() && attr.name() != "style" {
                cascade.insert(
                    attr.name().to_string(),
                    ((false, (0, 0, 0), 0), attr.value().to_string()),
                );
            }
        }
        for (index, rule) in self.rules.iter().enumerate() {
            if !rule.selector.matches(node) {
                continue;
            }
            for (key, value) in &rule.declarations {
                let rank = (value.important, rule.selector.specificity(), index + 1);
                if cascade.get(key).is_none_or(|old| rank >= old.0) {
                    cascade.insert(key.clone(), (rank, value.value.clone()));
                }
            }
        }
        if let Some(style) = node.attribute("style") {
            for (key, value) in declarations(style)? {
                let rank = (value.important, (u32::MAX, 0, 0), usize::MAX);
                if cascade.get(&key).is_none_or(|old| rank >= old.0) {
                    cascade.insert(key, (rank, value.value));
                }
            }
        }
        Ok(cascade
            .into_iter()
            .map(|(key, (_, value))| (key, value))
            .collect())
    }

    fn style(
        &mut self,
        node: XmlNode<'a, 'input>,
        inherited: &SvgStyle,
        viewport: Rect,
    ) -> ImportResult<(SvgStyle, Properties)> {
        let props = self.properties(node)?;
        let mut result = inherited.clone();
        result.opacity = 1.0;
        result.display = true;
        result.blend_mode = BlendMode::Normal;
        result.isolate = false;
        result.mask = None;
        result.clip_path = None;
        result.mask_mode = None;
        result.mask_type = MaskType::Luminance;
        if let Some(value) = props.get("font-size").filter(|v| v.as_str() != "inherit") {
            result.font_size = length(value, inherited.font_size, inherited.font_size)?.max(0.01);
        }
        if let Some(value) = props.get("color").filter(|v| v.as_str() != "inherit") {
            result.color = color(value, inherited.color)?;
        }
        let diagonal = (viewport.width.hypot(viewport.height) / std::f64::consts::SQRT_2).max(1.0);
        for (key, value) in &props {
            if value == "inherit" {
                match key.as_str() {
                    "opacity" => result.opacity = inherited.opacity,
                    "display" => result.display = inherited.display,
                    "mix-blend-mode" => result.blend_mode = inherited.blend_mode,
                    "isolation" => result.isolate = inherited.isolate,
                    "mask" | "mask-image" => result.mask = inherited.mask.clone(),
                    "clip-path" => result.clip_path = inherited.clip_path.clone(),
                    "mask-mode" => result.mask_mode = inherited.mask_mode,
                    "mask-type" => result.mask_type = inherited.mask_type,
                    _ => {}
                }
                continue;
            }
            match key.as_str() {
                "fill" => result.fill = value.clone(),
                "stroke" => result.stroke = value.clone(),
                "fill-opacity" => result.fill_opacity = unit(value, key)?,
                "stroke-opacity" => result.stroke_opacity = unit(value, key)?,
                "opacity" => result.opacity = unit(value, key)?,
                "stroke-width" => {
                    result.stroke_width = length(value, diagonal, result.font_size)?.max(0.0)
                }
                "stroke-linecap" => {
                    result.cap = match value.as_str() {
                        "butt" => StrokeCap::Butt,
                        "round" => StrokeCap::Round,
                        "square" => StrokeCap::Square,
                        _ => return Err(invalid("unsupported stroke-linecap")),
                    }
                }
                "stroke-linejoin" => {
                    result.join = match value.as_str() {
                        "miter" | "miter-clip" => StrokeJoin::Miter,
                        "round" => StrokeJoin::Round,
                        "bevel" => StrokeJoin::Bevel,
                        _ => return Err(invalid("unsupported stroke-linejoin")),
                    }
                }
                "stroke-dasharray" => {
                    result.dash = if value == "none" {
                        Vec::new()
                    } else {
                        value
                            .split(|c: char| c.is_whitespace() || c == ',')
                            .filter(|v| !v.is_empty())
                            .map(|v| length(v, diagonal, result.font_size))
                            .collect::<ImportResult<Vec<_>>>()?
                    };
                    if result.dash.iter().any(|v| *v < 0.0) {
                        return Err(invalid("negative stroke dash length"));
                    }
                    if result.dash.iter().all(|v| *v == 0.0) {
                        result.dash.clear()
                    }
                    if result.dash.len() % 2 != 0 {
                        let copy = result.dash.clone();
                        result.dash.extend(copy);
                    }
                }
                "stroke-dashoffset" if length(value, diagonal, result.font_size)?.abs() > 1e-10 => {
                    return Err(invalid("nonzero stroke-dashoffset is unsupported"))
                }
                "fill-rule" | "clip-rule" => {
                    let rule = match value.as_str() {
                        "evenodd" => FillRule::EvenOdd,
                        "nonzero" => FillRule::NonZero,
                        _ => return Err(invalid(format!("unsupported {key}"))),
                    };
                    if key == "fill-rule" {
                        result.fill_rule = rule
                    } else {
                        result.clip_rule = rule
                    }
                }
                "font-family" => {
                    result.font_family = value
                        .split(',')
                        .next()
                        .unwrap_or("Inter")
                        .trim()
                        .trim_matches(['\'', '"'])
                        .to_string()
                }
                "font-weight" => {
                    result.font_weight = match value.as_str() {
                        "normal" => 400,
                        "bold" => 700,
                        "bolder" => (inherited.font_weight + 300).min(900),
                        "lighter" => inherited.font_weight.saturating_sub(300).max(100),
                        _ => number(value, key)?.clamp(1.0, 1000.0) as u16,
                    }
                }
                "font-style" => {
                    result.italic = match value.as_str() {
                        "normal" => false,
                        "italic" | "oblique" => true,
                        _ => return Err(invalid("unsupported font-style")),
                    }
                }
                "letter-spacing" => {
                    result.letter_spacing = if value == "normal" {
                        0.0
                    } else {
                        length(value, result.font_size, result.font_size)?
                    }
                }
                "text-anchor" => {
                    if ["start", "middle", "end"].contains(&value.as_str()) {
                        result.anchor = value.clone()
                    } else {
                        return Err(invalid("unsupported text-anchor"));
                    }
                }
                "text-decoration" => {
                    result.decoration = match value.as_str() {
                        "none" => TextDecoration::None,
                        "underline" => TextDecoration::Underline,
                        "line-through" => TextDecoration::Strikethrough,
                        _ => return Err(invalid("unsupported text-decoration")),
                    }
                }
                "display" => result.display = value != "none",
                "visibility" => {
                    result.visibility = !["hidden", "collapse"].contains(&value.as_str())
                }
                "mix-blend-mode" => result.blend_mode = blend_mode(value)?,
                "isolation" => {
                    result.isolate = match value.as_str() {
                        "auto" => false,
                        "isolate" => true,
                        _ => return Err(invalid("unsupported isolation")),
                    }
                }
                "mask" | "mask-image" => result.mask = local_reference(value, key)?,
                "clip-path" => result.clip_path = local_reference(value, key)?,
                "mask-type" => {
                    result.mask_type = match value.as_str() {
                        "alpha" => MaskType::Alpha,
                        "luminance" => MaskType::Luminance,
                        _ => return Err(invalid("unsupported mask-type")),
                    }
                }
                "mask-mode" => {
                    result.mask_mode = match value.as_str() {
                        "match-source" => None,
                        "alpha" => Some(MaskType::Alpha),
                        "luminance" => Some(MaskType::Luminance),
                        _ => return Err(invalid("unsupported mask-mode")),
                    }
                }
                "color-interpolation" => {
                    result.linear_rgb = match value.as_str() {
                        "auto" | "sRGB" => false,
                        "linearRGB" => true,
                        _ => return Err(invalid("unsupported color-interpolation")),
                    }
                }
                "mask-composite" if value != "add" => {
                    return Err(invalid("multiple CSS mask layers are unsupported"))
                }
                "filter" | "marker" | "marker-start" | "marker-mid" | "marker-end"
                    if value != "none" && !self.clip_geometry =>
                {
                    return Err(invalid(format!(
                        "{key} is outside the editable SVG import subset"
                    )));
                }
                "vector-effect" if value != "none" => {
                    return Err(invalid("non-scaling strokes are unsupported"))
                }
                "writing-mode" if value != "horizontal-tb" => {
                    return Err(invalid("vertical text is unsupported"))
                }
                "direction" if value != "ltr" => {
                    return Err(invalid("explicit SVG text direction is unsupported"))
                }
                "stroke-miterlimit" if number(value, key)? != 4.0 => {
                    self.warnings
                        .insert("Custom stroke miter limits use the renderer default.".into());
                }
                _ => {}
            }
        }
        if self.clip_geometry {
            // SVG clip silhouettes do not depend on paint, stroke, opacity, or
            // filters. A child clip-path still intersects the silhouette.
            result.fill = "white".into();
            result.stroke = "none".into();
            result.fill_opacity = 1.0;
            result.stroke_opacity = 1.0;
            result.opacity = 1.0;
            result.fill_rule = result.clip_rule;
            result.blend_mode = BlendMode::Normal;
            result.isolate = false;
            result.mask = None;
        }
        Ok((result, props))
    }

    fn insert(&mut self, parent: NodeId, node: Node) -> ImportResult<NodeId> {
        if self.document.nodes.len() >= MAX_DOCUMENT_NODES {
            return Err(invalid("node limit exceeded"));
        }
        self.document.insert_node(parent, node)
    }

    fn label(node: XmlNode<'a, 'input>, default: &str) -> String {
        node.attribute("id")
            .map(str::to_string)
            .or_else(|| {
                node.children()
                    .find(|n| n.is_element() && n.tag_name().name() == "title")
                    .and_then(|n| n.text())
                    .map(str::to_string)
            })
            .unwrap_or_else(|| default.to_string())
    }

    fn identify(node: &mut Node, source: XmlNode<'a, 'input>) {
        if let Some(id) = source.attribute("id") {
            node.metadata.insert("svg_id".into(), id.into());
        }
        node.metadata
            .insert("svg_element".into(), source.tag_name().name().into());
    }

    fn gradient(
        &mut self,
        id: &str,
        bounds: Rect,
        viewport: Rect,
        current: Color,
    ) -> ImportResult<Paint> {
        let mut attributes = Properties::new();
        let mut stops_source = None;
        let mut visited = BTreeSet::new();
        let mut next = id.to_string();
        let mut kind = "".to_string();
        while !next.is_empty() {
            if !visited.insert(next.clone()) || visited.len() > 64 {
                return Err(invalid("cyclic gradient reference"));
            }
            let source = *self
                .ids
                .get(&next)
                .ok_or_else(|| invalid(format!("missing paint server #{next}")))?;
            let tag = source.tag_name().name();
            if !["linearGradient", "radialGradient"].contains(&tag) {
                return Err(invalid(format!("paint server <{tag}> is unsupported")));
            }
            if kind.is_empty() {
                kind = tag.to_string();
            }
            for (key, value) in self.properties(source)? {
                attributes.entry(key).or_insert(value);
            }
            if stops_source.is_none()
                && source
                    .children()
                    .any(|n| n.is_element() && n.tag_name().name() == "stop")
            {
                stops_source = Some(source);
            }
            next = href(source)
                .unwrap_or_default()
                .trim_start_matches('#')
                .to_string();
        }
        if attributes.get("spreadMethod").is_some_and(|v| v != "pad") {
            return Err(invalid(
                "repeat and reflect gradient spread methods are unsupported",
            ));
        }
        let mut stops = Vec::new();
        if let Some(source) = stops_source {
            let mut parent_style = SvgStyle::default();
            parent_style.color = current;
            let (parent_style, _) = self.style(source, &parent_style, viewport)?;
            let mut offset: f64 = 0.0;
            for stop in source
                .children()
                .filter(|n| n.is_element() && n.tag_name().name() == "stop")
            {
                let (style, props) = self.style(stop, &parent_style, viewport)?;
                offset = unit(
                    props.get("offset").map(String::as_str).unwrap_or("0"),
                    "stop offset",
                )?
                .max(offset);
                let mut color = color(
                    props
                        .get("stop-color")
                        .map(String::as_str)
                        .unwrap_or("black"),
                    style.color,
                )?;
                let opacity = unit(
                    props.get("stop-opacity").map(String::as_str).unwrap_or("1"),
                    "stop opacity",
                )?;
                color.a = (color.a as f64 * opacity).round() as u8;
                stops.push(GradientStop { offset, color });
            }
        }
        if stops.is_empty() {
            return Ok(Paint::Solid(Color::TRANSPARENT));
        }
        if stops.len() == 1 {
            return Ok(Paint::Solid(stops[0].color));
        }
        let object_units = match attributes
            .get("gradientUnits")
            .map(String::as_str)
            .unwrap_or("objectBoundingBox")
        {
            "objectBoundingBox" => true,
            "userSpaceOnUse" => false,
            _ => return Err(invalid("unsupported gradientUnits")),
        };
        // Degenerate bounds cannot establish object-bounding-box paint space.
        if bounds.width.abs() < GEOMETRY_EPSILON || bounds.height.abs() < GEOMETRY_EPSILON {
            return Ok(Paint::Solid(stops.last().unwrap().color));
        }
        let gradient_transform =
            transform(attributes.get("gradientTransform").map(String::as_str))?;
        let map = if object_units {
            Transform::translation(bounds.x, bounds.y)
                * Transform::scale(bounds.width, bounds.height)
                * gradient_transform
        } else {
            gradient_transform
        };
        let normalize = Transform::scale(1.0 / bounds.width, 1.0 / bounds.height)
            * Transform::translation(-bounds.x, -bounds.y);
        let coordinate = |key: &str, default: &str, reference: f64| -> ImportResult<f64> {
            length(
                attributes.get(key).map(String::as_str).unwrap_or(default),
                if object_units { 1.0 } else { reference },
                16.0,
            )
        };
        if kind == "linearGradient" {
            let start = Point::new(
                coordinate("x1", "0%", viewport.width)?,
                coordinate("y1", "0%", viewport.height)?,
            );
            let end = Point::new(
                coordinate("x2", "100%", viewport.width)?,
                coordinate("y2", "0%", viewport.height)?,
            );
            let vector = end - start;
            let length2 = vector.x * vector.x + vector.y * vector.y;
            let Some(inverse) = map.inverse() else {
                return Ok(Paint::Solid(stops.last().unwrap().color));
            };
            // Gradients transform covariantly: transform the gradient normal by
            // inverse transpose. Directly transforming endpoints fails for shear
            // and nonuniform object-bounding-box scaling.
            let normal = Point::new(
                inverse.a * vector.x + inverse.b * vector.y,
                inverse.c * vector.x + inverse.d * vector.y,
            );
            let norm2 = normal.x * normal.x + normal.y * normal.y;
            if length2 < 1e-20 || norm2 < 1e-20 {
                return Ok(Paint::Solid(stops.last().unwrap().color));
            }
            let start = map.apply(start);
            let end = start + normal * (length2 / norm2);
            Ok(Paint::LinearGradient {
                start: normalize.apply(start),
                end: normalize.apply(end),
                stops,
            })
        } else {
            let cx = coordinate("cx", "50%", viewport.width)?;
            let cy = coordinate("cy", "50%", viewport.height)?;
            let fx = attributes
                .get("fx")
                .map(|v| length(v, if object_units { 1.0 } else { viewport.width }, 16.0))
                .transpose()?
                .unwrap_or(cx);
            let fy = attributes
                .get("fy")
                .map(|v| length(v, if object_units { 1.0 } else { viewport.height }, 16.0))
                .transpose()?
                .unwrap_or(cy);
            let diagonal = viewport.width.hypot(viewport.height) / std::f64::consts::SQRT_2;
            let radius = coordinate("r", "50%", diagonal)?;
            if radius < 0.0 {
                return Err(invalid("negative radial gradient radius"));
            }
            if (fx - cx).abs() > 1e-8
                || (fy - cy).abs() > 1e-8
                || coordinate("fr", "0", diagonal)?.abs() > 1e-8
            {
                return Err(invalid(
                    "radial gradients with an offset focal point or inner radius are unsupported",
                ));
            }
            let normalized = normalize * map;
            let sx = normalized.a.hypot(normalized.b);
            let sy = normalized.c.hypot(normalized.d);
            if (sx - sy).abs() > 1e-8 * sx.max(sy).max(1.0)
                || (normalized.a * normalized.c + normalized.b * normalized.d).abs()
                    > 1e-8 * sx.max(sy).powi(2).max(1.0)
            {
                return Err(invalid("radial gradient ellipses that differ from the shape aspect ratio are unsupported"));
            }
            Ok(Paint::RadialGradient {
                center: normalized.apply(Point::new(cx, cy)),
                radius: radius * sx,
                stops,
            })
        }
    }

    fn paint(
        &mut self,
        value: &str,
        bounds: Rect,
        viewport: Rect,
        current: Color,
    ) -> ImportResult<Option<Paint>> {
        let value = value.trim();
        if value == "none" {
            return Ok(None);
        }
        if value.to_ascii_lowercase().starts_with("url(") {
            let Some(end) = value.find(')') else {
                return Err(invalid("invalid paint URL"));
            };
            let reference = value[4..end].trim().trim_matches(['\'', '"']);
            let Some(id) = reference.strip_prefix('#') else {
                return Err(invalid("external paint server"));
            };
            if !self.ids.contains_key(id) {
                let fallback = value[end + 1..].trim();
                if !fallback.is_empty() {
                    return self.paint(fallback, bounds, viewport, current);
                }
            }
            return Ok(Some(self.gradient(id, bounds, viewport, current)?));
        }
        Ok(Some(Paint::Solid(color(value, current)?)))
    }

    fn appearance(
        &mut self,
        node: &mut Node,
        style: &SvgStyle,
        bounds: Rect,
        viewport: Rect,
    ) -> ImportResult<()> {
        node.visible = style.display && style.visibility;
        if !style.display {
            node.metadata
                .insert("svg_display_none".into(), "true".into());
        }
        node.opacity = style.opacity;
        node.blend_mode = style.compositing_blend();
        node.fills = self
            .paint(&style.fill, bounds, viewport, style.color)?
            .map(|paint| {
                vec![Fill {
                    paint,
                    opacity: style.fill_opacity,
                    visible: true,
                }]
            })
            .unwrap_or_default();
        node.strokes = self
            .paint(&style.stroke, bounds, viewport, style.color)?
            .filter(|_| style.stroke_width > 0.0)
            .map(|paint| {
                vec![Stroke {
                    paint,
                    width: style.stroke_width,
                    opacity: style.stroke_opacity,
                    visible: true,
                    align: StrokeAlign::Center,
                    cap: style.cap,
                    join: style.join,
                    dash: style.dash.clone(),
                    width_profile: crate::StrokeWidthProfile::Uniform,
                }]
            })
            .unwrap_or_default();
        node.path.fill_rule = style.fill_rule;
        Ok(())
    }

    fn path(&mut self, source: &str) -> ImportResult<VectorPath> {
        use svgtypes::SimplePathSegment as S;
        let mut path = VectorPath::default();
        for segment in svgtypes::SimplifyingPathParser::from(source) {
            self.segments += 1;
            if self.segments > MAX_SVG_SEGMENTS {
                return Err(invalid("path segment limit exceeded"));
            }
            let point = |x, y| -> ImportResult<Point> {
                Ok(Point::new(
                    finite(x, "path coordinate")?,
                    finite(y, "path coordinate")?,
                ))
            };
            path.segments
                .push(match segment.map_err(|e| invalid(e.to_string()))? {
                    S::MoveTo { x, y } => PathSegment::MoveTo {
                        point: point(x, y)?,
                    },
                    S::LineTo { x, y } => PathSegment::LineTo {
                        point: point(x, y)?,
                    },
                    S::CurveTo {
                        x1,
                        y1,
                        x2,
                        y2,
                        x,
                        y,
                    } => PathSegment::CubicTo {
                        control1: point(x1, y1)?,
                        control2: point(x2, y2)?,
                        point: point(x, y)?,
                    },
                    S::Quadratic { x1, y1, x, y } => PathSegment::QuadraticTo {
                        control: point(x1, y1)?,
                        point: point(x, y)?,
                    },
                    S::ClosePath => PathSegment::Close,
                });
        }
        Ok(path)
    }

    /// Tight group bounds without changing any child's world transform.
    fn fit_group(&mut self, id: NodeId, source_transform: Transform) {
        let children = self.document.nodes[&id].children.clone();
        let bounds = children
            .iter()
            .filter_map(|child| self.document.nodes.get(child))
            .filter(|child| !child.is_mask)
            .filter(|child| !child.metadata.contains_key("svg_display_none"))
            .map(|child| child.local_transform().transform_rect(child.local_bounds()))
            .reduce(|a, b| a.union(b))
            .unwrap_or_default();
        for child in children {
            if let Some(node) = self.document.nodes.get_mut(&child) {
                node.set_local_transform(
                    Transform::translation(-bounds.x, -bounds.y) * node.local_transform(),
                );
            }
        }
        if let Some(node) = self.document.nodes.get_mut(&id) {
            node.width = bounds.width;
            node.height = bounds.height;
            node.set_local_transform(source_transform * Transform::translation(bounds.x, bounds.y));
        }
    }

    fn group(
        &mut self,
        element: XmlNode<'a, 'input>,
        parent: NodeId,
        style: &SvgStyle,
        viewport: Rect,
        matrix: Transform,
        depth: usize,
        references: &mut Vec<String>,
    ) -> ImportResult<NodeId> {
        let mut group = Node::new(
            NodeKind::Group,
            Self::label(element, "Group"),
            0.0,
            0.0,
            0.0,
            0.0,
        );
        group.opacity = style.opacity;
        // Visibility inherits to descendants, which may override it. Hiding a
        // paintless container itself would incorrectly suppress those children.
        group.visible = style.display;
        if !style.display {
            group
                .metadata
                .insert("svg_display_none".into(), "true".into());
        }
        group.blend_mode = style.compositing_blend();
        Self::identify(&mut group, element);
        let id = self.insert(parent, group)?;
        for child in element.children().filter(|n| n.is_element()) {
            self.element(child, id, style, viewport, depth + 1, references)?;
        }
        self.fit_group(id, matrix);
        Ok(id)
    }

    fn viewport(
        &mut self,
        element: XmlNode<'a, 'input>,
        parent: NodeId,
        style: &SvgStyle,
        props: &Properties,
        parent_viewport: Rect,
        depth: usize,
        references: &mut Vec<String>,
        root_name: Option<&str>,
    ) -> ImportResult<NodeId> {
        let view_box = parse_view_box(element.attribute("viewBox"))?;
        let default_width = view_box.map(|v| v.width).unwrap_or(parent_viewport.width);
        let default_height = view_box.map(|v| v.height).unwrap_or(parent_viewport.height);
        let width = attribute_length(
            element,
            "width",
            default_width,
            parent_viewport.width,
            style.font_size,
        )?;
        let height = attribute_length(
            element,
            "height",
            default_height,
            parent_viewport.height,
            style.font_size,
        )?;
        if width < 0.0 || height < 0.0 {
            return Err(invalid("negative SVG viewport size"));
        }
        let x = attribute_length(element, "x", 0.0, parent_viewport.width, style.font_size)?;
        let y = attribute_length(element, "y", 0.0, parent_viewport.height, style.font_size)?;
        let mut frame = Node::new(
            NodeKind::Frame,
            root_name
                .map(str::to_string)
                .unwrap_or_else(|| Self::label(element, "SVG viewport")),
            x,
            y,
            width,
            height,
        );
        frame.fills.clear();
        frame.opacity = style.opacity;
        frame.visible = style.display && width > 0.0 && height > 0.0;
        if !style.display {
            frame
                .metadata
                .insert("svg_display_none".into(), "true".into());
        }
        frame.blend_mode = style.compositing_blend();
        frame.clip_content = props.get("overflow").is_none_or(|value| value != "visible");
        frame.set_local_transform(
            transform(props.get("transform").map(String::as_str))? * Transform::translation(x, y),
        );
        Self::identify(&mut frame, element);
        let id = self.insert(parent, frame)?;
        let user_viewport = view_box.unwrap_or(Rect::new(0.0, 0.0, width, height));
        let matrix = view_box_transform(
            user_viewport,
            width,
            height,
            element.attribute("preserveAspectRatio"),
        )?;
        let mut content = Node::new(
            NodeKind::Group,
            "SVG contents",
            0.0,
            0.0,
            user_viewport.width,
            user_viewport.height,
        );
        content.fills.clear();
        content.blend_mode = BlendMode::PassThrough;
        let content_id = self.insert(id, content)?;
        for child in element.children().filter(|n| n.is_element()) {
            self.element(
                child,
                content_id,
                style,
                user_viewport,
                depth + 1,
                references,
            )?;
        }
        self.fit_group(content_id, matrix);
        Ok(id)
    }

    /// Definitions inherit from their XML ancestors, never from the element
    /// which references them. Paint inheritance in <use> remains separate.
    fn definition_style(
        &mut self,
        source: XmlNode<'a, 'input>,
        viewport: Rect,
    ) -> ImportResult<(SvgStyle, Properties)> {
        let mut ancestors: Vec<_> = source.ancestors().filter(|n| n.is_element()).collect();
        if ancestors.len() >= MAX_DOCUMENT_DEPTH {
            return Err(invalid("mask definition hierarchy depth limit exceeded"));
        }
        ancestors.reverse();
        let mut inherited = SvgStyle::default();
        let mut properties = Properties::new();
        for ancestor in ancestors {
            (inherited, properties) = self.style(ancestor, &inherited, viewport)?;
        }
        Ok((inherited, properties))
    }

    fn mask_source(
        &mut self,
        reference: &str,
        clipping: bool,
        mask_mode: Option<MaskType>,
        parent: NodeId,
        bounds: Rect,
        user_to_local: Transform,
        viewport: Rect,
        depth: usize,
        references: &mut Vec<String>,
    ) -> ImportResult<NodeId> {
        if references.len() >= 64 || references.iter().any(|id| id == reference) {
            return Err(invalid("cyclic or excessively deep mask/clip reference"));
        }
        let source = *self
            .ids
            .get(reference)
            .ok_or_else(|| invalid(format!("missing mask/clip definition #{reference}")))?;
        let expected = if clipping { "clipPath" } else { "mask" };
        if source.tag_name().name() != expected
            || source
                .tag_name()
                .namespace()
                .is_some_and(|ns| ns != "http://www.w3.org/2000/svg")
        {
            return Err(invalid(format!("#{reference} must reference <{expected}>")));
        }
        references.push(reference.into());
        let previous_geometry = self.clip_geometry;
        self.clip_geometry = clipping;
        let result = (|| {
            let (style, props) = self.definition_style(source, viewport)?;
            let units = |key: &str, default: &str| -> ImportResult<bool> {
                match props.get(key).map(String::as_str).unwrap_or(default) {
                    "objectBoundingBox" => Ok(true),
                    "userSpaceOnUse" => Ok(false),
                    _ => Err(invalid(format!("unsupported {key}"))),
                }
            };
            let object_content = units(
                if clipping {
                    "clipPathUnits"
                } else {
                    "maskContentUnits"
                },
                "userSpaceOnUse",
            )?;
            let content_map = if object_content {
                Transform::translation(bounds.x, bounds.y)
                    * Transform::scale(bounds.width, bounds.height)
            } else {
                Transform::IDENTITY
            };
            let content_viewport = if object_content {
                Rect::new(0.0, 0.0, 1.0, 1.0)
            } else {
                viewport
            };
            let mask_type = if clipping {
                MaskType::Vector
            } else {
                mask_mode.unwrap_or(style.mask_type)
            };
            if mask_type == MaskType::Luminance && style.linear_rgb {
                return Err(invalid(
                    "linearRGB luminance masks cannot be represented by the sRGB mask model",
                ));
            }
            let region = if clipping {
                Rect::default()
            } else {
                let object_region = units("maskUnits", "objectBoundingBox")?;
                let coordinate =
                    |key: &str, default: &str, horizontal: bool| -> ImportResult<f64> {
                        length(
                            props.get(key).map(String::as_str).unwrap_or(default),
                            if object_region {
                                1.0
                            } else if horizontal {
                                viewport.width
                            } else {
                                viewport.height
                            },
                            style.font_size,
                        )
                    };
                let mut region = Rect::new(
                    coordinate("x", "-10%", true)?,
                    coordinate("y", "-10%", false)?,
                    coordinate("width", "120%", true)?,
                    coordinate("height", "120%", false)?,
                );
                if region.width < 0.0 || region.height < 0.0 {
                    return Err(invalid("negative mask region size"));
                }
                if object_region {
                    region = Rect::new(
                        bounds.x + region.x * bounds.width,
                        bounds.y + region.y * bounds.height,
                        region.width * bounds.width,
                        region.height * bounds.height,
                    );
                }
                region
            };
            // The role stays visible even for an empty or display:none source:
            // transparent mask pixels must hide content, rather than disable it.
            let mut role = Node::new(
                if clipping {
                    NodeKind::Group
                } else {
                    NodeKind::Frame
                },
                format!("{expected} #{reference}"),
                0.0,
                0.0,
                region.width,
                region.height,
            );
            role.fills.clear();
            role.blend_mode = BlendMode::Normal;
            role.is_mask = true;
            role.mask_type = mask_type;
            role.clip_content = !clipping;
            role.metadata
                .insert("svg_mask_reference".into(), reference.into());
            role.metadata
                .insert("svg_mask_kind".into(), expected.into());
            let role_id = self.insert(parent, role)?;
            if clipping {
                for child in source.children().filter(|n| n.is_element()) {
                    if ![
                        "title", "desc", "metadata", "rect", "circle", "ellipse", "line",
                        "polyline", "polygon", "path", "text", "use",
                    ]
                    .contains(&child.tag_name().name())
                    {
                        return Err(invalid(
                            "clipPath children must be shapes, text, or direct use references",
                        ));
                    }
                }
            }
            let source_matrix = transform(props.get("transform").map(String::as_str))?;
            let content = self.group(
                source,
                role_id,
                &style,
                content_viewport,
                source_matrix,
                depth + 1,
                references,
            )?;
            let content = self.apply_compositing(
                source,
                content,
                &style,
                &props,
                content_viewport,
                depth + 1,
                references,
            )?;
            // A custom hint must never override the SVG's actual alpha
            // semantics. Only the opaque geometry emitted by our vector-mask
            // exporter is equivalent to an editable native vector mask.
            if !clipping
                && mask_type == MaskType::Alpha
                && props
                    .get("data-ferro-mask-type")
                    .is_some_and(|v| v == "vector")
                && self.vector_mask_hint_matches(content)
            {
                self.document.nodes.get_mut(&role_id).unwrap().mask_type = MaskType::Vector;
            }
            let node = self.document.nodes.get_mut(&content).unwrap();
            node.set_local_transform(
                Transform::translation(-region.x, -region.y) * content_map * node.local_transform(),
            );
            if clipping {
                self.fit_group(role_id, user_to_local);
            } else {
                self.document
                    .nodes
                    .get_mut(&role_id)
                    .unwrap()
                    .set_local_transform(
                        user_to_local * Transform::translation(region.x, region.y),
                    );
            }
            Ok(role_id)
        })();
        self.clip_geometry = previous_geometry;
        references.pop();
        result
    }

    fn vector_mask_hint_matches(&self, id: NodeId) -> bool {
        fn opaque(paint: &Paint) -> bool {
            match paint {
                Paint::Solid(color) => color.a == 255,
                Paint::LinearGradient { stops, .. } | Paint::RadialGradient { stops, .. }
                | Paint::AngularGradient { stops, .. } => {
                    !stops.is_empty() && stops.iter().all(|stop| stop.color.a == 255)
                }
                Paint::Image { .. } => false,
            }
        }
        self.document.descendants(id).iter().all(|id| {
            let node = &self.document.nodes[id];
            !node.visible
                || node.opacity == 0.0
                || (node.opacity == 1.0
                    && !(node.is_mask && node.mask_type == MaskType::Luminance)
                    && !node.effects.iter().any(|effect| effect.visible)
                    && node
                        .fills
                        .iter()
                        .filter(|fill| fill.visible)
                        .all(|fill| fill.opacity == 1.0 && opaque(&fill.paint))
                    && node
                        .strokes
                        .iter()
                        .filter(|stroke| stroke.visible && stroke.width > 0.0)
                        .all(|stroke| stroke.opacity == 1.0 && opaque(&stroke.paint))
                    && node
                        .text_runs
                        .iter()
                        .all(|run| run.color.is_none_or(|color| color.a == 255)))
        })
    }

    /// Materialize a reusable SVG definition as an editable sibling mask and
    /// isolate that one referenced element. Geometry normalization changes no
    /// world transform; opacity/blending are applied once outside the masks.
    fn apply_compositing(
        &mut self,
        element: XmlNode<'a, 'input>,
        id: NodeId,
        style: &SvgStyle,
        props: &Properties,
        viewport: Rect,
        depth: usize,
        references: &mut Vec<String>,
    ) -> ImportResult<NodeId> {
        if style.mask.is_none() && style.clip_path.is_none() {
            return Ok(id);
        }
        if depth >= MAX_DOCUMENT_DEPTH - 5 {
            return Err(invalid("masked SVG hierarchy depth limit exceeded"));
        }
        let original = self.document.nodes[&id].clone();
        let matrix = original.local_transform();
        let Some(inverse) = matrix.inverse() else {
            // A singular 2D transform has no visible area.
            self.document.nodes.get_mut(&id).unwrap().visible = false;
            return Ok(id);
        };
        let mut user_to_parent = transform(props.get("transform").map(String::as_str))?;
        let mut user_viewport = viewport;
        if element.tag_name().name() == "svg" {
            user_viewport = parse_view_box(element.attribute("viewBox"))?.unwrap_or(Rect::new(
                0.0,
                0.0,
                original.width,
                original.height,
            ));
            user_to_parent = matrix
                * view_box_transform(
                    user_viewport,
                    original.width,
                    original.height,
                    element.attribute("preserveAspectRatio"),
                )?;
        }
        let user_to_local = inverse * user_to_parent;
        let geometry_bounds = if element.tag_name().name() == "svg" {
            original
                .children
                .iter()
                .filter_map(|id| self.document.nodes.get(id))
                .filter(|n| !n.is_mask && !n.metadata.contains_key("svg_display_none"))
                .map(|n| n.local_transform().transform_rect(n.local_bounds()))
                .reduce(|a, b| a.union(b))
                .unwrap_or_default()
        } else {
            original.local_bounds()
        };
        let bounds = user_to_local
            .inverse()
            .map(|m| m.transform_rect(geometry_bounds))
            .unwrap_or_default();
        let parent = original
            .parent
            .ok_or_else(|| invalid("masked element has no parent"))?;
        let position = self
            .document
            .children(parent)
            .unwrap()
            .iter()
            .position(|n| *n == id)
            .unwrap();
        let mut wrapper = Node::new(
            NodeKind::Group,
            format!("{} compositing", original.name),
            0.0,
            0.0,
            original.width,
            original.height,
        );
        wrapper.fills.clear();
        wrapper.opacity = original.opacity;
        wrapper.visible = original.visible;
        if original.metadata.contains_key("svg_display_none") {
            wrapper
                .metadata
                .insert("svg_display_none".into(), "true".into());
        }
        wrapper.blend_mode = original.blend_mode;
        wrapper.set_local_transform(matrix);
        wrapper
            .metadata
            .insert("svg_compositing_wrapper".into(), "true".into());
        let wrapper_id = self.insert(parent, wrapper)?;
        self.document.reparent(wrapper_id, parent, position)?;
        if let Some(reference) = &style.clip_path {
            self.mask_source(
                reference,
                true,
                None,
                wrapper_id,
                bounds,
                user_to_local,
                user_viewport,
                depth + 1,
                references,
            )?;
        }
        if let Some(reference) = &style.mask {
            self.mask_source(
                reference,
                false,
                style.mask_mode,
                wrapper_id,
                bounds,
                user_to_local,
                user_viewport,
                depth + 1,
                references,
            )?;
        }
        self.document.reparent(id, wrapper_id, usize::MAX)?;
        let node = self.document.nodes.get_mut(&id).unwrap();
        node.set_local_transform(Transform::IDENTITY);
        node.opacity = 1.0;
        node.visible = true;
        node.blend_mode = BlendMode::PassThrough;
        Ok(wrapper_id)
    }

    fn element(
        &mut self,
        element: XmlNode<'a, 'input>,
        parent: NodeId,
        inherited: &SvgStyle,
        viewport: Rect,
        depth: usize,
        references: &mut Vec<String>,
    ) -> ImportResult<Option<NodeId>> {
        let Some(id) =
            self.element_unmasked(element, parent, inherited, viewport, depth, references)?
        else {
            return Ok(None);
        };
        let (style, props) = self.style(element, inherited, viewport)?;
        self.apply_compositing(element, id, &style, &props, viewport, depth, references)
            .map(Some)
    }

    fn element_unmasked(
        &mut self,
        element: XmlNode<'a, 'input>,
        parent: NodeId,
        inherited: &SvgStyle,
        viewport: Rect,
        depth: usize,
        references: &mut Vec<String>,
    ) -> ImportResult<Option<NodeId>> {
        if depth >= MAX_DOCUMENT_DEPTH - 3 {
            return Err(invalid("SVG hierarchy depth limit exceeded"));
        }
        let tag = element.tag_name().name();
        if [
            "defs",
            "title",
            "desc",
            "metadata",
            "style",
            "linearGradient",
            "radialGradient",
            "stop",
            "symbol",
            "clipPath",
            "mask",
            "filter",
            "pattern",
            "marker",
        ]
        .contains(&tag)
        {
            return Ok(None);
        }
        if element
            .tag_name()
            .namespace()
            .is_some_and(|ns| ns != "http://www.w3.org/2000/svg")
        {
            return Err(invalid(format!(
                "foreign namespace element <{tag}> is unsupported"
            )));
        }
        let (style, props) = self.style(element, inherited, viewport)?;
        let matrix = transform(props.get("transform").map(String::as_str))?;
        if tag == "svg" {
            return self
                .viewport(
                    element, parent, &style, &props, viewport, depth, references, None,
                )
                .map(Some);
        }
        if tag == "g" || tag == "a" {
            return self
                .group(element, parent, &style, viewport, matrix, depth, references)
                .map(Some);
        }
        if tag == "use" {
            let reference = href(element)
                .ok_or_else(|| invalid("use element has no href"))?
                .trim_start_matches('#');
            if references.iter().any(|id| id == reference) {
                return Err(invalid("cyclic use reference"));
            }
            let source = *self
                .ids
                .get(reference)
                .ok_or_else(|| invalid(format!("missing use target #{reference}")))?;
            if self.clip_geometry
                && ![
                    "rect", "circle", "ellipse", "line", "polyline", "polygon", "path", "text",
                ]
                .contains(&source.tag_name().name())
            {
                return Err(invalid(
                    "clipPath use must directly reference a shape or text",
                ));
            }
            let x = attribute_length(element, "x", 0.0, viewport.width, style.font_size)?;
            let y = attribute_length(element, "y", 0.0, viewport.height, style.font_size)?;
            let matrix = matrix * Transform::translation(x, y);
            let mut group = Node::new(
                NodeKind::Group,
                Self::label(element, "SVG instance"),
                0.0,
                0.0,
                0.0,
                0.0,
            );
            group.opacity = style.opacity;
            group.visible = style.display;
            if !style.display {
                group
                    .metadata
                    .insert("svg_display_none".into(), "true".into());
            }
            group.blend_mode = style.compositing_blend();
            Self::identify(&mut group, element);
            let id = self.insert(parent, group)?;
            references.push(reference.to_string());
            if source.tag_name().name() == "symbol" {
                let (symbol_style, symbol_props) = self.style(source, &style, viewport)?;
                let view_box = parse_view_box(source.attribute("viewBox"))?.unwrap_or(viewport);
                let width = attribute_length(
                    element,
                    "width",
                    view_box.width,
                    viewport.width,
                    style.font_size,
                )?;
                let height = attribute_length(
                    element,
                    "height",
                    view_box.height,
                    viewport.height,
                    style.font_size,
                )?;
                if width < 0.0 || height < 0.0 {
                    return Err(invalid("negative use size"));
                }
                let symbol_matrix =
                    view_box_transform(
                        view_box,
                        width,
                        height,
                        source.attribute("preserveAspectRatio"),
                    )? * transform(symbol_props.get("transform").map(String::as_str))?;
                self.group(
                    source,
                    id,
                    &symbol_style,
                    view_box,
                    symbol_matrix,
                    depth + 1,
                    references,
                )?;
            } else {
                self.element(source, id, &style, viewport, depth + 1, references)?;
            }
            references.pop();
            self.fit_group(id, matrix);
            return Ok(Some(id));
        }
        if tag == "text" {
            return self
                .text(element, parent, &style, &props, viewport, matrix)
                .map(Some);
        }
        let attr = |key, default, reference| {
            attribute_length(element, key, default, reference, style.font_size)
        };
        let mut path = None;
        let (kind, bounds, radius) = match tag {
            "rect" | "image" => {
                let x = attr("x", 0.0, viewport.width)?;
                let y = attr("y", 0.0, viewport.height)?;
                let width = attr("width", 0.0, viewport.width)?;
                let height = attr("height", 0.0, viewport.height)?;
                if width < 0.0 || height < 0.0 {
                    return Err(invalid("negative rectangle size"));
                }
                let rx = if element.attribute("rx").is_none() {
                    attr("ry", 0.0, viewport.width)?
                } else {
                    attr("rx", 0.0, viewport.width)?
                }
                .max(0.0)
                .min(width / 2.0);
                let ry = if element.attribute("ry").is_none() {
                    rx
                } else {
                    attr("ry", 0.0, viewport.height)?
                }
                .max(0.0)
                .min(height / 2.0);
                if tag == "rect" && (rx - ry).abs() > 1e-8 && rx > 0.0 && ry > 0.0 {
                    path = Some(rounded_rectangle(width, height, rx, ry));
                    (NodeKind::Vector, Rect::new(x, y, width, height), 0.0)
                } else {
                    (
                        NodeKind::Rectangle,
                        Rect::new(x, y, width, height),
                        rx.min(ry),
                    )
                }
            }
            "circle" | "ellipse" => {
                let cx = attr("cx", 0.0, viewport.width)?;
                let cy = attr("cy", 0.0, viewport.height)?;
                let diagonal = viewport.width.hypot(viewport.height) / std::f64::consts::SQRT_2;
                let rx = if tag == "circle" {
                    attr("r", 0.0, diagonal)?
                } else {
                    attr("rx", 0.0, viewport.width)?
                };
                let ry = if tag == "circle" {
                    rx
                } else {
                    attr("ry", 0.0, viewport.height)?
                };
                if rx < 0.0 || ry < 0.0 {
                    return Err(invalid("negative ellipse radius"));
                }
                (
                    NodeKind::Ellipse,
                    Rect::new(cx - rx, cy - ry, rx * 2.0, ry * 2.0),
                    0.0,
                )
            }
            "line" | "polyline" | "polygon" | "path" => {
                let absolute = match tag {
                    "line" => VectorPath::polygon(
                        &[
                            Point::new(
                                attr("x1", 0.0, viewport.width)?,
                                attr("y1", 0.0, viewport.height)?,
                            ),
                            Point::new(
                                attr("x2", 0.0, viewport.width)?,
                                attr("y2", 0.0, viewport.height)?,
                            ),
                        ],
                        false,
                    ),
                    "polyline" | "polygon" => {
                        let points = numbers(element.attribute("points").unwrap_or_default())?;
                        if points.len() % 2 != 0 {
                            return Err(invalid("odd number of polygon coordinates"));
                        }
                        self.segments += points.len() / 2;
                        if self.segments > MAX_SVG_SEGMENTS {
                            return Err(invalid("path segment limit exceeded"));
                        }
                        VectorPath::polygon(
                            &points
                                .chunks_exact(2)
                                .map(|p| Point::new(p[0], p[1]))
                                .collect::<Vec<_>>(),
                            tag == "polygon",
                        )
                    }
                    _ => self.path(element.attribute("d").unwrap_or_default())?,
                };
                let bounds = absolute.bounds().unwrap_or_default();
                path = Some(absolute.transformed(Transform::translation(-bounds.x, -bounds.y)));
                (NodeKind::Vector, bounds, 0.0)
            }
            _ => {
                return Err(invalid(format!(
                    "element <{tag}> is outside the editable SVG import subset"
                )))
            }
        };
        let mut node = Node::new(
            kind,
            Self::label(
                element,
                match tag {
                    "rect" => "Rectangle",
                    "circle" | "ellipse" => "Ellipse",
                    "image" => "Image",
                    "line" => "Line",
                    "polygon" => "Polygon",
                    "polyline" => "Polyline",
                    _ => "Vector",
                },
            ),
            bounds.x,
            bounds.y,
            bounds.width,
            bounds.height,
        );
        node.corner_radius = radius;
        if let Some(path) = path {
            node.path = path;
        }
        self.appearance(&mut node, &style, bounds, viewport)?;
        if tag == "line" {
            node.fills.clear();
        }
        if tag == "image" {
            let data = href(element).ok_or_else(|| invalid("image has no embedded data"))?;
            if !raster_data_url(data) {
                return Err(invalid("image must be an embedded raster data URL"));
            }
            let aspect = element
                .attribute("preserveAspectRatio")
                .unwrap_or("xMidYMid meet");
            let scale = if aspect == "none" {
                ImageScale::Stretch
            } else if aspect.starts_with("xMidYMid") {
                if aspect.split_whitespace().any(|v| v == "slice") {
                    ImageScale::Fill
                } else {
                    ImageScale::Fit
                }
            } else {
                return Err(invalid(
                    "image alignment other than xMidYMid is unsupported",
                ));
            };
            node.fills = vec![Fill {
                paint: Paint::Image {
                    href: data.to_string(),
                    scale,
                    crop: None,
                },
                opacity: 1.0,
                visible: true,
            }];
            node.strokes.clear();
        }
        node.set_local_transform(matrix * Transform::translation(bounds.x, bounds.y));
        Self::identify(&mut node, element);
        self.insert(parent, node).map(Some)
    }

    fn text(
        &mut self,
        element: XmlNode<'a, 'input>,
        parent: NodeId,
        style: &SvgStyle,
        props: &Properties,
        viewport: Rect,
        matrix: Transform,
    ) -> ImportResult<NodeId> {
        if element.attribute("textLength").is_some() || element.attribute("rotate").is_some() {
            return Err(invalid(
                "textLength and per-character rotation are outside the editable text subset",
            ));
        }
        let x = attribute_length(element, "x", 0.0, viewport.width, style.font_size)?
            + attribute_length(element, "dx", 0.0, viewport.width, style.font_size)?;
        let y = attribute_length(element, "y", 0.0, viewport.height, style.font_size)?
            + attribute_length(element, "dy", 0.0, viewport.height, style.font_size)?;
        let preserve = element
            .ancestors()
            .find_map(|n| n.attribute(("http://www.w3.org/XML/1998/namespace", "space")))
            .is_some_and(|v| v == "preserve");
        let mut content = String::new();
        let mut runs = Vec::new();
        self.text_content(
            element,
            style,
            viewport,
            preserve,
            &mut content,
            &mut runs,
            0,
        )?;
        if !preserve {
            let trimmed = content.trim_end_matches(' ').len();
            content.truncate(trimmed);
            for run in &mut runs {
                run.end = run.end.min(trimmed);
            }
            runs.retain(|run| run.start < run.end);
        }
        let mut estimated_width = 0.0;
        let mut max_font_size = style.font_size;
        for run in &runs {
            let size = run.font_size.unwrap_or(style.font_size);
            max_font_size = max_font_size.max(size);
            // Layout is unbounded in the renderer. These are selection bounds;
            // imported text remains Unicode text and is shaped by FerroUI.
            estimated_width += content[run.start..run.end]
                .chars()
                .map(|c| {
                    if c.is_whitespace() {
                        size * 0.33
                    } else if c.is_ascii_punctuation() || "ilI".contains(c) {
                        size * 0.3
                    } else if "MW@".contains(c) {
                        size * 0.9
                    } else if c as u32 > 0x2e80 {
                        size
                    } else {
                        size * 0.6
                    }
                })
                .sum::<f64>()
                + style.letter_spacing * content[run.start..run.end].chars().count() as f64;
        }
        let width = estimated_width.max(1.0);
        let height = max_font_size * 1.2;
        let mut base_color = if style.fill == "none" {
            Color::TRANSPARENT
        } else if style.fill.starts_with("url(") {
            style.color
        } else {
            color(&style.fill, style.color)?
        };
        base_color.a = (base_color.a as f64 * style.fill_opacity).round() as u8;
        for run in &mut runs {
            // Inherited values remain inherited when the layer is edited later.
            if run.font_family.as_ref() == Some(&style.font_family) {
                run.font_family = None;
            }
            if run.font_size == Some(style.font_size) {
                run.font_size = None;
            }
            if run.font_weight == Some(style.font_weight) {
                run.font_weight = None;
            }
            if run.italic == Some(style.italic) {
                run.italic = None;
            }
            if run.decoration == Some(style.decoration) {
                run.decoration = None;
            }
            if run.color == Some(base_color) {
                run.color = None;
            }
        }
        let baseline = match props
            .get("dominant-baseline")
            .or_else(|| props.get("alignment-baseline"))
            .map(String::as_str)
            .unwrap_or("alphabetic")
        {
            "auto" | "baseline" | "alphabetic" => max_font_size * 0.8,
            "hanging" | "text-before-edge" | "text-top" => 0.0,
            "middle" | "central" => height / 2.0,
            "text-after-edge" | "text-bottom" => height,
            _ => return Err(invalid("unsupported text baseline alignment")),
        };
        let left = x - match style.anchor.as_str() {
            "middle" => width / 2.0,
            "end" => width,
            _ => 0.0,
        };
        let bounds = Rect::new(left, y - baseline, width, height);
        let mut node = Node::new(
            NodeKind::Text,
            Self::label(element, if content.is_empty() { "Text" } else { &content }),
            bounds.x,
            bounds.y,
            bounds.width,
            bounds.height,
        );
        node.text = content;
        node.font_family = style.font_family.clone();
        node.font_size = style.font_size;
        node.font_weight = style.font_weight;
        node.italic = style.italic;
        node.letter_spacing = style.letter_spacing;
        node.text_decoration = style.decoration;
        node.line_height = 1.2;
        node.text_auto_resize = TextAutoResize::WidthAndHeight;
        node.text_runs = runs;
        self.appearance(&mut node, style, bounds, viewport)?;
        node.set_local_transform(matrix * Transform::translation(bounds.x, bounds.y));
        Self::identify(&mut node, element);
        node.metadata
            .insert("svg_text_anchor".into(), style.anchor.clone());
        node.metadata
            .insert("svg_text_baseline_offset".into(), baseline.to_string());
        self.warnings.insert("Text selection bounds use estimated font metrics; text remains editable and the renderer shapes the original Unicode content.".into());
        self.insert(parent, node)
    }

    fn text_content(
        &mut self,
        element: XmlNode<'a, 'input>,
        style: &SvgStyle,
        viewport: Rect,
        preserve: bool,
        text: &mut String,
        runs: &mut Vec<TextRun>,
        depth: usize,
    ) -> ImportResult<()> {
        if depth >= MAX_DOCUMENT_DEPTH - 4 {
            return Err(invalid("text hierarchy depth limit exceeded"));
        }
        for child in element.children() {
            if child.is_text() {
                let start = text.len();
                for mut c in child.text().unwrap_or_default().chars() {
                    if matches!(c, '\n' | '\r' | '\t') {
                        c = ' ';
                    }
                    if !preserve && c == ' ' && (text.is_empty() || text.ends_with(' ')) {
                        continue;
                    }
                    text.push(c);
                }
                let end = text.len();
                if end > start {
                    let mut run_color = if style.fill == "none" {
                        Color::TRANSPARENT
                    } else if style.fill.starts_with("url(") {
                        style.color
                    } else {
                        color(&style.fill, style.color)?
                    };
                    run_color.a = (run_color.a as f64 * style.fill_opacity).round() as u8;
                    runs.push(TextRun {
                        start,
                        end,
                        font_family: Some(style.font_family.clone()),
                        font_size: Some(style.font_size),
                        font_weight: Some(style.font_weight),
                        color: if style.fill.starts_with("url(") {
                            None
                        } else {
                            Some(run_color)
                        },
                        italic: Some(style.italic),
                        decoration: Some(style.decoration),
                    });
                }
            } else if child.is_element() {
                if ["title", "desc"].contains(&child.tag_name().name()) {
                    continue;
                }
                if child.tag_name().name() != "tspan" {
                    return Err(invalid(
                        "only text and tspan children are supported inside text",
                    ));
                }
                if ["x", "y", "dx", "dy", "rotate", "textLength"]
                    .iter()
                    .any(|key| child.attribute(*key).is_some())
                {
                    return Err(invalid("positioned tspan elements are unsupported; split them into separate text elements"));
                }
                let (child_style, props) = self.style(child, style, viewport)?;
                if props
                    .get("baseline-shift")
                    .is_some_and(|v| v != "baseline" && v != "0")
                    || child_style.letter_spacing != style.letter_spacing
                {
                    return Err(invalid(
                        "per-span baseline or letter-spacing changes are unsupported",
                    ));
                }
                if child_style.fill.starts_with("url(") && child_style.fill != style.fill {
                    return Err(invalid(
                        "different gradient paints in a text span are unsupported",
                    ));
                }
                if child_style.opacity != 1.0 {
                    return Err(invalid(
                        "independent tspan opacity is unsupported; use fill-opacity",
                    ));
                }
                if child_style.display && child_style.visibility {
                    let preserve = child
                        .attribute(("http://www.w3.org/XML/1998/namespace", "space"))
                        .map(|v| v == "preserve")
                        .unwrap_or(preserve);
                    self.text_content(
                        child,
                        &child_style,
                        viewport,
                        preserve,
                        text,
                        runs,
                        depth + 1,
                    )?;
                }
            }
        }
        Ok(())
    }
}

fn parse_view_box(value: Option<&str>) -> ImportResult<Option<Rect>> {
    let Some(value) = value else { return Ok(None) };
    let n = numbers(value)?;
    if n.len() != 4 || n[2] <= 0.0 || n[3] <= 0.0 {
        return Err(invalid(
            "viewBox must have four numbers and positive width/height",
        ));
    }
    Ok(Some(Rect::new(n[0], n[1], n[2], n[3])))
}

fn view_box_transform(
    view_box: Rect,
    width: f64,
    height: f64,
    aspect: Option<&str>,
) -> ImportResult<Transform> {
    if view_box.width == 0.0 || view_box.height == 0.0 {
        return Ok(Transform::IDENTITY);
    }
    let mut tokens = aspect.unwrap_or("xMidYMid meet").split_whitespace();
    let mut alignment = tokens.next().unwrap_or("xMidYMid");
    if alignment == "defer" {
        alignment = tokens.next().unwrap_or("xMidYMid");
    }
    let mode = tokens.next().unwrap_or("meet");
    if tokens.next().is_some() || !["meet", "slice"].contains(&mode) {
        return Err(invalid("invalid preserveAspectRatio"));
    }
    let sx = width / view_box.width;
    let sy = height / view_box.height;
    if alignment == "none" {
        return Ok(Transform::scale(sx, sy) * Transform::translation(-view_box.x, -view_box.y));
    }
    let alignment = match alignment {
        "xMinYMin" => (0.0, 0.0),
        "xMidYMin" => (0.5, 0.0),
        "xMaxYMin" => (1.0, 0.0),
        "xMinYMid" => (0.0, 0.5),
        "xMidYMid" => (0.5, 0.5),
        "xMaxYMid" => (1.0, 0.5),
        "xMinYMax" => (0.0, 1.0),
        "xMidYMax" => (0.5, 1.0),
        "xMaxYMax" => (1.0, 1.0),
        _ => return Err(invalid("invalid preserveAspectRatio alignment")),
    };
    let scale = if mode == "slice" {
        sx.max(sy)
    } else {
        sx.min(sy)
    };
    Ok(Transform::translation(
        (width - view_box.width * scale) * alignment.0,
        (height - view_box.height * scale) * alignment.1,
    ) * Transform::scale(scale, scale)
        * Transform::translation(-view_box.x, -view_box.y))
}

fn rounded_rectangle(w: f64, h: f64, rx: f64, ry: f64) -> VectorPath {
    let k = 0.5522847498307936;
    VectorPath {
        fill_rule: FillRule::NonZero,
        anchor_modes: Default::default(),
        segments: vec![
            PathSegment::MoveTo {
                point: Point::new(rx, 0.0),
            },
            PathSegment::LineTo {
                point: Point::new(w - rx, 0.0),
            },
            PathSegment::CubicTo {
                control1: Point::new(w - rx + rx * k, 0.0),
                control2: Point::new(w, ry - ry * k),
                point: Point::new(w, ry),
            },
            PathSegment::LineTo {
                point: Point::new(w, h - ry),
            },
            PathSegment::CubicTo {
                control1: Point::new(w, h - ry + ry * k),
                control2: Point::new(w - rx + rx * k, h),
                point: Point::new(w - rx, h),
            },
            PathSegment::LineTo {
                point: Point::new(rx, h),
            },
            PathSegment::CubicTo {
                control1: Point::new(rx - rx * k, h),
                control2: Point::new(0.0, h - ry + ry * k),
                point: Point::new(0.0, h - ry),
            },
            PathSegment::LineTo {
                point: Point::new(0.0, ry),
            },
            PathSegment::CubicTo {
                control1: Point::new(0.0, ry - ry * k),
                control2: Point::new(rx - rx * k, 0.0),
                point: Point::new(rx, 0.0),
            },
            PathSegment::Close,
        ],
    }
}

/// Parse into a new isolated document. Callers can inspect warnings before using
/// the editor API; a parse/validation error cannot mutate an existing document.
pub fn parse_svg(source: &str, name: &str) -> ImportResult<SvgImport> {
    if source.len() > MAX_SVG_BYTES {
        return Err(invalid("file exceeds the 16 MiB import limit"));
    }
    let xml = roxmltree::Document::parse_with_options(
        source,
        roxmltree::ParsingOptions {
            allow_dtd: false,
            nodes_limit: (MAX_DOCUMENT_NODES * 8) as u32,
        },
    )
    .map_err(|e| invalid(e.to_string()))?;
    preflight(&xml)?;
    let root = xml.root_element();
    if root.tag_name().name() != "svg"
        || root
            .tag_name()
            .namespace()
            .is_some_and(|ns| ns != "http://www.w3.org/2000/svg")
    {
        return Err(invalid("root element must be <svg>"));
    }
    let mut ids = BTreeMap::new();
    for node in xml.descendants().filter(|n| n.is_element()) {
        if let Some(id) = node.attribute("id") {
            if ids.insert(id.to_string(), node).is_some() {
                return Err(invalid(format!("duplicate SVG ID: {id}")));
            }
        }
    }
    let name = if name.trim().is_empty() {
        "Imported SVG"
    } else {
        name.trim()
    };
    let mut importer = Importer {
        document: Document::new(name),
        ids,
        rules: css_rules(&xml)?,
        warnings: BTreeSet::new(),
        segments: 0,
        clip_geometry: false,
    };
    let page = importer.document.pages[0].id;
    let outer_viewport = Rect::new(0.0, 0.0, 300.0, 150.0);
    let (style, props) = importer.style(root, &SvgStyle::default(), outer_viewport)?;
    let mut references = Vec::new();
    let root_id = importer.viewport(
        root,
        page,
        &style,
        &props,
        outer_viewport,
        0,
        &mut references,
        Some(name),
    )?;
    let root_id = importer.apply_compositing(
        root,
        root_id,
        &style,
        &props,
        outer_viewport,
        0,
        &mut references,
    )?;
    let warnings: Vec<_> = importer.warnings.into_iter().collect();
    if !warnings.is_empty() {
        importer
            .document
            .nodes
            .get_mut(&root_id)
            .unwrap()
            .metadata
            .insert("svg_import_warnings".into(), warnings.join("\n"));
    }
    importer.document.validate()?;
    Ok(SvgImport {
        document: importer.document,
        root: root_id,
        warnings,
    })
}

impl Document {
    pub fn from_svg(source: &str, name: &str) -> ImportResult<Self> {
        parse_svg(source, name).map(|result| result.document)
    }
}

#[cfg(test)]
#[path = "svg_compositing_tests.rs"]
mod compositing_tests;
