//! The `@font-palette-values` rule (CSS Fonts 4, section 9.2).

use super::descriptors::parse_family_name;
use super::tokenizer::{Token, Tokenizer};
use super::CssParseError;

/// The `base-palette` descriptor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BasePalette {
    /// The first palette the font marks as usable on a light background.
    Light,
    /// The first palette the font marks as usable on a dark background.
    Dark,
    Index(u16),
    /// The descriptor was not given, which behaves as palette 0.
    #[default]
    Default,
}

/// A color as red, green, blue and alpha.
pub type PaletteColor = [u8; 4];

/// One `@font-palette-values` rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FontPaletteValues {
    /// The `<dashed-ident>` of the rule.
    pub name: String,
    /// The families it applies to.
    pub families: Vec<String>,
    pub base_palette: BasePalette,
    /// Palette index and the color that replaces it.
    pub override_colors: Vec<(u16, PaletteColor)>,
}

const NAMED_COLORS: [(&str, PaletteColor); 18] = [
    ("transparent", [0, 0, 0, 0]),
    ("black", [0, 0, 0, 255]),
    ("silver", [192, 192, 192, 255]),
    ("gray", [128, 128, 128, 255]),
    ("grey", [128, 128, 128, 255]),
    ("white", [255, 255, 255, 255]),
    ("maroon", [128, 0, 0, 255]),
    ("red", [255, 0, 0, 255]),
    ("purple", [128, 0, 128, 255]),
    ("fuchsia", [255, 0, 255, 255]),
    ("green", [0, 128, 0, 255]),
    ("lime", [0, 255, 0, 255]),
    ("olive", [128, 128, 0, 255]),
    ("yellow", [255, 255, 0, 255]),
    ("navy", [0, 0, 128, 255]),
    ("blue", [0, 0, 255, 255]),
    ("teal", [0, 128, 128, 255]),
    ("aqua", [0, 255, 255, 255]),
];

fn parse_hex(text: &str) -> Option<PaletteColor> {
    if !text.is_ascii() || !text.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let short = |at: usize| u8::from_str_radix(&text[at..at + 1], 16).ok().map(|value| value * 17);
    let long = |at: usize| u8::from_str_radix(&text[at..at + 2], 16).ok();
    match text.len() {
        3 => Some([short(0)?, short(1)?, short(2)?, 255]),
        4 => Some([short(0)?, short(1)?, short(2)?, short(3)?]),
        6 => Some([long(0)?, long(2)?, long(4)?, 255]),
        8 => Some([long(0)?, long(2)?, long(4)?, long(6)?]),
        _ => None,
    }
}

fn to_byte(value: f64) -> u8 {
    value.clamp(0.0, 255.0).round() as u8
}

/// The arguments of `rgb()` or `rgba()`, comma or space separated, the function already read.
fn parse_rgb(tokenizer: &mut Tokenizer<'_>) -> Option<PaletteColor> {
    let mut channels = [0u8, 0, 0, 255];
    let mut count = 0;
    loop {
        match tokenizer.next_significant() {
            Token::RightParen => break,
            Token::Comma | Token::Delim('/') => {}
            Token::Number { value, .. } if count < 3 => {
                channels[count] = to_byte(value);
                count += 1;
            }
            Token::Percentage(value) if count < 3 => {
                channels[count] = to_byte(value / 100.0 * 255.0);
                count += 1;
            }
            Token::Number { value, .. } if count == 3 => {
                channels[3] = to_byte(value * 255.0);
                count += 1;
            }
            Token::Percentage(value) if count == 3 => {
                channels[3] = to_byte(value / 100.0 * 255.0);
                count += 1;
            }
            _ => return None,
        }
    }
    (count >= 3).then_some(channels)
}

