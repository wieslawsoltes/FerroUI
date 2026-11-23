//! Applying glyph-keyed patches (section 6.3 of the standard), in steps, so that a thread that
//! must stay responsive can give the work a budget at a time.

use super::map::PatchMap;
use super::{IFTX_TABLE, IFT_TABLE};
use crate::sfnt::{assemble, read_directory, ReadBe, TableRecord, Tag};
use crate::woff2::{BrotliDecoderFactory, BrotliStatus, IBrotliDecoder};
use crate::{FontDecodeError, WebFontLimits};
use std::ops::Range;

const MALFORMED: FontDecodeError = FontDecodeError::Malformed("glyph patch");
const TOO_LARGE: FontDecodeError = FontDecodeError::LimitExceeded("decoded size");
const PATCH_TAG: &[u8; 4] = b"ifgk";
/// `format`, `reserved`, `flags`, `compatibilityId` and `maxUncompressedLength`.
const PATCH_HEADER: usize = 29;
/// The most output of one decompression call, so that memory follows what a stream really holds.
const DECOMPRESS_CHUNK: usize = 256 * 1024;
/// Ids are unique and below `numGlyphs`, which is a 16-bit value.
const MAX_GLYPHS: usize = 0x1_0000;
/// The most data offsets of one patch: every glyph in several tables, and a limit on the work of
/// validating a hostile one.
const MAX_OFFSETS: usize = 1 << 20;

/// A patch that has been read: its decompressed `GlyphPatches` and the URL it came from.
struct Patch {
    url: String,
    map: usize,
    wide_ids: bool,
    max_length: usize,
    encoded: Vec<u8>,
    decoded: Vec<u8>,
}

/// Where a table keeps the data of its glyphs.
enum Layout {
    Glyf { long: bool, loca: Range<usize> },
    Gvar { long: bool, offsets: usize, data: usize, shared: Range<usize> },
    /// A CFF or CFF2 CharStrings INDEX, the last element of the table.
    Index { start: usize, count_size: usize, off_size: usize, offsets: usize, data: usize },
}

/// One table being written again, a glyph at a time.
struct Rebuild {
    tag: Tag,
    base: Range<usize>,
    layout: Layout,
    /// For each glyph, the patch and the range of its new data.
    glyphs: Vec<Option<(u32, u32, u32)>>,
    next: usize,
    data: Vec<u8>,
    offsets: Vec<u32>,
}

enum Stage {
    Decompress { patch: usize, decoder: Box<dyn IBrotliDecoder>, input: usize, written: usize },
    Rebuild { index: usize },
    Finish,
    Done,
}

/// Applies a batch of glyph-keyed patches to a font. No-invalidation patches can be applied
/// together (section 4.3), so the font is written once for all of them.
pub struct GlyphPatchJob {
    font: Vec<u8>,
    version: u32,
    records: Vec<TableRecord>,
    maps: Vec<PatchMap>,
    limits: WebFontLimits,
    brotli: BrotliDecoderFactory,
    patches: Vec<Patch>,
    rebuilds: Vec<Rebuild>,
    stage: Stage,
}

impl GlyphPatchJob {
    /// Starts applying `patches`, each with the URL its map writes for it. Headers are checked
    /// here, before anything is decompressed.
    pub fn new(
        font: Vec<u8>,
        maps: Vec<PatchMap>,
        patches: Vec<(String, Vec<u8>)>,
        brotli: BrotliDecoderFactory,
        limits: WebFontLimits,
    ) -> Result<Self, FontDecodeError> {
        let (version, records) = read_directory(&font, 0, &limits)?;
        let mut read = Vec::with_capacity(patches.len());
        for (url, encoded) in patches {
            let header = encoded.get(..PATCH_HEADER).ok_or(FontDecodeError::Truncated)?;
            if &header[..4] != PATCH_TAG {
                return Err(FontDecodeError::Unsupported("patch format"));
            }
            let map = maps
                .iter()
                .position(|map| map.compatibility_id()[..] == header[9..25])
                .ok_or(FontDecodeError::Malformed("the patch is for another font"))?;
            let max_length = header.u32_at(25).ok_or(FontDecodeError::Truncated)? as usize;
            if max_length > limits.max_decoded_size {
                return Err(TOO_LARGE);
            }
            read.push(Patch { url, map, wide_ids: header[8] & 1 != 0, max_length, encoded, decoded: Vec::new() });
        }
        let stage = if read.is_empty() {
            Stage::Finish
        } else {
            Stage::Decompress { patch: 0, decoder: brotli(), input: PATCH_HEADER, written: 0 }
        };
        Ok(Self { font, version, records, maps, limits, brotli, patches: read, rebuilds: Vec::new(), stage })
    }

