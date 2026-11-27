//! The OpenType container: reading its table directory, writing a new one, and the sanitizer.

mod glyf;
mod sanitize;
mod tables;

#[cfg(test)]
mod tests;

pub use sanitize::{sanitize, SanitizeOptions, Sanitizer};

use crate::{FontDecodeError, WebFontLimits};

pub(crate) type Tag = [u8; 4];

pub(crate) const TRUETYPE: u32 = 0x0001_0000;
pub(crate) const APPLE_TRUE: u32 = u32::from_be_bytes(*b"true");
pub(crate) const OPEN_TYPE_CFF: u32 = u32::from_be_bytes(*b"OTTO");
pub(crate) const COLLECTION: u32 = u32::from_be_bytes(*b"ttcf");

/// Big-endian reads that fail instead of panicking.
pub(crate) trait ReadBe {
    fn u8_at(&self, offset: usize) -> Option<u8>;
    fn u16_at(&self, offset: usize) -> Option<u16>;
    fn u32_at(&self, offset: usize) -> Option<u32>;
    fn i16_at(&self, offset: usize) -> Option<i16> {
        self.u16_at(offset).map(|value| value as i16)
    }
}

impl ReadBe for [u8] {
    #[inline]
    fn u8_at(&self, offset: usize) -> Option<u8> {
        self.get(offset).copied()
    }

