use std::collections::BTreeMap;

use crate::media::fonts::OpenTypeTag;
use crate::media::IFontMemory;
use crate::utilities::ReadOnlyMemory;

/// An editable, in-memory sfnt font for robustness / malformed-input tests.
///
/// The font subsystem reads every table through [`IFontMemory::try_get_table`],
/// and each OpenType table is self-contained (its internal offsets are
/// relative to the table start, never to the file). That means a faithful test
/// font is just a `tag → bytes` map: a test builds or parses one, mutates a
/// single table (truncate it, remove it, or patch a specific offset / count to
/// a hostile value), and hands the result to the table readers.
///
/// The font itself implements [`IFontMemory`] (serving the current tables);
/// [`to_font_memory`](SyntheticFont::to_font_memory) gives a snapshot that is
/// unaffected by later mutation.
#[derive(Clone, Debug)]
pub(crate) struct SyntheticFont {
    // A sorted map keeps `to_bytes` deterministic.
    tables: BTreeMap<OpenTypeTag, Vec<u8>>,
    sfnt_version: u32,
}

#[allow(dead_code)] // the full harness API is kept for the font tests built on top
impl SyntheticFont {
    /// An empty TrueType-flavoured font (no tables).
    pub(crate) fn new() -> Self {
        Self { tables: BTreeMap::new(), sfnt_version: 0x0001_0000 }
    }

    /// Parses the sfnt table directory of `font` into an editable table set.
    /// Each table is copied into its own buffer so later mutation can't alias
    /// the source bytes. TrueType Collections (`ttcf`) are not supported.
    pub(crate) fn from_bytes(font: &[u8]) -> Result<SyntheticFont, String> {
        if font.len() < 12 {
            return Err("Not a valid sfnt: shorter than the offset table.".to_string());
        }

        let sfnt_version = read_u32(font, 0);

        // 'ttcf' — a font collection. Out of scope: callers pass single-font data.
        if sfnt_version == 0x7474_6366 {
            return Err("TrueType Collections are not supported by SyntheticFont.".to_string());
        }

        let num_tables = u16::from_be_bytes([font[4], font[5]]) as usize;

        let mut tables = BTreeMap::new();

        for i in 0..num_tables {
            let record = 12 + i * 16;

            if record + 16 > font.len() {
                return Err("Not a valid sfnt: table directory exceeds the file.".to_string());
            }

            let tag = OpenTypeTag::new(read_u32(font, record));
            let offset = read_u32(font, record + 8) as u64;
            let length = read_u32(font, record + 12) as u64;

            if offset > i32::MAX as u64 || length > i32::MAX as u64 || offset + length > font.len() as u64 {
                return Err(format!("Not a valid sfnt: table '{tag}' is out of range."));
            }

            tables.insert(tag, font[offset as usize..(offset + length) as usize].to_vec());
        }

        Ok(SyntheticFont { tables, sfnt_version })
    }

    /// The tags present in the font.
    pub(crate) fn tags(&self) -> Vec<OpenTypeTag> {
        self.tables.keys().copied().collect()
    }

    /// Whether the named table is present.
    pub(crate) fn contains(&self, tag: &str) -> bool {
        self.tables.contains_key(&OpenTypeTag::parse(tag))
    }

    /// Returns a copy of the named table's current bytes.
    ///
    /// Panics when the font has no such table.
    pub(crate) fn get_table(&self, tag: &str) -> Vec<u8> {
        self.require(tag).clone()
    }

    /// The current byte length of the named table.
    ///
    /// Panics when the font has no such table.
    pub(crate) fn table_length(&self, tag: &str) -> usize {
        self.require(tag).len()
    }

    /// Replaces the named table's bytes wholesale (adding it if absent).
    pub(crate) fn replace(&mut self, tag: &str, bytes: Vec<u8>) -> &mut Self {
        self.tables.insert(OpenTypeTag::parse(tag), bytes);
        self
    }

    /// Removes the named table from the font (no-op if absent).
    pub(crate) fn remove(&mut self, tag: &str) -> &mut Self {
        self.tables.remove(&OpenTypeTag::parse(tag));
        self
    }

    /// Truncates the named table to `new_length` bytes — the canonical "table
    /// is shorter than its header claims" corruption.
    ///
    /// Panics when the font has no such table or `new_length` exceeds its length.
    pub(crate) fn truncate(&mut self, tag: &str, new_length: usize) -> &mut Self {
        let bytes = self.require_mut(tag);

        assert!(
            new_length <= bytes.len(),
            "Truncation length must be in [0, {}] for table '{tag}'.",
            bytes.len()
        );

        bytes.truncate(new_length);
        self
    }

