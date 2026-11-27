//! WOFF File Format 2.0.
//!
//! Brotli sits behind [`IBrotliDecoder`], so this crate stays free of it: the decoder is
//! supplied by `ferroui-fonts-web`, behind its feature `woff2`.

mod glyf;

#[cfg(any(test, feature = "testing"))]
#[doc(hidden)]
pub mod testing;
#[cfg(test)]
mod tests;

use crate::sfnt::{assemble, ReadBe, SanitizeOptions, Tag, APPLE_TRUE, COLLECTION, OPEN_TYPE_CFF, TRUETYPE};
use crate::{FontDecodeError, WebFontLimits};
use glyf::GlyphRebuilder;

pub(crate) const WOFF2_SIGNATURE: u32 = u32::from_be_bytes(*b"wOF2");

const HEADER_SIZE: usize = 48;
/// The most bytes decompressed between two checks of the budget.
const BROTLI_CHUNK: usize = 64 * 1024;

/// The known table tags of the WOFF2 directory, by the index in its flags.
pub(crate) const KNOWN_TAGS: [Tag; 63] = [
    *b"cmap", *b"head", *b"hhea", *b"hmtx", *b"maxp", *b"name", *b"OS/2", *b"post", *b"cvt ", *b"fpgm", *b"glyf",
    *b"loca", *b"prep", *b"CFF ", *b"VORG", *b"EBDT", *b"EBLC", *b"gasp", *b"hdmx", *b"kern", *b"LTSH", *b"PCLT",
    *b"VDMX", *b"vhea", *b"vmtx", *b"BASE", *b"GDEF", *b"GPOS", *b"GSUB", *b"EBSC", *b"JSTF", *b"MATH", *b"CBDT",
    *b"CBLC", *b"COLR", *b"CPAL", *b"SVG ", *b"sbix", *b"acnt", *b"avar", *b"bdat", *b"bloc", *b"bsln", *b"cvar",
    *b"fdsc", *b"feat", *b"fmtx", *b"fvar", *b"gvar", *b"hsty", *b"just", *b"lcar", *b"mort", *b"morx", *b"opbd",
    *b"prop", *b"trak", *b"Zapf", *b"Silf", *b"Glat", *b"Gloc", *b"Feat", *b"Sill",
];

const fn malformed(what: &'static str) -> FontDecodeError {
    FontDecodeError::Malformed(what)
}

/// What one call of a Brotli decoder ended with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrotliStatus {
    /// The stream ended.
    Finished,
    /// All the input was consumed and the stream is not over.
    NeedsMoreInput,
    /// The output is full and there is more to write.
    NeedsMoreOutput,
    /// The stream is invalid.
    Failed,
}

/// A resumable Brotli decoder.
pub trait IBrotliDecoder: Send {
    /// Decodes from `input` at `input_offset` into `output` at `output_offset`, moving both
    /// offsets by what it used. It never writes past the end of `output`.
    fn decompress(
        &mut self,
        input: &[u8],
        input_offset: &mut usize,
        output: &mut [u8],
        output_offset: &mut usize,
    ) -> BrotliStatus;
}

/// Makes a Brotli decoder for one stream.
pub type BrotliDecoderFactory = fn() -> Box<dyn IBrotliDecoder>;

/// A reader of the directory structures of WOFF2.
pub(crate) struct Reader<'a> {
    data: &'a [u8],
    pub(crate) position: usize,
}

impl<'a> Reader<'a> {
    pub(crate) fn new(data: &'a [u8], position: usize) -> Self {
        Self { data, position }
    }

    pub(crate) fn u8(&mut self) -> Result<u8, FontDecodeError> {
        let value = self.data.u8_at(self.position).ok_or(FontDecodeError::Truncated)?;
        self.position += 1;
        Ok(value)
    }

    pub(crate) fn u16(&mut self) -> Result<u16, FontDecodeError> {
        let value = self.data.u16_at(self.position).ok_or(FontDecodeError::Truncated)?;
        self.position += 2;
        Ok(value)
    }

