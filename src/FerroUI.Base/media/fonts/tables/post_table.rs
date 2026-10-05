use crate::media::fonts::OpenTypeTag;
use crate::media::IFontMemory;

use super::big_endian_binary_reader::{BigEndianBinaryReader, FontTableError};
use super::font_version::FontVersion;

/// The `post` (PostScript) table header.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct PostTable {
    pub version: FontVersion,
    pub italic_angle: f32,
    pub underline_position: i16,
    pub underline_thickness: i16,
    pub is_fixed_pitch: bool,
}

impl PostTable {
    pub const TABLE_NAME: &'static str = "post";
    pub const TAG: OpenTypeTag = OpenTypeTag::from_chars('p', 'o', 's', 't');

    /// Loads the table; an absent or malformed table gives the default value.
    pub fn load(font: &dyn IFontMemory) -> PostTable {
        let Some(table) = font.try_get_table(Self::TAG) else {
            return PostTable::default();
        };

        let mut binary_reader = BigEndianBinaryReader::new(table.span());

        // 'post' only carries cosmetic hints (underline metrics, italic angle, fixed-pitch
        // flag), so a present-but-malformed table must degrade to defaults rather than deny the
        // whole font — the same outcome as an absent 'post'.
        Self::load_from_reader(&mut binary_reader).unwrap_or_default()
    }

    fn load_from_reader(reader: &mut BigEndianBinaryReader<'_>) -> Result<PostTable, FontTableError> {
        let version = reader.read_version16_dot16()?;
        let italic_angle = reader.read_fixed()?;
        let underline_position = reader.read_fword()?;
        let underline_thickness = reader.read_fword()?;
        let is_fixed_pitch = reader.read_uint32()?;

        Ok(PostTable {
            version,
            italic_angle,
            underline_position,
            underline_thickness,
            is_fixed_pitch: is_fixed_pitch != 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::fonts::tables::testing::{BigEndianBuffer, SyntheticFont};

    fn build() -> Vec<u8> {
        let mut buffer = BigEndianBuffer::new();

        buffer.uint32(0x0003_0000).fixed(-9.5).int16(-200).int16(140).uint32(1);

        buffer.to_array()
    }

    #[test]
    fn loads_every_field() {
        let mut font = SyntheticFont::new();
        font.replace("post", build());

        let post = PostTable::load(&font);

        assert_eq!(post.version, FontVersion::from_parts(3, 0));
        assert_eq!(post.italic_angle, -9.5);
        assert_eq!(post.underline_position, -200);
        assert_eq!(post.underline_thickness, 140);
        assert!(post.is_fixed_pitch);
    }

    #[test]
    fn missing_post_table_gives_defaults() {
        assert_eq!(PostTable::load(&SyntheticFont::new()), PostTable::default());
    }

    #[test]
    fn truncated_post_table_degrades_to_defaults() {
        // Keep only the 4-byte version field; reading the rest of the header over-runs.
        let mut font = SyntheticFont::new();
        font.replace("post", build());
        font.truncate("post", 4);

        assert_eq!(PostTable::load(&font), PostTable::default());
    }
}
