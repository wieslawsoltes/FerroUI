use std::collections::HashSet;

use crate::media::fonts::OpenTypeTag;
use crate::media::IFontMemory;

use super::big_endian_binary_reader::{BigEndianBinaryReader, FontTableError};

const GSUB: OpenTypeTag = OpenTypeTag::from_chars('G', 'S', 'U', 'B');
const GPOS: OpenTypeTag = OpenTypeTag::from_chars('G', 'P', 'O', 'S');

/// Reads the script tags of the `GSUB` and `GPOS` script lists.
pub struct ScriptListTable;

impl ScriptListTable {
    /// Adds the script tags of both layout tables to `script_tags`.
    ///
    /// Returns `false` when a present table is malformed (the shaping
    /// capability is then unknown); tags read before the failure stay in the set.
    pub fn try_read_script_tags(font: &dyn IFontMemory, script_tags: &mut HashSet<OpenTypeTag>) -> bool {
        // Non-short-circuiting '&' so both tables are always attempted.
        Self::try_read_script_list(font, GSUB, script_tags) & Self::try_read_script_list(font, GPOS, script_tags)
    }

    fn try_read_script_list(
        font: &dyn IFontMemory,
        table_tag: OpenTypeTag,
        script_tags: &mut HashSet<OpenTypeTag>,
    ) -> bool {
        let Some(table) = font.try_get_table(table_tag) else {
            // An absent table simply contributes no scripts
            return true;
        };

        // Malformed layout table — capability unknown.
        Self::read_script_list(table.span(), script_tags).is_ok()
    }

    fn read_script_list(span: &[u8], script_tags: &mut HashSet<OpenTypeTag>) -> Result<(), FontTableError> {
        let mut reader = BigEndianBinaryReader::new(span);

        // GSUB/GPOS header: majorVersion, minorVersion, scriptListOffset, featureListOffset,
        // lookupListOffset.
        reader.read_uint16()?;
        reader.read_uint16()?;
        let script_list_offset = reader.read_offset16()?;

        // ScriptList: scriptCount, then scriptCount ScriptRecords of (Tag, Offset16).
        reader.seek(script_list_offset as i32)?;

        let script_count = reader.read_uint16()?;

        for _ in 0..script_count {
            script_tags.insert(OpenTypeTag::new(reader.read_uint32()?));
            reader.read_offset16()?; // scriptOffset — not needed
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::fonts::tables::testing::{BigEndianBuffer, SyntheticFont};

    fn build_layout_table(scripts: &[&str]) -> Vec<u8> {
        let mut buffer = BigEndianBuffer::new();

        buffer.uint16(1).uint16(0).uint16(10).uint16(0).uint16(0);
        buffer.uint16(scripts.len() as i32);

        for script in scripts {
            buffer.tag(script).uint16(0);
        }

        buffer.to_array()
    }

    #[test]
    fn absent_tables_contribute_no_scripts() {
        let mut tags = HashSet::new();

        assert!(ScriptListTable::try_read_script_tags(&SyntheticFont::new(), &mut tags));
        assert!(tags.is_empty());
    }

    #[test]
    fn reads_the_scripts_of_both_tables() {
        let mut font = SyntheticFont::new();
        font.replace("GSUB", build_layout_table(&["DFLT", "latn"]));
        font.replace("GPOS", build_layout_table(&["latn", "cyrl"]));

        let mut tags = HashSet::new();

        assert!(ScriptListTable::try_read_script_tags(&font, &mut tags));
        assert_eq!(tags.len(), 3);
        assert!(tags.contains(&OpenTypeTag::parse("DFLT")));
        assert!(tags.contains(&OpenTypeTag::parse("latn")));
        assert!(tags.contains(&OpenTypeTag::parse("cyrl")));
    }

    #[test]
    fn malformed_table_reports_unknown_but_still_reads_the_other_table() {
        let mut font = SyntheticFont::new();
        let gsub = build_layout_table(&["arab", "latn"]);

        // GSUB is cut inside its second script record.
        font.replace("GSUB", gsub[..gsub.len() - 3].to_vec());
        font.replace("GPOS", build_layout_table(&["cyrl"]));

        let mut tags = HashSet::new();

        assert!(!ScriptListTable::try_read_script_tags(&font, &mut tags));
        assert!(tags.contains(&OpenTypeTag::parse("arab")));
        assert!(tags.contains(&OpenTypeTag::parse("cyrl")));
        assert!(!tags.contains(&OpenTypeTag::parse("latn")));
    }
}
