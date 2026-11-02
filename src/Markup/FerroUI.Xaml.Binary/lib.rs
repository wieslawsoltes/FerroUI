//! FXB v1 compiled theme resource blocks. A block belongs to one dictionary run and
//! contains exactly one factory kind. There are no type names, constructors or pointers.
//! Strings are interned in lexical order, UTF-8 prefix-compressed, and expanded once
//! into a shared buffer. Every integer except ARGB is canonical unsigned LEB128.
//! The decoder validates all references before the caller can observe a record.

#![forbid(unsafe_code)]

use std::{fmt, sync::Arc};

const HEADER: &[u8; 5] = b"FXB\0\x01";
pub const MAX_BLOCK_BYTES: usize = 16 << 20;
const MAX_ITEMS: usize = 1 << 20;

/// FNV-1a/64 generation consistency check used by the development manifest.
/// This identifies torn publications or stale files; it is not an authentication tag.
pub fn fingerprint(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325u64, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x100000001b3)
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Kind {
    Alias = 1,
    LiteralBrush = 2,
    DynamicBrush = 3,
    StaticBrush = 4,
}

impl Kind {
    pub fn from_wire(tag: u8) -> Option<Self> {
        match tag {
            1 => Some(Self::Alias),
            2 => Some(Self::LiteralBrush),
            3 => Some(Self::DynamicBrush),
            4 => Some(Self::StaticBrush),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Value<'a> {
    Text(&'a str),
    Argb(u32),
}

/// Source positions of both the dictionary insertion and evaluation of the deferred value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Resource<'a> {
    pub key: &'a str,
    pub value: Value<'a>,
    pub line: u32,
    pub column: u32,
    pub value_line: u32,
    pub value_column: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Error(&'static str);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for Error {}

/// A deferred resource may outlive the file or its replacement during hot reload.
/// All text retained by one generation shares a single allocation.
#[derive(Debug, Clone)]
pub struct SharedText {
    text: Arc<str>,
    start: u32,
    end: u32,
}

impl AsRef<str> for SharedText {
    fn as_ref(&self) -> &str {
        &self.text[self.start as usize..self.end as usize]
    }
}

pub enum Payload {
    Text(SharedText),
    Argb(u32),
}

pub struct Record<'a> {
    pub key: &'a str,
    pub payload: Payload,
    pub line: i32,
    pub column: i32,
    pub value_line: i32,
    pub value_column: i32,
}

#[derive(Debug)]
pub struct Batch {
    kind: Kind,
    text: Arc<str>,
    ends: Vec<u32>,
    rows: Vec<[u32; 6]>,
}

impl Batch {
    pub fn kind(&self) -> Kind {
        self.kind
    }

    /// Decode one whole immutable block. Truncation, trailing data, unbounded counts,
    /// dangling references and out-of-range source locations are errors.
    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() > MAX_BLOCK_BYTES || !bytes.starts_with(HEADER) {
            return Err(Error("invalid FXB header, version or length"));
        }
        let mut bytes = &bytes[HEADER.len()..];
        let kind =
            Kind::from_wire(take(&mut bytes, 1)?[0]).ok_or(Error("unsupported FXB record kind"))?;
        let strings = uint(&mut bytes)? as usize;
        let count = uint(&mut bytes)? as usize;
        // Every string requires two lengths, every row at least six bytes.
        if strings > MAX_ITEMS || count > MAX_ITEMS || strings * 2 + count * 6 > bytes.len() {
            return Err(Error("invalid FXB counts"));
        }
        let mut text = String::new();
        let mut ends = Vec::with_capacity(strings + 1);
        ends.push(0);
        let mut previous_start = 0;
        for _ in 0..strings {
            let prefix = uint(&mut bytes)? as usize;
            let suffix = uint(&mut bytes)? as usize;
            let previous = &text[previous_start..];
            if !previous.is_char_boundary(prefix)
                || prefix > MAX_BLOCK_BYTES - text.len()
                || suffix > MAX_BLOCK_BYTES - text.len() - prefix
            {
                return Err(Error("invalid FXB string prefix or expanded length"));
            }
            let tail = std::str::from_utf8(take(&mut bytes, suffix)?)
                .map_err(|_| Error("invalid FXB UTF-8"))?;
            let start = text.len();
            text.extend_from_within(previous_start..previous_start + prefix);
            text.push_str(tail);
            ends.push(text.len() as u32);
            previous_start = start;
        }

        let mut rows = Vec::with_capacity(count);
        for _ in 0..count {
            let key = uint(&mut bytes)?;
            let value = match kind {
                Kind::LiteralBrush => u32::from_le_bytes(take(&mut bytes, 4)?.try_into().unwrap()),
                _ => uint(&mut bytes)?,
            };
            if key as usize >= strings || kind != Kind::LiteralBrush && value as usize >= strings {
                return Err(Error("invalid FXB string index"));
            }
            let line = uint(&mut bytes)?;
            let column = uint(&mut bytes)?;
            let value_line = relative(line, uint(&mut bytes)?)?;
            let value_column = relative(column, uint(&mut bytes)?)?;
            if line > i32::MAX as u32 || column > i32::MAX as u32 {
                return Err(Error("invalid FXB source position"));
            }
            rows.push([key, value, line, column, value_line, value_column]);
        }
        if !bytes.is_empty() {
            return Err(Error("trailing FXB data"));
        }
        Ok(Self {
            kind,
            text: Arc::from(text),
            ends,
            rows,
        })
    }

    pub fn records(&self) -> impl ExactSizeIterator<Item = Record<'_>> {
        self.rows
            .iter()
            .map(|&[key, value, line, column, value_line, value_column]| {
                let range = |id: u32| (self.ends[id as usize], self.ends[id as usize + 1]);
                let (a, b) = range(key);
                let payload = if self.kind == Kind::LiteralBrush {
                    Payload::Argb(value)
                } else {
                    let (start, end) = range(value);
                    Payload::Text(SharedText {
                        text: self.text.clone(),
                        start,
                        end,
                    })
                };
                Record {
                    key: &self.text[a as usize..b as usize],
                    payload,
                    line: line as i32,
                    column: column as i32,
                    value_line: value_line as i32,
                    value_column: value_column as i32,
                }
            })
    }
}

fn take<'a>(bytes: &mut &'a [u8], count: usize) -> Result<&'a [u8], Error> {
    let (value, rest) = bytes
        .split_at_checked(count)
        .ok_or(Error("truncated FXB block"))?;
    *bytes = rest;
    Ok(value)
}

fn uint(bytes: &mut &[u8]) -> Result<u32, Error> {
    let mut value = 0;
    for shift in (0..32).step_by(7) {
        let byte = take(bytes, 1)?[0];
        if shift == 28 && byte > 15 || shift != 0 && byte == 0 {
            return Err(Error("invalid FXB integer"));
        }
        value |= u32::from(byte & 127) << shift;
        if byte < 128 {
            return Ok(value);
        }
    }
    Err(Error("invalid FXB integer"))
}

fn relative(base: u32, zigzag: u32) -> Result<u32, Error> {
    let delta = (zigzag >> 1) as i64 ^ -i64::from(zigzag & 1);
    let value = i64::from(base) + delta;
    if !(0..=i64::from(i32::MAX)).contains(&value) {
        return Err(Error("invalid FXB source position"));
    }
    Ok(value as u32)
}

#[cfg(any(feature = "encode", test))]
mod encode;
#[cfg(any(feature = "encode", test))]
pub use encode::encode;

#[cfg(test)]
mod tests;
