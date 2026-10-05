// Derived from SixLabors.Fonts (Apache License 2.0), see NOTICE.md.

use crate::media::fonts::tables::encoding_id_extensions::Encoding;
use crate::media::fonts::tables::known_name_ids::KnownNameIds;
use crate::media::fonts::tables::platform_id::PlatformID;
use crate::utilities::ReadOnlyMemory;

/// One record of the `name` table; the string is decoded on demand.
#[derive(Clone, Debug)]
pub struct NameRecord {
    string_storage: ReadOnlyMemory<u8>,
    platform: PlatformID,
    language_id: u16,
    name_id: KnownNameIds,
    offset: u16,
    length: u16,
    encoding: Encoding,
}

impl NameRecord {
    pub fn new(
        string_storage: ReadOnlyMemory<u8>,
        platform: PlatformID,
        language_id: u16,
        name_id: KnownNameIds,
        offset: u16,
        length: u16,
        encoding: Encoding,
    ) -> Self {
        Self { string_storage, platform, language_id, name_id, offset, length, encoding }
    }

    pub fn platform(&self) -> PlatformID {
        self.platform
    }

    /// The language of the record (an LCID for the Windows platform).
    pub fn language_id(&self) -> u16 {
        self.language_id
    }

    pub fn name_id(&self) -> KnownNameIds {
        self.name_id
    }

    pub fn offset(&self) -> u16 {
        self.offset
    }

    pub fn length(&self) -> u16 {
        self.length
    }

    pub fn encoding(&self) -> Encoding {
        self.encoding
    }

    /// Decodes the string of the record.
    pub fn get_value(&self) -> String {
        if self.length == 0 {
            return String::new();
        }

        // Offset/Length come straight from the untrusted 'name' record. NameTable::load validates
        // the record array but not each record's storage slice, and get_value runs later during
        // typeface construction, so a record pointing past the string storage must degrade to an
        // empty value rather than deny the font.
        let start = self.offset as usize;
        let end = start + self.length as usize;

        match self.string_storage.span().get(start..end) {
            // The decoders substitute U+FFFD for malformed bytes.
            Some(span) => self.encoding.get_string(span),
            None => String::new(),
        }
    }
}