    pub(crate) fn u32(&mut self) -> Result<u32, FontDecodeError> {
        let value = self.data.u32_at(self.position).ok_or(FontDecodeError::Truncated)?;
        self.position += 4;
        Ok(value)
    }

    /// `UIntBase128`: at most five bytes, no leading zeros, no value above 32 bits.
    pub(crate) fn base128(&mut self) -> Result<u32, FontDecodeError> {
        const BAD: FontDecodeError = FontDecodeError::Malformed("WOFF2 UIntBase128");
        let mut value = 0u32;
        for index in 0..5 {
            let byte = self.u8()?;
            if index == 0 && byte == 0x80 {
                return Err(BAD);
            }
            if value & 0xFE00_0000 != 0 {
                return Err(BAD);
            }
            value = (value << 7) | (byte & 0x7F) as u32;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
        }
        Err(BAD)
    }

    /// `255UInt16`.
    pub(crate) fn u255(&mut self) -> Result<u16, FontDecodeError> {
        match self.u8()? {
            253 => self.u16(),
            255 => Ok(self.u8()? as u16 + 253),
            254 => Ok(self.u8()? as u16 + 506),
            code => Ok(code as u16),
        }
    }
}

/// A table of the directory.
#[derive(Clone, Copy, Debug)]
struct DirectoryEntry {
    tag: Tag,
    original: usize,
    /// The length of the table in the decompressed stream.
    stream_length: usize,
    transformed: bool,
    stream_offset: usize,
}

/// A table of the face being decoded.
fn read_directory(
    reader: &mut Reader<'_>,
    count: usize,
    limits: &WebFontLimits,
) -> Result<(Vec<DirectoryEntry>, usize), FontDecodeError> {
    const TOO_LARGE: FontDecodeError = FontDecodeError::LimitExceeded("decoded size");
    let mut entries = Vec::with_capacity(count);
    let mut stream_length = 0usize;
    let mut sfnt_size = 12usize + count * 16;
    for _ in 0..count {
        let flags = reader.u8()?;
        let index = (flags & 0x3F) as usize;
        let tag = match KNOWN_TAGS.get(index) {
            Some(tag) => *tag,
            None => reader.u32()?.to_be_bytes(),
        };
        let version = flags >> 6;
        let original = reader.base128()? as usize;
        let transformed = match &tag {
            b"glyf" | b"loca" => match version {
                0 => true,
                3 => false,
                _ => return Err(malformed("WOFF2 transform version")),
            },
            b"hmtx" => match version {
                0 => false,
                1 => true,
                _ => return Err(malformed("WOFF2 transform version")),
            },
            _ if version == 0 => false,
            _ => return Err(malformed("WOFF2 transform version")),
        };
        let length = if transformed { reader.base128()? as usize } else { original };
        if tag == *b"loca" && transformed && length != 0 {
            return Err(malformed("WOFF2 loca transform length"));
        }
        let stream_offset = stream_length;
        stream_length = stream_length.checked_add(length).ok_or(TOO_LARGE)?;
        let padded = original.checked_add(3).ok_or(TOO_LARGE)? & !3;
        sfnt_size = sfnt_size.checked_add(padded).ok_or(TOO_LARGE)?;
        if stream_length > limits.max_decoded_size || sfnt_size > limits.max_decoded_size {
            return Err(TOO_LARGE);
        }
        entries.push(DirectoryEntry { tag, original, stream_length: length, transformed, stream_offset });
    }
    Ok((entries, stream_length))
}

