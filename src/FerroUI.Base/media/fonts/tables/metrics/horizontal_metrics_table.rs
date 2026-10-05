use crate::media::fonts::tables::big_endian_binary_reader::{BigEndianBinaryReader, FontTableError};
use crate::media::fonts::OpenTypeTag;
use crate::media::IFontMemory;
use crate::utilities::ReadOnlyMemory;

use super::horizontal_glyph_metric::HorizontalGlyphMetric;

/// The `hmtx` (horizontal metrics) table. Metrics are read lazily from the
/// table data; lookups never allocate.
///
/// The lookups return `Err` when the table is shorter than the header counts
/// promise (the reference implementation throws there).
#[derive(Clone, Debug)]
pub struct HorizontalMetricsTable {
    data: ReadOnlyMemory<u8>,
    num_of_h_metrics: u16,
    num_glyphs: i32,
}

impl HorizontalMetricsTable {
    pub const TAG_NAME: &'static str = "hmtx";
    pub const TAG: OpenTypeTag = OpenTypeTag::from_chars('h', 'm', 't', 'x');

    pub(crate) fn new(data: ReadOnlyMemory<u8>, num_of_h_metrics: u16, num_glyphs: i32) -> Self {
        Self { data, num_of_h_metrics, num_glyphs }
    }

    /// Loads the table; `None` when the font has no `hmtx` table.
    pub fn load(font: &dyn IFontMemory, number_of_h_metrics: u16, glyph_count: i32) -> Option<HorizontalMetricsTable> {
        let table = font.try_get_table(Self::TAG)?;

        Some(HorizontalMetricsTable::new(table, number_of_h_metrics, glyph_count))
    }

    /// The metrics of a glyph; `Ok(None)` when the glyph index is out of range.
    pub fn try_get_metrics(&self, glyph_index: u16) -> Result<Option<HorizontalGlyphMetric>, FontTableError> {
        if glyph_index as i32 >= self.num_glyphs {
            return Ok(None);
        }

        let mut reader = BigEndianBinaryReader::new(self.data.span());

        if glyph_index < self.num_of_h_metrics {
            reader.seek(glyph_index as i32 * 4)?;

            let advance_width = reader.read_uint16()?;
            let left_side_bearing = reader.read_int16()?;

            Ok(Some(HorizontalGlyphMetric::new(advance_width, left_side_bearing)))
        } else {
            reader.seek((self.num_of_h_metrics as i32 - 1) * 4)?;

            let last_advance_width = reader.read_uint16()?;

            let lsb_index = glyph_index as i32 - self.num_of_h_metrics as i32;
            let lsb_offset = self.num_of_h_metrics as i32 * 4 + lsb_index * 2;

            reader.seek(lsb_offset)?;

            let left_side_bearing = reader.read_int16()?;

            Ok(Some(HorizontalGlyphMetric::new(last_advance_width, left_side_bearing)))
        }
    }

    /// The advance of a glyph; `Ok(None)` when the glyph index is out of range.
    pub fn try_get_advance(&self, glyph_index: u16) -> Result<Option<u16>, FontTableError> {
        if glyph_index as i32 >= self.num_glyphs {
            return Ok(None);
        }

        let mut reader = BigEndianBinaryReader::new(self.data.span());

        if glyph_index < self.num_of_h_metrics {
            reader.seek(glyph_index as i32 * 4)?;
        } else {
            reader.seek((self.num_of_h_metrics as i32 - 1) * 4)?;
        }

        Ok(Some(reader.read_uint16()?))
    }

    /// Reads the advances of a batch of glyphs.
    ///
    /// `Ok(false)` when `advances` is shorter than `glyph_indices` or a glyph
    /// index is out of range (the advances before it are already written).
    pub fn try_get_advances(&self, glyph_indices: &[u16], advances: &mut [u16]) -> Result<bool, FontTableError> {
        if advances.len() < glyph_indices.len() {
            return Ok(false);
        }

        let mut reader = BigEndianBinaryReader::new(self.data.span());

        // Cache the last advance width for glyphs beyond numOfHMetrics
        let mut last_advance_width: Option<u16> = None;

        for (i, &glyph_index) in glyph_indices.iter().enumerate() {
            if glyph_index as i32 >= self.num_glyphs {
                return Ok(false);
            }

            if glyph_index < self.num_of_h_metrics {
                reader.seek(glyph_index as i32 * 4)?;
                advances[i] = reader.read_uint16()?;
            } else {
                // All glyphs beyond numOfHMetrics share the same advance width
                let advance = match last_advance_width {
                    Some(advance) => advance,
                    None => {
                        reader.seek((self.num_of_h_metrics as i32 - 1) * 4)?;
                        let advance = reader.read_uint16()?;
                        last_advance_width = Some(advance);
                        advance
                    }
                };

                advances[i] = advance;
            }
        }

        Ok(true)
    }

