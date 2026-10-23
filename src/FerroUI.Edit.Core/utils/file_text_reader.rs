use super::{DecodingErrorPolicy, TextEncoding};
use crate::document::Utf16TextWriter;
use std::io::{self, BufRead, BufReader, Read};

/// Streaming encoded-byte reader with UTF-16 text-reader operations. It keeps
/// only a bounded detection prefix, an IO buffer and a partial scalar.
/// Invalid encoded UTF-16 is decoded according to `DecodingErrorPolicy`; use a
/// rope reader when existing isolated UTF-16 units must remain lossless.
pub struct FileTextReader<R> {
    source: BufReader<R>,
    prefix: Vec<u8>,
    prefix_position: usize,
    encoding: TextEncoding,
    error_policy: DecodingErrorPolicy,
    bom_length: usize,
    buffer: [u8; 4],
    buffered: usize,
    eof: bool,
    byte_position: u64,
    pending_unit: Option<u16>,
    peeked_unit: Option<u16>,
    deferred_error: Option<io::Error>,
}

impl<R: Read> FileTextReader<R> {
    /// Decodes an explicitly selected encoding without probing or stripping a
    /// BOM. This also supports unmarked UTF-16/32 and interactive streams.
    pub fn new(source: R, encoding: TextEncoding, error_policy: DecodingErrorPolicy) -> Self {
        Self::with_prefix(source, encoding, error_policy, Vec::new(), 0)
    }

    pub(super) fn with_prefix(
        source: R,
        encoding: TextEncoding,
        error_policy: DecodingErrorPolicy,
        prefix: Vec<u8>,
        bom_length: usize,
    ) -> Self {
        Self {
            source: BufReader::with_capacity(4096, source),
            prefix,
            prefix_position: bom_length,
            encoding,
            error_policy,
            bom_length,
            buffer: [0; 4],
            buffered: 0,
            eof: false,
            byte_position: bom_length as u64,
            pending_unit: None,
            peeked_unit: None,
            deferred_error: None,
        }
    }

    pub fn encoding(&self) -> TextEncoding {
        self.encoding
    }

    /// Bytes removed from an actual leading byte-order mark, never a guess
    /// based on the selected encoding's conventional preamble.
    pub fn byte_order_mark_length(&self) -> usize {
        self.bom_length
    }

    pub fn has_byte_order_mark(&self) -> bool {
        self.bom_length != 0
    }

    fn ensure_bytes(&mut self, count: usize) -> io::Result<()> {
        while self.buffered < count && !self.eof {
            if self.prefix_position < self.prefix.len() {
                let length = (count - self.buffered).min(self.prefix.len() - self.prefix_position);
                self.buffer[self.buffered..self.buffered + length].copy_from_slice(
                    &self.prefix[self.prefix_position..self.prefix_position + length],
                );
                self.prefix_position += length;
                self.buffered += length;
            } else {
                // Free the replay allocation once consumed rather than retain
                // up to 500 KB throughout a long document read.
                self.prefix = Vec::new();
                self.prefix_position = 0;
                match self.source.read(&mut self.buffer[self.buffered..count]) {
                    Ok(0) => self.eof = true,
                    Ok(length) => self.buffered += length,
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                    Err(error) => return Err(error),
                }
            }
        }
        Ok(())
    }

    fn consume_bytes(&mut self, count: usize) {
        self.buffer.copy_within(count..self.buffered, 0);
        self.buffered -= count;
        self.byte_position += count as u64;
    }

