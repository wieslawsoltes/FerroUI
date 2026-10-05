// Derived from SixLabors.Fonts (Apache License 2.0), see NOTICE.md.

use std::cell::RefCell;

use crate::media::fonts::tables::big_endian_binary_reader::{BigEndianBinaryReader, FontTableError};
use crate::media::fonts::tables::encoding_ids::EncodingIDs;
use crate::media::fonts::tables::known_name_ids::KnownNameIds;
use crate::media::fonts::tables::platform_id::PlatformID;
use crate::media::fonts::OpenTypeTag;
use crate::media::IFontMemory;
use crate::utilities::ReadOnlyMemory;

use super::name_record::NameRecord;

const US_ENGLISH_LANGUAGE_ID: u16 = 0x0409;

/// The `name` (naming) table.
///
/// Cultures are identified by their Windows language id (LCID), as stored in
/// the table's Windows-platform records.
#[derive(Clone, Debug)]
pub struct NameTable {
    names: Vec<NameRecord>,
    cached_family_name: RefCell<Option<String>>,
    cached_typographic_family_name: RefCell<Option<String>>,
}

impl NameTable {
    pub const TABLE_NAME: &'static str = "name";
    pub const TAG: OpenTypeTag = OpenTypeTag::from_chars('n', 'a', 'm', 'e');

    pub(crate) fn new(names: Vec<NameRecord>) -> Self {
        Self { names, cached_family_name: RefCell::new(None), cached_typographic_family_name: RefCell::new(None) }
    }

    pub fn id(&self, culture: u16) -> String {
        self.get_name_by_id(culture, KnownNameIds::UniqueFontID)
    }

    pub fn font_name(&self, culture: u16) -> String {
        self.get_name_by_id(culture, KnownNameIds::FullFontName)
    }

    pub fn font_family_name(&self, culture: u16) -> String {
        if culture == US_ENGLISH_LANGUAGE_ID {
            if let Some(cached) = self.cached_family_name.borrow().as_ref() {
                return cached.clone();
            }
        }

        let value = self.get_name_by_id(culture, KnownNameIds::FontFamilyName);

        if culture == US_ENGLISH_LANGUAGE_ID {
            *self.cached_family_name.borrow_mut() = Some(value.clone());
        }

        value
    }

    pub fn font_sub_family_name(&self, culture: u16) -> String {
        self.get_name_by_id(culture, KnownNameIds::FontSubfamilyName)
    }

    /// The name with the given id for the culture.
    ///
    /// Falls back to the US English Windows record, then the first Windows
    /// record, then the first record of any platform; the empty string when
    /// the table has no record with that id.
    pub fn get_name_by_id(&self, culture: u16, name_id: KnownNameIds) -> String {
        let is_cached_name = name_id == KnownNameIds::TypographicFamilyName && culture == US_ENGLISH_LANGUAGE_ID;

        if is_cached_name {
            if let Some(cached) = self.cached_typographic_family_name.borrow().as_ref() {
                return cached.clone();
            }
        }

        let language_id = culture;
        let mut usa_version: Option<&NameRecord> = None;
        let mut first_windows: Option<&NameRecord> = None;
        let mut first: Option<&NameRecord> = None;

        for name in &self.names {
            if name.name_id() == name_id {
                first = first.or(Some(name));

                if name.platform() == PlatformID::Windows {
                    first_windows = first_windows.or(Some(name));

                    if name.language_id() == US_ENGLISH_LANGUAGE_ID {
                        usa_version = usa_version.or(Some(name));
                    }

                    if name.language_id() == language_id {
                        return name.get_value();
                    }
                }
            }
        }

        let value = usa_version.or(first_windows).or(first).map(NameRecord::get_value).unwrap_or_default();

        if is_cached_name {
            *self.cached_typographic_family_name.borrow_mut() = Some(value.clone());
        }

        value
    }

    /// [`get_name_by_id`](Self::get_name_by_id) for a raw name id.
    pub fn get_name_by_id_u16(&self, culture: u16, name_id: u16) -> String {
        self.get_name_by_id(culture, KnownNameIds(name_id))
    }

