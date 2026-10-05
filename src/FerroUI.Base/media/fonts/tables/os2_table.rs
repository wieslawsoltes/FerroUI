// Derived from SixLabors.Fonts (Apache License 2.0), see NOTICE.md.

use bitflags::bitflags;

use crate::media::fonts::OpenTypeTag;
use crate::media::IFontMemory;

use super::big_endian_binary_reader::{BigEndianBinaryReader, FontTableError};
use super::panose::Panose;

bitflags! {
    /// The `fsSelection` flags of the `OS/2` table.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    pub struct FontSelectionFlags: u16 {
        const ITALIC = 1;
        const UNDERSCORE = 1 << 1;
        const NEGATIVE = 1 << 2;
        const OUTLINED = 1 << 3;
        const STRIKEOUT = 1 << 4;
        const BOLD = 1 << 5;
        const REGULAR = 1 << 6;
        const USE_TYPO_METRICS = 1 << 7;
        const WWS = 1 << 8;
        const OBLIQUE = 1 << 9;
    }
}

/// The `OS/2` (OS/2 and Windows metrics) table.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct OS2Table {
    pub version: u16,
    pub x_avg_char_width: i16,
    pub weight_class: u16,
    pub width_class: u16,
    pub fs_type: u16,
    pub y_subscript_x_size: i16,
    pub y_subscript_y_size: i16,
    pub y_subscript_x_offset: i16,
    pub y_subscript_y_offset: i16,
    pub y_superscript_x_size: i16,
    pub y_superscript_y_size: i16,
    pub y_superscript_x_offset: i16,
    pub y_superscript_y_offset: i16,
    pub strikeout_size: i16,
    pub strikeout_position: i16,
    pub family_class: i16,
    pub panose: Panose,
    pub unicode_range1: u32,
    pub unicode_range2: u32,
    pub unicode_range3: u32,
    pub unicode_range4: u32,
    pub vendor_id: u32,
    pub selection: FontSelectionFlags,
    pub first_char_index: u16,
    pub last_char_index: u16,
    pub typo_ascender: i16,
    pub typo_descender: i16,
    pub typo_line_gap: i16,
    pub win_ascent: u16,
    pub win_descent: u16,
    pub code_page_range1: u32,
    pub code_page_range2: u32,
    pub x_height: i16,
    pub cap_height: i16,
    pub default_char: u16,
    pub break_char: u16,
    pub max_context: u16,
    pub lower_optical_point_size: u16,
    pub upper_optical_point_size: u16,
}

impl OS2Table {
    pub const TABLE_NAME: &'static str = "OS/2";
    pub const TAG: OpenTypeTag = OpenTypeTag::from_chars('O', 'S', '/', '2');

    /// Loads the table.
    ///
    /// `Ok(None)` when the font has no `OS/2` table; `Err` when the table is
    /// present but too short for its version.
    pub fn try_load(font: &dyn IFontMemory) -> Result<Option<OS2Table>, FontTableError> {
        let Some(table) = font.try_get_table(Self::TAG) else {
            return Ok(None);
        };

        let mut binary_reader = BigEndianBinaryReader::new(table.span());

        Ok(Some(Self::load(&mut binary_reader)?))
    }