/// The tables and flavor of the face at `face_index`, from the collection directory.
fn read_collection(
    reader: &mut Reader<'_>,
    entries: &[DirectoryEntry],
    face_index: u32,
) -> Result<(u32, Vec<usize>), FontDecodeError> {
    let version = reader.u32()?;
    if version != 0x0001_0000 && version != 0x0002_0000 {
        return Err(malformed("WOFF2 collection version"));
    }
    let fonts = reader.u255()? as usize;
    if fonts == 0 {
        return Err(malformed("WOFF2 collection without fonts"));
    }
    let mut selected = None;
    for font in 0..fonts {
        let count = reader.u255()? as usize;
        let flavor = reader.u32()?;
        if !matches!(flavor, TRUETYPE | APPLE_TRUE | OPEN_TYPE_CFF) {
            return Err(malformed("WOFF2 collection font flavor"));
        }
        let mut indices: Vec<usize> = Vec::with_capacity(count.min(entries.len()));
        for _ in 0..count {
            let index = reader.u255()? as usize;
            if index >= entries.len() {
                return Err(malformed("WOFF2 collection table index"));
            }
            if indices.iter().any(|known| entries[*known].tag == entries[index].tag) {
                return Err(malformed("WOFF2 collection repeats a table"));
            }
            indices.push(index);
        }
        let find = |tag: &Tag| indices.iter().copied().find(|index| entries[*index].tag == *tag);
        match (find(b"glyf"), find(b"loca")) {
            (Some(glyf), Some(loca)) if loca == glyf + 1 => {}
            (None, None) => {}
            _ => return Err(malformed("WOFF2 collection glyf and loca")),
        }
        if font as u32 == face_index {
            selected = Some((flavor, indices));
        }
    }
    selected.ok_or(malformed("no face at that index of the collection"))
}

/// An optional block: where it is and how long, zero when absent.
fn optional_block(data: &[u8], field: usize) -> Result<(usize, usize), FontDecodeError> {
    let offset = data.u32_at(field).ok_or(FontDecodeError::Truncated)? as usize;
    let length = data.u32_at(field + 4).ok_or(FontDecodeError::Truncated)? as usize;
    if (offset == 0) != (length == 0) {
        return Err(malformed("WOFF2 block with an offset or a length alone"));
    }
    Ok((offset, length))
}

fn valid_padding(data: &[u8], end: usize, next: usize) -> bool {
    end.checked_add(3).map(|value| value & !3) == Some(next)
        && data.get(end..next).is_some_and(|padding| padding.iter().all(|byte| *byte == 0))
}

enum Stage {
    Decompress,
    /// All output is written; the stream must end now.
    VerifyEnd,
    Prepare,
    Glyphs(Box<GlyphRebuilder>),
    Assemble {
        glyf: Option<(Vec<u8>, Vec<u8>)>,
        x_mins: Vec<i16>,
    },
    Done,
}

/// Decodes a WOFF2 file in steps.
pub(crate) struct Woff2Decoder<D> {
    data: D,
    /// Where the compressed stream is in `data`.
    input: std::ops::Range<usize>,
    brotli: Box<dyn IBrotliDecoder>,
    limits: WebFontLimits,
    flavor: u32,
    tables: Vec<DirectoryEntry>,
    stream: Vec<u8>,
    input_position: usize,
    output_position: usize,
    stage: Stage,
}

