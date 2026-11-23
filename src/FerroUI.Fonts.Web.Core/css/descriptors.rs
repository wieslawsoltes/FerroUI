//! The descriptors of `@font-face` other than `src` and `unicode-range`.

use super::tokenizer::{Token, Tokenizer};
use super::CssParseError;

/// The tokens of a descriptor value, without white space.
fn tokens(value: &str) -> Vec<Token<'_>> {
    let mut tokenizer = Tokenizer::new(value);
    let mut tokens = Vec::new();
    loop {
        match tokenizer.next_significant() {
            Token::Eof => return tokens,
            token => tokens.push(token),
        }
    }
}

fn keyword<'t>(token: &'t Token<'_>) -> Option<&'t str> {
    match token {
        Token::Ident(name) => Some(name),
        _ => None,
    }
}

fn is_keyword(token: &Token<'_>, name: &str) -> bool {
    keyword(token).is_some_and(|ident| ident.eq_ignore_ascii_case(name))
}

/// Splits a token list at its top-level commas. An empty item makes the list invalid.
fn comma_separated<'t, 'a>(tokens: &'t [Token<'a>]) -> Option<Vec<&'t [Token<'a>]>> {
    let items: Vec<&[Token<'a>]> = tokens.split(|token| *token == Token::Comma).collect();
    items.iter().all(|item| !item.is_empty()).then_some(items)
}

/// The family names that must be quoted: the CSS-wide keywords, `default` and the generics.
const RESERVED_FAMILY_NAMES: [&str; 20] = [
    "inherit",
    "initial",
    "unset",
    "revert",
    "revert-layer",
    "default",
    "serif",
    "sans-serif",
    "cursive",
    "fantasy",
    "monospace",
    "system-ui",
    "emoji",
    "math",
    "fangsong",
    "ui-serif",
    "ui-sans-serif",
    "ui-monospace",
    "ui-rounded",
    "generic",
];

/// Parses a `<family-name>`: a string, or a sequence of identifiers joined by one space.
pub fn parse_family_name(value: &str) -> Result<String, CssParseError> {
    const INVALID: CssParseError = CssParseError::InvalidValue("font-family");
    let tokens = tokens(value);
    match tokens.as_slice() {
        [Token::String(name)] if !name.is_empty() => Ok(name.to_string()),
        [] => Err(INVALID),
        idents => {
            let mut names = Vec::with_capacity(idents.len());
            for token in idents {
                names.push(keyword(token).ok_or(INVALID)?);
            }
            if let [single] = names.as_slice() {
                if RESERVED_FAMILY_NAMES.iter().any(|name| single.eq_ignore_ascii_case(name)) {
                    return Err(INVALID);
                }
            }
            Ok(names.join(" "))
        }
    }
}

/// The `font-display` descriptor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FontDisplay {
    #[default]
    Auto,
    Block,
    Swap,
    Fallback,
    Optional,
}

/// Parses `font-display`.
pub fn parse_font_display(value: &str) -> Result<FontDisplay, CssParseError> {
    let tokens = tokens(value);
    let [token] = tokens.as_slice() else {
        return Err(CssParseError::InvalidValue("font-display"));
    };
    match keyword(token).map(str::to_ascii_lowercase).as_deref() {
        Some("auto") => Ok(FontDisplay::Auto),
        Some("block") => Ok(FontDisplay::Block),
        Some("swap") => Ok(FontDisplay::Swap),
        Some("fallback") => Ok(FontDisplay::Fallback),
        Some("optional") => Ok(FontDisplay::Optional),
        _ => Err(CssParseError::InvalidValue("font-display")),
    }
}

/// The `font-style` descriptor. Angles are in degrees, with `start <= end`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum FontStyleDescriptor {
    /// Selects as `normal` and does not clamp a variation axis.
    #[default]
    Auto,
    Normal,
    Italic,
    Left,
    Right,
    Oblique {
        start: f32,
        end: f32,
    },
}

impl FontStyleDescriptor {
    /// The angle of `oblique` without an angle (CSS Fonts 4).
    pub const DEFAULT_OBLIQUE_ANGLE: f32 = 14.0;
}

fn angle_degrees(token: &Token<'_>) -> Option<f32> {
    let (value, unit) = match token {
        Token::Dimension { value, unit } => (*value, unit.to_ascii_lowercase()),
        // The zero angle may be written without a unit.
        Token::Number { value, .. } if *value == 0.0 => return Some(0.0),
        _ => return None,
    };
    let degrees = match unit.as_str() {
        "deg" => value,
        "grad" => value * 0.9,
        "rad" => value.to_degrees(),
        "turn" => value * 360.0,
        _ => return None,
    };
    (-90.0..=90.0).contains(&degrees).then_some(degrees as f32)
}

