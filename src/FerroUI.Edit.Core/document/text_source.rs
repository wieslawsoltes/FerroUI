use super::{DocumentError, DocumentLine, DocumentVersion, ITextSourceVersion, TextLocation};
use crate::utils::Utf16Rope;
use std::sync::Arc;

/// Read-only UTF-16 text source. Snapshot values are `Send + Sync`; the mutable
/// document handle is deliberately local to its owning UI thread.
pub trait ITextSource {
    fn text_length(&self) -> usize;
    fn get_char_at(&self, offset: usize) -> Result<u16, DocumentError>;
    fn get_text(&self, offset: usize, length: usize) -> Result<String, DocumentError>;
    fn get_text_utf16(&self, offset: usize, length: usize) -> Result<Vec<u16>, DocumentError>;
    fn create_snapshot(&self) -> TextSnapshot;
    fn create_snapshot_range(
        &self,
        offset: usize,
        length: usize,
    ) -> Result<TextSnapshot, DocumentError> {
        self.create_snapshot().slice(offset, length)
    }
    fn version(&self) -> Option<Arc<dyn ITextSourceVersion>> {
        self.create_snapshot()
            .version()
            .map(|version| version.as_text_source_version())
    }
    fn text(&self) -> String {
        self.create_snapshot().text()
    }
}

/// Immutable, persistent UTF-16 snapshot. `text()` is a UTF-8 projection and
/// replaces lone surrogates; `to_utf16()` preserves all original code units.
#[derive(Clone, Debug, Default)]
pub struct TextSnapshot {
    pub(crate) rope: Utf16Rope,
    pub(crate) version: Option<DocumentVersion>,
}

/// Immutable string source uses the same UTF-16 snapshot representation.
pub type StringTextSource = TextSnapshot;

impl TextSnapshot {
    pub fn new(text: &str) -> Self {
        Self::from_text(text)
    }
    pub fn from_text(text: &str) -> Self {
        Self {
            rope: Utf16Rope::from_text(text),
            version: None,
        }
    }
    pub fn from_utf16(text: &[u16]) -> Self {
        Self {
            rope: Utf16Rope::from_utf16(text),
            version: None,
        }
    }
    pub fn empty() -> Self {
        Self::default()
    }
    pub fn text(&self) -> String {
        self.rope.text()
    }
    pub fn text_length(&self) -> usize {
        self.rope.len()
    }
    pub fn to_utf16(&self) -> Vec<u16> {
        self.rope.to_utf16()
    }
    pub fn line_count(&self) -> usize {
        self.rope.line_count()
    }
    pub fn version(&self) -> Option<DocumentVersion> {
        self.version.clone()
    }
    pub fn get_char_at(&self, offset: usize) -> Result<u16, DocumentError> {
        self.rope.get(offset).ok_or(DocumentError::InvalidOffset {
            offset,
            text_length: self.text_length(),
        })
    }
    pub fn create_snapshot(&self) -> Self {
        self.clone()
    }

    pub fn slice(&self, offset: usize, length: usize) -> Result<Self, DocumentError> {
        let rope = self
            .rope
            .slice(offset, length)
            .ok_or(DocumentError::InvalidRange {
                offset,
                length,
                text_length: self.text_length(),
            })?;
        Ok(Self {
            rope,
            version: None,
        })
    }

    pub fn get_text(&self, offset: usize, length: usize) -> Result<String, DocumentError> {
        Ok(self.slice(offset, length)?.text())
    }
    pub fn get_text_utf16(&self, offset: usize, length: usize) -> Result<Vec<u16>, DocumentError> {
        Ok(self.slice(offset, length)?.to_utf16())
    }

    pub fn get_line_by_number(&self, line_number: usize) -> Result<DocumentLine, DocumentError> {
        if line_number == 0 || line_number > self.line_count() {
            return Err(DocumentError::InvalidLine {
                line_number,
                line_count: self.line_count(),
            });
        }
        let offset = self
            .rope
            .line_start(line_number - 1)
            .expect("validated line number");
        let end = self
            .rope
            .line_start(line_number)
            .unwrap_or(self.text_length());
        let delimiter_length = if line_number == self.line_count() {
            0
        } else if end >= 2
            && self.rope.get(end - 2) == Some(13)
            && self.rope.get(end - 1) == Some(10)
        {
            2
        } else {
            1
        };
        Ok(DocumentLine {
            line_number,
            offset,
            length: end - offset - delimiter_length,
            delimiter_length,
        })
    }

    pub fn get_line_by_offset(&self, offset: usize) -> Result<DocumentLine, DocumentError> {
        let index = self
            .rope
            .line_index_at(offset)
            .ok_or(DocumentError::InvalidOffset {
                offset,
                text_length: self.text_length(),
            })?;
        self.get_line_by_number(index + 1)
    }

    pub fn get_location(&self, offset: usize) -> Result<TextLocation, DocumentError> {
        let line = self.get_line_by_offset(offset)?;
        Ok(TextLocation::new(
            line.line_number,
            offset - line.offset + 1,
        ))
    }

    /// Columns beyond the line end are clamped to the last insertion position;
    /// a zero column denotes the start of the line.
    pub fn get_offset(&self, location: TextLocation) -> Result<usize, DocumentError> {
        let line = self.get_line_by_number(location.line)?;
        Ok(line.offset + location.column.saturating_sub(1).min(line.length))
    }

    pub fn for_each_chunk(&self, visitor: impl FnMut(&[u16])) {
        self.rope.for_each_chunk(visitor);
    }

    pub fn index_of(
        &self,
        value: &str,
        offset: usize,
        length: usize,
    ) -> Result<Option<usize>, DocumentError> {
        let haystack = self.get_text_utf16(offset, length)?;
        let needle: Vec<_> = value.encode_utf16().collect();
        if needle.is_empty() {
            return Ok(Some(offset));
        }
        Ok(haystack
            .windows(needle.len())
            .position(|s| s == needle)
            .map(|i| offset + i))
    }

    pub fn last_index_of(
        &self,
        value: &str,
        offset: usize,
        length: usize,
    ) -> Result<Option<usize>, DocumentError> {
        let haystack = self.get_text_utf16(offset, length)?;
        let needle: Vec<_> = value.encode_utf16().collect();
        if needle.is_empty() {
            return Ok(Some(offset + length));
        }
        Ok(haystack
            .windows(needle.len())
            .rposition(|s| s == needle)
            .map(|i| offset + i))
    }
}

impl ITextSource for TextSnapshot {
    fn text_length(&self) -> usize {
        self.text_length()
    }
    fn get_char_at(&self, offset: usize) -> Result<u16, DocumentError> {
        self.get_char_at(offset)
    }
    fn get_text(&self, offset: usize, length: usize) -> Result<String, DocumentError> {
        self.get_text(offset, length)
    }
    fn get_text_utf16(&self, offset: usize, length: usize) -> Result<Vec<u16>, DocumentError> {
        self.get_text_utf16(offset, length)
    }
    fn create_snapshot(&self) -> TextSnapshot {
        self.clone()
    }
}
