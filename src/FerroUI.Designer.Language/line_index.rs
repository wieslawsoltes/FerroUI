use crate::{require, LanguageError, Result};
use ferroui_designer_core::SourceSpan;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextEdit {
    pub range: Range,
    pub new_text: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextChange {
    pub range: Option<Range>,
    pub range_length: Option<u32>,
    pub text: String,
}

/// UTF-16 LSP positions over UTF-8 authored bytes, retaining LF/CRLF/CR exactly.
#[derive(Clone, Debug)]
pub struct LineIndex {
    source: String,
    starts: Vec<usize>,
    unicode: Vec<UnicodeBlock>,
}

/// ASCII bytes need no entry: byte and UTF-16 offsets differ only after a
/// non-ASCII scalar. Checkpoints cover short blocks instead of every scalar, so
/// Unicode-heavy documents do not need a large per-character allocation. Each
/// conversion binary-searches the blocks and scans at most 256 authored bytes.
#[derive(Clone, Copy, Debug)]
struct UnicodeBlock {
    byte: usize,
    utf16: usize,
    end: usize,
    utf16_end: usize,
}
const UNICODE_BLOCK_BYTES: usize = 256;
impl LineIndex {
    pub fn new(source: &str) -> Self {
        let mut starts = vec![0];
        starts.extend(line_starts(source, 0, source.len()));
        Self {
            source: source.into(),
            starts,
            unicode: unicode_blocks(source, 0, 0),
        }
    }
    pub fn source(&self) -> &str {
        &self.source
    }
    pub fn line_count(&self) -> usize {
        self.starts.len()
    }
    pub fn offset(&self, position: Position) -> Result<usize> {
        let start = *self
            .starts
            .get(position.line as usize)
            .ok_or_else(|| LanguageError("Position line is outside the document".into()))?;
        let end = self.line_end(position.line as usize);
        let start_units = self.utf16_offset(start);
        require(
            position.character as usize <= self.utf16_offset(end) - start_units,
            "Position character is outside the line",
        )?;
        let units = start_units + position.character as usize;
        let preceding = self.unicode.partition_point(|block| block.utf16 <= units);
        let Some(block) = preceding.checked_sub(1).map(|i| self.unicode[i]) else {
            return Ok(units);
        };
        if units >= block.utf16_end {
            return Ok(block.end + units - block.utf16_end);
        }
        let mut current = block.utf16;
        for (offset, ch) in self.source[block.byte..block.end].char_indices() {
            if current == units {
                return Ok(block.byte + offset);
            }
            current += ch.len_utf16();
            require(current <= units, "Position splits a UTF-16 surrogate pair")?;
        }
        Ok(block.end)
    }
    pub fn position(&self, offset: usize) -> Result<Position> {
        require(
            offset <= self.source.len() && self.source.is_char_boundary(offset),
            "Offset is not a UTF-8 boundary",
        )?;
        let line = self
            .starts
            .partition_point(|start| *start <= offset)
            .saturating_sub(1);
        let end = offset.min(self.line_end(line));
        Ok(Position {
            line: line as u32,
            character: (self.utf16_offset(end) - self.utf16_offset(self.starts[line])) as u32,
        })
    }
    pub fn span(&self, range: Range) -> Result<SourceSpan> {
        let start = self.offset(range.start)?;
        let end = self.offset(range.end)?;
        require(start <= end, "Range ends before it starts")?;
        Ok(SourceSpan::new(start, end))
    }
    pub fn range(&self, span: SourceSpan) -> Result<Range> {
        require(span.start <= span.end, "Span ends before it starts")?;
        Ok(Range {
            start: self.position(span.start)?,
            end: self.position(span.end)?,
        })
    }
    pub fn apply(&mut self, change: &TextChange) -> Result<()> {
        let span = match change.range {
            Some(range) => self.span(range)?,
            None => SourceSpan::new(0, self.source.len()),
        };
        let start_units = self.utf16_offset(span.start);
        let removed_units = self.utf16_offset(span.end) - start_units;
        if let Some(length) = change.range_length {
            require(
                change.range.is_some() && removed_units == length as usize,
                "rangeLength does not match the replaced UTF-16 text",
            )?;
        }
        require(
            (self.source.len() - span.len())
                .checked_add(change.text.len())
                .is_some_and(|length| length <= 16 * 1024 * 1024),
            "Document exceeds 16 MiB",
        )?;
        // Keep the unaffected line prefix; rescan only replaced lines and adjust the suffix.
        let first = self
            .starts
            .partition_point(|start| *start <= span.start)
            .saturating_sub(2);
        let after = self.starts.partition_point(|start| *start <= span.end);
        let old_tail = self.starts.get(after).copied().unwrap_or(self.source.len());
        let delta = change.text.len() as isize - span.len() as isize;
        let inserted_units = change.text.encode_utf16().count();
        let units_delta = inserted_units as isize - removed_units as isize;
        let mut first_block = self
            .unicode
            .partition_point(|block| block.end <= span.start);
        let mut after_block = self.unicode.partition_point(|block| block.byte < span.end);
        // Include nearby neighbors so typing one Unicode character at a time
        // keeps dense blocks instead of allocating one checkpoint per keypress.
        // Distant ASCII runs remain untouched by this bounded rescan.
        if first_block > 0 && span.start - self.unicode[first_block - 1].byte <= UNICODE_BLOCK_BYTES
        {
            first_block -= 1;
        }
        if self
            .unicode
            .get(after_block)
            .is_some_and(|block| block.byte - span.end <= UNICODE_BLOCK_BYTES)
        {
            after_block += 1;
        }
        let (scan_start, scan_end) = if first_block < after_block {
            (
                span.start.min(self.unicode[first_block].byte),
                span.end.max(self.unicode[after_block - 1].end),
            )
        } else {
            (span.start, span.end)
        };
        let scan_units = self.utf16_offset(scan_start);
        for block in &mut self.unicode[after_block..] {
            block.byte = (block.byte as isize + delta) as usize;
            block.end = (block.end as isize + delta) as usize;
            block.utf16 = (block.utf16 as isize + units_delta) as usize;
            block.utf16_end = (block.utf16_end as isize + units_delta) as usize;
        }
        self.source.replace_range(span.range(), &change.text);
        let scan_end = (scan_end as isize + delta) as usize;
        self.unicode.splice(
            first_block..after_block,
            unicode_blocks(&self.source[scan_start..scan_end], scan_start, scan_units),
        );
        let new_tail = (old_tail as isize + delta) as usize;
        let mut starts = self.starts[..=first].to_vec();
        starts.extend(line_starts(&self.source, self.starts[first], new_tail));
        starts.extend(
            self.starts[after..]
                .iter()
                .map(|offset| (*offset as isize + delta) as usize),
        );
        starts.dedup();
        self.starts = starts;
        Ok(())
    }
    fn utf16_offset(&self, byte: usize) -> usize {
        let preceding = self.unicode.partition_point(|block| block.byte < byte);
        preceding.checked_sub(1).map_or(byte, |index| {
            let block = self.unicode[index];
            if byte >= block.end {
                block.utf16_end + byte - block.end
            } else {
                block.utf16 + self.source[block.byte..byte].encode_utf16().count()
            }
        })
    }
    fn line_end(&self, line: usize) -> usize {
        let mut end = self
            .starts
            .get(line + 1)
            .copied()
            .unwrap_or(self.source.len());
        if end > self.starts[line] && self.source.as_bytes()[end - 1] == b'\n' {
            end -= 1;
        }
        if end > self.starts[line] && self.source.as_bytes()[end - 1] == b'\r' {
            end -= 1;
        }
        end
    }
}

fn unicode_blocks(source: &str, byte: usize, utf16: usize) -> Vec<UnicodeBlock> {
    if source.is_ascii() {
        return Vec::new();
    }
    let mut units = utf16;
    let mut blocks: Vec<UnicodeBlock> = Vec::new();
    for (offset, ch) in source.char_indices() {
        if !ch.is_ascii() {
            let end = byte + offset + ch.len_utf8();
            let utf16_end = units + ch.len_utf16();
            if let Some(block) = blocks
                .last_mut()
                .filter(|block| end - block.byte <= UNICODE_BLOCK_BYTES)
            {
                block.end = end;
                block.utf16_end = utf16_end;
            } else {
                blocks.push(UnicodeBlock {
                    byte: byte + offset,
                    utf16: units,
                    end,
                    utf16_end,
                });
            }
        }
        units += ch.len_utf16();
    }
    blocks
}

fn line_starts(source: &str, start: usize, end: usize) -> impl Iterator<Item = usize> + '_ {
    (start..end).filter_map(|index| {
        (source.as_bytes()[index] == b'\n'
            || source.as_bytes()[index] == b'\r'
                && source.as_bytes().get(index + 1) != Some(&b'\n'))
        .then_some(index + 1)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unicode_checkpoints_stay_compact_during_character_by_character_typing() {
        let mut index = LineIndex::new("");
        for character in 0..512 {
            let position = Position { line: 0, character };
            index
                .apply(&TextChange {
                    range: Some(Range {
                        start: position,
                        end: position,
                    }),
                    range_length: Some(0),
                    text: "α".into(),
                })
                .unwrap();
        }
        assert_eq!(index.source(), "α".repeat(512));
        assert!(
            index.unicode.len() <= 5,
            "{} blocks for 512 characters",
            index.unicode.len()
        );
        for character in 0..=512 {
            assert_eq!(
                index.offset(Position { line: 0, character }).unwrap(),
                character as usize * 2
            );
        }
    }
}
