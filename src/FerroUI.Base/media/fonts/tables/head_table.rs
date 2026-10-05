use bitflags::bitflags;

use crate::media::fonts::OpenTypeTag;
use crate::media::IFontMemory;

use super::big_endian_binary_reader::{BigEndianBinaryReader, FontTableError};
use super::font_version::FontVersion;

/// Ticks (100 ns units since 0001-01-01T00:00:00 UTC) of the font epoch,
/// 1904-01-01T00:00:00 UTC.
const FONT_EPOCH_TICKS: i64 = 600_527_520_000_000_000;
const TICKS_PER_SECOND: i64 = 10_000_000;

/// The `head` (font header) table.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeadTable {
    pub version: FontVersion,
    pub font_revision: FontVersion,
    pub check_sum_adjustment: u32,
    pub magic_number: u32,
    pub flags: HeadFlags,
    pub units_per_em: u16,
    /// The creation time in ticks: 100 ns units since 0001-01-01T00:00:00 UTC,
    /// clamped to [[`HeadTable::DATE_TIME_MIN_TICKS`], [`HeadTable::DATE_TIME_MAX_TICKS`]].
    pub created: i64,
    /// The modification time in ticks, see [`HeadTable::created`].
    pub modified: i64,
    pub x_min: i16,
    pub y_min: i16,
    pub x_max: i16,
    pub y_max: i16,
    pub mac_style: MacStyleFlags,
    pub lowest_rec_ppem: u16,
    pub font_direction_hint: FontDirectionHint,
    pub index_to_loc_format: IndexToLocFormat,
    pub glyph_data_format: GlyphDataFormat,
}

impl HeadTable {
    pub const TABLE_NAME: &'static str = "head";
    pub const TAG: OpenTypeTag = OpenTypeTag::from_chars('h', 'e', 'a', 'd');

    /// The smallest representable timestamp (0001-01-01T00:00:00).
    pub const DATE_TIME_MIN_TICKS: i64 = 0;
    /// The largest representable timestamp (9999-12-31T23:59:59.9999999).
    pub const DATE_TIME_MAX_TICKS: i64 = 3_155_378_975_999_999_999;
    /// The font epoch (1904-01-01T00:00:00 UTC) in ticks.
    pub const FONT_EPOCH_TICKS: i64 = FONT_EPOCH_TICKS;

    /// Loads the table.
    ///
    /// `Ok(None)` when the font has no `head` table; `Err` when the table is
    /// present but too short.
    pub fn try_load(font: &dyn IFontMemory) -> Result<Option<HeadTable>, FontTableError> {
        let Some(table) = font.try_get_table(Self::TAG) else {
            return Ok(None);
        };

        let mut reader = BigEndianBinaryReader::new(table.span());

        Ok(Some(Self::load(&mut reader)?))
    }

    fn load(reader: &mut BigEndianBinaryReader<'_>) -> Result<HeadTable, FontTableError> {
        let version = reader.read_version16_dot16()?;
        let font_revision = reader.read_version16_dot16()?;
        let check_sum_adjustment = reader.read_uint32()?;
        let magic_number = reader.read_uint32()?;
        let flags = HeadFlags::from_bits_retain(reader.read_uint16()?);
        let units_per_em = reader.read_uint16()?;
        let created_raw = reader.read_int64()?;
        let modified_raw = reader.read_int64()?;
        let x_min = reader.read_int16()?;
        let y_min = reader.read_int16()?;
        let x_max = reader.read_int16()?;
        let y_max = reader.read_int16()?;
        let mac_style = MacStyleFlags::from_bits_retain(reader.read_uint16()?);
        let lowest_rec_ppem = reader.read_uint16()?;
        let font_direction_hint = FontDirectionHint(reader.read_int16()?);
        let index_to_loc_format = IndexToLocFormat(reader.read_int16()?);
        let glyph_data_format = GlyphDataFormat(reader.read_int16()?);

        let created = Self::safe_add_seconds(FONT_EPOCH_TICKS, created_raw);
        let modified = Self::safe_add_seconds(FONT_EPOCH_TICKS, modified_raw);

        Ok(HeadTable {
            version,
            font_revision,
            check_sum_adjustment,
            magic_number,
            flags,
            units_per_em,
            created,
            modified,
            x_min,
            y_min,
            x_max,
            y_max,
            mac_style,
            lowest_rec_ppem,
            font_direction_hint,
            index_to_loc_format,
            glyph_data_format,
        })
    }

