use super::bidi_class::BidiClass;
use super::bidi_paired_bracket_type::BidiPairedBracketType;
use super::bidi_trie::BiDiTrie;
use super::codepoint::Codepoint;
use super::east_asian_width_class::EastAsianWidthClass;
use super::east_asian_width_trie::EastAsianWidthTrie;
use super::general_category::GeneralCategory;
use super::grapheme_break_class::GraphemeBreakClass;
use super::indic_conjunct_break_class::IndicConjunctBreakClass;
use super::line_break_class::LineBreakClass;
use super::script::Script;
use super::script_extensions_data::{SCRIPT_EXTENSION_SETS, SCRIPT_EXTENSION_SET_OFFSETS};
use super::segmentation_trie::SegmentationTrie;
use super::sentence_break_class::SentenceBreakClass;
use super::unicode_data::UnicodeData;
use super::unicode_data_trie::UnicodeDataTrie;
use super::word_break_class::WordBreakClass;

impl UnicodeData {
    /// Gets the [`GeneralCategory`] for a Unicode codepoint.
    #[inline(always)]
    pub fn get_general_category(codepoint: u32) -> GeneralCategory {
        GeneralCategory::from_u32(UnicodeDataTrie::trie().get(codepoint) & Self::CATEGORY_MASK)
    }

    /// Gets the [`Script`] for a Unicode codepoint.
    #[inline(always)]
    pub fn get_script(codepoint: u32) -> Script {
        Script::from_u32((UnicodeDataTrie::trie().get(codepoint) >> Self::SCRIPT_SHIFT) & Self::SCRIPT_MASK)
    }

    /// Gets the [`BidiClass`] for a Unicode codepoint.
    #[inline(always)]
    pub fn get_bi_di_class(codepoint: u32) -> BidiClass {
        BidiClass::from_u32((BiDiTrie::trie().get(codepoint) >> Self::BIDICLASS_SHIFT) & Self::BIDICLASS_MASK)
    }

    /// Gets the [`BidiPairedBracketType`] for a Unicode codepoint.
    #[inline(always)]
    pub fn get_bi_di_paired_bracket_type(codepoint: u32) -> BidiPairedBracketType {
        BidiPairedBracketType::from_u32(
            (BiDiTrie::trie().get(codepoint) >> Self::BIDIPAIREDBRACKEDTYPE_SHIFT) & Self::BIDIPAIREDBRACKEDTYPE_MASK,
        )
    }

    /// Gets the paired bracket for a Unicode codepoint.
    #[inline(always)]
    pub fn get_bi_di_paired_bracket(codepoint: u32) -> Codepoint {
        Codepoint::new(BiDiTrie::trie().get(codepoint) & Self::BIDIPAIREDBRACKED_MASK)
    }

    /// Gets the line break class for a Unicode codepoint.
    #[inline(always)]
    pub fn get_line_break_class(codepoint: u32) -> LineBreakClass {
        LineBreakClass::from_u32((SegmentationTrie::trie().get(codepoint) >> Self::LINEBREAK_SHIFT) & Self::LINEBREAK_MASK)
    }

    /// Gets the word break class for a Unicode codepoint.
    #[inline(always)]
    pub fn get_word_break_class(codepoint: u32) -> WordBreakClass {
        WordBreakClass::from_u32((SegmentationTrie::trie().get(codepoint) >> Self::WORDBREAK_SHIFT) & Self::WORDBREAK_MASK)
    }

    /// Gets the grapheme break type for the Unicode codepoint.
    #[inline(always)]
    pub fn get_grapheme_cluster_break(codepoint: u32) -> GraphemeBreakClass {
        GraphemeBreakClass::from_u32(SegmentationTrie::trie().get(codepoint) & Self::GRAPHEMEBREAK_MASK)
    }

    /// Gets the Indic conjunct break class for the Unicode codepoint.
    #[inline(always)]
    pub(crate) fn get_indic_conjunct_break_class(codepoint: u32) -> IndicConjunctBreakClass {
        IndicConjunctBreakClass::from_u32(
            (SegmentationTrie::trie().get(codepoint) >> Self::INDICCONJUNCTBREAK_SHIFT) & Self::INDICCONJUNCTBREAK_MASK,
        )
    }

    /// Gets the sentence break class for the Unicode codepoint (UAX #29).
    #[inline(always)]
    pub fn get_sentence_break_class(codepoint: u32) -> SentenceBreakClass {
        SentenceBreakClass::from_u32(
            (SegmentationTrie::trie().get(codepoint) >> Self::SENTENCEBREAK_SHIFT) & Self::SENTENCEBREAK_MASK,
        )
    }

    /// Gets whether the codepoint has the Unicode `Emoji` property.
    #[inline(always)]
    pub fn get_is_emoji(codepoint: u32) -> bool {
        (SegmentationTrie::trie().get(codepoint) & Self::EMOJI_FLAG) != 0
    }

    /// Gets whether the codepoint has the Unicode `Emoji_Presentation` property, i.e. whether
    /// it defaults to an emoji presentation when no variation selector is present.
    #[inline(always)]
    pub fn get_has_emoji_presentation(codepoint: u32) -> bool {
        (SegmentationTrie::trie().get(codepoint) & Self::EMOJIPRESENTATION_FLAG) != 0
    }

    /// Gets whether the codepoint has the Unicode `Default_Ignorable_Code_Point` property.
    #[inline(always)]
    pub fn get_is_default_ignorable(codepoint: u32) -> bool {
        (SegmentationTrie::trie().get(codepoint) & Self::DEFAULTIGNORABLE_FLAG) != 0
    }

    /// Gets the EastAsianWidth class for the Unicode codepoint.
    #[inline(always)]
    pub fn get_east_asian_width_class(codepoint: u32) -> EastAsianWidthClass {
        EastAsianWidthClass::from_u32(EastAsianWidthTrie::trie().get(codepoint))
    }

    /// Determines whether the given codepoint's Script_Extensions property (UAX #24) contains
    /// the supplied script. When the codepoint has no explicit Script_Extensions entry the
    /// extensions set is taken to be the singleton of the codepoint's primary
    /// [`Script`] property.
    pub fn has_script_extension(codepoint: u32, script: Script) -> bool {
        if script == Script::Unknown {
            return false;
        }

        let packed = UnicodeDataTrie::trie().get(codepoint);
        let set_index = ((packed >> Self::SCRIPTEXTENSIONS_SHIFT) & Self::SCRIPTEXTENSIONS_MASK) as usize;

        if set_index == 0 {
            let primary = Script::from_u32((packed >> Self::SCRIPT_SHIFT) & Self::SCRIPT_MASK);
            return primary == script;
        }

        let offsets = &SCRIPT_EXTENSION_SET_OFFSETS;
        if set_index + 1 >= offsets.len() {
            return false;
        }

        let set_start = offsets[set_index] as usize;
        let set_end = offsets[set_index + 1] as usize;
        let sets = &SCRIPT_EXTENSION_SETS;

        if set_end > sets.len() || set_start > set_end {
            return false;
        }

        let target = script as i32 as u8;
        let set = &sets[set_start..set_end];

        set.contains(&target)
    }
}