    fn invalid(&mut self, count: usize, replacement: char) -> io::Result<Option<char>> {
        let position = self.byte_position;
        self.consume_bytes(count);
        match self.error_policy {
            DecodingErrorPolicy::Replace => Ok(Some(replacement)),
            DecodingErrorPolicy::Strict => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid {} data at byte offset {position}", self.encoding),
            )),
        }
    }

    fn read_scalar(&mut self) -> io::Result<Option<char>> {
        self.ensure_bytes(1)?;
        if self.buffered == 0 {
            return Ok(None);
        }
        match self.encoding {
            TextEncoding::Utf8 => {
                let length = match self.buffer[0] {
                    0xc2..=0xdf => 2,
                    0xe0..=0xef => 3,
                    0xf0..=0xf4 => 4,
                    _ => 1,
                };
                self.ensure_bytes(length)?;
                let available = length.min(self.buffered);
                match std::str::from_utf8(&self.buffer[..available]) {
                    Ok(text) => {
                        let scalar = text.chars().next().unwrap();
                        self.consume_bytes(scalar.len_utf8());
                        Ok(Some(scalar))
                    }
                    Err(error) => self.invalid(
                        error.error_len().unwrap_or(available),
                        char::REPLACEMENT_CHARACTER,
                    ),
                }
            }
            TextEncoding::Utf16Le | TextEncoding::Utf16Be => {
                self.ensure_bytes(2)?;
                if self.buffered < 2 {
                    return self.invalid(self.buffered, char::REPLACEMENT_CHARACTER);
                }
                let decode = if self.encoding == TextEncoding::Utf16Le {
                    u16::from_le_bytes
                } else {
                    u16::from_be_bytes
                };
                let first = decode([self.buffer[0], self.buffer[1]]);
                if (0xdc00..=0xdfff).contains(&first) {
                    return self.invalid(2, char::REPLACEMENT_CHARACTER);
                }
                if !(0xd800..=0xdbff).contains(&first) {
                    self.consume_bytes(2);
                    return Ok(char::from_u32(first as u32));
                }
                self.ensure_bytes(4)?;
                if self.buffered >= 4 {
                    let second = decode([self.buffer[2], self.buffer[3]]);
                    if (0xdc00..=0xdfff).contains(&second) {
                        let scalar =
                            0x10000 + (((first as u32 - 0xd800) << 10) | (second as u32 - 0xdc00));
                        self.consume_bytes(4);
                        return Ok(char::from_u32(scalar));
                    }
                }
                self.invalid(2, char::REPLACEMENT_CHARACTER)
            }
            TextEncoding::Utf32Le | TextEncoding::Utf32Be => {
                self.ensure_bytes(4)?;
                if self.buffered < 4 {
                    return self.invalid(self.buffered, char::REPLACEMENT_CHARACTER);
                }
                let scalar = if self.encoding == TextEncoding::Utf32Le {
                    u32::from_le_bytes(self.buffer)
                } else {
                    u32::from_be_bytes(self.buffer)
                };
                if let Some(scalar) = char::from_u32(scalar) {
                    self.consume_bytes(4);
                    Ok(Some(scalar))
                } else {
                    self.invalid(4, char::REPLACEMENT_CHARACTER)
                }
            }
            TextEncoding::Ascii if self.buffer[0] >= 0x80 => self.invalid(1, '?'),
            TextEncoding::Windows1252 => {
                const C1: [char; 32] = [
                    '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8d}',
                    'Ž', '\u{8f}', '\u{90}', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›',
                    'œ', '\u{9d}', 'ž', 'Ÿ',
                ];
                let byte = self.buffer[0];
                let scalar = if (0x80..=0x9f).contains(&byte) {
                    C1[(byte - 0x80) as usize]
                } else {
                    byte as char
                };
                self.consume_bytes(1);
                Ok(Some(scalar))
            }
            TextEncoding::Ascii | TextEncoding::Latin1 => {
                let scalar = self.buffer[0] as char;
                self.consume_bytes(1);
                Ok(Some(scalar))
            }
        }
    }

    pub fn peek(&mut self) -> io::Result<Option<u16>> {
        if self.peeked_unit.is_none() {
            self.peeked_unit = self.read()?;
        }
        Ok(self.peeked_unit)
    }

    pub fn read(&mut self) -> io::Result<Option<u16>> {
        if let Some(unit) = self.peeked_unit.take() {
            return Ok(Some(unit));
        }
        if let Some(error) = self.deferred_error.take() {
            return Err(error);
        }
        if let Some(unit) = self.pending_unit.take() {
            return Ok(Some(unit));
        }
        let Some(scalar) = self.read_scalar()? else {
            return Ok(None);
        };
        let mut units = [0; 2];
        let units = scalar.encode_utf16(&mut units);
        if units.len() == 2 {
            self.pending_unit = Some(units[1]);
        }
        Ok(Some(units[0]))
    }

    /// Reads UTF-16 units. If an error follows already decoded units, returns
    /// their count first and reports the error on the next nonempty read.
    pub fn read_into(&mut self, destination: &mut [u16]) -> io::Result<usize> {
        let mut written = 0;
        while written < destination.len() {
            match self.read() {
                Ok(Some(unit)) => {
                    destination[written] = unit;
                    written += 1;
                }
                Ok(None) => break,
                Err(error) if written == 0 => return Err(error),
                Err(error) => {
                    self.deferred_error = Some(error);
                    break;
                }
            }
        }
        Ok(written)
    }

    /// Consumes CR, LF or CRLF and excludes the delimiter from the result.
    pub fn read_line_utf16(&mut self) -> io::Result<Option<Vec<u16>>> {
        if self.peek()?.is_none() {
            return Ok(None);
        }
        let mut line = Vec::new();
        while let Some(unit) = self.read()? {
            match unit {
                10 => break,
                13 => {
                    if self.peek()? == Some(10) {
                        self.read()?;
                    }
                    break;
                }
                _ => line.push(unit),
            }
        }
        Ok(Some(line))
    }

    pub fn read_line(&mut self) -> io::Result<Option<String>> {
        self.read_line_utf16()
            .map(|line| line.map(|units| String::from_utf16_lossy(&units)))
    }

    pub fn read_to_end_utf16(&mut self) -> io::Result<Vec<u16>> {
        let mut result = Vec::new();
        self.copy_to(&mut result)?;
        Ok(result)
    }

    pub fn read_to_end(&mut self) -> io::Result<String> {
        let mut result = String::new();
        while let Some(scalar) = self.read_projected_scalar()? {
            result.push(scalar);
        }
        Ok(result)
    }

    /// Streams into an existing UTF-16 sink, including DocumentTextWriter,
    /// without a full-file intermediate string or unit vector. Earlier writes
    /// remain accepted if a later read or write fails.
    pub fn copy_to<W: Utf16TextWriter + ?Sized>(&mut self, writer: &mut W) -> io::Result<usize> {
        let mut buffer = [0; 4096];
        let mut written = 0;
        loop {
            let length = self.read_into(&mut buffer)?;
            if length == 0 {
                return Ok(written);
            }
            writer.write_utf16(&buffer[..length])?;
            written += length;
        }
    }

    fn read_projected_scalar(&mut self) -> io::Result<Option<char>> {
        let Some(first) = self.read()? else {
            return Ok(None);
        };
        let scalar = if (0xd800..=0xdbff).contains(&first) {
            match self.peek()? {
                Some(second) if (0xdc00..=0xdfff).contains(&second) => {
                    self.read()?;
                    0x10000 + (((first as u32 - 0xd800) << 10) | (second as u32 - 0xdc00))
                }
                _ => 0xfffd,
            }
        } else if (0xdc00..=0xdfff).contains(&first) {
            0xfffd
        } else {
            first as u32
        };
        Ok(char::from_u32(scalar))
    }

    /// Explicit UTF-8 byte projection implementing standard Read and BufRead.
    /// A previously consumed leading surrogate leaves its remaining low unit
    /// as U+FFFD, matching the rope reader's explicit projection boundary.
    pub fn into_utf8(self) -> Utf8FileTextReader<R> {
        Utf8FileTextReader {
            source: self,
            buffer: [0; 4],
            position: 0,
            length: 0,
            deferred_error: None,
        }
    }
}

