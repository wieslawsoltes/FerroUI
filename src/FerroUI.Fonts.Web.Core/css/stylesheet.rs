//! `@font-face` rules in a style sheet.

use std::borrow::Cow;

use super::descriptors::*;
use super::source::{parse_src, FontSource};
use super::tokenizer::{Token, Tokenizer};
use super::unicode_range::{parse_unicode_range, UnicodeRangeSet};
use super::CssParseError;

/// The descriptors of one `@font-face` rule, with their initial values where a rule has none.
#[derive(Clone, Debug, PartialEq)]
pub struct FontFaceDescriptors {
    pub family: String,
    pub sources: Vec<FontSource>,
    pub style: FontStyleDescriptor,
    /// `None` is `auto`.
    pub weight: Option<FontWeightRange>,
    /// `None` is `auto`.
    pub width: Option<FontWidthRange>,
    pub unicode_range: UnicodeRangeSet,
    pub display: FontDisplay,
    pub feature_settings: Vec<FontFeatureSetting>,
    pub variation_settings: Vec<FontVariationSetting>,
    /// `None` is `auto`.
    pub named_instance: Option<String>,
    /// `None` is `normal`.
    pub language_override: Option<String>,
    pub ascent_override: MetricOverride,
    pub descent_override: MetricOverride,
    pub line_gap_override: MetricOverride,
    /// A fraction: `1.0` is `100%`.
    pub size_adjust: f32,
}

impl FontFaceDescriptors {
    /// A face of a family and its sources, every other descriptor at its initial value.
    pub fn new(family: impl Into<String>, sources: Vec<FontSource>) -> Self {
        Self {
            family: family.into(),
            sources,
            style: FontStyleDescriptor::Auto,
            weight: None,
            width: None,
            unicode_range: UnicodeRangeSet::default(),
            display: FontDisplay::Auto,
            feature_settings: Vec::new(),
            variation_settings: Vec::new(),
            named_instance: None,
            language_override: None,
            ascent_override: MetricOverride::Normal,
            descent_override: MetricOverride::Normal,
            line_gap_override: MetricOverride::Normal,
            size_adjust: 1.0,
        }
    }

    /// Applies one declaration. An unknown descriptor is ignored, and an invalid value leaves
    /// the descriptor as it was. Both are what CSS does with a declaration it cannot use.
    fn apply(&mut self, name: &str, value: &str) {
        match name.to_ascii_lowercase().as_str() {
            "font-style" => set(&mut self.style, parse_font_style(value)),
            "font-weight" => set(&mut self.weight, parse_font_weight(value)),
            "font-width" | "font-stretch" => set(&mut self.width, parse_font_width(value)),
            "unicode-range" => set(&mut self.unicode_range, parse_unicode_range(value)),
            "font-display" => set(&mut self.display, parse_font_display(value)),
            "font-feature-settings" => set(&mut self.feature_settings, parse_feature_settings(value)),
            "font-variation-settings" => set(&mut self.variation_settings, parse_variation_settings(value)),
            "font-named-instance" => set(&mut self.named_instance, parse_named_instance(value)),
            "font-language-override" => set(&mut self.language_override, parse_language_override(value)),
            "ascent-override" => set(&mut self.ascent_override, parse_metric_override(value)),
            "descent-override" => set(&mut self.descent_override, parse_metric_override(value)),
            "line-gap-override" => set(&mut self.line_gap_override, parse_metric_override(value)),
            "size-adjust" => set(&mut self.size_adjust, parse_size_adjust(value)),
            _ => {}
        }
    }
}

fn set<T>(target: &mut T, value: Result<T, CssParseError>) {
    if let Ok(value) = value {
        *target = value;
    }
}

/// The name and value of each declaration of a block's content. A declaration with a bad
/// token is dropped, and parsing resumes at the next semicolon.
fn declarations(block: &str) -> Vec<(Cow<'_, str>, &str)> {
    let mut tokenizer = Tokenizer::new(block);
    let mut result = Vec::new();
    loop {
        let name = match tokenizer.next_significant() {
            Token::Eof => return result,
            Token::Semicolon => continue,
            Token::Ident(name) => name,
            _ => {
                if skip_declaration(&mut tokenizer).is_none() {
                    return result;
                }
                continue;
            }
        };
        if tokenizer.next_significant() != Token::Colon {
            if skip_declaration(&mut tokenizer).is_none() {
                return result;
            }
            continue;
        }
        let start = tokenizer.position();
        let (end, valid, more) = scan_value(&mut tokenizer, block.len());
        if valid {
            result.push((name, &block[start..end]));
        }
        if !more {
            return result;
        }
    }
}