    /// Loads the table; `None` when the font has no `name` table or the table
    /// is malformed.
    pub fn load(font: &dyn IFontMemory) -> Option<NameTable> {
        let table = font.try_get_table(Self::TAG)?;

        // A present-but-malformed 'name' table must not deny the font; callers fall back to a
        // default family name, the same outcome as an absent 'name'.
        Self::load_from_table(&table).ok().flatten()
    }

    fn load_from_table(table: &ReadOnlyMemory<u8>) -> Result<Option<NameTable>, FontTableError> {
        let mut reader = BigEndianBinaryReader::new(table.span());

        reader.read_uint16()?;
        let count = reader.read_uint16()? as usize;
        let storage_offset = reader.read_uint16()? as usize;

        const HEADER_SIZE: usize = 6;
        const RECORD_SIZE: usize = 12;

        // The three reads above already require the header.
        if table.len() < HEADER_SIZE {
            return Ok(None);
        }

        let records_size = count * RECORD_SIZE;
        if records_size > table.len() - HEADER_SIZE {
            return Ok(None);
        }

        if storage_offset > table.len() {
            return Ok(None);
        }

        let name_storage = table.slice_from(storage_offset);

        let mut names = Vec::with_capacity(count);

        for _ in 0..count {
            let platform = reader.read_uint16_as::<PlatformID>()?;
            let encoding_id = reader.read_uint16_as::<EncodingIDs>()?;
            let encoding = encoding_id.as_encoding();
            let language_id = reader.read_uint16()?;
            let name_id = reader.read_uint16_as::<KnownNameIds>()?;
            let length = reader.read_uint16()?;
            let offset = reader.read_uint16()?;

            names.push(NameRecord::new(name_storage.clone(), platform, language_id, name_id, offset, length, encoding));
        }

        Ok(Some(NameTable::new(names)))
    }

    /// Enumerates the records in table order.
    pub fn iter(&self) -> std::slice::Iter<'_, NameRecord> {
        self.names.iter()
    }
}

impl<'a> IntoIterator for &'a NameTable {
    type Item = &'a NameRecord;
    type IntoIter = std::slice::Iter<'a, NameRecord>;

