use crate::collections::RibbonFixedItems;
use crate::collections::{RibbonArray, RibbonList, RibbonReadOnlyList};
use crate::text::{RibbonString, key_is, trim, whitespace};
use ferroui_system::invariant_numbers::{InvariantF64, NumberStyles, try_parse_double};
use std::rc::Rc;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct RibbonIconKind(i32);
#[allow(non_upper_case_globals)]
impl RibbonIconKind {
    pub const Glyph: Self = Self(0);
    pub const Path: Self = Self(1);
    pub const Image: Self = Self(2);
    pub const Text: Self = Self(3);
    pub const fn from_raw(value: i32) -> Self {
        Self(value)
    }
    pub const fn raw(self) -> i32 {
        self.0
    }
}
impl std::fmt::Display for RibbonIconKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Glyph => f.write_str("Glyph"),
            Self::Path => f.write_str("Path"),
            Self::Image => f.write_str("Image"),
            Self::Text => f.write_str("Text"),
            _ => write!(f, "{}", self.0),
        }
    }
}

/// Source sealed record: reference identity is retained by Rc; record equality is structural.
#[derive(Clone, Debug)]
pub struct RibbonIcon {
    pub kind: RibbonIconKind,
    pub value: Option<RibbonString>,
    pub font_family: Option<RibbonString>,
    pub foreground: Option<RibbonString>,
    pub view_box_size: f64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RibbonIconError {
    NullLayers,
    NullLayer,
}
impl std::fmt::Display for RibbonIconError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NullLayers => "ArgumentNullException: layers",
            Self::NullLayer => "NullReferenceException",
        })
    }
}
impl std::error::Error for RibbonIconError {}
pub type RibbonIconLayerArray = RibbonArray<Option<Rc<RibbonIconLayer>>>;
pub type RibbonIconLayerList = RibbonList<Option<Rc<RibbonIconLayer>>>;
pub type RibbonIconLayers = RibbonReadOnlyList<Option<Rc<RibbonIconLayer>>>;
impl PartialEq for RibbonIcon {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
            && self.value == other.value
            && self.font_family == other.font_family
            && self.foreground == other.foreground
            && double_equals(self.view_box_size, other.view_box_size)
    }
}
impl RibbonIcon {
    pub fn new(kind: RibbonIconKind, value: Option<RibbonString>) -> Rc<Self> {
        Self::new_with_options(kind, value, None, None, 24.)
    }
    pub fn glyph(value: Option<RibbonString>) -> Rc<Self> {
        Self::glyph_with_options(value, None, None)
    }
    pub fn path(data: Option<RibbonString>) -> Rc<Self> {
        Self::path_with_options(data, 24., None)
    }
    pub fn stroke(data: Option<RibbonString>) -> Rc<Self> {
        Self::stroke_with_options(data, 1.5, 24., None)
    }
    pub fn text(value: Option<RibbonString>) -> Rc<Self> {
        Self::text_with_foreground(value, None)
    }
    pub fn new_with_options(
        kind: RibbonIconKind,
        value: Option<RibbonString>,
        font_family: Option<RibbonString>,
        foreground: Option<RibbonString>,
        view_box_size: f64,
    ) -> Rc<Self> {
        Rc::new(Self {
            kind,
            value,
            font_family,
            foreground,
            view_box_size,
        })
    }
    pub fn glyph_with_options(
        value: Option<RibbonString>,
        font_family: Option<RibbonString>,
        foreground: Option<RibbonString>,
    ) -> Rc<Self> {
        Self::new_with_options(RibbonIconKind::Glyph, value, font_family, foreground, 24.)
    }
    pub fn path_with_options(
        data: Option<RibbonString>,
        view_box_size: f64,
        foreground: Option<RibbonString>,
    ) -> Rc<Self> {
        Self::new_with_options(RibbonIconKind::Path, data, None, foreground, view_box_size)
    }
    pub fn stroke_with_options(
        data: Option<RibbonString>,
        thickness: f64,
        view_box_size: f64,
        foreground: Option<RibbonString>,
    ) -> Rc<Self> {
        Self::path_with_options(
            RibbonIconLayer::with_options(data, thickness, None, 1.).to_text(),
            view_box_size,
            foreground,
        )
    }
    /// Source params-array entry point, including null array and null element errors.
    pub fn layers(
        view_box_size: f64,
        layers: Option<&RibbonIconLayerArray>,
    ) -> Result<Rc<Self>, RibbonIconError> {
        let layers = layers.ok_or(RibbonIconError::NullLayers)?;
        let mut units = Vec::new();
        for (index, layer) in layers.iter().enumerate() {
            let layer = layer.ok_or(RibbonIconError::NullLayer)?;
            if index > 0 {
                units.push(b'|' as u16);
            }
            if let Some(text) = layer.to_text() {
                units.extend_from_slice(text.as_utf16());
            }
        }
        Ok(Self::path_with_options(
            Some(RibbonString::from_utf16(units)),
            view_box_size,
            None,
        ))
    }
    /// Rust convenience for non-null layers; source Layers maps to the array entry point.
    pub fn layers_from_items(view_box_size: f64, layers: &[Rc<RibbonIconLayer>]) -> Rc<Self> {
        Self::layers(
            view_box_size,
            Some(&RibbonFixedItems::new(
                layers.iter().cloned().map(Some).collect(),
            )),
        )
        .expect("non-null layer slice")
    }
    pub fn image(uri: Option<RibbonString>) -> Rc<Self> {
        Self::new_with_options(RibbonIconKind::Image, uri, None, None, 24.)
    }
    pub fn text_with_foreground(
        text: Option<RibbonString>,
        foreground: Option<RibbonString>,
    ) -> Rc<Self> {
        Self::new_with_options(RibbonIconKind::Text, text, None, foreground, 24.)
    }
    pub fn with_foreground(&self, color: Option<RibbonString>) -> Rc<Self> {
        Rc::new(Self {
            foreground: color,
            ..self.clone()
        })
    }
}
impl From<RibbonString> for RibbonIcon {
    fn from(value: RibbonString) -> Self {
        Self {
            kind: RibbonIconKind::Glyph,
            value: Some(value),
            font_family: None,
            foreground: None,
            view_box_size: 24.,
        }
    }
}
impl From<&str> for RibbonIcon {
    fn from(value: &str) -> Self {
        RibbonString::from(value).into()
    }
}

