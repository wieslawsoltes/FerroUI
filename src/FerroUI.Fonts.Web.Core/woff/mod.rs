//! WOFF File Format 1.0.

use crate::sfnt::{ReadBe, Tag};
use crate::{FontDecodeError, WebFontLimits};
use flate2::{Decompress, FlushDecompress, Status};

#[cfg(test)]
mod tests;

pub(crate) const WOFF1_SIGNATURE: u32 = u32::from_be_bytes(*b"wOFF");

const HEADER_SIZE: usize = 44;
const ENTRY_SIZE: usize = 20;
/// The most bytes inflated between two checks of the budget.
const INFLATE_CHUNK: usize = 64 * 1024;

const fn malformed(what: &'static str) -> FontDecodeError {
    FontDecodeError::Malformed(what)
}

fn align4(value: usize) -> Option<usize> {
    Some(value.checked_add(3)? & !3)
}

/// Whether the bytes between two positions are at most three zeros up to a four-byte boundary.
fn valid_padding(data: &[u8], end: usize, next: usize) -> bool {
    align4(end) == Some(next) && data.get(end..next).is_some_and(|padding| padding.iter().all(|byte| *byte == 0))
}

#[derive(Clone, Copy)]
struct Entry {
    offset: usize,
    compressed: usize,
    original: usize,
    output: usize,
}

/// An optional block of the file: where it is and how long, zero when absent.
fn optional_block(data: &[u8], field: usize) -> Result<(usize, usize), FontDecodeError> {
    let offset = data.u32_at(field).ok_or(FontDecodeError::Truncated)? as usize;
    let length = data.u32_at(field + 4).ok_or(FontDecodeError::Truncated)? as usize;
    if (offset == 0) != (length == 0) {
        return Err(malformed("WOFF block with an offset or a length alone"));
    }
    Ok((offset, length))
}

/// Decodes the tables of a WOFF file into one buffer, which is a plain sfnt.
pub(crate) struct Woff1Decoder<'a> {
    data: &'a [u8],
    entries: Vec<Entry>,
    output: Vec<u8>,
    next: usize,
    position: usize,
    inflate: Option<Decompress>,
}

impl<'a> Woff1Decoder<'a> {
    /// Validates the whole layout of the file. The output is allocated only after that.
    pub(crate) fn new(data: &'a [u8], limits: &WebFontLimits) -> Result<Self, FontDecodeError> {
        const TRUNCATED: FontDecodeError = FontDecodeError::Truncated;
        if data.len() < HEADER_SIZE {
            return Err(TRUNCATED);
        }
        if data.u32_at(0) != Some(WOFF1_SIGNATURE) {
            return Err(FontDecodeError::UnknownFormat);
        }
        let flavor = data.u32_at(4).ok_or(TRUNCATED)?;
        if data.u32_at(8).ok_or(TRUNCATED)? as usize != data.len() {
            return Err(malformed("WOFF length"));
        }
        let count = data.u16_at(12).ok_or(TRUNCATED)? as usize;
        if data.u16_at(14) != Some(0) {
            return Err(malformed("WOFF reserved field"));
        }
        if count == 0 {
            return Err(malformed("WOFF without tables"));
        }
        if count > limits.max_tables {
            return Err(FontDecodeError::LimitExceeded("tables"));
        }
        let total = data.u32_at(16).ok_or(TRUNCATED)? as usize;
        if total % 4 != 0 {
            return Err(malformed("WOFF totalSfntSize"));
        }
        if total > limits.max_decoded_size {
            return Err(FontDecodeError::LimitExceeded("decoded size"));
        }
        let (meta_offset, meta_length) = optional_block(data, 24)?;
        let meta_original = data.u32_at(32).ok_or(TRUNCATED)?;
        if meta_length == 0 && meta_original != 0 {
            return Err(malformed("WOFF metadata length"));
        }
        let (private_offset, private_length) = optional_block(data, 36)?;

        let directory_end = HEADER_SIZE + count * ENTRY_SIZE;
        let directory = data.get(HEADER_SIZE..directory_end).ok_or(TRUNCATED)?;
        let mut tags: Vec<Tag> = Vec::with_capacity(count);
        let mut entries = Vec::with_capacity(count);
        let mut expected = 12usize + 16 * count;
        for index in 0..count {
            let entry = &directory[index * ENTRY_SIZE..(index + 1) * ENTRY_SIZE];
            let tag: Tag = [entry[0], entry[1], entry[2], entry[3]];
            if tags.last().is_some_and(|last| *last >= tag) {
                return Err(malformed("WOFF directory order"));
            }
            tags.push(tag);
            let offset = entry.u32_at(4).ok_or(TRUNCATED)? as usize;
            let compressed = entry.u32_at(8).ok_or(TRUNCATED)? as usize;
            let original = entry.u32_at(12).ok_or(TRUNCATED)? as usize;
            if compressed > original {
                return Err(malformed("WOFF table larger than its original"));
            }
            if offset % 4 != 0 {
                return Err(malformed("WOFF table alignment"));
            }
            if offset.checked_add(compressed).is_none_or(|end| end > data.len()) {
                return Err(TRUNCATED);
            }
            let output = expected;
            expected = align4(original)
                .and_then(|padded| expected.checked_add(padded))
                .filter(|size| *size <= limits.max_decoded_size)
                .ok_or(FontDecodeError::LimitExceeded("decoded size"))?;
            entries.push(Entry { offset, compressed, original, output });
        }
        if expected != total {
            return Err(malformed("WOFF totalSfntSize"));
        }

        // The blocks follow one another with nothing between them but padding.
        let mut by_offset: Vec<Entry> = entries.clone();
        by_offset.sort_unstable_by_key(|entry| entry.offset);
        let mut end = directory_end;
        for entry in &by_offset {
            if !valid_padding(data, end, entry.offset) {
                return Err(malformed("WOFF table layout"));
            }
            end = entry.offset + entry.compressed;
        }
        for (offset, length) in [(meta_offset, meta_length), (private_offset, private_length)] {
            if length == 0 {
                continue;
            }
            if !valid_padding(data, end, offset) {
                return Err(malformed("WOFF block layout"));
            }
            end = offset.checked_add(length).filter(|end| *end <= data.len()).ok_or(TRUNCATED)?;
        }
        // The last block ends the file, with at most its padding after it.
        if end != data.len() && !valid_padding(data, end, data.len()) {
            return Err(malformed("WOFF data after the last block"));
        }

        let mut output = vec![0u8; total];
        output[..4].copy_from_slice(&flavor.to_be_bytes());
        output[4..6].copy_from_slice(&(count as u16).to_be_bytes());
        let selector = 15 - (count as u16).leading_zeros() as u16;
        let range = (1u16 << selector).wrapping_mul(16);
        output[6..8].copy_from_slice(&range.to_be_bytes());
        output[8..10].copy_from_slice(&selector.to_be_bytes());
        output[10..12].copy_from_slice(&(count as u16).wrapping_mul(16).wrapping_sub(range).to_be_bytes());
        for (index, (tag, entry)) in tags.iter().zip(&entries).enumerate() {
            let record = 12 + index * 16;
            output[record..record + 4].copy_from_slice(tag);
            output[record + 8..record + 12].copy_from_slice(&(entry.output as u32).to_be_bytes());
            output[record + 12..record + 16].copy_from_slice(&(entry.original as u32).to_be_bytes());
        }
        Ok(Self { data, entries, output, next: 0, position: 0, inflate: None })
    }