impl<D: AsRef<[u8]>> Woff2Decoder<D> {
    /// Validates the whole layout of the file. The stream is allocated only after that.
    pub(crate) fn new(
        source: D,
        options: &SanitizeOptions,
        brotli: BrotliDecoderFactory,
    ) -> Result<Self, FontDecodeError> {
        const TRUNCATED: FontDecodeError = FontDecodeError::Truncated;
        let data = source.as_ref();
        let limits = options.limits;
        if data.len() < HEADER_SIZE {
            return Err(TRUNCATED);
        }
        if data.u32_at(0) != Some(WOFF2_SIGNATURE) {
            return Err(FontDecodeError::UnknownFormat);
        }
        let flavor = data.u32_at(4).ok_or(TRUNCATED)?;
        if data.u32_at(8).ok_or(TRUNCATED)? as usize != data.len() {
            return Err(malformed("WOFF2 length"));
        }
        let count = data.u16_at(12).ok_or(TRUNCATED)? as usize;
        if count == 0 {
            return Err(malformed("WOFF2 without tables"));
        }
        if count > limits.max_tables {
            return Err(FontDecodeError::LimitExceeded("tables"));
        }
        let compressed = data.u32_at(20).ok_or(TRUNCATED)? as usize;
        let (meta_offset, meta_length) = optional_block(data, 28)?;
        if meta_length == 0 && data.u32_at(36) != Some(0) {
            return Err(malformed("WOFF2 metadata length"));
        }
        let (private_offset, private_length) = optional_block(data, 40)?;

        let mut reader = Reader::new(data, HEADER_SIZE);
        let (entries, stream_length) = read_directory(&mut reader, count, &limits)?;
        let (flavor, selected) = if flavor == COLLECTION {
            read_collection(&mut reader, &entries, options.face_index)?
        } else {
            if options.face_index != 0 {
                return Err(malformed("no face at that index of a single font"));
            }
            for (index, entry) in entries.iter().enumerate() {
                if entries[..index].iter().any(|other| other.tag == entry.tag) {
                    return Err(malformed("WOFF2 repeats a table"));
                }
            }
            (flavor, (0..entries.len()).collect())
        };
        let tables: Vec<DirectoryEntry> = selected.iter().map(|index| entries[*index]).collect();
        let find = |tag: &Tag| tables.iter().find(|entry| entry.tag == *tag);
        match (find(b"glyf"), find(b"loca")) {
            (Some(glyf), Some(loca)) if glyf.transformed == loca.transformed => {}
            (None, None) => {}
            _ => return Err(malformed("WOFF2 glyf and loca")),
        }
        if find(b"hmtx").is_some_and(|hmtx| hmtx.transformed) && !find(b"glyf").is_some_and(|glyf| glyf.transformed) {
            return Err(malformed("WOFF2 hmtx transform without a glyf transform"));
        }

        let start = reader.position;
        let mut end = start.checked_add(compressed).filter(|end| *end <= data.len()).ok_or(TRUNCATED)?;
        let input = start..end;
        // The compressed data is padded to four bytes, even at the end of the file: decoders of
        // browsers refuse a file that is not.
        if end.checked_add(3).map(|value| value & !3).is_none_or(|padded| padded > data.len()) {
            return Err(malformed("WOFF2 compressed data without its padding"));
        }
        for (offset, length) in [(meta_offset, meta_length), (private_offset, private_length)] {
            if length == 0 {
                continue;
            }
            if !valid_padding(data, end, offset) {
                return Err(malformed("WOFF2 block layout"));
            }
            end = offset.checked_add(length).filter(|end| *end <= data.len()).ok_or(TRUNCATED)?;
        }
        if end != data.len() && !valid_padding(data, end, data.len()) {
            return Err(malformed("WOFF2 data after the last block"));
        }
        Ok(Self {
            data: source,
            input,
            brotli: brotli(),
            limits,
            flavor,
            tables,
            stream: vec![0u8; stream_length],
            input_position: 0,
            output_position: 0,
            stage: Stage::Decompress,
        })
    }

    fn table(&self, tag: &Tag) -> Option<&DirectoryEntry> {
        self.tables.iter().find(|entry| entry.tag == *tag)
    }

    fn table_bytes(&self, entry: &DirectoryEntry) -> &[u8] {
        &self.stream[entry.stream_offset..entry.stream_offset + entry.stream_length]
    }

