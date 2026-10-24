use super::ordinal_casing::simple_upper;
use super::{DocumentError, TextSnapshot};
use crate::utils::RopeTextReader;
use std::collections::VecDeque;

/// Culture-independent comparisons over UTF-16 coordinates. Ignore-case uses
/// Unicode 17 simple uppercase with .NET's ordinal exceptions for U+0131 and
/// U+017F. It never expands a scalar, normalizes text, or performs collation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextComparison {
    #[default]
    Ordinal,
    OrdinalIgnoreCase,
}

impl TextSnapshot {
    pub fn index_of_char(
        &self,
        value: u16,
        offset: usize,
        length: usize,
    ) -> Result<Option<usize>, DocumentError> {
        Ok(self
            .create_reader_range(offset, length)?
            .position(|unit| unit == value)
            .map(|i| offset + i))
    }

    pub fn last_index_of_char(
        &self,
        value: u16,
        offset: usize,
        length: usize,
    ) -> Result<Option<usize>, DocumentError> {
        Ok(self
            .create_reader_range(offset, length)?
            .enumerate()
            .filter(|&(_, unit)| unit == value)
            .map(|(i, _)| offset + i)
            .last())
    }

    pub fn index_of_any(
        &self,
        values: &[u16],
        offset: usize,
        length: usize,
    ) -> Result<Option<usize>, DocumentError> {
        let mut reader = self.create_reader_range(offset, length)?;
        if values.is_empty() {
            return Ok(None);
        }
        if values.len() <= 8 {
            return Ok(reader
                .position(|unit| values.contains(&unit))
                .map(|i| offset + i));
        }
        let mut set = [0u64; 1024];
        for &unit in values {
            set[unit as usize / 64] |= 1 << (unit % 64);
        }
        Ok(reader
            .position(|unit| set[unit as usize / 64] & (1 << (unit % 64)) != 0)
            .map(|i| offset + i))
    }

    pub fn index_of_with_comparison(
        &self,
        value: &str,
        offset: usize,
        length: usize,
        comparison: TextComparison,
    ) -> Result<Option<usize>, DocumentError> {
        self.index_of_utf16(
            &value.encode_utf16().collect::<Vec<_>>(),
            offset,
            length,
            comparison,
        )
    }

    pub fn last_index_of_with_comparison(
        &self,
        value: &str,
        offset: usize,
        length: usize,
        comparison: TextComparison,
    ) -> Result<Option<usize>, DocumentError> {
        self.last_index_of_utf16(
            &value.encode_utf16().collect::<Vec<_>>(),
            offset,
            length,
            comparison,
        )
    }

    /// Streaming KMP search; O(n + m) time and O(m + log n) auxiliary space.
    /// Ignore-case queries containing unpaired surrogates use a bounded window
    /// matcher because a half-surrogate can match inside a cased scalar pair.
    pub fn index_of_utf16(
        &self,
        value: &[u16],
        offset: usize,
        length: usize,
        comparison: TextComparison,
    ) -> Result<Option<usize>, DocumentError> {
        self.search_utf16(value, offset, length, comparison, false)
    }

    /// Uses the same forward range convention as `index_of_utf16`; returns the
    /// last (possibly overlapping) match, and range end for an empty needle.
    pub fn last_index_of_utf16(
        &self,
        value: &[u16],
        offset: usize,
        length: usize,
        comparison: TextComparison,
    ) -> Result<Option<usize>, DocumentError> {
        self.search_utf16(value, offset, length, comparison, true)
    }

    fn search_utf16(
        &self,
        value: &[u16],
        offset: usize,
        length: usize,
        comparison: TextComparison,
        last: bool,
    ) -> Result<Option<usize>, DocumentError> {
        let reader = self.create_reader_range(offset, length)?;
        if value.is_empty() {
            return Ok(Some(offset + if last { length } else { 0 }));
        }
        if value.len() > length {
            return Ok(None);
        }
        let result = match comparison {
            TextComparison::Ordinal => kmp(reader, value, last),
            TextComparison::OrdinalIgnoreCase => {
                if char::decode_utf16(value.iter().copied()).any(|unit| unit.is_err()) {
                    malformed_search(reader, value, last)
                } else {
                    let pattern: Vec<_> = OrdinalUnits::new(value.iter().copied()).collect();
                    kmp(OrdinalUnits::new(reader), &pattern, last)
                }
            }
        };
        Ok(result.map(|i| offset + i))
    }
}

fn kmp(source: impl Iterator<Item = u16>, pattern: &[u16], last: bool) -> Option<usize> {
    let mut failure = vec![0; pattern.len()];
    let mut matched = 0;
    for index in 1..pattern.len() {
        while matched > 0 && pattern[index] != pattern[matched] {
            matched = failure[matched - 1];
        }
        if pattern[index] == pattern[matched] {
            matched += 1;
        }
        failure[index] = matched;
    }
    matched = 0;
    let mut result = None;
    for (index, unit) in source.enumerate() {
        while matched > 0 && unit != pattern[matched] {
            matched = failure[matched - 1];
        }
        if unit == pattern[matched] {
            matched += 1;
        }
        if matched == pattern.len() {
            result = Some(index + 1 - matched);
            if !last {
                return result;
            }
            matched = failure[matched - 1];
        }
    }
    result
}

struct OrdinalUnits<I: Iterator<Item = u16>> {
    source: std::iter::Peekable<I>,
    pending: Option<u16>,
}
impl<I: Iterator<Item = u16>> OrdinalUnits<I> {
    fn new(source: I) -> Self {
        Self {
            source: source.peekable(),
            pending: None,
        }
    }
}
impl<I: Iterator<Item = u16>> Iterator for OrdinalUnits<I> {
    type Item = u16;
    fn next(&mut self) -> Option<u16> {
        if let Some(unit) = self.pending.take() {
            return Some(unit);
        }
        let first = self.source.next()?;
        if (0xd800..=0xdbff).contains(&first) {
            if let Some(&second) = self.source.peek() {
                if (0xdc00..=0xdfff).contains(&second) {
                    self.source.next();
                    let upper = simple_upper(
                        0x10000 + ((first as u32 - 0xd800) << 10) + second as u32 - 0xdc00,
                    ) - 0x10000;
                    self.pending = Some(0xdc00 + (upper & 0x3ff) as u16);
                    return Some(0xd800 + (upper >> 10) as u16);
                }
            }
        }
        Some(simple_upper(first as u32) as u16)
    }
}

// A malformed query may begin with a low surrogate or end with a high surrogate
// which matches one raw half of a valid pair. Whole-source normalization changes
// that half during supplementary casing, so compare each candidate boundary in
// its original representation. Memory remains O(m), even for a large document.
fn malformed_search(source: RopeTextReader, pattern: &[u16], last: bool) -> Option<usize> {
    let mut window = VecDeque::with_capacity(pattern.len());
    let mut result = None;
    for (index, unit) in source.enumerate() {
        window.push_back(unit);
        if window.len() != pattern.len() {
            continue;
        }
        if OrdinalUnits::new(window.iter().copied()).eq(OrdinalUnits::new(pattern.iter().copied()))
        {
            result = Some(index + 1 - pattern.len());
            if !last {
                return result;
            }
        }
        window.pop_front();
    }
    result
}