    /// Overwrites a single byte at `offset` within the named table.
    pub(crate) fn patch_uint8(&mut self, tag: &str, offset: usize, value: u8) -> &mut Self {
        self.editable_slice(tag, offset, 1)[0] = value;
        self
    }

    /// Overwrites a big-endian `u16` at `offset` within the named table.
    pub(crate) fn patch_uint16(&mut self, tag: &str, offset: usize, value: u16) -> &mut Self {
        self.editable_slice(tag, offset, 2).copy_from_slice(&value.to_be_bytes());
        self
    }

    /// Overwrites a big-endian `u32` at `offset` within the named table.
    pub(crate) fn patch_uint32(&mut self, tag: &str, offset: usize, value: u32) -> &mut Self {
        self.editable_slice(tag, offset, 4).copy_from_slice(&value.to_be_bytes());
        self
    }

    /// Runs an arbitrary edit against the named table's backing buffer.
    pub(crate) fn mutate(&mut self, tag: &str, edit: impl FnOnce(&mut Vec<u8>)) -> &mut Self {
        edit(self.require_mut(tag));
        self
    }

    /// Returns an [`IFontMemory`] that serves the current (possibly corrupted)
    /// tables. Each call snapshots the current table set, so the result is
    /// unaffected by later mutation of this font.
    pub(crate) fn to_font_memory(&self) -> SyntheticFontMemory {
        SyntheticFontMemory {
            tables: self
                .tables
                .iter()
                .map(|(tag, bytes)| (*tag, ReadOnlyMemory::from_slice(bytes)))
                .collect(),
        }
    }

    /// Re-assembles the current table set into a valid sfnt byte array (offset
    /// table + 4-byte-aligned tables, directory sorted by tag). Table checksums
    /// are written as zero.
    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let num_tables = self.tables.len();

        let directory_size = 12 + num_tables * 16;
        let total_size = directory_size + self.tables.values().map(|bytes| align4(bytes.len())).sum::<usize>();

        let mut font = vec![0u8; total_size];

        // Offset table.
        font[0..4].copy_from_slice(&self.sfnt_version.to_be_bytes());
        font[4..6].copy_from_slice(&(num_tables as u16).to_be_bytes());

        let mut max_pow2 = 1;
        let mut entry_selector = 0;

        while max_pow2 * 2 <= num_tables {
            max_pow2 *= 2;
            entry_selector += 1;
        }

        let search_range = max_pow2 * 16;

        font[6..8].copy_from_slice(&(search_range as u16).to_be_bytes());
        font[8..10].copy_from_slice(&(entry_selector as u16).to_be_bytes());
        font[10..12].copy_from_slice(&((num_tables * 16).wrapping_sub(search_range) as u16).to_be_bytes());

        // Directory records + table bodies.
        let mut data_offset = directory_size;

        for (i, (tag, bytes)) in self.tables.iter().enumerate() {
            let record = 12 + i * 16;

            font[record..record + 4].copy_from_slice(&tag.value().to_be_bytes());
            // record + 4: checksum (not validated), left as zero
            font[record + 8..record + 12].copy_from_slice(&(data_offset as u32).to_be_bytes());
            font[record + 12..record + 16].copy_from_slice(&(bytes.len() as u32).to_be_bytes());

            font[data_offset..data_offset + bytes.len()].copy_from_slice(bytes);
            data_offset += align4(bytes.len());
        }

        font
    }

    fn require(&self, tag: &str) -> &Vec<u8> {
        match self.tables.get(&OpenTypeTag::parse(tag)) {
            Some(bytes) => bytes,
            None => panic!("Font has no '{tag}' table."),
        }
    }

    fn require_mut(&mut self, tag: &str) -> &mut Vec<u8> {
        match self.tables.get_mut(&OpenTypeTag::parse(tag)) {
            Some(bytes) => bytes,
            None => panic!("Font has no '{tag}' table."),
        }
    }

    fn editable_slice(&mut self, tag: &str, offset: usize, size: usize) -> &mut [u8] {
        let bytes = self.require_mut(tag);
        let length = bytes.len();

        match bytes.get_mut(offset..offset + size) {
            Some(slice) => slice,
            None => panic!("[{offset}, {}) is out of range for table '{tag}' (length {length}).", offset + size),
        }
    }
}

impl IFontMemory for SyntheticFont {
    fn try_get_table(&self, tag: OpenTypeTag) -> Option<ReadOnlyMemory<u8>> {
        self.tables.get(&tag).map(|bytes| ReadOnlyMemory::from_slice(bytes))
    }

