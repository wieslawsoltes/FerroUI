use crate::media::fonts::tables::big_endian_binary_reader::{BigEndianBinaryReader, FontTableError};

use super::composite_flags::CompositeFlags;
use super::glyph_component::GlyphComponent;

// Most composite glyphs have fewer than 8 components
const ESTIMATED_COMPONENT_COUNT: usize = 8;

/// A decoded composite glyph: a list of component glyph references.
#[derive(Clone, Debug, Default)]
pub struct CompositeGlyph<'a> {
    components: Vec<GlyphComponent>,
    instructions: &'a [u8],
    uses_point_matching: bool,
}

impl<'a> CompositeGlyph<'a> {
    pub fn components(&self) -> &[GlyphComponent] {
        &self.components
    }

    /// The glyph instructions (currently never read, always empty).
    pub fn instructions(&self) -> &'a [u8] {
        self.instructions
    }

    /// Whether at least one component is placed by point matching
    /// (`ARGS_ARE_XY_VALUES` clear).
    pub fn uses_point_matching(&self) -> bool {
        self.uses_point_matching
    }

    /// Decodes the glyph body that follows the 10 byte glyph header.
    ///
    /// `Err` when the component records run past the end of the data.
    pub fn create(data: &'a [u8]) -> Result<CompositeGlyph<'a>, FontTableError> {
        let mut components = Vec::with_capacity(ESTIMATED_COMPONENT_COUNT);
        let mut uses_point_matching = false;

        let mut reader = BigEndianBinaryReader::new(data);

        loop {
            // Read flags and glyph index
            let flags = CompositeFlags::from_bits_retain(reader.read_uint16()?);

            // When ARGS_ARE_XY_VALUES is clear the component is placed by point matching.
            if !flags.contains(CompositeFlags::ArgsAreXYValues) {
                uses_point_matching = true;
            }

            let glyph_index = reader.read_uint16()?;

            // Read arguments
            let (arg1, arg2) = if flags.contains(CompositeFlags::ArgsAreWords) {
                (reader.read_int16()?, reader.read_int16()?)
            } else {
                // Arguments are bytes
                (reader.read_sbyte()? as i16, reader.read_sbyte()? as i16)
            };

            // Optional transformation
            let mut scale = 1.0f32;
            let mut scale_x = 1.0f32;
            let mut scale_y = 1.0f32;
            let mut scale01 = 0.0f32;
            let mut scale10 = 0.0f32;

            if flags.contains(CompositeFlags::WeHaveAScale) {
                // Uniform scale
                scale = reader.read_f2dot14()?;
            } else if flags.contains(CompositeFlags::WeHaveAnXAndYScale) {
                // Separate x and y scales
                scale_x = reader.read_f2dot14()?;
                scale_y = reader.read_f2dot14()?;
            } else if flags.contains(CompositeFlags::WeHaveATwoByTwo) {
                // Two by two transformation matrix
                scale_x = reader.read_f2dot14()?;
                scale01 = reader.read_f2dot14()?;
                scale10 = reader.read_f2dot14()?;
                scale_y = reader.read_f2dot14()?;
            }

            components.push(GlyphComponent {
                flags,
                glyph_index,
                arg1,
                arg2,
                scale,
                scale_x,
                scale_y,
                scale01,
                scale10,
            });

            if !flags.contains(CompositeFlags::MoreComponents) {
                break;
            }
        }

        Ok(CompositeGlyph { components, instructions: &[], uses_point_matching })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_reads_components_until_the_last_one() {
        let mut data = Vec::new();

        // Component 0: word args, x/y offsets, uniform scale 0.5, more components.
        let flags = CompositeFlags::ArgsAreWords
            | CompositeFlags::ArgsAreXYValues
            | CompositeFlags::WeHaveAScale
            | CompositeFlags::MoreComponents;
        data.extend_from_slice(&flags.bits().to_be_bytes());
        data.extend_from_slice(&7u16.to_be_bytes());
        data.extend_from_slice(&(-300i16).to_be_bytes());
        data.extend_from_slice(&400i16.to_be_bytes());
        data.extend_from_slice(&0x2000i16.to_be_bytes());

        // Component 1: byte args, two by two matrix, last.
        let flags = CompositeFlags::ArgsAreXYValues | CompositeFlags::WeHaveATwoByTwo;
        data.extend_from_slice(&flags.bits().to_be_bytes());
        data.extend_from_slice(&9u16.to_be_bytes());
        data.extend_from_slice(&[0xFE, 3]);
        for value in [0x4000i16, 0x2000, -0x2000, 0x4000] {
            data.extend_from_slice(&value.to_be_bytes());
        }

        let glyph = CompositeGlyph::create(&data).unwrap();

        assert!(!glyph.uses_point_matching());
        assert!(glyph.instructions().is_empty());
        assert_eq!(glyph.components().len(), 2);

        let first = glyph.components()[0];
        assert_eq!(first.glyph_index, 7);
        assert_eq!((first.arg1, first.arg2), (-300, 400));
        assert_eq!(first.scale, 0.5);
        assert_eq!((first.scale_x, first.scale_y, first.scale01, first.scale10), (1.0, 1.0, 0.0, 0.0));

        let second = glyph.components()[1];
        assert_eq!(second.glyph_index, 9);
        assert_eq!((second.arg1, second.arg2), (-2, 3));
        assert_eq!(second.scale, 1.0);
        assert_eq!((second.scale_x, second.scale01, second.scale10, second.scale_y), (1.0, 0.5, -0.5, 1.0));
    }

    #[test]
    fn create_flags_point_matching() {
        // flags 0: byte args, point numbers, last component.
        let data = [0u8, 0, 0, 1, 2, 0];

        let glyph = CompositeGlyph::create(&data).unwrap();

        assert!(glyph.uses_point_matching());
        assert_eq!(glyph.components().len(), 1);
    }

    #[test]
    fn create_fails_on_truncated_components() {
        assert!(CompositeGlyph::create(&[]).is_err());
        // The MORE_COMPONENTS flag promises a second record that is missing.
        assert!(CompositeGlyph::create(&[0x00, 0x22, 0, 1, 0, 0]).is_err());
        // Missing argument bytes.
        assert!(CompositeGlyph::create(&[0x00, 0x03, 0, 1, 0]).is_err());
    }
}