/// Parses `font-style`. A reversed range is put in order.
pub fn parse_font_style(value: &str) -> Result<FontStyleDescriptor, CssParseError> {
    const INVALID: CssParseError = CssParseError::InvalidValue("font-style");
    let tokens = tokens(value);
    let (first, rest) = tokens.split_first().ok_or(INVALID)?;
    let name = keyword(first).ok_or(INVALID)?.to_ascii_lowercase();
    if name == "oblique" {
        let default = FontStyleDescriptor::DEFAULT_OBLIQUE_ANGLE;
        let (start, end) = match rest {
            [] => (default, default),
            [angle] => {
                let angle = angle_degrees(angle).ok_or(INVALID)?;
                (angle, angle)
            }
            [start, end] => (angle_degrees(start).ok_or(INVALID)?, angle_degrees(end).ok_or(INVALID)?),
            _ => return Err(INVALID),
        };
        return Ok(FontStyleDescriptor::Oblique { start: start.min(end), end: start.max(end) });
    }
    if !rest.is_empty() {
        return Err(INVALID);
    }
    match name.as_str() {
        "auto" => Ok(FontStyleDescriptor::Auto),
        "normal" => Ok(FontStyleDescriptor::Normal),
        "italic" => Ok(FontStyleDescriptor::Italic),
        "left" => Ok(FontStyleDescriptor::Left),
        "right" => Ok(FontStyleDescriptor::Right),
        _ => Err(INVALID),
    }
}

/// An inclusive range of `font-weight`, in the units of the property (1 to 1000).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FontWeightRange {
    pub start: f32,
    pub end: f32,
}

fn weight_value(token: &Token<'_>) -> Option<f32> {
    match token {
        Token::Number { value, .. } if (1.0..=1000.0).contains(value) => Some(*value as f32),
        token if is_keyword(token, "normal") => Some(400.0),
        token if is_keyword(token, "bold") => Some(700.0),
        _ => None,
    }
}

/// Parses `font-weight`; `None` is `auto`. A reversed range is put in order.
pub fn parse_font_weight(value: &str) -> Result<Option<FontWeightRange>, CssParseError> {
    const INVALID: CssParseError = CssParseError::InvalidValue("font-weight");
    let tokens = tokens(value);
    match tokens.as_slice() {
        [token] if is_keyword(token, "auto") => Ok(None),
        [token] => {
            let weight = weight_value(token).ok_or(INVALID)?;
            Ok(Some(FontWeightRange { start: weight, end: weight }))
        }
        [first, second] => {
            let (first, second) = (weight_value(first).ok_or(INVALID)?, weight_value(second).ok_or(INVALID)?);
            Ok(Some(FontWeightRange { start: first.min(second), end: first.max(second) }))
        }
        _ => Err(INVALID),
    }
}

/// An inclusive range of `font-width`, as a percentage of the normal width.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FontWidthRange {
    pub start: f32,
    pub end: f32,
}

fn width_value(token: &Token<'_>) -> Option<f32> {
    if let Token::Percentage(value) = token {
        return (*value >= 0.0 && value.is_finite()).then_some(*value as f32);
    }
    match keyword(token)?.to_ascii_lowercase().as_str() {
        "ultra-condensed" => Some(50.0),
        "extra-condensed" => Some(62.5),
        "condensed" => Some(75.0),
        "semi-condensed" => Some(87.5),
        "normal" => Some(100.0),
        "semi-expanded" => Some(112.5),
        "expanded" => Some(125.0),
        "extra-expanded" => Some(150.0),
        "ultra-expanded" => Some(200.0),
        _ => None,
    }
}

/// Parses `font-width` (or its legacy name `font-stretch`); `None` is `auto`.
pub fn parse_font_width(value: &str) -> Result<Option<FontWidthRange>, CssParseError> {
    const INVALID: CssParseError = CssParseError::InvalidValue("font-width");
    let tokens = tokens(value);
    match tokens.as_slice() {
        [token] if is_keyword(token, "auto") => Ok(None),
        [token] => {
            let width = width_value(token).ok_or(INVALID)?;
            Ok(Some(FontWidthRange { start: width, end: width }))
        }
        [first, second] => {
            let (first, second) = (width_value(first).ok_or(INVALID)?, width_value(second).ok_or(INVALID)?);
            Ok(Some(FontWidthRange { start: first.min(second), end: first.max(second) }))
        }
        _ => Err(INVALID),
    }
}

/// One OpenType feature of `font-feature-settings`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FontFeatureSetting {
    pub tag: [u8; 4],
    pub value: u32,
}

/// One axis of `font-variation-settings`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FontVariationSetting {
    pub tag: [u8; 4],
    pub value: f32,
}

/// A four-character OpenType tag: printable ASCII only.
fn open_type_tag(text: &str) -> Option<[u8; 4]> {
    let bytes: [u8; 4] = text.as_bytes().try_into().ok()?;
    bytes.iter().all(|byte| (0x20..=0x7e).contains(byte)).then_some(bytes)
}

