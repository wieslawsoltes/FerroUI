use crate::media::fonts::tables::big_endian_binary_reader::{BigEndianBinaryReader, FontTableError};
use crate::media::fonts::tables::platform_id::PlatformID;
use crate::media::fonts::OpenTypeTag;
use crate::media::IFontMemory;

use super::character_to_glyph_map::CharacterToGlyphMap;
use super::cmap_encoding::CmapEncoding;
use super::cmap_format::CmapFormat;
use super::cmap_format12_or13_table::CmapFormat12Or13Table;
use super::cmap_format4_table::CmapFormat4Table;
use super::cmap_subtable_entry::CmapSubtableEntry;

/// The `cmap` (character to glyph index mapping) table loader.
pub struct CmapTable;

impl CmapTable {
    pub const TABLE_NAME: &'static str = "cmap";
    pub const TAG: OpenTypeTag = OpenTypeTag::from_chars('c', 'm', 'a', 'p');

    /// Loads the best character map of the font.
    ///
    /// `Err` when the font has no `cmap` table, the table is malformed or it
    /// has no format 12, 4 or 13 subtable.
    pub fn load(font: &dyn IFontMemory) -> Result<CharacterToGlyphMap, FontTableError> {
        let Some(table) = font.try_get_table(Self::TAG) else {
            return Err(FontTableError::InvalidOperation("No cmap table found.".to_string()));
        };

        let mut reader = BigEndianBinaryReader::new(table.span());

        reader.read_uint16()?; // version

        let num_tables = reader.read_uint16()?;

        let mut entries = Vec::with_capacity(num_tables as usize);

        for _ in 0..num_tables {
            let platform_id = PlatformID(reader.read_uint16()?);
            let encoding_id = CmapEncoding(reader.read_uint16()?);
            let offset = reader.read_uint32()? as i32;

            let position = reader.position();

            reader.seek(offset)?;

            let format = CmapFormat(reader.read_uint16()?);

            reader.seek(position)?;

            entries.push(CmapSubtableEntry::new(platform_id, encoding_id, offset, format));
        }

        // Try to find the best Format 12 subtable entry
        if let Some(format12_entry) = Self::try_find_format12_or13_entry(&entries, CmapFormat::Format12) {
            // Prefer Format 12 if available
            return Ok(CharacterToGlyphMap::from_format12_or13(CmapFormat12Or13Table::new(
                format12_entry.get_subtable_memory(&table)?,
            )?));
        }

        // Then Format 4
        if let Some(format4_entry) = Self::try_find_format4_entry(&entries) {
            return Ok(CharacterToGlyphMap::from_format4(CmapFormat4Table::new(
                format4_entry.get_subtable_memory(&table)?,
            )?));
        }

        // Fallback to Format 13, which is a "last resort" format mapping many codepoints to a single glyph
        if let Some(format13_entry) = Self::try_find_format12_or13_entry(&entries, CmapFormat::Format13) {
            return Ok(CharacterToGlyphMap::from_format12_or13(CmapFormat12Or13Table::new(
                format13_entry.get_subtable_memory(&table)?,
            )?));
        }

        Err(FontTableError::InvalidOperation("No suitable cmap subtable found.".to_string()))
    }

    /// Tries to find the best Format 12 (or 13) subtable entry based on platform and encoding preferences.
    fn try_find_format12_or13_entry(
        entries: &[CmapSubtableEntry],
        expected_format: CmapFormat,
    ) -> Option<CmapSubtableEntry> {
        let mut result = CmapSubtableEntry::default();
        let mut found_platform_score = i32::MAX;
        let mut found_encoding_score = i32::MAX;

        for entry in entries {
            if entry.format != expected_format {
                continue;
            }

            let platform_score = if entry.platform == PlatformID::Unicode {
                0
            } else if entry.platform == PlatformID::Windows {
                1
            } else {
                2
            };

            let mut encoding_score = 2; // Default: lowest preference

            if entry.platform == PlatformID::Unicode {
                if entry.encoding == CmapEncoding::Unicode_2_0_full {
                    encoding_score = 0; // non-BMP preferred
                } else if entry.encoding == CmapEncoding::Unicode_2_0_BMP {
                    encoding_score = 1; // BMP
                }
            } else if entry.platform == PlatformID::Windows && platform_score != 0 {
                if entry.encoding == CmapEncoding::Microsoft_UCS4 {
                    encoding_score = 0; // non-BMP preferred
                } else if entry.encoding == CmapEncoding::Microsoft_UnicodeBMP {
                    encoding_score = 1; // BMP
                }
            }

            // A better encoding wins; otherwise a better platform wins (also over a better
            // encoding found earlier), exactly as the reference selection does.
            let better_encoding = encoding_score < found_encoding_score;
            let better_platform = platform_score < found_platform_score;

            if better_encoding || better_platform {
                result = *entry;
                found_encoding_score = encoding_score;
                found_platform_score = platform_score;
            }

            if found_platform_score == 0 && found_encoding_score == 0 {
                break; // Best possible match found
            }
        }

        if result.format != CmapFormat::Format0 {
            Some(result)
        } else {
            None
        }
    }

    /// Tries to find the best Format 4 subtable entry based on encoding then platform.
    fn try_find_format4_entry(entries: &[CmapSubtableEntry]) -> Option<CmapSubtableEntry> {
        let mut result = CmapSubtableEntry::default();
        let mut found_encoding_score = i32::MAX;
        let mut found_platform_score = i32::MAX;

        for entry in entries {
            if entry.format != CmapFormat::Format4 {
                continue;
            }

            // A Windows 'Symbol' (encoding 0) subtable maps the F000–F0FF private-use range,
            // not real Unicode, so it must not be chosen over a Unicode subtable for normal
            // text. Score the Symbol encoding worse than everything else.
            let encoding_score =
                if entry.platform == PlatformID::Windows && entry.encoding == CmapEncoding::Microsoft_Symbol {
                    1
                } else {
                    0
                };

            let platform_score = if entry.platform == PlatformID::Unicode {
                0
            } else if entry.platform == PlatformID::Windows {
                1
            } else {
                2
            };

            // Lower is better: encoding dominates, platform breaks ties.
            if encoding_score < found_encoding_score
                || (encoding_score == found_encoding_score && platform_score < found_platform_score)
            {
                result = *entry;
                found_encoding_score = encoding_score;
                found_platform_score = platform_score;
            }
        }

        if result.format != CmapFormat::Format0 {
            Some(result)
        } else {
            None
        }
    }
}
