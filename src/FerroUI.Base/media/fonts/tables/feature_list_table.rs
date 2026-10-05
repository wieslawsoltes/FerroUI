// Derived from SixLabors.Fonts (Apache License 2.0), see NOTICE.md.

use crate::media::fonts::OpenTypeTag;
use crate::media::IFontMemory;

use super::big_endian_binary_reader::{BigEndianBinaryReader, FontTableError};

const GSUB_TAG: OpenTypeTag = OpenTypeTag::from_chars('G', 'S', 'U', 'B');
const GPOS_TAG: OpenTypeTag = OpenTypeTag::from_chars('G', 'P', 'O', 'S');

/// The feature tags of a `GSUB` or `GPOS` table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeatureListTable {
    features: Vec<OpenTypeTag>,
}

impl FeatureListTable {
    /// The distinct feature tags, in table order.
    pub fn features(&self) -> &[OpenTypeTag] {
        &self.features
    }

    /// Loads the features of the `GSUB` table.
    ///
    /// `Ok(None)` when the font has no `GSUB` table; `Err` when it is malformed.
    pub fn load_gsub(font: &dyn IFontMemory) -> Result<Option<FeatureListTable>, FontTableError> {
        let Some(table) = font.try_get_table(GSUB_TAG) else {
            return Ok(None);
        };

        let mut reader = BigEndianBinaryReader::new(table.span());

        Ok(Some(Self::load(&mut reader)?))
    }

    /// Loads the features of the `GPOS` table.
    ///
    /// `Ok(None)` when the font has no `GPOS` table; `Err` when it is malformed.
    pub fn load_gpos(font: &dyn IFontMemory) -> Result<Option<FeatureListTable>, FontTableError> {
        let Some(table) = font.try_get_table(GPOS_TAG) else {
            return Ok(None);
        };

        let mut reader = BigEndianBinaryReader::new(table.span());

        Ok(Some(Self::load(&mut reader)?))
    }

    fn load(reader: &mut BigEndianBinaryReader<'_>) -> Result<FeatureListTable, FontTableError> {
        // GPOS/GSUB Header, Version 1.0
        // +----------+-------------------+-----------------------------------------------------------+
        // | Type     | Name              | Description                                               |
        // +==========+===================+===========================================================+
        // | uint16   | majorVersion      | Major version of the GPOS table, = 1                      |
        // | uint16   | minorVersion      | Minor version of the GPOS table, = 0                      |
        // | Offset16 | scriptListOffset  | Offset to ScriptList table, from beginning of GPOS table  |
        // | Offset16 | featureListOffset | Offset to FeatureList table, from beginning of GPOS table |
        // | Offset16 | lookupListOffset  | Offset to LookupList table, from beginning of GPOS table  |
        // +----------+-------------------+-----------------------------------------------------------+

        reader.read_uint16()?;
        reader.read_uint16()?;
        reader.read_offset16()?;

        let feature_list_offset = reader.read_offset16()?;

        Self::load_at(reader, feature_list_offset as i32)
    }

    fn load_at(reader: &mut BigEndianBinaryReader<'_>, offset: i32) -> Result<FeatureListTable, FontTableError> {
        // FeatureList
        // +---------------+------------------------------+------------------------------------------------------------+
        // | Type          | Name                         | Description                                                |
        // +===============+==============================+============================================================+
        // | uint16        | featureCount                 | Number of FeatureRecords in this table                     |
        // | FeatureRecord | featureRecords[featureCount] | Array of FeatureRecords — zero-based, listed alphabetically |
        // +---------------+------------------------------+------------------------------------------------------------+
        reader.seek(offset)?;

        let feature_count = reader.read_uint16()?;

        if feature_count == 0 {
            return Ok(FeatureListTable { features: Vec::new() });
        }

        let mut features: Vec<OpenTypeTag> = Vec::with_capacity(feature_count as usize);

        for _ in 0..feature_count {
            // FeatureRecord
            // +----------+---------------+--------------------------------------------------------+
            // | Type     | Name          | Description                                            |
            // +==========+===============+========================================================+
            // | Tag      | featureTag    | 4-byte feature identification tag                      |
            // | Offset16 | featureOffset | Offset to Feature table, from beginning of FeatureList |
            // +----------+---------------+--------------------------------------------------------+
            let feature_tag = reader.read_uint32()?;
            reader.read_offset16()?;

            let tag = OpenTypeTag::new(feature_tag);

            // Check for duplicates in already added features
            if !features.contains(&tag) {
                features.push(tag);
            }
        }

        // Keep only the unique features.
        features.shrink_to_fit();

        Ok(FeatureListTable { features })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::fonts::tables::testing::{BigEndianBuffer, SyntheticFont};

    fn build_layout_table(scripts: &[&str], features: &[&str]) -> Vec<u8> {
        let mut buffer = BigEndianBuffer::new();

        buffer.uint16(1).uint16(0);
        let script_list_offset = buffer.reserve_offset16();
        let feature_list_offset = buffer.reserve_offset16();
        buffer.uint16(0); // lookupListOffset

        let position = buffer.position();
        buffer.patch_uint16(script_list_offset, position as i32);
        buffer.uint16(scripts.len() as i32);

        for script in scripts {
            buffer.tag(script).uint16(0);
        }

        let position = buffer.position();
        buffer.patch_uint16(feature_list_offset, position as i32);
        buffer.uint16(features.len() as i32);

        for feature in features {
            buffer.tag(feature).uint16(0);
        }

        buffer.to_array()
    }

    #[test]
    fn missing_tables_give_none() {
        let font = SyntheticFont::new();

        assert_eq!(FeatureListTable::load_gsub(&font), Ok(None));
        assert_eq!(FeatureListTable::load_gpos(&font), Ok(None));
    }

    #[test]
    fn loads_unique_features_in_table_order() {
        let mut font = SyntheticFont::new();
        font.replace("GSUB", build_layout_table(&["latn"], &["calt", "liga", "calt", "ss01", "liga"]));
        font.replace("GPOS", build_layout_table(&[], &["kern"]));

        let gsub = FeatureListTable::load_gsub(&font).unwrap().unwrap();
        let gpos = FeatureListTable::load_gpos(&font).unwrap().unwrap();

        assert_eq!(
            gsub.features(),
            [OpenTypeTag::parse("calt"), OpenTypeTag::parse("liga"), OpenTypeTag::parse("ss01")]
        );
        assert_eq!(gpos.features(), [OpenTypeTag::parse("kern")]);
    }

    #[test]
    fn empty_feature_list_gives_no_features() {
        let mut font = SyntheticFont::new();
        font.replace("GSUB", build_layout_table(&[], &[]));

        assert!(FeatureListTable::load_gsub(&font).unwrap().unwrap().features().is_empty());
    }

    #[test]
    fn malformed_tables_are_errors() {
        let mut font = SyntheticFont::new();
        let table = build_layout_table(&[], &["calt", "liga"]);

        // The second feature record is cut short.
        font.replace("GSUB", table[..table.len() - 1].to_vec());
        assert!(FeatureListTable::load_gsub(&font).unwrap_err().is_invalid_operation());

        // The feature list offset points past the table.
        let mut table = table;
        table[6..8].copy_from_slice(&0xFFFFu16.to_be_bytes());
        font.replace("GPOS", table);
        assert!(FeatureListTable::load_gpos(&font).unwrap_err().is_argument_out_of_range());
    }
}
