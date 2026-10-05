use crate::media::fonts::OpenTypeTag;
use crate::media::IFontMemory;

use super::big_endian_binary_reader::{BigEndianBinaryReader, FontTableError};
use super::encoding_id_extensions::Encoding;

const HEADER_SIZE: usize = 16;
const DATA_MAP_RECORD_SIZE: usize = 12;

// Tags whose data is a UTF-8 string of comma-separated ScriptLangTags (BCP 47 language identifiers).
const DLNG_TAG: OpenTypeTag = OpenTypeTag::from_chars('d', 'l', 'n', 'g');
const SLNG_TAG: OpenTypeTag = OpenTypeTag::from_chars('s', 'l', 'n', 'g');

/// The `meta` (metadata) table: the design and supported languages of a font.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct MetaTable {
    design_languages: Vec<String>,
    supported_languages: Vec<String>,
}

impl MetaTable {
    pub const TABLE_NAME: &'static str = "meta";
    pub const TAG: OpenTypeTag = OpenTypeTag::from_chars('m', 'e', 't', 'a');

    pub fn design_languages(&self) -> &[String] {
        &self.design_languages
    }

    pub fn supported_languages(&self) -> &[String] {
        &self.supported_languages
    }

    /// Loads the table; `None` when the font has no (valid) `meta` table.
    pub fn try_load(font: &dyn IFontMemory) -> Option<MetaTable> {
        let table = font.try_get_table(Self::TAG)?;

        Self::try_parse(table.span())
    }

    pub(crate) fn try_parse(span: &[u8]) -> Option<MetaTable> {
        // OpenType 'meta' layout:
        //   uint32 version
        //   uint32 flags
        //   uint32 reserved
        //   uint32 dataMapsCount
        //   DataMap[dataMapsCount]
        //
        // DataMap layout:
        //   Tag tag (4-byte packed value)
        //   Offset32 dataOffset (from start of this table)
        //   uint32 dataLength
        if span.len() < HEADER_SIZE {
            return None;
        }

        // The length checks above and below make every read succeed; a failed
        // read would still only reject the table.
        Self::parse(span).ok().flatten()
    }

    fn parse(span: &[u8]) -> Result<Option<MetaTable>, FontTableError> {
        let mut reader = BigEndianBinaryReader::new(span);

        let version = reader.read_uint32()?;

        if version != 1 {
            return Ok(None);
        }

        // flags + reserved
        reader.read_uint32()?;
        reader.read_uint32()?;

        let data_maps_count = reader.read_uint32()?;

        if data_maps_count == 0 {
            return Ok(Some(MetaTable::default()));
        }

        // Validate that the declared record array fully fits after the header.
        let max_data_maps_by_length = ((span.len() - HEADER_SIZE) / DATA_MAP_RECORD_SIZE) as u64;

        if data_maps_count as u64 > max_data_maps_by_length {
            return Ok(None);
        }

        let mut design_languages: Option<Vec<String>> = None;
        let mut supported_languages: Option<Vec<String>> = None;

        for _ in 0..data_maps_count {
            // Tag values are 4-byte OpenType identifiers (packed into a u32).
            let entry_tag = OpenTypeTag::new(reader.read_uint32()?);
            let data_offset = reader.read_uint32()? as u64;
            let data_length = reader.read_uint32()? as u64;

            if entry_tag != DLNG_TAG && entry_tag != SLNG_TAG {
                continue;
            }

            // Each data payload is referenced by (offset, length) from the start of this table.
            let span_length = span.len() as u64;

            if data_offset > span_length || data_length > span_length - data_offset {
                continue;
            }

            let data = &span[data_offset as usize..(data_offset + data_length) as usize];

            // Spec says only one instance is used; ignore subsequent duplicates.
            if entry_tag == DLNG_TAG {
                if design_languages.is_none() {
                    design_languages = Some(Self::parse_language_tags(data));
                }
            } else if supported_languages.is_none() {
                supported_languages = Some(Self::parse_language_tags(data));
            }
        }

        Ok(Some(MetaTable {
            design_languages: design_languages.unwrap_or_default(),
            supported_languages: supported_languages.unwrap_or_default(),
        }))
    }