    fn safe_add_seconds(epoch: i64, seconds: i64) -> i64 {
        // Handle invalid/corrupted timestamps gracefully.
        // Valid range for font timestamps is roughly 1904-01-01 to ~2040;
        // negative values or extremely large values indicate corrupted data
        // and clamp to the representable range.
        let ticks = epoch as i128 + seconds as i128 * TICKS_PER_SECOND as i128;

        if ticks < Self::DATE_TIME_MIN_TICKS as i128 {
            Self::DATE_TIME_MIN_TICKS
        } else if ticks > Self::DATE_TIME_MAX_TICKS as i128 {
            Self::DATE_TIME_MAX_TICKS
        } else {
            ticks as i64
        }
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    pub struct HeadFlags: u16 {
        const BaselineAtY0 = 1 << 0;
        const LeftSidebearingAtX0 = 1 << 1;
        const InstructionsDependOnPointSize = 1 << 2;
        const ForcePpemToInteger = 1 << 3;
        const InstructionsMayAlterAdvanceWidth = 1 << 4;
        const VerticalBaseline = 1 << 5;
        const Lossless = 1 << 7;
        const FontConverted = 1 << 8;
        const ClearTypeOptimized = 1 << 9;
        const LastResortFont = 1 << 10;
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    pub struct MacStyleFlags: u16 {
        const Bold = 1 << 0;
        const Italic = 1 << 1;
        const Underline = 1 << 2;
        const Outline = 1 << 3;
        const Shadow = 1 << 4;
        const Condensed = 1 << 5;
        const Extended = 1 << 6;
    }
}

/// The format of the `loca` table offsets; any 16 bit value is representable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct IndexToLocFormat(pub i16);

#[allow(non_upper_case_globals)]
impl IndexToLocFormat {
    pub const Short: IndexToLocFormat = IndexToLocFormat(0);
    pub const Long: IndexToLocFormat = IndexToLocFormat(1);
}

/// The glyph data format; any 16 bit value is representable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct GlyphDataFormat(pub i16);

#[allow(non_upper_case_globals)]
impl GlyphDataFormat {
    pub const Current: GlyphDataFormat = GlyphDataFormat(0);
}

/// The font direction hint; any 16 bit value is representable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct FontDirectionHint(pub i16);

#[allow(non_upper_case_globals)]
impl FontDirectionHint {
    pub const FullyMixed: FontDirectionHint = FontDirectionHint(0);
    pub const OnlyLeftToRight: FontDirectionHint = FontDirectionHint(1);
    pub const LeftToRightWithNeutrals: FontDirectionHint = FontDirectionHint(2);
    pub const OnlyRightToLeft: FontDirectionHint = FontDirectionHint(-1);
    pub const RightToLeftWithNeutrals: FontDirectionHint = FontDirectionHint(-2);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::fonts::tables::testing::{BigEndianBuffer, SyntheticFont};

    /// A `head` table with the values the reference tests pin for their
    /// static TrueType test font.
    fn build_head(created: i64, modified: i64) -> Vec<u8> {
        let mut buffer = BigEndianBuffer::new();

        buffer
            .uint32(0x0001_0000) // version
            .uint32(0x0004_0000) // fontRevision
            .uint32(0x1234_5678) // checkSumAdjustment
            .uint32(0x5F0F_3CF5) // magicNumber
            .uint16(0b1011) // flags
            .uint16(2816) // unitsPerEm
            .int32((created >> 32) as i32)
            .uint32(created as u32)
            .int32((modified >> 32) as i32)
            .uint32(modified as u32)
            .int16(-2080) // xMin
            .int16(-900) // yMin
            .int16(7274) // xMax
            .int16(3072) // yMax
            .uint16(0b11) // macStyle
            .uint16(6) // lowestRecPPEM
            .int16(2) // fontDirectionHint
            .int16(1) // indexToLocFormat
            .int16(0); // glyphDataFormat

        buffer.to_array()
    }

    fn load(head: Vec<u8>) -> HeadTable {
        let mut font = SyntheticFont::new();
        font.replace("head", head);

        HeadTable::try_load(&font).unwrap().unwrap()
    }

    #[test]
    fn missing_table_gives_none() {
        assert_eq!(HeadTable::try_load(&SyntheticFont::new()), Ok(None));
    }

    #[test]
    fn truncated_table_is_an_error() {
        let mut font = SyntheticFont::new();
        font.replace("head", build_head(0, 0));
        font.truncate("head", 53);

        let error = HeadTable::try_load(&font).unwrap_err();

        assert_eq!(error, FontTableError::EndOfSpan { missing: 1 });
    }

    #[test]
    fn loads_every_field() {
        // 2020-01-01T00:00:00 UTC is 3660681600 seconds after the font epoch.
        let head = load(build_head(3_660_681_600, 3_660_681_601));

        assert_eq!(head.version, FontVersion::from_parts(1, 0));
        assert!(head.font_revision.to_float() > 0.0);
        assert_eq!(head.check_sum_adjustment, 0x1234_5678);
        assert_eq!(head.magic_number, 0x5F0F_3CF5);
        assert!(head.flags.contains(HeadFlags::BaselineAtY0));
        assert!(head.flags.contains(HeadFlags::ForcePpemToInteger));
        assert!(!head.flags.contains(HeadFlags::InstructionsDependOnPointSize));
        assert_eq!(head.units_per_em, 2816);
        assert_eq!(head.x_min, -2080);
        assert_eq!(head.x_max, 7274);
        assert_eq!(head.y_min, -900);
        assert_eq!(head.y_max, 3072);
        assert_eq!(head.mac_style, MacStyleFlags::Bold | MacStyleFlags::Italic);
        assert_eq!(head.lowest_rec_ppem, 6);
        assert_eq!(head.font_direction_hint, FontDirectionHint::LeftToRightWithNeutrals);
        assert_eq!(head.index_to_loc_format, IndexToLocFormat::Long);
        assert_eq!(head.glyph_data_format, GlyphDataFormat::Current);

        // 2020-01-01T00:00:00 is 637134336000000000 ticks.
        assert_eq!(head.created, 637_134_336_000_000_000);
        assert_eq!(head.modified, 637_134_336_010_000_000);
        assert!(head.created > HeadTable::FONT_EPOCH_TICKS);
    }

    #[test]
    fn corrupt_timestamps_are_clamped() {
        let head = load(build_head(i64::MIN, i64::MAX));

        assert_eq!(head.created, HeadTable::DATE_TIME_MIN_TICKS);
        assert_eq!(head.modified, HeadTable::DATE_TIME_MAX_TICKS);

        // Exactly the last representable second and the first one past it.
        let head = load(build_head(255_485_145_599, 255_485_145_600));

        assert_eq!(head.created, HeadTable::DATE_TIME_MAX_TICKS - 9_999_999);
        assert_eq!(head.modified, HeadTable::DATE_TIME_MAX_TICKS);

        // The earliest representable second and the one before it.
        let head = load(build_head(-60_052_752_000, -60_052_752_001));

        assert_eq!(head.created, 0);
        assert_eq!(head.modified, HeadTable::DATE_TIME_MIN_TICKS);
    }
}