    /// Does up to about `budget` bytes of work. Returns the extended font when it is finished.
    pub fn step(&mut self, budget: usize) -> Result<Option<Vec<u8>>, FontDecodeError> {
        let mut budget = budget.max(1);
        let result = loop {
            match self.advance(&mut budget) {
                Ok(Some(font)) => break Ok(Some(font)),
                Ok(None) if budget == 0 => break Ok(None),
                Ok(None) => {}
                Err(error) => break Err(error),
            }
        };
        if !matches!(result, Ok(None)) {
            self.stage = Stage::Done;
        }
        result
    }

    fn advance(&mut self, budget: &mut usize) -> Result<Option<Vec<u8>>, FontDecodeError> {
        match &mut self.stage {
            Stage::Decompress { patch, decoder, input, written } => {
                let index = *patch;
                let entry = &mut self.patches[index];
                let wanted = (*budget).clamp(1, DECOMPRESS_CHUNK);
                let end = written.saturating_add(wanted).min(entry.max_length);
                if entry.decoded.len() < end {
                    entry.decoded.resize(end, 0);
                }
                let before = *written;
                let status = decoder.decompress(&entry.encoded, input, &mut entry.decoded[..end], written);
                *budget = budget.saturating_sub((*written - before).max(1));
                match status {
                    BrotliStatus::Finished => {
                        entry.decoded.truncate(*written);
                        entry.encoded = Vec::new();
                        self.index_patch(index)?;
                        self.stage = if index + 1 < self.patches.len() {
                            Stage::Decompress {
                                patch: index + 1,
                                decoder: (self.brotli)(),
                                input: PATCH_HEADER,
                                written: 0,
                            }
                        } else {
                            Stage::Rebuild { index: 0 }
                        };
                    }
                    BrotliStatus::NeedsMoreOutput if *written >= entry.max_length => return Err(TOO_LARGE),
                    BrotliStatus::NeedsMoreOutput => {}
                    BrotliStatus::NeedsMoreInput => return Err(FontDecodeError::Truncated),
                    BrotliStatus::Failed => return Err(MALFORMED),
                }
                Ok(None)
            }
            Stage::Rebuild { index } => {
                let at = *index;
                let Some(rebuild) = self.rebuilds.get_mut(at) else {
                    self.stage = Stage::Finish;
                    return Ok(None);
                };
                if rebuild.step(&self.font, &self.patches, &self.limits, budget)? {
                    self.stage = Stage::Rebuild { index: at + 1 };
                }
                Ok(None)
            }
            Stage::Finish => self.finish().map(Some),
            Stage::Done => Err(FontDecodeError::Malformed("the patch job has already finished")),
        }
    }

    fn table_range(&self, tag: &Tag) -> Option<Range<usize>> {
        let record = self.records.iter().find(|record| record.tag == *tag)?;
        Some(record.offset..record.offset + record.length)
    }

    /// Validates a decompressed patch and notes which glyphs it replaces.
    fn index_patch(&mut self, index: usize) -> Result<(), FontDecodeError> {
        let decoded = std::mem::take(&mut self.patches[index].decoded);
        let result = self.index_decoded(index, &decoded);
        self.patches[index].decoded = decoded;
        result
    }