    /// Reads the metrics of a batch of glyphs.
    ///
    /// `Ok(false)` when `metrics` is shorter than `glyph_indices` or a glyph
    /// index is out of range (the metrics before it are already written).
    pub fn try_get_metrics_batch(
        &self,
        glyph_indices: &[u16],
        metrics: &mut [HorizontalGlyphMetric],
    ) -> Result<bool, FontTableError> {
        if metrics.len() < glyph_indices.len() {
            return Ok(false);
        }

        let mut reader = BigEndianBinaryReader::new(self.data.span());

        // Cache the last advance width for glyphs beyond numOfHMetrics
        let mut last_advance_width: Option<u16> = None;

        for (i, &glyph_index) in glyph_indices.iter().enumerate() {
            if glyph_index as i32 >= self.num_glyphs {
                return Ok(false);
            }

            if glyph_index < self.num_of_h_metrics {
                reader.seek(glyph_index as i32 * 4)?;

                let advance_width = reader.read_uint16()?;
                let left_side_bearing = reader.read_int16()?;

                metrics[i] = HorizontalGlyphMetric::new(advance_width, left_side_bearing);
            } else {
                // All glyphs beyond numOfHMetrics share the same advance width
                let advance_width = match last_advance_width {
                    Some(advance) => advance,
                    None => {
                        reader.seek((self.num_of_h_metrics as i32 - 1) * 4)?;
                        let advance = reader.read_uint16()?;
                        last_advance_width = Some(advance);
                        advance
                    }
                };

                let lsb_index = glyph_index as i32 - self.num_of_h_metrics as i32;
                let lsb_offset = self.num_of_h_metrics as i32 * 4 + lsb_index * 2;

                reader.seek(lsb_offset)?;
                let left_side_bearing = reader.read_int16()?;

                metrics[i] = HorizontalGlyphMetric::new(advance_width, left_side_bearing);
            }
        }

        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::fonts::tables::testing::{BigEndianBuffer, SyntheticFont};

    /// Two long metrics (500/10, 600/-20) and two trailing side bearings (30, -40).
    fn build() -> Vec<u8> {
        let mut buffer = BigEndianBuffer::new();

        buffer.uint16(500).int16(10).uint16(600).int16(-20).int16(30).int16(-40);

        buffer.to_array()
    }

    fn load(num_glyphs: i32) -> HorizontalMetricsTable {
        let mut font = SyntheticFont::new();
        font.replace("hmtx", build());

        HorizontalMetricsTable::load(&font, 2, num_glyphs).unwrap()
    }

    #[test]
    fn missing_table_gives_none() {
        assert!(HorizontalMetricsTable::load(&SyntheticFont::new(), 1, 1).is_none());
    }

    #[test]
    fn single_lookups_cover_long_metrics_and_trailing_bearings() {
        let table = load(4);

        assert_eq!(table.try_get_metrics(0), Ok(Some(HorizontalGlyphMetric::new(500, 10))));
        assert_eq!(table.try_get_metrics(1), Ok(Some(HorizontalGlyphMetric::new(600, -20))));
        assert_eq!(table.try_get_metrics(2), Ok(Some(HorizontalGlyphMetric::new(600, 30))));
        assert_eq!(table.try_get_metrics(3), Ok(Some(HorizontalGlyphMetric::new(600, -40))));
        assert_eq!(table.try_get_metrics(4), Ok(None));

        assert_eq!(table.try_get_advance(0), Ok(Some(500)));
        assert_eq!(table.try_get_advance(3), Ok(Some(600)));
        assert_eq!(table.try_get_advance(4), Ok(None));

        assert_eq!(HorizontalGlyphMetric::new(500, -3).to_string(), "Advance=500, LSB=-3");
    }

    #[test]
    fn batch_lookups_match_single_lookups() {
        let table = load(4);

        let mut advances = [0u16; 4];
        assert_eq!(table.try_get_advances(&[3, 0, 2, 1], &mut advances), Ok(true));
        assert_eq!(advances, [600, 500, 600, 600]);

        let mut metrics = [HorizontalGlyphMetric::default(); 4];
        assert_eq!(table.try_get_metrics_batch(&[3, 0, 2, 1], &mut metrics), Ok(true));
        assert_eq!(
            metrics,
            [
                HorizontalGlyphMetric::new(600, -40),
                HorizontalGlyphMetric::new(500, 10),
                HorizontalGlyphMetric::new(600, 30),
                HorizontalGlyphMetric::new(600, -20),
            ]
        );
    }

    #[test]
    fn batch_lookups_reject_short_outputs_and_out_of_range_glyphs() {
        let table = load(4);

        let mut advances = [0u16; 1];
        assert_eq!(table.try_get_advances(&[0, 1], &mut advances), Ok(false));

        let mut advances = [0u16; 2];
        assert_eq!(table.try_get_advances(&[1, 4], &mut advances), Ok(false));
        assert_eq!(advances[0], 600);

        let mut metrics = [HorizontalGlyphMetric::default(); 2];
        assert_eq!(table.try_get_metrics_batch(&[4, 0], &mut metrics), Ok(false));
        assert_eq!(table.try_get_metrics_batch(&[0, 1, 2], &mut metrics), Ok(false));
    }

    #[test]
    fn truncated_table_reports_errors_instead_of_panicking() {
        // The header counts promise more glyphs than the data covers.
        let table = load(100);

        assert!(table.try_get_metrics(4).unwrap_err().is_invalid_operation());
        assert!(table.try_get_metrics(50).unwrap_err().is_argument_out_of_range());
        assert_eq!(table.try_get_advance(50), Ok(Some(600)));

        let mut metrics = [HorizontalGlyphMetric::default(); 2];
        assert!(table.try_get_metrics_batch(&[0, 50], &mut metrics).is_err());

        // No long metrics at all: the "last advance" lookup seeks before the table.
        let empty = HorizontalMetricsTable::new(ReadOnlyMemory::from_vec(Vec::new()), 0, 10);

        assert!(empty.try_get_advance(0).unwrap_err().is_argument_out_of_range());
        assert!(empty.try_get_metrics(0).is_err());

        let mut advances = [0u16; 1];
        assert!(empty.try_get_advances(&[0], &mut advances).is_err());
    }
}
