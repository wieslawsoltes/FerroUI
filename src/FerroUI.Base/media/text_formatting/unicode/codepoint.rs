use super::bidi_class::BidiClass;
use super::bidi_paired_bracket_type::BidiPairedBracketType;
use super::east_asian_width_class::EastAsianWidthClass;
use super::general_category::GeneralCategory;
use super::grapheme_break_class::GraphemeBreakClass;
use super::line_break_class::LineBreakClass;
use super::script::Script;
use super::sentence_break_class::SentenceBreakClass;
use super::unicode_data::UnicodeData;
use super::word_break_class::WordBreakClass;

/// A Unicode code point (not necessarily a scalar value: lone surrogates and
/// values above U+10FFFF can be represented, as upstream).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct Codepoint {
    value: u32,
}

impl Codepoint {
    /// The replacement codepoint that is used for non supported values.
    pub const REPLACEMENT_CODEPOINT: Codepoint = Codepoint::new(0xFFFD);

    /// The replacement codepoint that is used for non supported values.
    #[inline(always)]
    pub const fn replacement_codepoint() -> Codepoint {
        Self::REPLACEMENT_CODEPOINT
    }

    /// Creates a new instance of [`Codepoint`] with the specified value.
    #[inline(always)]
    pub const fn new(value: u32) -> Self {
        Self { value }
    }

    /// Get the codepoint's value.
    #[inline(always)]
    pub const fn value(self) -> u32 {
        self.value
    }

    /// Gets the [`GeneralCategory`].
    #[inline]
    pub fn general_category(self) -> GeneralCategory {
        UnicodeData::get_general_category(self.value)
    }

    /// Gets the [`Script`].
    #[inline]
    pub fn script(self) -> Script {
        UnicodeData::get_script(self.value)
    }

    /// Gets the [`BidiClass`].
    #[inline]
    pub fn bi_di_class(self) -> BidiClass {
        UnicodeData::get_bi_di_class(self.value)
    }

    /// Gets the [`BidiPairedBracketType`].
    #[inline]
    pub fn paired_bracket_type(self) -> BidiPairedBracketType {
        UnicodeData::get_bi_di_paired_bracket_type(self.value)
    }

    /// Gets the [`LineBreakClass`].
    #[inline]
    pub fn line_break_class(self) -> LineBreakClass {
        UnicodeData::get_line_break_class(self.value)
    }

    /// Gets the [`WordBreakClass`].
    #[inline]
    pub fn word_break_class(self) -> WordBreakClass {
        UnicodeData::get_word_break_class(self.value)
    }

    /// Gets the [`SentenceBreakClass`] (UAX #29).
    #[inline]
    pub fn sentence_break_class(self) -> SentenceBreakClass {
        UnicodeData::get_sentence_break_class(self.value)
    }

    /// Gets the [`GraphemeBreakClass`].
    #[inline]
    pub fn grapheme_break_class(self) -> GraphemeBreakClass {
        UnicodeData::get_grapheme_cluster_break(self.value)
    }

    /// Gets the [`EastAsianWidthClass`].
    #[inline]
    pub fn east_asian_width_class(self) -> EastAsianWidthClass {
        UnicodeData::get_east_asian_width_class(self.value)
    }

    /// Determines whether this [`Codepoint`] has the Unicode `Emoji` property.
    ///
    /// The property marks a codepoint that can be presented as emoji; it says nothing about which
    /// presentation is the default. Use [`Codepoint::has_emoji_presentation`] for that.
    #[inline]
    pub fn is_emoji(self) -> bool {
        UnicodeData::get_is_emoji(self.value)
    }

    /// Determines whether this [`Codepoint`] has the Unicode `Emoji_Presentation`
    /// property, i.e. whether it defaults to an emoji presentation when no variation selector
    /// follows it.
    #[inline]
    pub fn has_emoji_presentation(self) -> bool {
        UnicodeData::get_has_emoji_presentation(self.value)
    }

    /// Determines whether this [`Codepoint`] has the Unicode
    /// `Default_Ignorable_Code_Point` property.
    ///
    /// Default ignorable codepoints (variation selectors, joiners, the byte order mark, ...) are
    /// meant to have no visible rendering of their own, so a font is not expected to provide a
    /// glyph for them.
    #[inline]
    pub fn is_default_ignorable(self) -> bool {
        UnicodeData::get_is_default_ignorable(self.value)
    }

    /// Determines whether the codepoint's Unicode Script_Extensions property contains
    /// `script`.
    ///
    /// Returns `true` when the codepoint participates in `script` per UAX #24
    /// (Script_Extensions); `false` otherwise.
    ///
    /// Backed by the UCD `ScriptExtensions.txt` data baked into [`UnicodeData`].
    /// Codepoints without an explicit Script_Extensions entry fall back to the singleton set
    /// of their primary [`Script`] property.
    #[inline(always)]
    pub fn has_script_extension(self, script: Script) -> bool {
        UnicodeData::has_script_extension(self.value, script)
    }

