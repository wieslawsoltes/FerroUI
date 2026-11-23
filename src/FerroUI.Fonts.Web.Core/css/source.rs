//! The `src` descriptor (CSS Fonts 4, section 4.3).

use super::descriptors::parse_family_name;
use super::tokenizer::{Token, Tokenizer};
use super::CssParseError;

/// The values of `format()`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FontFormat {
    Collection,
    EmbeddedOpenType,
    OpenType,
    Svg,
    TrueType,
    Woff,
    Woff2,
}

impl FontFormat {
    fn parse(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "collection" => Some(Self::Collection),
            "embedded-opentype" => Some(Self::EmbeddedOpenType),
            "opentype" => Some(Self::OpenType),
            "svg" => Some(Self::Svg),
            "truetype" => Some(Self::TrueType),
            "woff" => Some(Self::Woff),
            "woff2" => Some(Self::Woff2),
            _ => None,
        }
    }

    /// The legacy strings that stand for a format with `tech(variations)`.
    fn parse_legacy_variations(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "woff2-variations" => Some(Self::Woff2),
            "woff-variations" => Some(Self::Woff),
            "truetype-variations" => Some(Self::TrueType),
            "opentype-variations" => Some(Self::OpenType),
            _ => None,
        }
    }

    /// Whether FerroUI can load the format at all: EOT and SVG fonts are left out
    /// (`docs/porting/web-fonts.md`, section 12).
    pub fn is_loadable(self) -> bool {
        !matches!(self, Self::EmbeddedOpenType | Self::Svg)
    }
}

/// The values of `tech()`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum FontTech {
    FeaturesOpenType = 1 << 0,
    FeaturesAat = 1 << 1,
    FeaturesGraphite = 1 << 2,
    ColorColrV0 = 1 << 3,
    ColorColrV1 = 1 << 4,
    ColorSvg = 1 << 5,
    ColorSbix = 1 << 6,
    ColorCbdt = 1 << 7,
    Variations = 1 << 8,
    Palettes = 1 << 9,
    Incremental = 1 << 10,
    Avar2 = 1 << 11,
}

impl FontTech {
    fn parse(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "features-opentype" => Some(Self::FeaturesOpenType),
            "features-aat" => Some(Self::FeaturesAat),
            "features-graphite" => Some(Self::FeaturesGraphite),
            "color-colrv0" => Some(Self::ColorColrV0),
            "color-colrv1" => Some(Self::ColorColrV1),
            "color-svg" => Some(Self::ColorSvg),
            "color-sbix" => Some(Self::ColorSbix),
            "color-cbdt" => Some(Self::ColorCbdt),
            "variations" => Some(Self::Variations),
            "palettes" => Some(Self::Palettes),
            "incremental" => Some(Self::Incremental),
            "avar2" => Some(Self::Avar2),
            _ => None,
        }
    }
}

/// A set of [`FontTech`] values.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct FontTechs(u16);

impl FontTechs {
    /// The empty set.
    pub const NONE: Self = Self(0);

    /// The set with one more technology.
    pub const fn with(self, tech: FontTech) -> Self {
        Self(self.0 | tech as u16)
    }

    /// Whether the set has a technology.
    pub const fn contains(self, tech: FontTech) -> bool {
        self.0 & tech as u16 != 0
    }

    /// Whether every technology of this set is in `supported`.
    pub const fn is_subset_of(self, supported: Self) -> bool {
        self.0 & !supported.0 == 0
    }
}

/// One entry of `src`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FontSource {
    /// `url()` with its optional `format()` and `tech()`.
    Url { url: String, format: Option<FontFormat>, techs: FontTechs },
    /// `local()`: an installed font by its full or PostScript name.
    Local { name: String },
}

impl FontSource {
    /// Whether this entry can be used with the given technologies. An entry that cannot be
    /// used is skipped and the next one is tried.
    pub fn is_supported(&self, supported: FontTechs) -> bool {
        match self {
            Self::Url { format, techs, .. } => {
                format.is_none_or(FontFormat::is_loadable) && techs.is_subset_of(supported)
            }
            Self::Local { .. } => true,
        }
    }
}

