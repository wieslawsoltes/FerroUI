use std::rc::Rc;

use super::character_to_glyph_map_dictionary::CharacterToGlyphMapDictionary;
use super::cmap_format::CmapFormat;
use super::cmap_format12_or13_table::CmapFormat12Or13Table;
use super::cmap_format4_table::CmapFormat4Table;
use super::codepoint_range_enumerator::CodepointRangeEnumerator;

/// Maps Unicode code points to glyph indices using the font's `cmap` table.
///
/// A cheap handle: cloning shares the underlying subtable. The default value
/// maps nothing. Lookups never allocate.
#[derive(Clone, Default)]
pub struct CharacterToGlyphMap {
    table: Table,
}

#[derive(Clone, Default)]
enum Table {
    #[default]
    None,
    Format4(Rc<CmapFormat4Table>),
    Format12Or13(Rc<CmapFormat12Or13Table>),
}

impl CharacterToGlyphMap {
    #[inline]
    pub(crate) fn from_format4(table: CmapFormat4Table) -> Self {
        Self { table: Table::Format4(Rc::new(table)) }
    }

    #[inline]
    pub(crate) fn from_format12_or13(table: CmapFormat12Or13Table) -> Self {
        Self { table: Table::Format12Or13(Rc::new(table)) }
    }

    /// The format of the subtable the map reads.
    pub fn format(&self) -> CmapFormat {
        match &self.table {
            Table::None => CmapFormat::Format0,
            Table::Format4(_) => CmapFormat::Format4,
            Table::Format12Or13(table) => table.format(),
        }
    }

    /// The glyph for the code point, 0 when it is not mapped (the reference's
    /// indexer and `GetGlyph`).
    #[inline]
    pub fn get_glyph(&self, code_point: i32) -> u16 {
        match &self.table {
            Table::Format4(table) => table.get_glyph(code_point),
            Table::Format12Or13(table) => table.get_glyph(code_point),
            Table::None => 0,
        }
    }

    #[inline]
    pub fn contains_glyph(&self, code_point: i32) -> bool {
        match &self.table {
            Table::Format4(table) => table.contains_glyph(code_point),
            Table::Format12Or13(table) => table.contains_glyph(code_point),
            Table::None => false,
        }
    }

    /// Maps every code point to its glyph (0 when unmapped).
    ///
    /// Panics when `glyph_ids` is shorter than `code_points`.
    #[inline]
    pub fn get_glyphs(&self, code_points: &[i32], glyph_ids: &mut [u16]) {
        match &self.table {
            Table::Format4(table) => table.get_glyphs(code_points, glyph_ids),
            Table::Format12Or13(table) => table.get_glyphs(code_points, glyph_ids),
            Table::None => glyph_ids.fill(0),
        }
    }

    /// The glyph for the code point; `None` when it is not mapped or maps to
    /// glyph 0.
    #[inline]
    pub fn try_get_glyph(&self, code_point: i32) -> Option<u16> {
        match &self.table {
            Table::Format4(table) => table.try_get_glyph(code_point),
            Table::Format12Or13(table) => table.try_get_glyph(code_point),
            Table::None => None,
        }
    }

    /// Enumerates the code point ranges the map covers.
    #[inline]
    pub fn get_mapped_ranges(&self) -> CodepointRangeEnumerator {
        match &self.table {
            Table::Format4(table) => CodepointRangeEnumerator::new(CmapFormat::Format4, Some(table.clone()), None),
            Table::Format12Or13(table) => CodepointRangeEnumerator::new(table.format(), None, Some(table.clone())),
            Table::None => CodepointRangeEnumerator::new(CmapFormat::Format0, None, None),
        }
    }

    /// A dictionary view (code point to glyph) of the map.
    #[inline]
    pub fn as_read_only_dictionary(&self) -> CharacterToGlyphMapDictionary {
        CharacterToGlyphMapDictionary::new(self.clone())
    }
}