    /// Determines whether this [`Codepoint`] is an east asian char.
    pub fn is_east_asian(self) -> bool {
        matches!(
            self.east_asian_width_class(),
            EastAsianWidthClass::Fullwidth | EastAsianWidthClass::Halfwidth | EastAsianWidthClass::Wide
        )
    }

    /// Determines whether this [`Codepoint`] is a break char.
    pub const fn is_break_char(self) -> bool {
        matches!(self.value, 0x000A | 0x000B | 0x000C | 0x000D | 0x0085 | 0x2028 | 0x2029)
    }

    /// Determines whether this [`Codepoint`] is white space.
    #[inline(always)]
    pub fn is_white_space(self) -> bool {
        const WHITE_SPACE_MASK: u64 = (1u64 << GeneralCategory::Control as i32)
            | (1u64 << GeneralCategory::Format as i32)
            | (1u64 << GeneralCategory::SpaceSeparator as i32);

        ((1u64 << self.general_category() as i32) & WHITE_SPACE_MASK) != 0
    }

    /// Gets the canonical representation of a given codepoint.
    /// <https://www.unicode.org/L2/L2013/13123-norm-and-bpa.pdf>
    ///
    /// Returns the mapped canonical code point, or the passed `code_point`.
    #[inline(always)]
    pub(crate) const fn get_canonical_type(code_point: Codepoint) -> Codepoint {
        if code_point.value == 0x3008 {
            return Codepoint::new(0x2329);
        }

        if code_point.value == 0x3009 {
            return Codepoint::new(0x232A);
        }

        code_point
    }

    /// Gets the codepoint representing the bracket pairing for this instance,
    /// or `None` if this instance has no bracket pairing.
    #[inline(always)]
    pub fn try_get_paired_bracket(self) -> Option<Codepoint> {
        if self.paired_bracket_type() == BidiPairedBracketType::None {
            return None;
        }

        Some(UnicodeData::get_bi_di_paired_bracket(self.value))
    }

    /// Reads the [`Codepoint`] at specified position.
    ///
    /// Returns the codepoint and the count of code units that were read
    /// (always 1 or 2; an index outside of the text or an unpaired surrogate
    /// yields the replacement codepoint and a count of 1).
    #[inline(always)]
    pub fn read_at(text: &[u16], index: usize) -> (Codepoint, usize) {
        // Perf note: this method is performance critical for text layout, modify with care!

        let Some(&unit) = text.get(index) else {
            return (Self::REPLACEMENT_CODEPOINT, 1);
        };

        let code = unit as u32;

        //# Surrogate
        if is_in_range_inclusive(code, 0xD800, 0xDFFF) {
            //# High surrogate
            if code <= 0xDBFF {
                if let Some(&next) = text.get(index + 1) {
                    let hi = code;
                    let low = next as u32;

                    if is_in_range_inclusive(low, 0xDC00, 0xDFFF) {
                        return (Codepoint::new((hi << 10) + low - ((0xD800 << 10) + 0xDC00 - (1 << 16))), 2);
                    }
                }
            }
            //# Low surrogate
            else if index > 0 {
                let low = code;
                let hi = text[index - 1] as u32;

                if is_in_range_inclusive(hi, 0xD800, 0xDBFF) {
                    return (Codepoint::new((hi << 10) + low - ((0xD800 << 10) + 0xDC00 - (1 << 16))), 2);
                }
            }

            return (Self::REPLACEMENT_CODEPOINT, 1);
        }

        (Codepoint::new(code), 1)
    }

    /// Returns `true` if `cp` is between `lower_bound` and `upper_bound`, inclusive.
    #[inline(always)]
    pub const fn is_in_range_inclusive(cp: Codepoint, lower_bound: u32, upper_bound: u32) -> bool {
        is_in_range_inclusive(cp.value, lower_bound, upper_bound)
    }
}

#[inline(always)]
const fn is_in_range_inclusive(value: u32, lower_bound: u32, upper_bound: u32) -> bool {
    value.wrapping_sub(lower_bound) <= upper_bound.wrapping_sub(lower_bound)
}

/// The implicit conversion to `int` upstream.
impl From<Codepoint> for i32 {
    #[inline]
    fn from(codepoint: Codepoint) -> i32 {
        codepoint.value as i32
    }
}

/// The implicit conversion to `uint` upstream.
impl From<Codepoint> for u32 {
    #[inline]
    fn from(codepoint: Codepoint) -> u32 {
        codepoint.value
    }
}