    fn step_decompress(&mut self, budget: &mut usize) -> Result<bool, FontDecodeError> {
        const BAD: FontDecodeError = FontDecodeError::Malformed("WOFF2 compressed stream");
        let input = &self.data.as_ref()[self.input.clone()];
        while *budget > 0 {
            if self.output_position == self.stream.len() {
                self.stage = Stage::VerifyEnd;
                return Ok(true);
            }
            let chunk = (self.stream.len() - self.output_position).min(BROTLI_CHUNK).min(*budget);
            let before = (self.input_position, self.output_position);
            let window = &mut self.stream[self.output_position..self.output_position + chunk];
            let mut written = 0;
            let status = self.brotli.decompress(input, &mut self.input_position, window, &mut written);
            if written > chunk || self.input_position > input.len() {
                return Err(BAD);
            }
            self.output_position += written;
            *budget = budget.saturating_sub(written.max(1));
            match status {
                BrotliStatus::Finished => {
                    if self.output_position != self.stream.len() || self.input_position != input.len() {
                        return Err(BAD);
                    }
                    self.stage = Stage::Prepare;
                    return Ok(true);
                }
                BrotliStatus::Failed => return Err(BAD),
                BrotliStatus::NeedsMoreInput if self.input_position == input.len() && written == 0 => {
                    return Err(FontDecodeError::Truncated);
                }
                _ => {}
            }
            if (self.input_position, self.output_position) == before {
                return Err(BAD);
            }
        }
        Ok(false)
    }

    /// The stream has given all it declared. It must end here, with no input left. One spare
    /// byte shows a stream that would write more than it declared.
    fn verify_end(&mut self) -> Result<(), FontDecodeError> {
        const BAD: FontDecodeError = FontDecodeError::Malformed("WOFF2 compressed stream");
        let input = &self.data.as_ref()[self.input.clone()];
        let mut spare = [0u8; 1];
        loop {
            let before = self.input_position;
            let mut written = 0;
            let status = self.brotli.decompress(input, &mut self.input_position, &mut spare, &mut written);
            if written != 0 {
                return Err(BAD);
            }
            match status {
                BrotliStatus::Finished if self.input_position == input.len() => {
                    self.stage = Stage::Prepare;
                    return Ok(());
                }
                BrotliStatus::Finished | BrotliStatus::Failed => return Err(BAD),
                BrotliStatus::NeedsMoreInput if self.input_position == input.len() => {
                    return Err(FontDecodeError::Truncated);
                }
                _ if self.input_position == before => return Err(BAD),
                _ => {}
            }
        }
    }

    /// Starts the reconstruction of `glyf` and `loca`, when they are transformed.
    fn prepare(&mut self) -> Result<(), FontDecodeError> {
        let (Some(glyf), Some(loca)) = (self.table(b"glyf").copied(), self.table(b"loca").copied()) else {
            self.stage = Stage::Assemble { glyf: None, x_mins: Vec::new() };
            return Ok(());
        };
        if !glyf.transformed {
            self.stage = Stage::Assemble { glyf: None, x_mins: Vec::new() };
            return Ok(());
        }
        let head = self.table(b"head").copied().ok_or(FontDecodeError::MissingTable(*b"head"))?;
        let maxp = self.table(b"maxp").copied().ok_or(FontDecodeError::MissingTable(*b"maxp"))?;
        let long_loca = self.table_bytes(&head).i16_at(50).ok_or(malformed("head"))?;
        let glyph_count = self.table_bytes(&maxp).u16_at(4).ok_or(malformed("maxp"))?;
        let rebuilder = GlyphRebuilder::new(
            &self.stream,
            glyf.stream_offset,
            glyf.stream_length,
            loca.original,
            glyph_count,
            long_loca,
            glyf.original,
            self.limits.max_decoded_size,
        )?;
        self.stage = Stage::Glyphs(Box::new(rebuilder));
        Ok(())
    }