#[derive(Clone, Debug)]
pub struct RibbonIconLayer {
    data: Option<RibbonString>,
    stroke_thickness: f64,
    color: Option<RibbonString>,
    opacity: f64,
}
impl PartialEq for RibbonIconLayer {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
            && self.color == other.color
            && double_equals(self.stroke_thickness, other.stroke_thickness)
            && double_equals(self.opacity, other.opacity)
    }
}
impl RibbonIconLayer {
    pub fn new(data: Option<RibbonString>) -> Rc<Self> {
        Self::with_options(data, 0., None, 1.)
    }
    pub fn with_options(
        data: Option<RibbonString>,
        stroke_thickness: f64,
        color: Option<RibbonString>,
        opacity: f64,
    ) -> Rc<Self> {
        Rc::new(Self {
            data,
            stroke_thickness,
            color,
            opacity,
        })
    }
    pub fn data(&self) -> Option<RibbonString> {
        self.data.clone()
    }
    pub fn stroke_thickness(&self) -> f64 {
        self.stroke_thickness
    }
    pub fn color(&self) -> Option<RibbonString> {
        self.color.clone()
    }
    pub fn opacity(&self) -> f64 {
        self.opacity
    }
    /// Source ToString, including its possible null return for a null Data with no header.
    pub fn to_text(&self) -> Option<RibbonString> {
        let mut parts: Vec<RibbonString> = Vec::new();
        if self.stroke_thickness > 0. {
            parts.push(format!("stroke={}", InvariantF64(self.stroke_thickness)).into());
        }
        if let Some(color) = self.color.as_ref().filter(|s| !s.is_empty()) {
            let mut units: Vec<u16> = "color=".encode_utf16().collect();
            units.extend_from_slice(color.as_utf16());
            parts.push(RibbonString::from_utf16(units));
        }
        if self.opacity < 1. {
            parts.push(format!("opacity={}", InvariantF64(self.opacity)).into());
        }
        if parts.is_empty() {
            return self.data.clone();
        }
        let mut units = vec![b'[' as u16];
        for (index, part) in parts.iter().enumerate() {
            if index > 0 {
                units.push(b';' as u16);
            }
            units.extend_from_slice(part.as_utf16());
        }
        units.push(b']' as u16);
        if let Some(data) = &self.data {
            units.extend_from_slice(data.as_utf16());
        }
        Some(RibbonString::from_utf16(units))
    }
    pub fn parse_view_box(value: Option<&RibbonString>) -> Option<f64> {
        let units = value?.as_utf16();
        let start = units.iter().position(|&c| !whitespace(c))?;
        let text = &units[start..];
        let end = header_end(text)?;
        for setting in settings(&text[1..end]) {
            let Some((key, value)) = pair(setting) else {
                continue;
            };
            if key_is(key, "viewbox")
                && let Some(size) = number(value)
                && size > 0.
            {
                return Some(size);
            }
        }
        None
    }
    pub fn parse(value: Option<&RibbonString>) -> RibbonIconLayers {
        let layers = crate::collections::RibbonItems::new();
        let Some(value) = value else {
            return layers as RibbonIconLayers;
        };
        for raw in value
            .as_utf16()
            .split(|&c| c == b'|' as u16)
            .map(trim)
            .filter(|v| !v.is_empty())
        {
            let mut data = raw;
            let mut stroke = 0.;
            let mut opacity = 1.;
            let mut color = None;
            if let Some(end) = header_end(data) {
                for setting in settings(&data[1..end]) {
                    let (key, value) = pair(setting).unwrap_or((setting, &[]));
                    if key_is(key, "stroke") {
                        stroke = number(value).unwrap_or(1.5);
                    } else if key_is(key, "color") || key_is(key, "fill") {
                        color = (!value.is_empty()).then(|| RibbonString::from(value));
                    } else if key_is(key, "opacity") {
                        opacity = number(value).map_or(1., |v| v.clamp(0., 1.));
                    }
                }
                data = trim(&data[end + 1..]);
            }
            if !data.is_empty() {
                layers.add(Some(Self::with_options(
                    Some(data.into()),
                    stroke,
                    color,
                    opacity,
                )));
            }
        }
        layers as RibbonIconLayers
    }
}
fn header_end(text: &[u16]) -> Option<usize> {
    let close = match text.first()? {
        0x5b => 0x5d,
        0x7b => 0x7d,
        _ => return None,
    };
    text.iter().position(|&c| c == close).filter(|&i| i > 0)
}
fn settings(text: &[u16]) -> impl Iterator<Item = &[u16]> {
    text.split(|&c| c == b';' as u16)
        .map(trim)
        .filter(|v| !v.is_empty())
}
fn pair(text: &[u16]) -> Option<(&[u16], &[u16])> {
    let index = text.iter().position(|&c| c == b'=' as u16)?;
    Some((trim(&text[..index]), trim(&text[index + 1..])))
}
fn number(value: &[u16]) -> Option<f64> {
    try_parse_double(&String::from_utf16(value).ok()?, NumberStyles::FLOAT)
}
fn double_equals(a: f64, b: f64) -> bool {
    a == b || a.is_nan() && b.is_nan()
}