/// Standard Rust IO adapter. Encoded input is decoded incrementally; callers
/// receive UTF-8 bytes regardless of the original file encoding.
pub struct Utf8FileTextReader<R> {
    source: FileTextReader<R>,
    buffer: [u8; 4],
    position: usize,
    length: usize,
    deferred_error: Option<io::Error>,
}

impl<R: Read> Utf8FileTextReader<R> {
    pub fn encoding(&self) -> TextEncoding {
        self.source.encoding()
    }

    pub fn byte_order_mark_length(&self) -> usize {
        self.source.byte_order_mark_length()
    }
}

impl<R: Read> BufRead for Utf8FileTextReader<R> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        if self.position == self.length {
            if let Some(error) = self.deferred_error.take() {
                return Err(error);
            }
            self.position = 0;
            self.length = 0;
            self.length = match self.source.read_projected_scalar()? {
                Some(scalar) => scalar.encode_utf8(&mut self.buffer).len(),
                None => 0,
            };
        }
        Ok(&self.buffer[self.position..self.length])
    }

    fn consume(&mut self, amount: usize) {
        self.position += amount.min(self.length - self.position);
    }
}

impl<R: Read> Read for Utf8FileTextReader<R> {
    fn read(&mut self, destination: &mut [u8]) -> io::Result<usize> {
        let mut written = 0;
        while written < destination.len() {
            let pending = match self.fill_buf() {
                Ok(pending) => pending,
                Err(error) if written == 0 => return Err(error),
                Err(error) => {
                    self.deferred_error = Some(error);
                    break;
                }
            };
            if pending.is_empty() {
                break;
            }
            let length = pending.len().min(destination.len() - written);
            destination[written..written + length].copy_from_slice(&pending[..length]);
            self.consume(length);
            written += length;
        }
        Ok(written)
    }
}
