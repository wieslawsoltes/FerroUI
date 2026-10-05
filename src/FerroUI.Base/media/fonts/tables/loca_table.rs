use crate::media::fonts::OpenTypeTag;
use crate::media::IFontMemory;
use crate::utilities::ReadOnlyMemory;

use super::head_table::{HeadTable, IndexToLocFormat};
use super::maxp_table::MaxpTable;

/// The `loca` (index to location) table: the offsets of the glyphs in the
/// `glyf` table.
#[derive(Clone, Debug)]
pub struct LocaTable {
    data: ReadOnlyMemory<u8>,
    glyph_count: i32,
    is_short_format: bool,
}

impl LocaTable {
    pub const TABLE_NAME: &'static str = "loca";
    pub const TAG: OpenTypeTag = OpenTypeTag::from_chars('l', 'o', 'c', 'a');

    pub(crate) fn new(data: ReadOnlyMemory<u8>, glyph_count: i32, is_short_format: bool) -> Self {
        // The table stores glyphCount + 1 offsets. A truncated table cannot index every
        // glyph the font declares, so clamp the usable count to what the data covers —
        // lookups past it then fail the index check up front instead of reading a
        // partial entry.
        let entry_size: i64 = if is_short_format { 2 } else { 4 };
        let covered_glyphs = data.len() as i64 / entry_size - 1;

        let glyph_count = (glyph_count as i64).min(covered_glyphs).max(0) as i32;

        Self { data, glyph_count, is_short_format }
    }

    pub fn glyph_count(&self) -> i32 {
        self.glyph_count
    }

    pub(crate) fn raw_data(&self) -> &[u8] {
        self.data.span()
    }

    pub(crate) fn is_short_format(&self) -> bool {
        self.is_short_format
    }

    /// Loads the table; `None` when the font has no `loca` table.
    pub fn load(font: &dyn IFontMemory, head: &HeadTable, maxp: &MaxpTable) -> Option<LocaTable> {
        let loca_data = font.try_get_table(Self::TAG)?;

        let is_short_format = head.index_to_loc_format == IndexToLocFormat::Short;

        // The constructor clamps the glyph count to what the table's data actually covers.
        Some(LocaTable::new(loca_data, maxp.num_glyphs as i32, is_short_format))
    }

    /// The `(start, end)` offsets of the glyph's data in the `glyf` table;
    /// `None` when the glyph index is out of range.
    pub fn try_get_offsets(&self, glyph_index: i32) -> Option<(i32, i32)> {
        if glyph_index < 0 || glyph_index >= self.glyph_count {
            return None;
        }

        let start = self.get_offset(glyph_index);
        let end = self.get_offset(glyph_index + 1);

        Some((start, end))
    }