/// Reads `format(...)`: a keyword or a string. The token of the function is consumed.
fn parse_format_arguments(tokenizer: &mut Tokenizer<'_>) -> Option<(FontFormat, bool)> {
    let result = match tokenizer.next_significant() {
        Token::Ident(name) => FontFormat::parse(&name).map(|format| (format, false)),
        Token::String(name) => FontFormat::parse(&name)
            .map(|format| (format, false))
            .or_else(|| FontFormat::parse_legacy_variations(&name).map(|format| (format, true))),
        _ => None,
    }?;
    (tokenizer.next_significant() == Token::RightParen).then_some(result)
}

/// Reads `tech(a, b)`. The token of the function is consumed.
fn parse_tech_arguments(tokenizer: &mut Tokenizer<'_>) -> Option<FontTechs> {
    let mut techs = FontTechs::NONE;
    loop {
        let Token::Ident(name) = tokenizer.next_significant() else {
            return None;
        };
        techs = techs.with(FontTech::parse(&name)?);
        match tokenizer.next_significant() {
            Token::Comma => {}
            Token::RightParen => return Some(techs),
            _ => return None,
        }
    }
}

/// One `src` entry, from its own text.
fn parse_entry(entry: &str) -> Option<FontSource> {
    let mut tokenizer = Tokenizer::new(entry);
    match tokenizer.next_significant() {
        Token::Url(url) => {
            let (mut format, mut techs) = (None, FontTechs::NONE);
            let mut token = tokenizer.next_significant();
            if matches!(&token, Token::Function(name) if name.eq_ignore_ascii_case("format")) {
                let (parsed, variations) = parse_format_arguments(&mut tokenizer)?;
                format = Some(parsed);
                if variations {
                    techs = techs.with(FontTech::Variations);
                }
                token = tokenizer.next_significant();
            }
            if matches!(&token, Token::Function(name) if name.eq_ignore_ascii_case("tech")) {
                let listed = parse_tech_arguments(&mut tokenizer)?;
                techs = FontTechs(techs.0 | listed.0);
                token = tokenizer.next_significant();
            }
            (token == Token::Eof).then(|| FontSource::Url { url: url.into_owned(), format, techs })
        }
        Token::Function(name) if name.eq_ignore_ascii_case("local") => {
            let start = tokenizer.position();
            let mut end = start;
            loop {
                match tokenizer.next_token() {
                    Token::RightParen => break,
                    Token::Eof => {
                        end = entry.len();
                        break;
                    }
                    Token::BadString | Token::BadUrl => return None,
                    _ => end = tokenizer.position(),
                }
            }
            if tokenizer.next_significant() != Token::Eof {
                return None;
            }
            let name = parse_family_name(&entry[start..end]).ok()?;
            Some(FontSource::Local { name })
        }
        _ => None,
    }
}

/// The byte ranges of the top-level comma-separated items of a value.
fn split_top_level(value: &str) -> Vec<&str> {
    let mut tokenizer = Tokenizer::new(value);
    let (mut items, mut start, mut depth) = (Vec::new(), 0, 0usize);
    loop {
        let before = tokenizer.position();
        match tokenizer.next_token() {
            Token::Eof => {
                items.push(&value[start..]);
                return items;
            }
            Token::Comma if depth == 0 => {
                items.push(&value[start..before]);
                start = tokenizer.position();
            }
            Token::Function(_) | Token::LeftParen | Token::LeftBracket | Token::LeftBrace => depth += 1,
            Token::RightParen | Token::RightBracket | Token::RightBrace => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
}

/// Parses `src`. An entry that does not parse is skipped; the descriptor is invalid only when
/// no entry remains.
pub fn parse_src(value: &str) -> Result<Vec<FontSource>, CssParseError> {
    let sources: Vec<FontSource> = split_top_level(value).into_iter().filter_map(parse_entry).collect();
    if sources.is_empty() {
        return Err(CssParseError::InvalidValue("src"));
    }
    Ok(sources)
}