    fn index_decoded(&mut self, index: usize, data: &[u8]) -> Result<(), FontDecodeError> {
        let glyph_count = data.u32_at(0).ok_or(FontDecodeError::Truncated)? as usize;
        let table_count = data.u8_at(4).ok_or(FontDecodeError::Truncated)? as usize;
        if glyph_count > MAX_GLYPHS || glyph_count * table_count >= MAX_OFFSETS {
            return Err(MALFORMED);
        }
        let id_size = if self.patches[index].wide_ids { 3 } else { 2 };
        let tables_at = 5 + glyph_count * id_size;
        let offsets_at = tables_at + table_count * 4;
        let offset_count = glyph_count * table_count + 1;
        let data_at = offsets_at + offset_count * 4;
        if data_at > data.len() {
            return Err(FontDecodeError::Truncated);
        }
        let mut ids = Vec::with_capacity(glyph_count);
        for position in (5..tables_at).step_by(id_size.max(1)) {
            let id = if id_size == 2 {
                u32::from(data.u16_at(position).ok_or(FontDecodeError::Truncated)?)
            } else {
                u32::from(data[position]) << 16 | u32::from(data[position + 1]) << 8 | u32::from(data[position + 2])
            };
            if ids.last().is_some_and(|last| *last >= id) {
                return Err(MALFORMED);
            }
            ids.push(id);
        }
        let mut previous = data_at as u32;
        for at in 0..offset_count {
            let offset = data.u32_at(offsets_at + at * 4).ok_or(FontDecodeError::Truncated)?;
            if offset < previous || offset as usize > data.len() {
                return Err(MALFORMED);
            }
            previous = offset;
        }
        let mut last_tag = None;
        for table in 0..table_count {
            let tag: Tag = data.u32_at(tables_at + table * 4).ok_or(FontDecodeError::Truncated)?.to_be_bytes();
            if last_tag.is_some_and(|last| last >= tag) {
                return Err(MALFORMED);
            }
            last_tag = Some(tag);
            if !matches!(&tag, b"glyf" | b"gvar" | b"CFF " | b"CFF2") {
                continue;
            }
            let rebuild = self.rebuild_for(tag, index)?;
            for (glyph, id) in ids.iter().enumerate() {
                let at = offsets_at + (table * glyph_count + glyph) * 4;
                let start = data.u32_at(at).ok_or(FontDecodeError::Truncated)?;
                let end = data.u32_at(at + 4).ok_or(FontDecodeError::Truncated)?;
                let slot = self.rebuilds[rebuild].glyphs.get_mut(*id as usize).ok_or(MALFORMED)?;
                *slot = Some((index as u32, start, end));
            }
        }
        Ok(())
    }

    fn rebuild_for(&mut self, tag: Tag, patch: usize) -> Result<usize, FontDecodeError> {
        if let Some(found) = self.rebuilds.iter().position(|rebuild| rebuild.tag == tag) {
            return Ok(found);
        }
        let base = self.table_range(&tag).ok_or(FontDecodeError::MissingTable(tag))?;
        let table = &self.font[base.clone()];
        let (layout, count) = match &tag {
            b"glyf" => {
                let head = self.table_range(b"head").ok_or(FontDecodeError::MissingTable(*b"head"))?;
                let maxp = self.table_range(b"maxp").ok_or(FontDecodeError::MissingTable(*b"maxp"))?;
                let loca = self.table_range(b"loca").ok_or(FontDecodeError::MissingTable(*b"loca"))?;
                let long = self.font[head].u16_at(50).ok_or(MALFORMED)? != 0;
                let count = self.font[maxp].u16_at(4).ok_or(MALFORMED)? as usize;
                if loca.len() < (count + 1) * if long { 4 } else { 2 } {
                    return Err(MALFORMED);
                }
                (Layout::Glyf { long, loca }, count)
            }
            b"gvar" => {
                let axes = table.u16_at(4).ok_or(MALFORMED)? as usize;
                let tuples = table.u16_at(6).ok_or(MALFORMED)? as usize;
                let shared_at = table.u32_at(8).ok_or(MALFORMED)? as usize;
                let count = table.u16_at(12).ok_or(MALFORMED)? as usize;
                let long = table.u16_at(14).ok_or(MALFORMED)? & 1 != 0;
                let data = table.u32_at(16).ok_or(MALFORMED)? as usize;
                let shared = shared_at..shared_at + tuples * axes * 2;
                if table.get(shared.clone()).is_none() || data > table.len() {
                    return Err(MALFORMED);
                }
                if 20 + (count + 1) * if long { 4 } else { 2 } > table.len() {
                    return Err(FontDecodeError::Truncated);
                }
                (Layout::Gvar { long, offsets: 20, data, shared }, count)
            }
            _ => {
                let own = self.maps[self.patches[patch].map].charstrings_offset(&tag);
                let start = own.or_else(|| self.maps.iter().find_map(|map| map.charstrings_offset(&tag)));
                let start = start.ok_or(MALFORMED)? as usize;
                let count_size = if tag == *b"CFF " { 2 } else { 4 };
                let count = if count_size == 2 {
                    table.u16_at(start).ok_or(MALFORMED)? as usize
                } else {
                    table.u32_at(start).ok_or(MALFORMED)? as usize
                };
                if count == 0 || count > MAX_GLYPHS {
                    return Err(MALFORMED);
                }
                let off_size = table.u8_at(start + count_size).ok_or(MALFORMED)? as usize;
                if !(1..=4).contains(&off_size) {
                    return Err(MALFORMED);
                }
                let offsets = start + count_size + 1;
                let data = offsets + (count + 1) * off_size;
                if data > table.len() {
                    return Err(FontDecodeError::Truncated);
                }
                (Layout::Index { start, count_size, off_size, offsets, data }, count)
            }
        };
        self.rebuilds.push(Rebuild {
            tag,
            base,
            layout,
            glyphs: vec![None; count],
            next: 0,
            data: Vec::new(),
            offsets: Vec::with_capacity(count + 1),
        });
        Ok(self.rebuilds.len() - 1)
    }