    #[inline]
    fn u16_at(&self, offset: usize) -> Option<u16> {
        let bytes = self.get(offset..offset.checked_add(2)?)?;
        Some(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    #[inline]
    fn u32_at(&self, offset: usize) -> Option<u32> {
        let bytes = self.get(offset..offset.checked_add(4)?)?;
        Some(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }
}

/// One entry of a table directory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TableRecord {
    pub tag: Tag,
    pub offset: usize,
    pub length: usize,
}

/// The table directory of the face at `face_index`: sorted by tag, with no duplicate tag, each
/// table inside the file and no two tables overlapping.
pub(crate) fn read_directory(
    data: &[u8],
    face_index: u32,
    limits: &WebFontLimits,
) -> Result<(u32, Vec<TableRecord>), FontDecodeError> {
    let mut offset = 0usize;
    let mut version = data.u32_at(0).ok_or(FontDecodeError::Truncated)?;
    if version == COLLECTION {
        let count = data.u32_at(8).ok_or(FontDecodeError::Truncated)?;
        if face_index >= count {
            return Err(FontDecodeError::Malformed("no face at that index of the collection"));
        }
        let entry = 12usize + face_index as usize * 4;
        offset = data.u32_at(entry).ok_or(FontDecodeError::Truncated)? as usize;
        version = data.u32_at(offset).ok_or(FontDecodeError::Truncated)?;
    } else if face_index != 0 {
        return Err(FontDecodeError::Malformed("no face at that index of a single font"));
    }
    if !matches!(version, TRUETYPE | APPLE_TRUE | OPEN_TYPE_CFF) {
        return Err(FontDecodeError::UnknownFormat);
    }
    let count = data.u16_at(offset + 4).ok_or(FontDecodeError::Truncated)? as usize;
    if count == 0 {
        return Err(FontDecodeError::Malformed("the font has no tables"));
    }
    if count > limits.max_tables {
        return Err(FontDecodeError::LimitExceeded("tables"));
    }
    let directory = offset + 12;
    let mut records = Vec::with_capacity(count);
    for index in 0..count {
        let entry = directory + index * 16;
        let bytes = data.get(entry..entry + 16).ok_or(FontDecodeError::Truncated)?;
        let tag: Tag = [bytes[0], bytes[1], bytes[2], bytes[3]];
        let table_offset = bytes.u32_at(8).ok_or(FontDecodeError::Truncated)? as usize;
        let length = bytes.u32_at(12).ok_or(FontDecodeError::Truncated)? as usize;
        let end = table_offset.checked_add(length).ok_or(FontDecodeError::Truncated)?;
        if end > data.len() {
            return Err(FontDecodeError::Truncated);
        }
        records.push(TableRecord { tag, offset: table_offset, length });
    }
    records.sort_unstable_by_key(|record| record.tag);
    if records.windows(2).any(|pair| pair[0].tag == pair[1].tag) {
        return Err(FontDecodeError::Malformed("a table appears twice"));
    }
    let mut by_offset: Vec<&TableRecord> = records.iter().filter(|record| record.length > 0).collect();
    by_offset.sort_unstable_by_key(|record| record.offset);
    if by_offset.windows(2).any(|pair| pair[0].offset + pair[0].length > pair[1].offset) {
        return Err(FontDecodeError::Malformed("two tables overlap"));
    }
    Ok((version, records))
}

/// The checksum of a table: the sum of its big-endian words, the last one padded with zeros.
pub(crate) fn checksum(data: &[u8]) -> u32 {
    let mut chunks = data.chunks_exact(4);
    let mut sum = 0u32;
    for chunk in &mut chunks {
        sum = sum.wrapping_add(u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }
    let rest = chunks.remainder();
    if !rest.is_empty() {
        let mut last = [0u8; 4];
        last[..rest.len()].copy_from_slice(rest);
        sum = sum.wrapping_add(u32::from_be_bytes(last));
    }
    sum
}

const fn padded(length: usize) -> Option<usize> {
    match length.checked_add(3) {
        Some(value) => Some(value & !3),
        None => None,
    }
}

/// Writes a font of the given tables: a directory sorted by tag, each table on a four-byte
/// boundary, with the checksums and `head.checkSumAdjustment` computed.
pub(crate) fn assemble(
    version: u32,
    tables: &mut [(Tag, &[u8])],
    limits: &WebFontLimits,
) -> Result<Vec<u8>, FontDecodeError> {
    const TOO_LARGE: FontDecodeError = FontDecodeError::LimitExceeded("decoded size");
    tables.sort_unstable_by_key(|(tag, _)| *tag);
    let count = u16::try_from(tables.len()).map_err(|_| FontDecodeError::LimitExceeded("tables"))?;
    let header = 12 + tables.len() * 16;
    let mut total = header;
    for (_, table) in tables.iter() {
        total = total.checked_add(padded(table.len()).ok_or(TOO_LARGE)?).ok_or(TOO_LARGE)?;
    }
    if total > limits.max_decoded_size || u32::try_from(total).is_err() {
        return Err(TOO_LARGE);
    }
    let mut output = Vec::with_capacity(total);
    let entry_selector = 15 - count.max(1).leading_zeros() as u16;
    let search_range = (1u16 << entry_selector.min(12)).wrapping_mul(16);
    output.extend_from_slice(&version.to_be_bytes());
    output.extend_from_slice(&count.to_be_bytes());
    output.extend_from_slice(&search_range.to_be_bytes());
    output.extend_from_slice(&entry_selector.to_be_bytes());
    output.extend_from_slice(&count.wrapping_mul(16).wrapping_sub(search_range).to_be_bytes());
    let mut offset = header;
    let mut head_offset = None;
    for (tag, table) in tables.iter() {
        let sum = if *tag == *b"head" && table.len() >= 12 {
            head_offset = Some(offset);
            // The checksum of `head` is taken with its adjustment field at zero.
            checksum(table).wrapping_sub(table.u32_at(8).unwrap_or(0))
        } else {
            checksum(table)
        };
        output.extend_from_slice(tag);
        output.extend_from_slice(&sum.to_be_bytes());
        output.extend_from_slice(&(offset as u32).to_be_bytes());
        output.extend_from_slice(&(table.len() as u32).to_be_bytes());
        offset += padded(table.len()).ok_or(TOO_LARGE)?;
    }
    for (_, table) in tables.iter() {
        output.extend_from_slice(table);
        output.resize(padded(output.len()).ok_or(TOO_LARGE)?, 0);
    }
    if let Some(head) = head_offset {
        output[head + 8..head + 12].fill(0);
        let adjustment = 0xB1B0_AFBAu32.wrapping_sub(checksum(&output));
        output[head + 8..head + 12].copy_from_slice(&adjustment.to_be_bytes());
    }
    Ok(output)
}