/// Parses `font-feature-settings`; `normal` is the empty list.
pub fn parse_feature_settings(value: &str) -> Result<Vec<FontFeatureSetting>, CssParseError> {
    const INVALID: CssParseError = CssParseError::InvalidValue("font-feature-settings");
    let tokens = tokens(value);
    if let [token] = tokens.as_slice() {
        if is_keyword(token, "normal") {
            return Ok(Vec::new());
        }
    }
    let mut settings = Vec::new();
    for item in comma_separated(&tokens).ok_or(INVALID)? {
        let (tag, setting) = item.split_first().ok_or(INVALID)?;
        let Token::String(tag) = tag else {
            return Err(INVALID);
        };
        let tag = open_type_tag(tag).ok_or(INVALID)?;
        let value = match setting {
            [] => 1,
            [token] if is_keyword(token, "on") => 1,
            [token] if is_keyword(token, "off") => 0,
            [Token::Number { value, is_integer: true }] if *value >= 0.0 && *value <= u32::MAX as f64 => *value as u32,
            _ => return Err(INVALID),
        };
        settings.push(FontFeatureSetting { tag, value });
    }
    Ok(settings)
}

/// Parses `font-variation-settings`; `normal` is the empty list.
pub fn parse_variation_settings(value: &str) -> Result<Vec<FontVariationSetting>, CssParseError> {
    const INVALID: CssParseError = CssParseError::InvalidValue("font-variation-settings");
    let tokens = tokens(value);
    if let [token] = tokens.as_slice() {
        if is_keyword(token, "normal") {
            return Ok(Vec::new());
        }
    }
    let mut settings = Vec::new();
    for item in comma_separated(&tokens).ok_or(INVALID)? {
        let [Token::String(tag), Token::Number { value, .. }] = item else {
            return Err(INVALID);
        };
        let tag = open_type_tag(tag).ok_or(INVALID)?;
        if !value.is_finite() {
            return Err(INVALID);
        }
        settings.push(FontVariationSetting { tag, value: *value as f32 });
    }
    Ok(settings)
}

/// Parses `font-named-instance`; `None` is `auto`.
pub fn parse_named_instance(value: &str) -> Result<Option<String>, CssParseError> {
    let tokens = tokens(value);
    match tokens.as_slice() {
        [token] if is_keyword(token, "auto") => Ok(None),
        [Token::String(name)] => Ok(Some(name.to_string())),
        _ => Err(CssParseError::InvalidValue("font-named-instance")),
    }
}

/// Parses `font-language-override`; `None` is `normal`. The value is an OpenType language
/// system tag of up to four ASCII characters.
pub fn parse_language_override(value: &str) -> Result<Option<String>, CssParseError> {
    const INVALID: CssParseError = CssParseError::InvalidValue("font-language-override");
    let tokens = tokens(value);
    match tokens.as_slice() {
        [token] if is_keyword(token, "normal") => Ok(None),
        [Token::String(tag)] if (1..=4).contains(&tag.len()) && tag.bytes().all(|b| (0x20..=0x7e).contains(&b)) => {
            Ok(Some(tag.to_string()))
        }
        _ => Err(INVALID),
    }
}

/// `ascent-override`, `descent-override` or `line-gap-override`, as fractions of the font size.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum MetricOverride {
    /// The value of the font file.
    #[default]
    Normal,
    /// The override. Level 5 allows a second value, for the other axis.
    Value { first: f32, second: Option<f32> },
}

fn non_negative_fraction(token: &Token<'_>) -> Option<f32> {
    match token {
        Token::Percentage(value) if *value >= 0.0 && value.is_finite() => Some((*value / 100.0) as f32),
        _ => None,
    }
}

/// Parses one of the metric override descriptors.
pub fn parse_metric_override(value: &str) -> Result<MetricOverride, CssParseError> {
    const INVALID: CssParseError = CssParseError::InvalidValue("metric override");
    let tokens = tokens(value);
    match tokens.as_slice() {
        [token] if is_keyword(token, "normal") => Ok(MetricOverride::Normal),
        [first] => Ok(MetricOverride::Value { first: non_negative_fraction(first).ok_or(INVALID)?, second: None }),
        [first, second] => Ok(MetricOverride::Value {
            first: non_negative_fraction(first).ok_or(INVALID)?,
            second: Some(non_negative_fraction(second).ok_or(INVALID)?),
        }),
        _ => Err(INVALID),
    }
}

/// Parses `size-adjust`, as a fraction (`90%` is `0.9`).
pub fn parse_size_adjust(value: &str) -> Result<f32, CssParseError> {
    let tokens = tokens(value);
    match tokens.as_slice() {
        [token] => non_negative_fraction(token).ok_or(CssParseError::InvalidValue("size-adjust")),
        _ => Err(CssParseError::InvalidValue("size-adjust")),
    }
}
