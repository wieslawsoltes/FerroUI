use crate::media::fonts::OpenTypeTag;
use crate::media::IFontMemory;

use super::big_endian_binary_reader::{BigEndianBinaryReader, FontTableError};
use super::font_version::FontVersion;

/// The `maxp` (maximum profile) table.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct MaxpTable {
    pub version: FontVersion,
    pub num_glyphs: u16,
    pub max_points: u16,
    pub max_contours: u16,
    pub max_composite_points: u16,
    pub max_composite_contours: u16,
    pub max_zones: u16,
    pub max_twilight_points: u16,
    pub max_storage: u16,
    pub max_function_defs: u16,
    pub max_instruction_defs: u16,
    pub max_stack_elements: u16,
    pub max_size_of_instructions: u16,
    pub max_component_elements: u16,
    pub max_component_depth: u16,
}

impl MaxpTable {
    pub const TABLE_NAME: &'static str = "maxp";
    pub const TAG: OpenTypeTag = OpenTypeTag::from_chars('m', 'a', 'x', 'p');

    /// Loads the table.
    ///
    /// `Err` when the font has no `maxp` table or the table is too short.
    pub fn load(font: &dyn IFontMemory) -> Result<MaxpTable, FontTableError> {
        let Some(table) = font.try_get_table(Self::TAG) else {
            return Err(FontTableError::InvalidOperation(format!(
                "Could not load the '{}' table.",
                Self::TABLE_NAME
            )));
        };

        let mut binary_reader = BigEndianBinaryReader::new(table.span());

        Self::load_from_reader(&mut binary_reader)
    }

    fn load_from_reader(reader: &mut BigEndianBinaryReader<'_>) -> Result<MaxpTable, FontTableError> {
        // Version 0.5 (CFF/CFF2 fonts):
        // | Version16Dot16 | version   | 0x00005000 for version 0.5      |
        // | uint16         | numGlyphs | The number of glyphs in the font|

        // Version 1.0 (TrueType fonts):
        // | Version16Dot16 | version                | 0x00010000 for version 1.0                          |
        // | uint16         | numGlyphs              | The number of glyphs in the font                    |
        // | uint16         | maxPoints              | Maximum points in a non-composite glyph             |
        // | uint16         | maxContours            | Maximum contours in a non-composite glyph           |
        // | uint16         | maxCompositePoints     | Maximum points in a composite glyph                 |
        // | uint16         | maxCompositeContours   | Maximum contours in a composite glyph               |
        // | uint16         | maxZones               | 1 or 2; should be set to 2 in most cases            |
        // | uint16         | maxTwilightPoints      | Maximum points used in Z0                           |
        // | uint16         | maxStorage             | Number of Storage Area locations                    |
        // | uint16         | maxFunctionDefs        | Number of FDEFs                                     |
        // | uint16         | maxInstructionDefs     | Number of IDEFs                                     |
        // | uint16         | maxStackElements       | Maximum stack depth                                 |
        // | uint16         | maxSizeOfInstructions  | Maximum byte count for glyph instructions           |
        // | uint16         | maxComponentElements   | Maximum number of components at top level           |
        // | uint16         | maxComponentDepth      | Maximum levels of recursion                         |

        let version = reader.read_version16_dot16()?;
        let num_glyphs = reader.read_uint16()?;

        if version.major < 1 {
            return Ok(MaxpTable { version, num_glyphs, ..MaxpTable::default() });
        }

        Ok(MaxpTable {
            version,
            num_glyphs,
            max_points: reader.read_uint16()?,
            max_contours: reader.read_uint16()?,
            max_composite_points: reader.read_uint16()?,
            max_composite_contours: reader.read_uint16()?,
            max_zones: reader.read_uint16()?,
            max_twilight_points: reader.read_uint16()?,
            max_storage: reader.read_uint16()?,
            max_function_defs: reader.read_uint16()?,
            max_instruction_defs: reader.read_uint16()?,
            max_stack_elements: reader.read_uint16()?,
            max_size_of_instructions: reader.read_uint16()?,
            max_component_elements: reader.read_uint16()?,
            max_component_depth: reader.read_uint16()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::fonts::tables::testing::{BigEndianBuffer, SyntheticFont};

    /// A version 1.0 `maxp` with the values the reference tests pin for their
    /// static TrueType test font.
    fn build_version_1() -> Vec<u8> {
        let mut buffer = BigEndianBuffer::new();

        buffer.uint32(0x0001_0000).uint16(2547);

        for value in [148, 12, 112, 7, 1, 0, 0, 0, 0, 0, 0, 5, 1] {
            buffer.uint16(value);
        }

        buffer.to_array()
    }

    #[test]
    fn missing_table_is_an_error() {
        let error = MaxpTable::load(&SyntheticFont::new()).unwrap_err();

        assert!(error.is_invalid_operation());
        assert_eq!(error.to_string(), "Could not load the 'maxp' table.");
    }

    #[test]
    fn loads_version_1_0() {
        let mut font = SyntheticFont::new();
        font.replace("maxp", build_version_1());

        let maxp = MaxpTable::load(&font).unwrap();

        assert_ne!(maxp, MaxpTable::default());
        assert_eq!(maxp.version.major, 1);
        assert_eq!(maxp.version.minor, 0);
        assert_eq!(maxp.num_glyphs, 2547);
        assert_eq!(maxp.max_points, 148);
        assert_eq!(maxp.max_contours, 12);
        assert_eq!(maxp.max_composite_points, 112);
        assert_eq!(maxp.max_composite_contours, 7);
        assert_eq!(maxp.max_zones, 1);
        assert_eq!(maxp.max_stack_elements, 0);
        assert_eq!(maxp.max_component_elements, 5);
        assert_eq!(maxp.max_component_depth, 1);
    }

    #[test]
    fn loads_version_0_5_without_the_truetype_fields() {
        let mut buffer = BigEndianBuffer::new();
        buffer.uint32(0x0000_5000).uint16(42);

        let mut font = SyntheticFont::new();
        font.replace("maxp", buffer.to_array());

        let maxp = MaxpTable::load(&font).unwrap();

        assert_eq!(maxp.version, FontVersion::from_parts(0, 0x5000));
        assert_eq!(maxp.num_glyphs, 42);
        assert_eq!(maxp.max_points, 0);
        assert_eq!(maxp.max_component_depth, 0);
    }

    #[test]
    fn truncated_version_1_0_is_an_error() {
        let mut font = SyntheticFont::new();
        font.replace("maxp", build_version_1());
        font.truncate("maxp", 31);

        assert_eq!(MaxpTable::load(&font), Err(FontTableError::EndOfSpan { missing: 1 }));
    }
}
