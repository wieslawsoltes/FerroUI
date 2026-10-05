use crate::media::fonts::tables::big_endian_binary_reader::{BigEndianBinaryReader, FontTableError};
use crate::utilities::ReadOnlyMemory;
use crate::Rect;

use super::composite_glyph::CompositeGlyph;
use super::simple_glyph::SimpleGlyph;

/// The header of a glyph in the `glyf` table plus access to its outline.
#[derive(Clone, Debug)]
pub struct GlyphDescriptor {
    glyph_data: ReadOnlyMemory<u8>,
    number_of_contours: i16,
    conservative_bounds: Rect,
}

impl GlyphDescriptor {
    /// Reads the 10 byte glyph header; `Err` when the data is shorter.
    pub fn new(data: ReadOnlyMemory<u8>) -> Result<Self, FontTableError> {
        let mut reader = BigEndianBinaryReader::new(data.span());

        let number_of_contours = reader.read_int16()?;

        let x_min = reader.read_int16()?;
        let y_min = reader.read_int16()?;
        let x_max = reader.read_int16()?;
        let y_max = reader.read_int16()?;

        // Store as Rect - note: coordinates are in font design units
        let conservative_bounds = Rect::new(
            x_min as f64,
            y_min as f64,
            (x_max as i32 - x_min as i32) as f64,
            (y_max as i32 - y_min as i32) as f64,
        );

        Ok(Self { glyph_data: data.slice_from(10), number_of_contours, conservative_bounds })
    }

    pub fn number_of_contours(&self) -> i16 {
        self.number_of_contours
    }

    pub fn conservative_bounds(&self) -> Rect {
        self.conservative_bounds
    }

    pub fn is_simple_glyph(&self) -> bool {
        self.number_of_contours >= 0
    }

    pub fn simple_glyph(&self) -> SimpleGlyph<'_> {
        SimpleGlyph::create(self.glyph_data.span(), self.number_of_contours as i32)
    }

    pub fn composite_glyph(&self) -> Result<CompositeGlyph<'_>, FontTableError> {
        CompositeGlyph::create(self.glyph_data.span())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_header() {
        let mut data = Vec::new();
        for value in [2i16, -10, -20, 110, 220] {
            data.extend_from_slice(&value.to_be_bytes());
        }

        let descriptor = GlyphDescriptor::new(ReadOnlyMemory::from_vec(data)).unwrap();

        assert_eq!(descriptor.number_of_contours(), 2);
        assert!(descriptor.is_simple_glyph());
        assert_eq!(descriptor.conservative_bounds(), Rect::new(-10.0, -20.0, 120.0, 240.0));
        // The body is empty, so the outline is too.
        assert!(descriptor.simple_glyph().flags().is_empty());
        assert!(descriptor.composite_glyph().is_err());
    }

    #[test]
    fn negative_contour_count_marks_a_composite() {
        let mut data = Vec::new();
        for value in [-1i16, 0, 0, 0, 0] {
            data.extend_from_slice(&value.to_be_bytes());
        }
        data.extend_from_slice(&[0x00, 0x02, 0, 5, 1, 2]);

        let descriptor = GlyphDescriptor::new(ReadOnlyMemory::from_vec(data)).unwrap();

        assert!(!descriptor.is_simple_glyph());
        assert_eq!(descriptor.composite_glyph().unwrap().components()[0].glyph_index, 5);
    }

    #[test]
    fn short_data_is_an_error() {
        assert!(GlyphDescriptor::new(ReadOnlyMemory::from_vec(vec![0u8; 9])).is_err());
    }
}
