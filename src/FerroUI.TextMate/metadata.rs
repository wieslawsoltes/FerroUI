use crate::FontStyle;
use std::sync::LazyLock;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum StandardTokenType {
    #[default]
    Other = 0,
    Comment = 1,
    String = 2,
    RegEx = 3,
}

/// Packed attributes with the same bit layout as the TextMate token format.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EncodedTokenAttributes(pub u32);
impl EncodedTokenAttributes {
    pub const fn new(
        language: u8,
        token_type: StandardTokenType,
        balanced: bool,
        style: FontStyle,
        foreground: u16,
        background: u8,
    ) -> Self {
        Self(
            language as u32
                | (token_type as u32) << 8
                | (balanced as u32) << 10
                | ((style.0 as u32) & 15) << 11
                | ((foreground as u32) & 511) << 15
                | (background as u32) << 24,
        )
    }
    pub const fn language_id(self) -> u8 {
        self.0 as u8
    }
    pub const fn token_type(self) -> StandardTokenType {
        match (self.0 >> 8) & 3 {
            1 => StandardTokenType::Comment,
            2 => StandardTokenType::String,
            3 => StandardTokenType::RegEx,
            _ => StandardTokenType::Other,
        }
    }
    pub const fn contains_balanced_brackets(self) -> bool {
        self.0 & (1 << 10) != 0
    }
    pub const fn font_style(self) -> FontStyle {
        FontStyle(((self.0 >> 11) & 15) as u8)
    }
    pub const fn foreground(self) -> u16 {
        ((self.0 >> 15) & 511) as u16
    }
    pub const fn background(self) -> u8 {
        (self.0 >> 24) as u8
    }
    /// Inherit the nearest typed scope. Within each scope, the first complete
    /// word `comment`, `string`, `regex` or `meta.embedded` determines the type;
    /// `meta.embedded` explicitly resets an enclosing type to `Other`.
    pub fn infer_token_type(scopes: &[String]) -> StandardTokenType {
        scopes
            .iter()
            .rev()
            .find_map(|scope| scope_token_type(scope))
            .unwrap_or_default()
    }
}

// TextMateSharp BasicScopeAttributesProvider.ToStandardTokenType searches
// \b(comment|string|regex|meta\.embedded)\b rather than testing scope prefixes.
// Keep that first-match order without invoking a scanner for ordinary ASCII
// scopes. None is the upstream NotSet sentinel; Other is an explicit reset.
fn scope_token_type(scope: &str) -> Option<StandardTokenType> {
    let mut previous = None;
    for (offset, character) in scope.char_indices() {
        let at_boundary = !previous.is_some_and(is_boundary_word);
        previous = Some(character);
        if !at_boundary {
            continue;
        }
        let (name, token_type) = match character {
            'c' => ("comment", StandardTokenType::Comment),
            's' => ("string", StandardTokenType::String),
            'r' => ("regex", StandardTokenType::RegEx),
            'm' => ("meta.embedded", StandardTokenType::Other),
            _ => continue,
        };
        if let Some(after) = scope[offset..].strip_prefix(name) {
            if !after.chars().next().is_some_and(is_boundary_word) {
                return Some(token_type);
            }
        }
    }
    None
}

fn is_boundary_word(character: char) -> bool {
    if character.is_ascii() {
        return character.is_ascii_alphanumeric() || character == '_';
    }
    // .NET RegexCharClass.IsBoundaryWordChar examines UTF-16 code units.
    // Both surrogates of a supplementary scalar have category Cs, not L/Mn.
    if character.len_utf16() == 2 {
        return false;
    }
    // Reuse the existing regex dependency's Unicode category tables. Its plain
    // \w is broader than .NET's category set (notably Mc, Me and Nl), so use
    // the exact categories plus the join controls accepted by .NET boundaries.
    static WORD: LazyLock<fancy_regex::Regex> = LazyLock::new(|| {
        fancy_regex::Regex::new(r"^[\p{L}\p{Mn}\p{Nd}\p{Pc}\x{200C}\x{200D}]$")
            .expect("the fixed Unicode boundary class is valid")
    });
    WORD.is_match(character.encode_utf8(&mut [0; 4]))
        .expect("the fixed character class cannot backtrack")
}
