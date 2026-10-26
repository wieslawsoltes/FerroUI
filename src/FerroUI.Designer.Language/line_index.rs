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
}
impl LineIndex {
    pub fn new(source: &str) -> Self {
        let mut starts = vec![0];
        starts.extend(line_starts(source, 0, source.len()));
        Self {
            source: source.into(),
            starts,
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
        let mut units = 0;
        for (index, ch) in self.source[start..end].char_indices() {
            if units == position.character {
                return Ok(start + index);
            }
            units += ch.len_utf16() as u32;
            require(
                units <= position.character,
                "Position splits a UTF-16 surrogate pair",
            )?;
        }
        require(
            units == position.character,
            "Position character is outside the line",
        )?;
        Ok(end)
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
            character: self.source[self.starts[line]..end].encode_utf16().count() as u32,
        })
    }
    pub fn span(&self, range: Range) -> Result<SourceSpan> {
        let start = self.offset(range.start)?;
        let end = self.offset(range.end)?;
        require(start <= end, "Range ends before it starts")?;
        Ok(SourceSpan::new(start, end))
    }
    pub fn range(&self, span: SourceSpan) -> Result<Range> {
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
        if let Some(length) = change.range_length {
            require(
                change.range.is_some()
                    && self.source[span.range()].encode_utf16().count() == length as usize,
                "rangeLength does not match the replaced UTF-16 text",
            )?;
        }
        require(
            self.source.len() - span.len() + change.text.len() <= 16 * 1024 * 1024,
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
        self.source.replace_range(span.range(), &change.text);
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

fn line_starts(source: &str, start: usize, end: usize) -> impl Iterator<Item = usize> + '_ {
    (start..end).filter_map(|index| {
        (source.as_bytes()[index] == b'\n'
            || source.as_bytes()[index] == b'\r'
                && source.as_bytes().get(index + 1) != Some(&b'\n'))
        .then_some(index + 1)
    })
}