    /// `hmtx` from its transform, with the left side bearings that were left out taken from
    /// the `xMin` of the rebuilt glyphs.
    fn rebuild_hmtx(&self, entry: &DirectoryEntry, x_mins: &[i16]) -> Result<Vec<u8>, FontDecodeError> {
        const BAD: FontDecodeError = FontDecodeError::Malformed("WOFF2 hmtx transform");
        let hhea = self.table(b"hhea").ok_or(FontDecodeError::MissingTable(*b"hhea"))?;
        let metrics = self.table_bytes(hhea).u16_at(34).ok_or(malformed("hhea"))? as usize;
        let glyphs = x_mins.len();
        if metrics == 0 || metrics > glyphs {
            return Err(BAD);
        }
        let table = self.table_bytes(entry);
        let flags = table.u8_at(0).ok_or(BAD)?;
        if flags & 0xFC != 0 || flags & 0x03 == 0 {
            return Err(BAD);
        }
        let (has_lsb, has_bearing) = (flags & 1 == 0, flags & 2 == 0);
        let advances = 1usize;
        let lsbs = advances + metrics * 2;
        let bearings = lsbs + if has_lsb { metrics * 2 } else { 0 };
        let end = bearings + if has_bearing { (glyphs - metrics) * 2 } else { 0 };
        if table.len() != end || entry.original != metrics * 4 + (glyphs - metrics) * 2 {
            return Err(BAD);
        }
        let mut output = Vec::with_capacity(entry.original);
        for index in 0..metrics {
            output.extend_from_slice(&table[advances + index * 2..advances + index * 2 + 2]);
            if has_lsb {
                output.extend_from_slice(&table[lsbs + index * 2..lsbs + index * 2 + 2]);
            } else {
                output.extend_from_slice(&x_mins[index].to_be_bytes());
            }
        }
        for index in metrics..glyphs {
            if has_bearing {
                let at = bearings + (index - metrics) * 2;
                output.extend_from_slice(&table[at..at + 2]);
            } else {
                output.extend_from_slice(&x_mins[index].to_be_bytes());
            }
        }
        Ok(output)
    }

    fn assemble(&self, glyf: Option<(Vec<u8>, Vec<u8>)>, x_mins: &[i16]) -> Result<Vec<u8>, FontDecodeError> {
        let hmtx = match self.table(b"hmtx") {
            Some(entry) if entry.transformed => Some(self.rebuild_hmtx(entry, x_mins)?),
            _ => None,
        };
        let mut tables: Vec<(Tag, &[u8])> = Vec::with_capacity(self.tables.len());
        for entry in &self.tables {
            let bytes: &[u8] = match (&entry.tag, &glyf, &hmtx) {
                (b"glyf", Some((rebuilt, _)), _) => rebuilt,
                (b"loca", Some((_, rebuilt)), _) => rebuilt,
                (b"hmtx", _, Some(rebuilt)) => rebuilt,
                _ => self.table_bytes(entry),
            };
            tables.push((entry.tag, bytes));
        }
        assemble(self.flavor, &mut tables, &self.limits)
    }

    /// Does up to about `budget` bytes of work. Returns the font when it is finished.
    pub(crate) fn step(&mut self, budget: usize) -> Result<Option<Vec<u8>>, FontDecodeError> {
        let mut budget = budget.max(1);
        loop {
            match std::mem::replace(&mut self.stage, Stage::Done) {
                Stage::Decompress => {
                    self.stage = Stage::Decompress;
                    if !self.step_decompress(&mut budget)? {
                        return Ok(None);
                    }
                }
                Stage::VerifyEnd => self.verify_end()?,
                Stage::Prepare => self.prepare()?,
                Stage::Glyphs(mut rebuilder) => {
                    if !rebuilder.step(&self.stream, &mut budget)? {
                        self.stage = Stage::Glyphs(rebuilder);
                        return Ok(None);
                    }
                    let (glyf, loca, x_mins) = rebuilder.finish()?;
                    self.stage = Stage::Assemble { glyf: Some((glyf, loca)), x_mins };
                }
                Stage::Assemble { glyf, x_mins } => {
                    let font = self.assemble(glyf, &x_mins)?;
                    self.stream = Vec::new();
                    return Ok(Some(font));
                }
                Stage::Done => return Err(malformed("the WOFF2 decoder has already finished")),
            }
            if budget == 0 {
                return Ok(None);
            }
        }
    }
}