/// Finds the end of a value: the next top-level semicolon, or the end of the block.
fn scan_value(tokenizer: &mut Tokenizer<'_>, length: usize) -> (usize, bool, bool) {
    let (mut depth, mut valid) = (0usize, true);
    loop {
        let before = tokenizer.position();
        match tokenizer.next_token() {
            Token::Eof => return (length, valid, false),
            Token::Semicolon if depth == 0 => return (before, valid, true),
            Token::BadString | Token::BadUrl => valid = false,
            Token::Function(_) | Token::LeftParen | Token::LeftBracket | Token::LeftBrace => depth += 1,
            Token::RightParen | Token::RightBracket | Token::RightBrace => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
}

/// Skips to the token after the next top-level semicolon; `None` at the end of the input.
fn skip_declaration(tokenizer: &mut Tokenizer<'_>) -> Option<()> {
    let mut depth = 0usize;
    loop {
        match tokenizer.next_token() {
            Token::Eof => return None,
            Token::Semicolon if depth == 0 => return Some(()),
            Token::Function(_) | Token::LeftParen | Token::LeftBracket | Token::LeftBrace => depth += 1,
            Token::RightParen | Token::RightBracket | Token::RightBrace => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
}

/// The last value of a descriptor that parses.
fn last_valid<T>(
    declarations: &[(Cow<'_, str>, &str)],
    descriptor: &str,
    parse: impl Fn(&str) -> Result<T, CssParseError>,
) -> Option<T> {
    declarations
        .iter()
        .rev()
        .filter(|(name, _)| name.eq_ignore_ascii_case(descriptor))
        .find_map(|(_, value)| parse(value).ok())
}

/// Parses the content of an `@font-face` block, without its braces. The last valid value of a
/// descriptor wins. The rule needs a valid `font-family` and a valid `src`.
pub fn parse_font_face_block(block: &str) -> Result<FontFaceDescriptors, CssParseError> {
    let declarations = declarations(block);
    let family = last_valid(&declarations, "font-family", parse_family_name).ok_or(CssParseError::MissingFamily)?;
    let sources = last_valid(&declarations, "src", parse_src).ok_or(CssParseError::MissingSource)?;
    let mut face = FontFaceDescriptors::new(family, sources);
    for (name, value) in &declarations {
        face.apply(name, value);
    }
    Ok(face)
}

/// Parses the `@font-face` rules of a style sheet, in order. Every other rule is skipped, and
/// an invalid `@font-face` rule is dropped.
pub fn parse_stylesheet(text: &str) -> Vec<FontFaceDescriptors> {
    let mut tokenizer = Tokenizer::new(text);
    let mut faces = Vec::new();
    loop {
        let is_font_face = match tokenizer.next_significant() {
            Token::Eof => return faces,
            Token::AtKeyword(name) => name.eq_ignore_ascii_case("font-face"),
            _ => false,
        };
        // Up to the block of the rule, or the semicolon of a rule without a block.
        let mut depth = 0usize;
        let opened = loop {
            match tokenizer.next_token() {
                Token::Eof => return faces,
                Token::LeftBrace if depth == 0 => break true,
                Token::Semicolon if depth == 0 => break false,
                Token::Function(_) | Token::LeftParen | Token::LeftBracket | Token::LeftBrace => depth += 1,
                Token::RightParen | Token::RightBracket | Token::RightBrace => depth = depth.saturating_sub(1),
                _ => {}
            }
        };
        if !opened {
            continue;
        }
        let start = tokenizer.position();
        let mut depth = 1usize;
        let end = loop {
            let before = tokenizer.position();
            match tokenizer.next_token() {
                Token::Eof => break text.len(),
                Token::LeftBrace | Token::Function(_) | Token::LeftParen | Token::LeftBracket => depth += 1,
                Token::RightBrace | Token::RightParen | Token::RightBracket => {
                    depth -= 1;
                    if depth == 0 {
                        break before;
                    }
                }
                _ => {}
            }
        };
        if is_font_face {
            if let Ok(face) = parse_font_face_block(&text[start..end]) {
                faces.push(face);
            }
        }
    }
}