    fn finish(&mut self) -> Result<Vec<u8>, FontDecodeError> {
        let mut written: Vec<(Tag, Vec<u8>)> = Vec::new();
        for rebuild in &mut self.rebuilds {
            rebuild.write(&self.font, &self.limits, &mut written)?;
        }
        for (index, map) in self.maps.iter().enumerate() {
            let tag = map.tag();
            debug_assert!(tag == IFT_TABLE || tag == IFTX_TABLE);
            let range = self.records.iter().find(|record| record.tag == tag).ok_or(FontDecodeError::MissingTable(tag))?;
            let mut table = self.font[range.offset..][..range.length].to_vec();
            for patch in self.patches.iter().filter(|patch| patch.map == index) {
                map.remove_entry(&mut table, &patch.url)?;
            }
            written.push((tag, table));
        }
        let mut tables: Vec<(Tag, &[u8])> = Vec::with_capacity(self.records.len());
        for record in &self.records {
            match written.iter().find(|(tag, _)| *tag == record.tag) {
                Some((tag, bytes)) => tables.push((*tag, bytes)),
                None => tables.push((record.tag, &self.font[record.offset..][..record.length])),
            }
        }
        assemble(self.version, &mut tables, &self.limits)
    }
}

impl Rebuild {
    /// The bytes a glyph has in the base table.
    fn base_glyph<'a>(&self, font: &'a [u8], glyph: usize) -> Result<&'a [u8], FontDecodeError> {
        let table = &font[self.base.clone()];
        let range = match &self.layout {
            Layout::Glyf { long, loca } => {
                let loca = &font[loca.clone()];
                if *long {
                    let start = loca.u32_at(glyph * 4).ok_or(MALFORMED)? as usize;
                    start..loca.u32_at(glyph * 4 + 4).ok_or(MALFORMED)? as usize
                } else {
                    let start = loca.u16_at(glyph * 2).ok_or(MALFORMED)? as usize * 2;
                    start..loca.u16_at(glyph * 2 + 2).ok_or(MALFORMED)? as usize * 2
                }
            }
            Layout::Gvar { long, offsets, data, .. } => {
                if *long {
                    let start = table.u32_at(offsets + glyph * 4).ok_or(MALFORMED)? as usize;
                    data + start..data + table.u32_at(offsets + glyph * 4 + 4).ok_or(MALFORMED)? as usize
                } else {
                    let start = table.u16_at(offsets + glyph * 2).ok_or(MALFORMED)? as usize * 2;
                    data + start..data + table.u16_at(offsets + glyph * 2 + 2).ok_or(MALFORMED)? as usize * 2
                }
            }
            Layout::Index { off_size, offsets, data, .. } => {
                let read = |index: usize| -> Result<usize, FontDecodeError> {
                    let at = offsets + index * off_size;
                    let bytes = table.get(at..at + off_size).ok_or(MALFORMED)?;
                    Ok(bytes.iter().fold(0usize, |value, byte| value << 8 | *byte as usize))
                };
                let (start, end) = (read(glyph)?, read(glyph + 1)?);
                if start == 0 || end == 0 {
                    return Err(MALFORMED);
                }
                data - 1 + start..data - 1 + end
            }
        };
        if range.start > range.end {
            return Err(MALFORMED);
        }
        table.get(range).ok_or(MALFORMED)
    }

    /// Copies glyphs until the budget is spent. Returns whether the table is done.
    fn step(
        &mut self,
        font: &[u8],
        patches: &[Patch],
        limits: &WebFontLimits,
        budget: &mut usize,
    ) -> Result<bool, FontDecodeError> {
        // Offsets of a short format are in units of two bytes.
        let even = match self.layout {
            Layout::Glyf { long, .. } => !long,
            Layout::Gvar { .. } => true,
            Layout::Index { .. } => false,
        };
        while self.next < self.glyphs.len() {
            let glyph = self.next;
            let bytes = match self.glyphs[glyph] {
                Some((patch, start, end)) => {
                    patches[patch as usize].decoded.get(start as usize..end as usize).ok_or(MALFORMED)?
                }
                None => self.base_glyph(font, glyph)?,
            };
            if self.data.len() + bytes.len() + 1 > limits.max_decoded_size {
                return Err(TOO_LARGE);
            }
            self.offsets.push(self.data.len() as u32);
            self.data.extend_from_slice(bytes);
            if even && self.data.len() % 2 != 0 {
                self.data.push(0);
            }
            self.next += 1;
            *budget = budget.saturating_sub(bytes.len() + 8);
            if *budget == 0 && self.next < self.glyphs.len() {
                return Ok(false);
            }
        }
        self.offsets.push(self.data.len() as u32);
        Ok(true)
    }

    /// The new table, and `loca` for `glyf`.
    fn write(
        &mut self,
        font: &[u8],
        limits: &WebFontLimits,
        output: &mut Vec<(Tag, Vec<u8>)>,
    ) -> Result<(), FontDecodeError> {
        let table = &font[self.base.clone()];
        match &self.layout {
            Layout::Glyf { long, loca } => {
                let last = self.data.len();
                if !*long && last > 0x1_FFFE {
                    return Err(FontDecodeError::Malformed("the glyphs do not fit a short loca"));
                }
                let mut new_loca = Vec::with_capacity(loca.len());
                for offset in &self.offsets {
                    if *long {
                        new_loca.extend_from_slice(&offset.to_be_bytes());
                    } else {
                        new_loca.extend_from_slice(&((*offset / 2) as u16).to_be_bytes());
                    }
                }
                output.push((*b"glyf", std::mem::take(&mut self.data)));
                output.push((*b"loca", new_loca));
            }
            Layout::Gvar { long, shared, .. } => {
                // A short table stays short while it fits; offsets may only grow.
                let long = *long || self.data.len() > 0x1_FFFE;
                let offsets_size = self.offsets.len() * if long { 4 } else { 2 };
                let shared_at = 20 + offsets_size;
                let data_at = shared_at + shared.len();
                let total = data_at + self.data.len();
                if total > limits.max_decoded_size || u32::try_from(total).is_err() {
                    return Err(TOO_LARGE);
                }
                let mut new = Vec::with_capacity(total);
                new.extend_from_slice(table.get(..20).ok_or(MALFORMED)?);
                new[8..12].copy_from_slice(&(shared_at as u32).to_be_bytes());
                let flags = (table.u16_at(14).ok_or(MALFORMED)? & !1) | u16::from(long);
                new[14..16].copy_from_slice(&flags.to_be_bytes());
                new[16..20].copy_from_slice(&(data_at as u32).to_be_bytes());
                for offset in &self.offsets {
                    if long {
                        new.extend_from_slice(&offset.to_be_bytes());
                    } else {
                        new.extend_from_slice(&((*offset / 2) as u16).to_be_bytes());
                    }
                }
                new.extend_from_slice(table.get(shared.clone()).ok_or(MALFORMED)?);
                new.extend_from_slice(&self.data);
                output.push((self.tag, new));
            }
            Layout::Index { start, count_size, .. } => {
                // Object offsets start at one.
                let last = self.data.len() + 1;
                let off_size = match last {
                    0..=0xFF => 1,
                    0x100..=0xFFFF => 2,
                    0x1_0000..=0xFF_FFFF => 3,
                    _ => 4,
                };
                if u32::try_from(last).is_err() {
                    return Err(TOO_LARGE);
                }
                let total = start + count_size + 1 + self.offsets.len() * off_size + self.data.len();
                if total > limits.max_decoded_size {
                    return Err(TOO_LARGE);
                }
                let mut new = Vec::with_capacity(total);
                new.extend_from_slice(table.get(..start + count_size).ok_or(MALFORMED)?);
                new.push(off_size as u8);
                for offset in &self.offsets {
                    new.extend_from_slice(&(offset + 1).to_be_bytes()[4 - off_size..]);
                }
                new.extend_from_slice(&self.data);
                output.push((self.tag, new));
            }
        }
        Ok(())
    }
}
