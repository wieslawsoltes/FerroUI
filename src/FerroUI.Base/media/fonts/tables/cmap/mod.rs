//! The `cmap` (character to glyph index mapping) table reader.

mod character_to_glyph_map;
mod character_to_glyph_map_dictionary;
mod cmap_encoding;
mod cmap_format;
mod cmap_format12_or13_table;
mod cmap_format4_table;
mod cmap_subtable_entry;
mod cmap_table;
mod codepoint_range;
mod codepoint_range_enumerator;

#[cfg(test)]
mod cmap_table_tests;

pub use character_to_glyph_map::CharacterToGlyphMap;
pub use character_to_glyph_map_dictionary::CharacterToGlyphMapDictionary;
pub use cmap_encoding::CmapEncoding;
pub use cmap_format::CmapFormat;
pub use cmap_format12_or13_table::CmapFormat12Or13Table;
pub use cmap_format4_table::CmapFormat4Table;
pub use cmap_subtable_entry::CmapSubtableEntry;
pub use cmap_table::CmapTable;
pub use codepoint_range::CodepointRange;
pub use codepoint_range_enumerator::CodepointRangeEnumerator;