    fn load(reader: &mut BigEndianBinaryReader<'_>) -> Result<OS2Table, FontTableError> {
        let version = reader.read_uint16()?;
        let x_avg_char_width = reader.read_int16()?;
        let weight_class = reader.read_uint16()?;
        let width_class = reader.read_uint16()?;
        let fs_type = reader.read_uint16()?;
        let y_subscript_x_size = reader.read_int16()?;
        let y_subscript_y_size = reader.read_int16()?;
        let y_subscript_x_offset = reader.read_int16()?;
        let y_subscript_y_offset = reader.read_int16()?;
        let y_superscript_x_size = reader.read_int16()?;
        let y_superscript_y_size = reader.read_int16()?;
        let y_superscript_x_offset = reader.read_int16()?;
        let y_superscript_y_offset = reader.read_int16()?;
        let strikeout_size = reader.read_int16()?;
        let strikeout_position = reader.read_int16()?;
        let family_class = reader.read_int16()?;
        let panose = Panose::load(reader)?;
        let unicode_range1 = reader.read_uint32()?;
        let unicode_range2 = reader.read_uint32()?;
        let unicode_range3 = reader.read_uint32()?;
        let unicode_range4 = reader.read_uint32()?;
        let vendor_id = reader.read_uint32()?;
        let selection = FontSelectionFlags::from_bits_retain(reader.read_uint16()?);
        let first_char_index = reader.read_uint16()?;
        let last_char_index = reader.read_uint16()?;
        let typo_ascender = reader.read_int16()?;
        let typo_descender = reader.read_int16()?;
        let typo_line_gap = reader.read_int16()?;
        let win_ascent = reader.read_uint16()?;
        let win_descent = reader.read_uint16()?;

        let mut code_page_range1 = 0u32;
        let mut code_page_range2 = 0u32;
        let mut x_height = 0i16;
        let mut cap_height = 0i16;
        let mut default_char = 0u16;
        let mut break_char = 0u16;
        let mut max_context = 0u16;
        let mut lower_optical_point_size = 0u16;
        let mut upper_optical_point_size = 0xFFFFu16;

        if version >= 1 {
            code_page_range1 = reader.read_uint32()?;
            code_page_range2 = reader.read_uint32()?;
        }

        if version >= 2 {
            x_height = reader.read_int16()?;
            cap_height = reader.read_int16()?;
            default_char = reader.read_uint16()?;
            break_char = reader.read_uint16()?;
            max_context = reader.read_uint16()?;
        }

        if version >= 5 {
            lower_optical_point_size = reader.read_uint16()?;
            upper_optical_point_size = reader.read_uint16()?;
        }

        Ok(OS2Table {
            version,
            x_avg_char_width,
            weight_class,
            width_class,
            fs_type,
            y_subscript_x_size,
            y_subscript_y_size,
            y_subscript_x_offset,
            y_subscript_y_offset,
            y_superscript_x_size,
            y_superscript_y_size,
            y_superscript_x_offset,
            y_superscript_y_offset,
            strikeout_size,
            strikeout_position,
            family_class,
            panose,
            unicode_range1,
            unicode_range2,
            unicode_range3,
            unicode_range4,
            vendor_id,
            selection,
            first_char_index,
            last_char_index,
            typo_ascender,
            typo_descender,
            typo_line_gap,
            win_ascent,
            win_descent,
            code_page_range1,
            code_page_range2,
            x_height,
            cap_height,
            default_char,
            break_char,
            max_context,
            lower_optical_point_size,
            upper_optical_point_size,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::fonts::tables::panose::PanoseFamilyKind;
    use crate::media::fonts::tables::testing::{BigEndianBuffer, SyntheticFont};

    /// An `OS/2` table with the values the reference tests pin for their
    /// static TrueType test font.
    fn build(version: i32) -> Vec<u8> {
        let mut buffer = BigEndianBuffer::new();

        buffer
            .uint16(version)
            .int16(1600) // xAvgCharWidth
            .uint16(400) // usWeightClass
            .uint16(5) // usWidthClass
            .uint16(0) // fsType
            .int16(1) // ySubscriptXSize
            .int16(2) // ySubscriptYSize
            .int16(3) // ySubscriptXOffset
            .int16(4) // ySubscriptYOffset
            .int16(5) // ySuperscriptXSize
            .int16(6) // ySuperscriptYSize
            .int16(7) // ySuperscriptXOffset
            .int16(8) // ySuperscriptYOffset
            .int16(192) // yStrikeoutSize
            .int16(900) // yStrikeoutPosition
            .int16(-1) // sFamilyClass
            .bytes(&[2, 11, 5, 3, 0, 0, 0, 2, 0, 4]) // panose
            .uint32(0xE00002FF) // ulUnicodeRange1
            .uint32(2) // ulUnicodeRange2
            .uint32(3) // ulUnicodeRange3
            .uint32(4) // ulUnicodeRange4
            .tag("RSMS") // achVendID
            .uint16(0x40) // fsSelection
            .uint16(0x20) // usFirstCharIndex
            .uint16(0xFFFF) // usLastCharIndex
            .int16(2728) // sTypoAscender
            .int16(-680) // sTypoDescender
            .int16(0) // sTypoLineGap
            .uint16(2728) // usWinAscent
            .uint16(680); // usWinDescent

        if version >= 1 {
            buffer.uint32(0x2000019F).uint32(9);
        }

        if version >= 2 {
            buffer.int16(1536).int16(2048).uint16(0).uint16(32).uint16(4);
        }

        if version >= 5 {
            buffer.uint16(10).uint16(20);
        }

        buffer.to_array()
    }

    fn load(version: i32) -> OS2Table {
        let mut font = SyntheticFont::new();
        font.replace("OS/2", build(version));

        OS2Table::try_load(&font).unwrap().unwrap()
    }

    #[test]
    fn missing_table_gives_none() {
        assert_eq!(OS2Table::try_load(&SyntheticFont::new()), Ok(None));
    }

    #[test]
    fn loads_the_version_0_fields() {
        let os2 = load(4);

        assert_eq!(os2.version, 4);
        assert_eq!(os2.x_avg_char_width, 1600);
        assert_eq!(os2.weight_class, 400);
        assert_eq!(os2.width_class, 5);
        assert_eq!(os2.y_subscript_x_size, 1);
        assert_eq!(os2.y_superscript_y_offset, 8);
        assert_eq!(os2.strikeout_size, 192);
        assert_eq!(os2.strikeout_position, 900);
        assert_eq!(os2.family_class, -1);
        assert_eq!(os2.panose.family_kind(), PanoseFamilyKind::LatinText);
        assert_eq!(os2.unicode_range1, 0xE00002FF);
        assert_eq!(os2.unicode_range4, 4);
        assert_eq!(os2.vendor_id, OpenTypeTag::parse("RSMS").value());
        assert!(os2.selection.contains(FontSelectionFlags::REGULAR));
        assert!(!os2.selection.contains(FontSelectionFlags::ITALIC));
        assert_eq!(os2.first_char_index, 0x20);
        assert_eq!(os2.last_char_index, 0xFFFF);
        assert_eq!(os2.typo_ascender, 2728);
        assert_eq!(os2.typo_descender, -680);
        assert!(os2.typo_ascender > os2.typo_descender);
        assert_eq!(os2.typo_line_gap, 0);
        assert_eq!(os2.win_ascent, 2728);
        assert_eq!(os2.win_descent, 680);
    }

    #[test]
    fn version_dependent_fields_default_when_absent() {
        let v0 = load(0);

        assert_eq!(v0.code_page_range1, 0);
        assert_eq!(v0.x_height, 0);
        assert_eq!(v0.lower_optical_point_size, 0);
        assert_eq!(v0.upper_optical_point_size, 0xFFFF);

        let v1 = load(1);

        assert_eq!(v1.code_page_range1, 0x2000019F);
        assert_eq!(v1.code_page_range2, 9);
        assert_eq!(v1.cap_height, 0);

        let v4 = load(4);

        assert_eq!(v4.x_height, 1536);
        assert_eq!(v4.cap_height, 2048);
        assert_eq!(v4.default_char, 0);
        assert_eq!(v4.break_char, 32);
        assert_eq!(v4.max_context, 4);
        assert_eq!(v4.upper_optical_point_size, 0xFFFF);

        let v5 = load(5);

        assert_eq!(v5.lower_optical_point_size, 10);
        assert_eq!(v5.upper_optical_point_size, 20);
    }

    #[test]
    fn table_too_short_for_its_version_is_an_error() {
        let mut font = SyntheticFont::new();

        // Claims version 5 but carries only the version 4 fields.
        let mut table = build(4);
        table[1] = 5;
        font.replace("OS/2", table);

        assert_eq!(OS2Table::try_load(&font), Err(FontTableError::EndOfSpan { missing: 2 }));
    }
}
