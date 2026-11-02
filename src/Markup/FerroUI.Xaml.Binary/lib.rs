//! FXB compiled theme resource blocks. A block belongs to one dictionary run and
//! contains exactly one factory kind. There are no type names, constructors or pointers.
//! Strings are interned in lexical order, UTF-8 front-coded, and expanded once
//! into a shared buffer. Every integer except ARGB is canonical unsigned LEB128.
//! The decoder validates all references before the caller can observe a record.

#![forbid(unsafe_code)]

use std::{fmt, sync::Arc};

const MAGIC: &[u8; 4] = b"FXB\0";
const STEP_LINE: u8 = 16;
const REPEAT: u8 = 15;
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
    /// Invoke a typed deferred builder from the caller's local import table.
    DeferredFactory = 5,
    Color = 6,
    Double = 7,
    Thickness = 8,
    CornerRadius = 9,
}

impl Kind {
    pub fn from_wire(tag: u8) -> Option<Self> {
        match tag {
            1 => Some(Self::Alias),
            2 => Some(Self::LiteralBrush),
            3 => Some(Self::DynamicBrush),
            4 => Some(Self::StaticBrush),
            5 => Some(Self::DeferredFactory),
            6 => Some(Self::Color),
            7 => Some(Self::Double),
            8 => Some(Self::Thickness),
            9 => Some(Self::CornerRadius),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Value<'a> {
    Text(&'a str),
    Argb(u32),
    Import(u32),
    Double(u64),
    Quad([u64; 4]),
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
/// Deferred values share the generation's expanded UTF-8 buffer.
#[derive(Debug, Clone)]
pub struct SharedText {
    text: Arc<String>,
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
    Import(u32),
    Double(u64),
    Quad([u64; 4]),
}

pub struct Record<'a> {
    pub key: &'a str,
    pub payload: Payload,
    pub line: i32,
    pub column: i32,
    pub value_line: i32,
    pub value_column: i32,
}

/// Only text-valued factories let a payload escape the decoded batch.
#[derive(Debug)]
enum TextPool {
    Keys(String),
    Values(Arc<String>),
}

impl TextPool {
    fn as_str(&self) -> &str {
        match self {
            Self::Keys(text) => text,
            Self::Values(text) => text,
        }
    }
}

#[derive(Debug)]
pub struct Batch {
    kind: Kind,
    text: TextPool,
    ends: Vec<u32>,
    rows: Vec<[u32; 6]>,
    numbers: Vec<u64>,
}

impl Batch {
    pub fn kind(&self) -> Kind {
        self.kind
    }

    /// Heap buffer capacity retained by a decoded block, used by multi-block
    /// readers to enforce an aggregate decompression budget.
    pub fn allocated_bytes(&self) -> usize {
        let text = match &self.text {
            TextPool::Keys(s) => s.capacity(),
            TextPool::Values(s) => s.capacity(),
        };
        text + self.ends.capacity() * std::mem::size_of::<u32>()
            + self.rows.capacity() * std::mem::size_of::<[u32; 6]>()
            + self.numbers.capacity() * std::mem::size_of::<u64>()
    }

    /// Decode one whole immutable block. Truncation, trailing data, unbounded counts,
    /// dangling references and out-of-range source locations are errors.
    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() > MAX_BLOCK_BYTES || !bytes.starts_with(MAGIC) {
            return Err(Error("invalid FXB header, version or length"));
        }
        let mut bytes = &bytes[MAGIC.len()..];
        let version = take(&mut bytes, 1)?[0];
        let kind =
            Kind::from_wire(take(&mut bytes, 1)?[0]).ok_or(Error("unsupported FXB record kind"))?;
        let repeat = match version {
            1 => 0,
            2 | 3 => {
                let flags = take(&mut bytes, 1)?[0];
                if (version == 2 && flags & REPEAT == 0) || flags > (STEP_LINE | REPEAT)
                    || flags & STEP_LINE != 0 && flags & 1 == 0 {
                    return Err(Error("invalid FXB location flags"));
                }
                flags
            }
            _ => return Err(Error("unsupported FXB version")),
        };
        let strings = uint(&mut bytes)? as usize;
        let count = uint(&mut bytes)? as usize;
        if strings > MAX_ITEMS || count > MAX_ITEMS {
            return Err(Error("invalid FXB counts"));
        }
        let shared = (repeat & REPEAT).count_ones() as usize;
        // First row stores all four coordinates. Later rows omit predictable fields.
        let location_bytes = count * 4 - count.saturating_sub(1) * shared;
        let components = match kind {
            Kind::Double => 1,
            Kind::Thickness | Kind::CornerRadius => 4,
            _ => 0,
        };
        let value_bytes = match kind {
            Kind::Color | Kind::LiteralBrush => 4,
            _ => components.max(1),
        };
        let minimum_rows = location_bytes + count * (1 + value_bytes);
        if strings * 2 + minimum_rows > bytes.len() {
            return Err(Error("invalid FXB counts"));
        }
        // A resource name is often longer than its compressed spelling. Seed the
        // pool without trusting a count alone or reserving space used by records.
        let estimated_text = strings.saturating_mul(32)
            .min((bytes.len() - minimum_rows).saturating_mul(2))
            .min(MAX_BLOCK_BYTES);
        let mut text: Vec<u8> = Vec::with_capacity(estimated_text);
        let mut ends = Vec::with_capacity(strings + 1);
        ends.push(0);
        let mut previous_start = 0;
        for _ in 0..strings {
            let prefix = uint(&mut bytes)? as usize;
            let length = uint(&mut bytes)? as usize;
            let (literal, suffix) = if version == 3 {
                let suffix = if length & 1 != 0 {
                    let suffix = uint(&mut bytes)? as usize;
                    if suffix == 0 { return Err(Error("empty FXB suffix reference")); }
                    suffix
                } else {
                    0
                };
                (length >> 1, suffix)
            } else {
                (length, 0)
            };
            let start = text.len();
            let previous = &text[previous_start..];
            if prefix > previous.len() || !scalar_boundary(previous, prefix)
                || suffix > previous.len() - prefix
                || !scalar_boundary(previous, previous.len() - suffix)
                || prefix > MAX_BLOCK_BYTES - start
                || suffix > MAX_BLOCK_BYTES - start - prefix
                || literal > MAX_BLOCK_BYTES - start - prefix - suffix
            {
                return Err(Error("invalid FXB string reference or expanded length"));
            }
            let middle = take(&mut bytes, literal)?;
            text.extend_from_within(previous_start..previous_start + prefix);
            text.extend_from_slice(middle);
            if suffix != 0 { text.extend_from_within(start - suffix..start); }
            ends.push(text.len() as u32);
            previous_start = start;
        }

        // Validate the expanded pool in one pass. Reject a scalar split between
        // consecutive entries even if their concatenation is well-formed UTF-8.
        let text = String::from_utf8(text).map_err(|_| Error("invalid FXB UTF-8"))?;
        if ends.iter().any(|&end| !text.is_char_boundary(end as usize)) {
            return Err(Error("invalid FXB string boundary"));
        }

        if minimum_rows > bytes.len() {
            return Err(Error("truncated FXB block"));
        }
        let mut rows = Vec::with_capacity(count);
        let mut numbers = Vec::with_capacity(count * components);
        let mut locations = [0u32; 4];
        for row in 0..count {
            let key = uint(&mut bytes)?;
            let value = match kind {
                Kind::Double | Kind::Thickness | Kind::CornerRadius => {
                    let start = numbers.len() as u32;
                    for _ in 0..components {
                        let tag = take(&mut bytes, 1)?[0];
                        let bits = if tag == 255 {
                            u64::from_le_bytes(take(&mut bytes, 8)?.try_into().unwrap())
                        } else {
                            (f64::from(tag) - 32.0).to_bits()
                        };
                        numbers.push(bits);
                    }
                    start
                }
                Kind::Color | Kind::LiteralBrush => u32::from_le_bytes(take(&mut bytes, 4)?.try_into().unwrap()),
                _ => uint(&mut bytes)?,
            };
            if key as usize >= strings
                || matches!(kind, Kind::Alias | Kind::DynamicBrush | Kind::StaticBrush)
                    && value as usize >= strings
            {
                return Err(Error("invalid FXB string index"));
            }
            let mask = if row == 0 { 0 } else { repeat };
            if mask & STEP_LINE != 0 {
                locations[0] += 1; // the preceding line was checked against i32::MAX
            }
            for (field, value) in locations.iter_mut().enumerate() {
                if mask & (1 << field) == 0 {
                    *value = uint(&mut bytes)?;
                }
            }
            let [line, column, dl, dc] = locations;
            let value_line = relative(line, dl)?;
            let value_column = relative(column, dc)?;
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
            // Move the expanded buffer into the shared owner. Converting the String
            // to Arc<str> would allocate and copy every reconstructed byte again.
            text: match kind {
                Kind::Alias | Kind::DynamicBrush | Kind::StaticBrush => TextPool::Values(Arc::new(text)),
                _ => TextPool::Keys(text),
            },
            ends,
            rows,
            numbers,
        })
    }

    /// Check every host-relative factory reference before the first dictionary write.
    /// A block may reuse or reorder imports, including across a hot reload.
    pub fn validate_imports(&self, available: usize) -> Result<(), Error> {
        if self.kind != Kind::DeferredFactory
            || self.rows.iter().any(|row| row[1] as usize >= available)
        {
            return Err(Error("invalid FXB deferred factory import"));
        }
        Ok(())
    }

    pub fn records(&self) -> impl ExactSizeIterator<Item = Record<'_>> {
        self.rows
            .iter()
            .map(|&[key, value, line, column, value_line, value_column]| {
                let range = |id: u32| (self.ends[id as usize], self.ends[id as usize + 1]);
                let (a, b) = range(key);
                let payload = if self.kind == Kind::Double {
                    Payload::Double(self.numbers[value as usize])
                } else if matches!(self.kind, Kind::Thickness | Kind::CornerRadius) {
                    Payload::Quad(self.numbers[value as usize..value as usize + 4].try_into().unwrap())
                } else if matches!(self.kind, Kind::LiteralBrush | Kind::Color) {
                    Payload::Argb(value)
                } else if self.kind == Kind::DeferredFactory {
                    Payload::Import(value)
                } else {
                    let (start, end) = range(value);
                    let TextPool::Values(text) = &self.text else { unreachable!("validated text factory") };
                    Payload::Text(SharedText {
                        text: text.clone(),
                        start,
                        end,
                    })
                };
                Record {
                    key: &self.text.as_str()[a as usize..b as usize],
                    payload,
                    line: line as i32,
                    column: column as i32,
                    value_line: value_line as i32,
                    value_column: value_column as i32,
                }
            })
    }
}

// Callers check the index is in bounds. The full buffer is validated after expansion.
fn scalar_boundary(bytes: &[u8], index: usize) -> bool {
    bytes.get(index).is_none_or(|b| b & 0xc0 != 0x80)
}

fn take<'a>(bytes: &mut &'a [u8], count: usize) -> Result<&'a [u8], Error> {
    let (value, rest) = bytes
        .split_at_checked(count)
        .ok_or(Error("truncated FXB block"))?;
    *bytes = rest;
    Ok(value)
}

// Inline the common one-byte operand. All call sites share the checked slow path.
#[inline(always)]
fn uint(bytes: &mut &[u8]) -> Result<u32, Error> {
    match bytes.split_first() {
        Some((&value @ 0..=127, rest)) => {
            *bytes = rest;
            Ok(u32::from(value))
        }
        _ => multibyte_uint(bytes),
    }
}

#[inline(never)]
fn multibyte_uint(bytes: &mut &[u8]) -> Result<u32, Error> {
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