    /// Decodes up to about `budget` bytes. Returns whether every table is done.
    pub(crate) fn step(&mut self, mut budget: usize) -> Result<bool, FontDecodeError> {
        while let Some(entry) = self.entries.get(self.next).copied() {
            if budget == 0 {
                return Ok(false);
            }
            if entry.compressed == entry.original {
                let source = &self.data[entry.offset..entry.offset + entry.original];
                self.output[entry.output..entry.output + entry.original].copy_from_slice(source);
                budget = budget.saturating_sub(entry.original.max(1));
                self.next += 1;
                continue;
            }
            if self.inflate_table(entry, &mut budget)? {
                self.next += 1;
                self.position = 0;
                self.inflate = None;
            }
        }
        Ok(true)
    }

    /// Inflates a table until it is done or the budget is spent. The output of a table can
    /// never go past its declared length.
    fn inflate_table(&mut self, entry: Entry, budget: &mut usize) -> Result<bool, FontDecodeError> {
        const BAD: FontDecodeError = FontDecodeError::Malformed("WOFF compressed table");
        let input = &self.data[entry.offset..entry.offset + entry.compressed];
        let inflate = self.inflate.get_or_insert_with(|| Decompress::new(true));
        while *budget > 0 && self.position < entry.original {
            let consumed = inflate.total_in() as usize;
            let chunk = (entry.original - self.position).min(INFLATE_CHUNK).min(*budget);
            let start = entry.output + self.position;
            let before = inflate.total_out();
            let status = inflate
                .decompress(&input[consumed..], &mut self.output[start..start + chunk], FlushDecompress::None)
                .map_err(|_| BAD)?;
            let written = (inflate.total_out() - before) as usize;
            self.position += written;
            *budget = budget.saturating_sub(written.max(1));
            if matches!(status, Status::StreamEnd) && self.position < entry.original {
                return Err(BAD);
            }
            if written == 0 && inflate.total_in() as usize == consumed {
                return Err(BAD);
            }
        }
        if self.position < entry.original {
            return Ok(false);
        }
        // Every byte is out. The stream must now end here, with its checksum and nothing after
        // it. One spare byte shows a stream that would write more than it declared.
        let mut spare = [0u8; 1];
        loop {
            let consumed = inflate.total_in() as usize;
            let before = inflate.total_out();
            let status =
                inflate.decompress(&input[consumed..], &mut spare, FlushDecompress::Finish).map_err(|_| BAD)?;
            if inflate.total_out() != before {
                return Err(BAD);
            }
            match status {
                Status::StreamEnd if inflate.total_in() as usize == input.len() => return Ok(true),
                Status::StreamEnd => return Err(BAD),
                _ if inflate.total_in() as usize == consumed => return Err(BAD),
                _ => {}
            }
        }
    }

    /// The decoded font, in canonical layout and with its checksums.
    pub(crate) fn into_font(mut self) -> Vec<u8> {
        crate::sfnt::update_checksums(&mut self.output);
        self.output
    }
}
