// Derived from SixLabors.Fonts (Apache License 2.0), see NOTICE.md.

use crate::media::fonts::OpenTypeTag;
use crate::media::IFontMemory;

use super::big_endian_binary_reader::{BigEndianBinaryReader, FontTableError};
use super::font_version::FontVersion;

/// The `hhea` (horizontal header) table.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct HorizontalHeaderTable {
    pub version: FontVersion,
    pub advance_width_max: u16,
    pub ascender: i16,
    pub caret_offset: i16,
    pub caret_slope_rise: i16,
    pub caret_slope_run: i16,
    pub descender: i16,
    pub line_gap: i16,
    pub min_left_side_bearing: i16,
    pub min_right_side_bearing: i16,
    pub number_of_h_metrics: u16,
    pub x_max_extent: i16,
}

impl HorizontalHeaderTable {
    pub const TABLE_NAME: &'static str = "hhea";
    pub const TAG: OpenTypeTag = OpenTypeTag::from_chars('h', 'h', 'e', 'a');

    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        version: FontVersion,
        ascender: i16,
        descender: i16,
        line_gap: i16,
        advance_width_max: u16,
        min_left_side_bearing: i16,
        min_right_side_bearing: i16,
        x_max_extent: i16,
        caret_slope_rise: i16,
        caret_slope_run: i16,
        caret_offset: i16,
        number_of_h_metrics: u16,
    ) -> Self {
        Self {
            version,
            advance_width_max,
            ascender,
            caret_offset,
            caret_slope_rise,
            caret_slope_run,
            descender,
            line_gap,
            min_left_side_bearing,
            min_right_side_bearing,
            number_of_h_metrics,
            x_max_extent,
        }
    }

    /// Loads the table.
    ///
    /// `Ok(None)` when the font has no `hhea` table or its metric data format
    /// is not 0; `Err` when the table is present but too short.
    pub fn try_load(font: &dyn IFontMemory) -> Result<Option<HorizontalHeaderTable>, FontTableError> {
        let Some(table) = font.try_get_table(Self::TAG) else {
            return Ok(None);
        };

        let mut binary_reader = BigEndianBinaryReader::new(table.span());

        Self::try_load_from_reader(&mut binary_reader)
    }

    fn try_load_from_reader(
        reader: &mut BigEndianBinaryReader<'_>,
    ) -> Result<Option<HorizontalHeaderTable>, FontTableError> {
        // +--------+---------------------+---------------------------------------------------------------------------------+
        // | Type   | Name                | Description                                                                     |
        // +========+=====================+=================================================================================+
        // | Version16Dot16 | version     | 0x00010000 (1.0)                                                                |
        // | FWord  | ascent              | Distance from baseline of highest ascender                                      |
        // | FWord  | descent             | Distance from baseline of lowest descender                                      |
        // | FWord  | lineGap             | typographic line gap                                                            |
        // | uFWord | advanceWidthMax     | must be consistent with horizontal metrics                                      |
        // | FWord  | minLeftSideBearing  | must be consistent with horizontal metrics                                      |
        // | FWord  | minRightSideBearing | must be consistent with horizontal metrics                                      |
        // | FWord  | xMaxExtent          | max(lsb + (xMax-xMin))                                                          |
        // | int16  | caretSlopeRise      | used to calculate the slope of the caret (rise/run) set to 1 for vertical caret |
        // | int16  | caretSlopeRun       | 0 for vertical                                                                  |
        // | FWord  | caretOffset         | set value to 0 for non-slanted fonts                                            |
        // | int16  | reserved (x4)       | set value to 0                                                                  |
        // | int16  | metricDataFormat    | 0 for current format                                                            |
        // | uint16 | numOfLongHorMetrics | number of advance widths in metrics table                                       |
        // +--------+---------------------+---------------------------------------------------------------------------------+
        let version = reader.read_version16_dot16()?;
        let ascender = reader.read_fword()?;
        let descender = reader.read_fword()?;
        let line_gap = reader.read_fword()?;
        let advance_width_max = reader.read_ufword()?;
        let min_left_side_bearing = reader.read_fword()?;
        let min_right_side_bearing = reader.read_fword()?;
        let x_max_extent = reader.read_fword()?;
        let caret_slope_rise = reader.read_int16()?;
        let caret_slope_run = reader.read_int16()?;
        let caret_offset = reader.read_int16()?;
        reader.read_int16()?; // reserved
        reader.read_int16()?; // reserved
        reader.read_int16()?; // reserved
        reader.read_int16()?; // reserved
        let metric_data_format = reader.read_int16()?; // 0

        if metric_data_format != 0 {
            return Ok(None);
        }

        let number_of_h_metrics = reader.read_uint16()?;

        Ok(Some(HorizontalHeaderTable::new(
            version,
            ascender,
            descender,
            line_gap,
            advance_width_max,
            min_left_side_bearing,
            min_right_side_bearing,
            x_max_extent,
            caret_slope_rise,
            caret_slope_run,
            caret_offset,
            number_of_h_metrics,
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::fonts::tables::testing::{BigEndianBuffer, SyntheticFont};

    fn build(metric_data_format: i32) -> Vec<u8> {
        let mut buffer = BigEndianBuffer::new();

        buffer
            .uint32(0x0001_0000)
            .int16(2728) // ascender
            .int16(-680) // descender
            .int16(10) // lineGap
            .uint16(7000) // advanceWidthMax
            .int16(-20) // minLeftSideBearing
            .int16(-30) // minRightSideBearing
            .int16(6900) // xMaxExtent
            .int16(1) // caretSlopeRise
            .int16(2) // caretSlopeRun
            .int16(3) // caretOffset
            .zeros(8)
            .int16(metric_data_format)
            .uint16(2500); // numberOfHMetrics

        buffer.to_array()
    }

    #[test]
    fn loads_every_field() {
        let mut font = SyntheticFont::new();
        font.replace("hhea", build(0));

        let table = HorizontalHeaderTable::try_load(&font).unwrap().unwrap();

        assert_eq!(
            table,
            HorizontalHeaderTable::new(FontVersion::from_parts(1, 0), 2728, -680, 10, 7000, -20, -30, 6900, 1, 2, 3, 2500)
        );
        assert_eq!(table.number_of_h_metrics, 2500);
        assert_eq!(table.ascender, 2728);
        assert_eq!(table.descender, -680);
    }

    #[test]
    fn missing_table_or_unknown_metric_format_gives_none() {
        let mut font = SyntheticFont::new();

        assert_eq!(HorizontalHeaderTable::try_load(&font), Ok(None));

        font.replace("hhea", build(1));

        assert_eq!(HorizontalHeaderTable::try_load(&font), Ok(None));
    }

    #[test]
    fn truncated_table_is_an_error() {
        let mut font = SyntheticFont::new();
        font.replace("hhea", build(0));
        font.truncate("hhea", 35);

        assert!(HorizontalHeaderTable::try_load(&font).unwrap_err().is_invalid_operation());
    }
}
