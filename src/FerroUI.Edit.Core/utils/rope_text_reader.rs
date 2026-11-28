use super::{RopeChunks, Utf16Memory, Utf16Rope};
use std::io::{self, Read};

/// Snapshot-retaining streaming reader. All positions and reads count UTF-16
/// units, including unpaired surrogates. Constructing it takes O(log n) space;
/// sequential reading is O(n), without a complete text or chunk-index copy.
#[derive(Clone, Debug)]
pub struct RopeTextReader {
    chunks: RopeChunks,
    current: Utf16Memory,
    index: usize,
    position: usize,
    length: usize,
}

impl RopeTextReader {
    pub fn new(rope: &Utf16Rope) -> Self {
        Self::from_chunks(rope.chunks())
    }

    pub fn from_range(rope: &Utf16Rope, offset: usize, length: usize) -> Option<Self> {
        Some(Self::from_chunks(rope.chunks_range(offset, length)?))
    }

    fn from_chunks(chunks: RopeChunks) -> Self {
        let length = chunks.remaining_units();
        Self {
            chunks,
            current: Utf16Memory::default(),
            index: 0,
            position: 0,
            length,
        }
    }

    /// Offset relative to the reader's original range.
    pub fn position_utf16(&self) -> usize {
        self.position
    }
    pub fn remaining(&self) -> usize {
        self.length - self.position
    }

    fn ensure_chunk(&mut self) -> bool {
        if self.remaining() == 0 {
            return false;
        }
        if self.index == self.current.len() {
            self.current = self.chunks.next().unwrap_or_default();
            self.index = 0;
        }
        !self.current.is_empty()
    }

    pub fn peek(&mut self) -> Option<u16> {
        self.ensure_chunk().then(|| self.current[self.index])
    }

    pub fn read(&mut self) -> Option<u16> {
        let unit = self.peek()?;
        self.index += 1;
        self.position += 1;
        Some(unit)
    }

    /// Reads at most one contiguous leaf, matching TextReader's short-read
    /// contract. Use `read_block` to fill the destination across leaves.
    pub fn read_into(&mut self, destination: &mut [u16]) -> usize {
        if destination.is_empty() || !self.ensure_chunk() {
            return 0;
        }
        let count = destination.len().min(self.current.len() - self.index);
        destination[..count].copy_from_slice(&self.current[self.index..self.index + count]);
        self.index += count;
        self.position += count;
        count
    }

    pub fn read_block(&mut self, mut destination: &mut [u16]) -> usize {
        let mut total = 0;
        while !destination.is_empty() {
            let count = self.read_into(destination);
            if count == 0 {
                break;
            }
            total += count;
            destination = &mut destination[count..];
        }
        total
    }

    /// Returns the next shared contiguous region and advances past it.
    pub fn read_chunk(&mut self) -> Option<Utf16Memory> {
        if !self.ensure_chunk() {
            return None;
        }
        let count = self.current.len() - self.index;
        let result = self.current.slice(self.index, count);
        self.index += count;
        self.position += count;
        result
    }

    /// CR, LF and CRLF terminate a line; the delimiter is consumed but omitted.
    /// EOF returns None, including immediately after a final delimiter.
    pub fn read_line_utf16(&mut self) -> Option<Vec<u16>> {
        self.peek()?;
        let mut result = Vec::new();
        while let Some(unit) = self.read() {
            match unit {
                10 => break,
                13 => {
                    if self.peek() == Some(10) {
                        self.read();
                    }
                    break;
                }
                _ => result.push(unit),
            }
        }
        Some(result)
    }

    /// UTF-8 projection of `read_line_utf16`; lone surrogates become U+FFFD.
    pub fn read_line(&mut self) -> Option<String> {
        self.read_line_utf16()
            .map(|line| String::from_utf16_lossy(&line))
    }

    pub fn read_to_end_utf16(&mut self) -> Vec<u16> {
        let mut result = Vec::with_capacity(self.remaining());
        while let Some(chunk) = self.read_chunk() {
            result.extend_from_slice(&chunk);
        }
        result
    }

    pub fn read_to_end(&mut self) -> String {
        String::from_utf16_lossy(&self.read_to_end_utf16())
    }

    /// Explicit UTF-8 byte-stream projection for Rust IO consumers. The adapter
    /// joins surrogate pairs across leaves and replaces unpaired surrogates.
    pub fn into_utf8(self) -> Utf8TextReader {
        Utf8TextReader::new(self)
    }
}

impl Iterator for RopeTextReader {
    type Item = u16;
    fn next(&mut self) -> Option<u16> {
        self.read()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining(), Some(self.remaining()))
    }
}
impl ExactSizeIterator for RopeTextReader {}
impl std::iter::FusedIterator for RopeTextReader {}

/// Bounded-memory UTF-8 adapter over a UTF-16 snapshot reader.
#[derive(Clone, Debug)]
pub struct Utf8TextReader {
    source: RopeTextReader,
    pending: [u8; 4],
    start: usize,
    end: usize,
}

impl Utf8TextReader {
    pub fn new(source: RopeTextReader) -> Self {
        Self {
            source,
            pending: [0; 4],
            start: 0,
            end: 0,
        }
    }

    fn fill_pending(&mut self) -> bool {
        let Some(first) = self.source.read() else {
            return false;
        };
        let scalar = if (0xd800..=0xdbff).contains(&first) {
            match self.source.peek() {
                Some(second) if (0xdc00..=0xdfff).contains(&second) => {
                    self.source.read();
                    0x10000 + (((first as u32 - 0xd800) << 10) | (second as u32 - 0xdc00))
                }
                _ => 0xfffd,
            }
        } else if (0xdc00..=0xdfff).contains(&first) {
            0xfffd
        } else {
            first as u32
        };
        self.start = 0;
        self.end = char::from_u32(scalar)
            .expect("decoded scalar")
            .encode_utf8(&mut self.pending)
            .len();
        true
    }
}

impl Read for Utf8TextReader {
    fn read(&mut self, destination: &mut [u8]) -> io::Result<usize> {
        let mut written = 0;
        while written < destination.len() {
            if self.start == self.end && !self.fill_pending() {
                break;
            }
            let count = (destination.len() - written).min(self.end - self.start);
            destination[written..written + count]
                .copy_from_slice(&self.pending[self.start..self.start + count]);
            self.start += count;
            written += count;
        }
        Ok(written)
    }
}