/// The colors this parser reads: hexadecimal, `rgb()`, `rgba()` and the basic color keywords.
fn parse_color(tokenizer: &mut Tokenizer<'_>, first: Token<'_>) -> Option<PaletteColor> {
    match first {
        Token::Delim('#') => match tokenizer.next_token() {
            Token::Ident(text) => parse_hex(&text),
            Token::Number { .. } | Token::Dimension { .. } => None,
            _ => None,
        },
        Token::Ident(name) => {
            NAMED_COLORS.iter().find(|(known, _)| name.eq_ignore_ascii_case(known)).map(|(_, color)| *color)
        }
        Token::Function(name) if name.eq_ignore_ascii_case("rgb") || name.eq_ignore_ascii_case("rgba") => {
            parse_rgb(tokenizer)
        }
        _ => None,
    }
}

/// A hexadecimal color may start with a digit, which is not an identifier: it is read here from
/// the text.
fn split_hex(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 8);
    let mut rest = value;
    while let Some(at) = rest.find('#') {
        output.push_str(&rest[..at]);
        let digits = rest[at + 1..].find(|c: char| !c.is_ascii_hexdigit()).unwrap_or(rest.len() - at - 1);
        let hex = &rest[at + 1..at + 1 + digits];
        match parse_hex(hex) {
            Some([r, g, b, a]) => output.push_str(&format!("rgba({r} {g} {b} {})", a as f64 / 255.0)),
            None => output.push_str("invalid-color"),
        }
        rest = &rest[at + 1 + digits..];
    }
    output.push_str(rest);
    output
}

/// Parses `override-colors`: pairs of a palette index and a color.
pub fn parse_override_colors(value: &str) -> Result<Vec<(u16, PaletteColor)>, CssParseError> {
    const INVALID: CssParseError = CssParseError::InvalidValue("override-colors");
    let text = split_hex(value);
    let mut tokenizer = Tokenizer::new(&text);
    let mut colors = Vec::new();
    loop {
        let index = match tokenizer.next_significant() {
            Token::Number { value, is_integer: true } if (0.0..=65535.0).contains(&value) => value as u16,
            _ => return Err(INVALID),
        };
        let first = tokenizer.next_significant();
        colors.push((index, parse_color(&mut tokenizer, first).ok_or(INVALID)?));
        match tokenizer.next_significant() {
            Token::Eof => return Ok(colors),
            Token::Comma => {}
            _ => return Err(INVALID),
        }
    }
}

/// Parses `base-palette`.
pub fn parse_base_palette(value: &str) -> Result<BasePalette, CssParseError> {
    const INVALID: CssParseError = CssParseError::InvalidValue("base-palette");
    let mut tokenizer = Tokenizer::new(value);
    let token = tokenizer.next_significant();
    if tokenizer.next_significant() != Token::Eof {
        return Err(INVALID);
    }
    match token {
        Token::Ident(name) if name.eq_ignore_ascii_case("light") => Ok(BasePalette::Light),
        Token::Ident(name) if name.eq_ignore_ascii_case("dark") => Ok(BasePalette::Dark),
        Token::Number { value, is_integer: true } if (0.0..=65535.0).contains(&value) => {
            Ok(BasePalette::Index(value as u16))
        }
        _ => Err(INVALID),
    }
}

/// The content of one rule. It needs a valid `font-family`, and the last declaration of a
/// descriptor counts.
pub(super) fn parse_palette_block(
    name: &str,
    declarations: &[(std::borrow::Cow<'_, str>, &str)],
) -> Option<FontPaletteValues> {
    let mut rule = FontPaletteValues {
        name: name.to_owned(),
        families: Vec::new(),
        base_palette: BasePalette::Default,
        override_colors: Vec::new(),
    };
    for (descriptor, value) in declarations {
        match descriptor.to_ascii_lowercase().as_str() {
            "font-family" => {
                let families: Result<Vec<String>, _> = value.split(',').map(parse_family_name).collect();
                if let Ok(families) = families {
                    rule.families = families;
                }
            }
            "base-palette" => {
                if let Ok(palette) = parse_base_palette(value) {
                    rule.base_palette = palette;
                }
            }
            "override-colors" => {
                if let Ok(colors) = parse_override_colors(value) {
                    rule.override_colors = colors;
                }
            }
            _ => {}
        }
    }
    (!rule.families.is_empty()).then_some(rule)
}
