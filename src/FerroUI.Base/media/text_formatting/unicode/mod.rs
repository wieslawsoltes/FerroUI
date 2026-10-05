//! Unicode data and segmentation algorithms.

mod bidi_algorithm;
mod bidi_class;
mod bidi_data;
mod bidi_paired_bracket_type;
mod bidi_trie;
mod binary_reader_extensions;
mod codepoint;
mod codepoint_enumerator;
mod east_asian_width_class;
mod east_asian_width_trie;
mod general_category;
mod grapheme;
mod grapheme_break;
mod grapheme_break_class;
mod grapheme_enumerator;
mod indic_conjunct_break_class;
mod line_break;
mod line_break_class;
mod line_break_enumerator;
mod property_value_alias_helper;
mod script;
mod script_extensions_data;
mod segmentation_trie;
mod sentence_break_class;
mod sentence_break_enumerator;
mod sentence_segment;
mod unicode_data;
mod unicode_data_lookups;
mod unicode_data_source;
mod unicode_data_trie;
mod unicode_trie;
mod unicode_trie_builder;
mod unicode_trie_builder_constants;
mod utf16_utils;
mod word_break_class;
mod word_break_enumerator;
mod word_segment;

pub use bidi_algorithm::BidiAlgorithm;
pub use bidi_class::BidiClass;
pub use bidi_data::BidiData;
pub use bidi_paired_bracket_type::BidiPairedBracketType;
pub use codepoint::Codepoint;
pub use codepoint_enumerator::CodepointEnumerator;
pub use east_asian_width_class::EastAsianWidthClass;
pub use general_category::GeneralCategory;
pub use grapheme::Grapheme;
pub use grapheme_break_class::GraphemeBreakClass;
pub use grapheme_enumerator::GraphemeEnumerator;
pub use line_break::LineBreak;
pub use line_break_class::LineBreakClass;
pub use line_break_enumerator::LineBreakEnumerator;
pub use script::Script;
pub use sentence_break_class::SentenceBreakClass;
pub use sentence_break_enumerator::SentenceBreakEnumerator;
pub use sentence_segment::SentenceSegment;
pub use unicode_data::UnicodeData;
pub use word_break_class::WordBreakClass;
pub use word_break_enumerator::WordBreakEnumerator;
pub use word_segment::WordSegment;

#[allow(unused_imports)] // internal surface for the sibling text formatting modules
pub(crate) use {
    bidi_trie::BiDiTrie,
    binary_reader_extensions::{BinaryReaderExtensions, BinaryWriterExtensions},
    east_asian_width_trie::EastAsianWidthTrie,
    grapheme_break::GraphemeBreak,
    indic_conjunct_break_class::IndicConjunctBreakClass,
    property_value_alias_helper::PropertyValueAliasHelper,
    segmentation_trie::SegmentationTrie,
    unicode_data_source::UnicodeDataSource,
    unicode_data_trie::UnicodeDataTrie,
    unicode_trie::UnicodeTrie,
    unicode_trie_builder::UnicodeTrieBuilder,
    utf16_utils::Utf16Utils,
};

#[cfg(test)]
mod bidi_algorithm_tests;
#[cfg(test)]
mod bidi_class_tests;
#[cfg(test)]
mod codepoint_has_script_extension_tests;
#[cfg(test)]
mod codepoint_tests;
#[cfg(test)]
mod grapheme_break_class_trie_generator_tests;
#[cfg(test)]
mod line_break_enumerator_tests;
#[cfg(test)]
mod property_value_alias_helper_tests;
#[cfg(test)]
mod sentence_break_enumerator_tests;
#[cfg(test)]
mod ucd_test_data;
#[cfg(test)]
mod unicode_data_tests;
#[cfg(test)]
mod unicode_trie_tests;
#[cfg(test)]
mod utf16_utils_tests;
#[cfg(test)]
mod word_break_enumerator_tests;
