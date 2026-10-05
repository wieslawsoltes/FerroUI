use crate::media::fonts::OpenTypeTag;
use crate::media::IFontMemory;

use super::big_endian_binary_reader::{BigEndianBinaryReader, FontTableError};
use super::font_version::FontVersion;

/// The `vhea` (vertical header) table.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct VerticalHeaderTable {
    pub version: FontVersion,
    pub advance_height_max: u16,
    pub ascender: i16,
    pub caret_offset: i16,
    pub caret_slope_rise: i16,
    pub caret_slope_run: i16,
    pub descender: i16,
    pub line_gap: i16,
    pub min_top_side_bearing: i16,
    pub min_bottom_side_bearing: i16,
    pub number_of_v_metrics: u16,
    pub y_max_extent: i16,
}

impl VerticalHeaderTable {
    pub const TABLE_NAME: &'static str = "vhea";
    pub const TAG: OpenTypeTag = OpenTypeTag::from_chars('v', 'h', 'e', 'a');

    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        version: FontVersion,
        ascender: i16,
        descender: i16,
        line_gap: i16,
        advance_height_max: u16,
        min_top_side_bearing: i16,
        min_bottom_side_bearing: i16,
        y_max_extent: i16,
        caret_slope_rise: i16,
        caret_slope_run: i16,
        caret_offset: i16,
        number_of_v_metrics: u16,
    ) -> Self {
        Self {
            version,
            advance_height_max,
            ascender,
            caret_offset,
            caret_slope_rise,
            caret_slope_run,
            descender,
            line_gap,
            min_top_side_bearing,
            min_bottom_side_bearing,
            number_of_v_metrics,
            y_max_extent,
        }
    }

    /// Loads the table.
    ///
    /// `Ok(None)` when the font has no `vhea` table or its metric data format
    /// is not 0; `Err` when the table is present but too short.
    pub fn try_load(font: &dyn IFontMemory) -> Result<Option<VerticalHeaderTable>, FontTableError> {
        let Some(table) = font.try_get_table(Self::TAG) else {
            return Ok(None);
        };

        let mut binary_reader = BigEndianBinaryReader::new(table.span());

        Self::try_load_from_reader(&mut binary_reader)
    }

    fn try_load_from_reader(
        reader: &mut BigEndianBinaryReader<'_>,
    ) -> Result<Option<VerticalHeaderTable>, FontTableError> {
        // +--------+---------------------+---------------------------------------------------------------------------------+
        // | Type   | Name                | Description                                                                     |
        // +========+=====================+=================================================================================+
        // | Version16Dot16 | version     | 0x00010000 (1.0) or 0x00011000 (1.1)                                            |
        // | FWord  | ascent              | Distance from baseline of highest ascender                                      |
        // | FWord  | descent             | Distance from baseline of lowest descender                                      |
        // | FWord  | lineGap             | typographic line gap                                                            |
        // | uFWord | advanceHeightMax    | must be consistent with vertical metrics                                        |
        // | FWord  | minTopSideBearing   | must be consistent with vertical metrics                                        |
        // | FWord  | minBottomSideBearing | must be consistent with vertical metrics                                        |
        // | FWord  | yMaxExtent          | max(tsb + (yMax-yMin))                                                          |
        // | int16  | caretSlopeRise      | used to calculate the slope of the caret (rise/run) set to 1 for vertical caret |
        // | int16  | caretSlopeRun       | 0 for vertical                                                                  |
        // | FWord  | caretOffset         | set value to 0 for non-slanted fonts                                            |
        // | int16  | reserved (x4)       | set value to 0                                                                  |
        // | int16  | metricDataFormat    | 0 for current format                                                            |
        // | uint16 | numOfLongVerMetrics | number of advance heights in metrics table                                      |
        // +--------+---------------------+---------------------------------------------------------------------------------+
        let version = reader.read_version16_dot16()?;
        let ascender = reader.read_fword()?;
        let descender = reader.read_fword()?;
        let line_gap = reader.read_fword()?;
        let advance_height_max = reader.read_ufword()?;
        let min_top_side_bearing = reader.read_fword()?;
        let min_bottom_side_bearing = reader.read_fword()?;
        let y_max_extent = reader.read_fword()?;
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

        let number_of_v_metrics = reader.read_uint16()?;

        Ok(Some(VerticalHeaderTable::new(
            version,
            ascender,
            descender,
            line_gap,
            advance_height_max,
            min_top_side_bearing,
            min_bottom_side_bearing,
            y_max_extent,
            caret_slope_rise,
            caret_slope_run,
            caret_offset,
            number_of_v_metrics,
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
            .uint16(7000) // advanceHeightMax
            .int16(-20) // minTopSideBearing
            .int16(-30) // minBottomSideBearing
            .int16(6900) // yMaxExtent
            .int16(1) // caretSlopeRise
            .int16(2) // caretSlopeRun
            .int16(3) // caretOffset
            .zeros(8)
            .int16(metric_data_format)
            .uint16(2500); // numberOfVMetrics

        buffer.to_array()
    }

    #[test]
    fn loads_every_field() {
        let mut font = SyntheticFont::new();
        font.replace("vhea", build(0));

        let table = VerticalHeaderTable::try_load(&font).unwrap().unwrap();

        assert_eq!(
            table,
            VerticalHeaderTable::new(FontVersion::from_parts(1, 0), 2728, -680, 10, 7000, -20, -30, 6900, 1, 2, 3, 2500)
        );
        assert_eq!(table.number_of_v_metrics, 2500);
        assert_eq!(table.ascender, 2728);
        assert_eq!(table.descender, -680);
    }

    #[test]
    fn missing_table_or_unknown_metric_format_gives_none() {
        let mut font = SyntheticFont::new();

        assert_eq!(VerticalHeaderTable::try_load(&font), Ok(None));

        font.replace("vhea", build(1));

        assert_eq!(VerticalHeaderTable::try_load(&font), Ok(None));
    }

    #[test]
    fn truncated_table_is_an_error() {
        let mut font = SyntheticFont::new();
        font.replace("vhea", build(0));
        font.truncate("vhea", 35);

        assert!(VerticalHeaderTable::try_load(&font).unwrap_err().is_invalid_operation());
    }
}