    fn get_offset(&self, glyph_index: i32) -> i32 {
        // Note: allows glyphCount for the end offset
        if glyph_index < 0 || glyph_index > self.glyph_count {
            return 0;
        }

        let span = self.data.span();
        let glyph_index = glyph_index as usize;

        if self.is_short_format {
            let byte_offset = glyph_index * 2;

            // Short format: uint16 values stored divided by 2
            match span.get(byte_offset..byte_offset + 2) {
                Some(bytes) => u16::from_be_bytes([bytes[0], bytes[1]]) as i32 * 2,
                None => 0,
            }
        } else {
            let byte_offset = glyph_index * 4;

            // Long format: uint32 values
            match span.get(byte_offset..byte_offset + 4) {
                Some(bytes) => {
                    let value = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);

                    // Clamp to i32::MAX to avoid overflow
                    value.min(i32::MAX as u32) as i32
                }
                None => 0,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::fonts::tables::font_version::FontVersion;
    use crate::media::fonts::tables::testing::SyntheticFont;

    fn create_loca(loca_data: Vec<u8>, glyph_count: i32, is_short_format: bool) -> LocaTable {
        LocaTable::new(ReadOnlyMemory::from_vec(loca_data), glyph_count, is_short_format)
    }

    fn head(index_to_loc_format: IndexToLocFormat) -> HeadTable {
        HeadTable {
            version: FontVersion::from_parts(1, 0),
            font_revision: FontVersion::default(),
            check_sum_adjustment: 0,
            magic_number: 0x5F0F_3CF5,
            flags: Default::default(),
            units_per_em: 1000,
            created: 0,
            modified: 0,
            x_min: 0,
            y_min: 0,
            x_max: 0,
            y_max: 0,
            mac_style: Default::default(),
            lowest_rec_ppem: 0,
            font_direction_hint: Default::default(),
            index_to_loc_format,
            glyph_data_format: Default::default(),
        }
    }

    #[test]
    fn glyph_count_is_clamped_to_what_the_table_covers() {
        // Three short-format entries (6 bytes) cover two glyphs (entries = glyphCount + 1),
        // however many glyphs maxp declares. Lookups past the covered range must fail the
        // index check instead of reading a truncated entry.
        let loca = create_loca(vec![0u8; 6], 10, true);

        assert_eq!(loca.glyph_count(), 2);
        assert!(loca.try_get_offsets(1).is_some());
        assert!(loca.try_get_offsets(2).is_none());
    }

    #[test]
    fn try_get_offsets_returns_false_for_out_of_range() {
        let loca = create_loca(vec![0u8; 8], 3, true);

        assert!(loca.try_get_offsets(-1).is_none());
        assert!(loca.try_get_offsets(i32::MAX).is_none());
        // The glyph count is one past the last valid glyph index.
        assert!(loca.try_get_offsets(loca.glyph_count()).is_none());
    }

    #[test]
    fn short_format_offsets_are_doubled() {
        let loca = create_loca(vec![0, 0, 0, 5, 0, 5, 0xFF, 0xFF], 3, true);

        assert_eq!(loca.glyph_count(), 3);
        assert_eq!(loca.try_get_offsets(0), Some((0, 10)));
        // An empty glyph has equal offsets.
        assert_eq!(loca.try_get_offsets(1), Some((10, 10)));
        assert_eq!(loca.try_get_offsets(2), Some((10, 0x1FFFE)));
    }

    #[test]
    fn long_format_offsets_are_clamped_to_the_signed_range() {
        let mut data = Vec::new();
        data.extend_from_slice(&12u32.to_be_bytes());
        data.extend_from_slice(&0xFFFF_FFFFu32.to_be_bytes());

        let loca = create_loca(data, 1, false);

        assert_eq!(loca.glyph_count(), 1);
        assert_eq!(loca.try_get_offsets(0), Some((12, i32::MAX)));
    }

    #[test]
    fn empty_table_has_no_glyphs() {
        let loca = create_loca(Vec::new(), 10, false);

        assert_eq!(loca.glyph_count(), 0);
        assert!(loca.try_get_offsets(0).is_none());
    }

    #[test]
    fn load_uses_the_head_format_and_the_maxp_glyph_count() {
        let mut font = SyntheticFont::new();
        let maxp = MaxpTable { num_glyphs: 2, ..MaxpTable::default() };

        assert!(LocaTable::load(&font, &head(IndexToLocFormat::Short), &maxp).is_none());

        font.replace("loca", vec![0, 0, 0, 2, 0, 4, 0, 0, 0, 0, 0, 0]);

        let short = LocaTable::load(&font, &head(IndexToLocFormat::Short), &maxp).unwrap();

        assert!(short.is_short_format());
        assert_eq!(short.glyph_count(), 2);
        assert_eq!(short.try_get_offsets(1), Some((4, 8)));

        let long = LocaTable::load(&font, &head(IndexToLocFormat::Long), &maxp).unwrap();

        assert!(!long.is_short_format());
        assert_eq!(long.glyph_count(), 2);
        assert_eq!(long.try_get_offsets(0), Some((2, 0x0004_0000)));
        assert_eq!(long.raw_data().len(), 12);
    }
}