    fn into_iter(self) -> Self::IntoIter {
        self.names.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::fonts::tables::encoding_id_extensions::Encoding;
    use crate::media::fonts::tables::testing::{BigEndianBuffer, SyntheticFont};

    struct Record {
        platform: u16,
        encoding: u16,
        language: u16,
        name_id: u16,
        text: &'static str,
    }

    const fn windows(language: u16, name_id: u16, text: &'static str) -> Record {
        Record { platform: 3, encoding: 1, language, name_id, text }
    }

    const fn mac(name_id: u16, text: &'static str) -> Record {
        Record { platform: 1, encoding: 0, language: 0, name_id, text }
    }

    fn build_name_table(records: &[Record]) -> Vec<u8> {
        let mut buffer = BigEndianBuffer::new();
        let mut storage = Vec::new();

        buffer.uint16(0).uint16(records.len() as i32).uint16((6 + records.len() * 12) as i32);

        for record in records {
            let bytes: Vec<u8> = if record.platform == 3 {
                record.text.encode_utf16().flat_map(u16::to_be_bytes).collect()
            } else {
                record.text.as_bytes().to_vec()
            };

            buffer
                .uint16(record.platform as i32)
                .uint16(record.encoding as i32)
                .uint16(record.language as i32)
                .uint16(record.name_id as i32)
                .uint16(bytes.len() as i32)
                .uint16(storage.len() as i32);

            storage.extend_from_slice(&bytes);
        }

        buffer.bytes(&storage);
        buffer.to_array()
    }

    fn font_with(records: &[Record]) -> SyntheticFont {
        let mut font = SyntheticFont::new();
        font.replace("name", build_name_table(records));
        font
    }

    fn inter_like() -> SyntheticFont {
        font_with(&[
            mac(1, "Inter Mac"),
            windows(0x0409, 1, "Inter"),
            windows(0x0409, 2, "Regular"),
            windows(0x0409, 3, "4.000;RSMS;Inter-Regular"),
            windows(0x0409, 4, "Inter Regular"),
            windows(0x0407, 1, "Inter Deutsch"),
            windows(0x0409, 16, "Inter Typographic"),
        ])
    }

    #[test]
    fn missing_name_table_gives_none() {
        // An absent name table is tolerated; callers fall back to a default family name.
        assert!(NameTable::load(&SyntheticFont::new()).is_none());
    }

    #[test]
    fn truncated_name_table_gives_none() {
        // Keep only format + count; reading stringOffset and the record array over-runs.
        let mut font = inter_like();
        font.truncate("name", 4);

        assert!(NameTable::load(&font).is_none());

        // The record array is cut short.
        let mut font = inter_like();
        font.truncate("name", 6 + 12 * 3);

        assert!(NameTable::load(&font).is_none());
    }

    #[test]
    fn storage_offset_past_the_table_gives_none() {
        let mut font = inter_like();
        font.patch_uint16("name", 4, 0xFFFF);

        assert!(NameTable::load(&font).is_none());
    }

    #[test]
    fn resolves_names_by_culture_with_fallbacks() {
        let names = NameTable::load(&inter_like()).unwrap();

        // The invariant culture (LCID 127) has no record: the US English one is used.
        assert_eq!(names.font_family_name(127), "Inter");
        assert_eq!(names.font_family_name(0x0409), "Inter");
        // Cached for US English.
        assert_eq!(names.font_family_name(0x0409), "Inter");
        assert_eq!(names.font_family_name(0x0407), "Inter Deutsch");
        assert_eq!(names.font_sub_family_name(0x0407), "Regular");
        assert_eq!(names.id(127), "4.000;RSMS;Inter-Regular");
        assert_eq!(names.font_name(127), "Inter Regular");
        assert_eq!(names.get_name_by_id(0x0409, KnownNameIds::TypographicFamilyName), "Inter Typographic");
        assert_eq!(names.get_name_by_id(0x0409, KnownNameIds::TypographicFamilyName), "Inter Typographic");
        assert_eq!(names.get_name_by_id_u16(127, 16), "Inter Typographic");
        assert_eq!(names.get_name_by_id(127, KnownNameIds::Designer), "");
    }

    #[test]
    fn falls_back_to_first_windows_then_first_record() {
        let names = NameTable::load(&font_with(&[mac(1, "Mac Family"), windows(0x0411, 1, "Japanese Family")])).unwrap();

        assert_eq!(names.font_family_name(0x0409), "Japanese Family");

        let names = NameTable::load(&font_with(&[mac(1, "Mac Family")])).unwrap();

        assert_eq!(names.font_family_name(0x0409), "Mac Family");
    }

    #[test]
    fn enumerates_records_in_table_order() {
        let names = NameTable::load(&inter_like()).unwrap();

        let records: Vec<&NameRecord> = names.iter().collect();

        assert_eq!(records.len(), 7);
        assert_eq!(records[0].platform(), PlatformID::Macintosh);
        assert_eq!(records[0].encoding(), Encoding::UTF8);
        assert_eq!(records[0].get_value(), "Inter Mac");
        assert_eq!(records[5].platform(), PlatformID::Windows);
        assert_eq!(records[5].language_id(), 0x0407);
        assert_eq!(records[5].name_id(), KnownNameIds::FontFamilyName);
        assert_eq!(records[5].encoding(), Encoding::BigEndianUnicode);
        assert_eq!(records[5].length(), 26);
        assert_eq!((&names).into_iter().count(), 7);
    }

    #[test]
    fn record_pointing_past_the_storage_decodes_to_empty() {
        let mut font = font_with(&[windows(0x0409, 1, "Inter")]);

        // Record 0: length at offset 6 + 8, string offset at 6 + 10.
        font.patch_uint16("name", 16, 0xFFF0);

        let names = NameTable::load(&font).unwrap();

        assert_eq!(names.font_family_name(0x0409), "");

        let mut font = font_with(&[windows(0x0409, 1, "Inter")]);
        font.patch_uint16("name", 14, 0xFFFF);

        assert_eq!(NameTable::load(&font).unwrap().font_family_name(0x0409), "");
    }

    #[test]
    fn empty_record_decodes_to_empty() {
        let names = NameTable::load(&font_with(&[windows(0x0409, 1, "")])).unwrap();

        assert_eq!(names.font_family_name(0x0409), "");
        assert_eq!(names.iter().next().unwrap().offset(), 0);
    }
}
