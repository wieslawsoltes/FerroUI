//! The colours of SVG paints.

use ferroui_base::media::Color;

/// A CSS colour. The hexadecimal forms are read here because CSS puts the alpha last where
/// [`Color::try_parse`] puts it first; names and functions are that parser's.
pub(crate) fn parse_color(text: &str) -> Option<Color> {
    let text = text.trim();
    let Some(hex) = text.strip_prefix('#') else {
        return Color::try_parse(text);
    };
    if !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let digit = |index: usize| u8::from_str_radix(&hex[index..index + 1], 16).ok().map(|value| value * 17);
    let pair = |index: usize| u8::from_str_radix(&hex[index..index + 2], 16).ok();
    match hex.len() {
        3 => Some(Color::from_rgb(digit(0)?, digit(1)?, digit(2)?)),
        4 => Some(Color::from_argb(digit(3)?, digit(0)?, digit(1)?, digit(2)?)),
        6 => Some(Color::from_rgb(pair(0)?, pair(2)?, pair(4)?)),
        8 => Some(Color::from_argb(pair(6)?, pair(0)?, pair(2)?, pair(4)?)),
        _ => None,
    }
}