    fn dispose(&self) {}
}

/// An [`IFontMemory`] that serves a fixed `tag → bytes` map: a snapshot of a
/// [`SyntheticFont`].
#[derive(Clone, Debug)]
pub(crate) struct SyntheticFontMemory {
    tables: BTreeMap<OpenTypeTag, ReadOnlyMemory<u8>>,
}

impl IFontMemory for SyntheticFontMemory {
    fn try_get_table(&self, tag: OpenTypeTag) -> Option<ReadOnlyMemory<u8>> {
        self.tables.get(&tag).cloned()
    }

    fn dispose(&self) {}
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes([bytes[offset], bytes[offset + 1], bytes[offset + 2], bytes[offset + 3]])
}

fn align4(length: usize) -> usize {
    (length + 3) & !3
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> SyntheticFont {
        let mut font = SyntheticFont::new();

        font.replace("head", (0u8..54).collect())
            .replace("maxp", vec![0, 0, 0x50, 0, 0, 7])
            .replace("post", vec![1u8; 32]);

        font
    }

    #[test]
    fn truncate_shortens_the_table() {
        let mut font = sample();
        let original_length = font.table_length("post");

        font.truncate("post", 4);

        assert_eq!(font.table_length("post"), 4);
        assert!(original_length > 4);
    }

    #[test]
    fn remove_drops_the_table() {
        let mut font = sample();

        assert!(font.contains("post"));

        font.remove("post");

        assert!(!font.contains("post"));
    }

    #[test]
    fn patch_uint16_writes_big_endian_at_the_given_offset() {
        let mut font = sample();

        // head.unitsPerEm lives at offset 18.
        font.patch_uint16("head", 18, 0x0801);

        let head = font.get_table("head");

        assert_eq!(head[18], 0x08);
        assert_eq!(head[19], 0x01);
    }

    #[test]
    fn patches_and_mutations_edit_the_table_in_place() {
        let mut font = sample();

        font.patch_uint8("head", 0, 0xAA).patch_uint32("head", 4, 0x0102_0304).mutate("head", |bytes| bytes.push(0xEE));

        let head = font.get_table("head");

        assert_eq!(head[0], 0xAA);
        assert_eq!(head[4..8], [1, 2, 3, 4]);
        assert_eq!(head.len(), 55);
        assert_eq!(head[54], 0xEE);
    }

    #[test]
    fn to_font_memory_snapshots_the_tables_at_call_time() {
        let mut font = sample();

        let head_tag = OpenTypeTag::parse("head");
        let before_mutation = font.get_table("head");

        let font_memory = font.to_font_memory();

        // Mutate the source after taking the snapshot.
        font.patch_uint16("head", 18, 0x1234);

        let snapshot = font_memory.try_get_table(head_tag).unwrap();

        // The snapshot reflects the pre-mutation bytes; the source reflects the mutation.
        assert_eq!(snapshot.to_vec(), before_mutation);
        assert_ne!(font.get_table("head"), before_mutation);
        assert_eq!(font.try_get_table(head_tag).unwrap().to_vec(), font.get_table("head"));
        assert!(font_memory.try_get_table(OpenTypeTag::parse("glyf")).is_none());
    }

    #[test]
    fn to_bytes_round_trips_through_from_bytes_preserving_tables() {
        let original = sample();

        let bytes = original.to_bytes();
        let reparsed = SyntheticFont::from_bytes(&bytes).unwrap();

        assert_eq!(original.tags(), reparsed.tags());

        // Table contents survive the round-trip byte-for-byte.
        assert_eq!(original.get_table("maxp"), reparsed.get_table("maxp"));
        assert_eq!(original.get_table("head"), reparsed.get_table("head"));

        // Offset table: version, numTables, searchRange, entrySelector, rangeShift.
        assert_eq!(bytes[..12], [0, 1, 0, 0, 0, 3, 0, 32, 0, 1, 0, 16]);
        assert_eq!(bytes.len() % 4, 0);
    }

    #[test]
    fn from_bytes_rejects_invalid_directories() {
        assert!(SyntheticFont::from_bytes(&[0u8; 11]).is_err());
        assert!(SyntheticFont::from_bytes(b"ttcf\0\0\0\0\0\0\0\0").is_err());

        // One table declared, no directory record.
        assert!(SyntheticFont::from_bytes(&[0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0]).is_err());

        // A table that points past the file.
        let mut bytes = sample().to_bytes();
        bytes[12 + 12..12 + 16].copy_from_slice(&0x00FF_FFFFu32.to_be_bytes());
        assert!(SyntheticFont::from_bytes(&bytes).is_err());
    }
}