    fn parse_language_tags(data: &[u8]) -> Vec<String> {
        // The data is UTF-8 text consisting of one or more ScriptLangTags separated by commas,
        // and spaces around separators are ignored.
        let text = Encoding::UTF8.get_string(data);

        text.split(',')
            .map(str::trim)
            .filter(|tag| !tag.is_empty())
            .map(str::to_string)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::fonts::tables::testing::SyntheticFont;

    #[test]
    fn try_parse_empty_span_returns_false() {
        assert!(MetaTable::try_parse(&[]).is_none());
    }

    #[test]
    fn try_parse_wrong_version_returns_false() {
        let mut bytes = vec![0u8; 16];
        // version 2 is not supported
        bytes[0..4].copy_from_slice(&2u32.to_be_bytes());

        assert!(MetaTable::try_parse(&bytes).is_none());
    }

    #[test]
    fn try_parse_reads_dlng_and_slng_tags() {
        let bytes = build_meta_table(&[("dlng", "ja,zh-Hant"), ("slng", "en")]);

        let table = MetaTable::try_parse(&bytes).unwrap();

        assert_eq!(table.design_languages(), ["ja", "zh-Hant"]);
        assert_eq!(table.supported_languages(), ["en"]);
    }

    #[test]
    fn try_parse_trims_whitespace_and_skips_empty_tags() {
        let bytes = build_meta_table(&[("dlng", " ja , , zh-Hant "), ("slng", "en-US")]);

        let table = MetaTable::try_parse(&bytes).unwrap();

        assert_eq!(table.design_languages(), ["ja", "zh-Hant"]);
        assert_eq!(table.supported_languages(), ["en-US"]);
    }

    #[test]
    fn try_parse_returns_empty_arrays_when_only_unknown_maps() {
        // A meta table with a single data map using an unknown tag.
        let bytes = build_meta_table(&[("xxxx", "ignored")]);

        let table = MetaTable::try_parse(&bytes).unwrap();

        assert!(table.design_languages().is_empty());
        assert!(table.supported_languages().is_empty());
    }

    #[test]
    fn try_parse_truncated_data_map_array_returns_false() {
        let mut bytes = vec![0u8; 16];

        // Header claims one data map but no data map records follow.
        bytes[0..4].copy_from_slice(&1u32.to_be_bytes()); // version
        bytes[12..16].copy_from_slice(&1u32.to_be_bytes()); // dataMapsCount

        assert!(MetaTable::try_parse(&bytes).is_none());
    }

    #[test]
    fn try_parse_duplicate_dlng_uses_first_record() {
        let bytes = build_meta_table(&[("dlng", "ja"), ("dlng", "en"), ("slng", "en")]);

        let table = MetaTable::try_parse(&bytes).unwrap();

        assert_eq!(table.design_languages(), ["ja"]);
        assert_eq!(table.supported_languages(), ["en"]);
    }

    #[test]
    fn try_parse_skips_payloads_outside_of_the_table() {
        let mut bytes = build_meta_table(&[("dlng", "ja"), ("slng", "en")]);

        // Point the dlng payload past the end of the table.
        bytes[20..24].copy_from_slice(&u32::MAX.to_be_bytes());

        let table = MetaTable::try_parse(&bytes).unwrap();

        assert!(table.design_languages().is_empty());
        assert_eq!(table.supported_languages(), ["en"]);
    }

    #[test]
    fn try_load_reads_the_table_from_the_font() {
        let mut font = SyntheticFont::new();

        assert!(MetaTable::try_load(&font).is_none());

        font.replace("meta", build_meta_table(&[("dlng", "ja"), ("slng", "en")]));

        assert_eq!(MetaTable::try_load(&font).unwrap().design_languages(), ["ja"]);
    }

    fn build_meta_table(maps: &[(&str, &str)]) -> Vec<u8> {
        const HEADER: usize = 16;
        const MAP_SIZE: usize = 12;

        let mut payload_offset = HEADER + MAP_SIZE * maps.len();
        let total_length = payload_offset + maps.iter().map(|(_, value)| value.len()).sum::<usize>();

        let mut bytes = vec![0u8; total_length];

        // Header
        bytes[0..4].copy_from_slice(&1u32.to_be_bytes()); // version
        bytes[12..16].copy_from_slice(&(maps.len() as u32).to_be_bytes()); // dataMapsCount

        for (i, (tag, value)) in maps.iter().enumerate() {
            let map_offset = HEADER + i * MAP_SIZE;

            bytes[map_offset..map_offset + 4].copy_from_slice(tag.as_bytes());
            bytes[map_offset + 4..map_offset + 8].copy_from_slice(&(payload_offset as u32).to_be_bytes());
            bytes[map_offset + 8..map_offset + 12].copy_from_slice(&(value.len() as u32).to_be_bytes());
            bytes[payload_offset..payload_offset + value.len()].copy_from_slice(value.as_bytes());

            payload_offset += value.len();
        }

        bytes
    }
}
