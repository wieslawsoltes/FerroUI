use super::line_manager::LineManager;
use super::{DocumentError, DocumentLine};
use std::cell::Cell;
use std::rc::Rc;

pub(crate) struct LiveLineInner {
    pub node: Cell<Option<usize>>,
    pub deleted: Cell<bool>,
    pub total_length: Cell<usize>,
    pub delimiter_length: Cell<usize>,
}

/// Stable identity for a document line. Unlike the Copy `DocumentLine` query
/// value, this handle follows edits and exposes deletion and sibling identity.
/// A retained line keeps its line index alive after the document is released.
#[derive(Clone)]
pub struct LiveDocumentLine {
    pub(crate) inner: Rc<LiveLineInner>,
    pub(crate) manager: Rc<LineManager>,
}
impl PartialEq for LiveDocumentLine {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}
impl Eq for LiveDocumentLine {}
impl std::fmt::Debug for LiveDocumentLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LiveDocumentLine")
            .field("deleted", &self.is_deleted())
            .field("line", &self.snapshot())
            .finish()
    }
}
impl LiveDocumentLine {
    pub fn is_deleted(&self) -> bool {
        self.inner.deleted.get()
    }
    /// Content length. During low-level tracker callbacks the old delimiter can
    /// briefly exceed the new total length; that transitional value is zero.
    pub fn length(&self) -> usize {
        self.total_length().saturating_sub(self.delimiter_length())
    }
    pub fn total_length(&self) -> usize {
        self.inner.total_length.get()
    }
    pub fn delimiter_length(&self) -> usize {
        self.inner.delimiter_length.get()
    }
    pub fn snapshot(&self) -> Result<DocumentLine, DocumentError> {
        self.manager.snapshot(self)
    }
    pub fn offset(&self) -> Result<usize, DocumentError> {
        Ok(self.snapshot()?.offset)
    }
    pub fn line_number(&self) -> Result<usize, DocumentError> {
        Ok(self.snapshot()?.line_number)
    }
    pub fn end_offset(&self) -> Result<usize, DocumentError> {
        Ok(self.offset()? + self.length())
    }
    pub fn next_line(&self) -> Option<Self> {
        self.manager.neighbor(self, true)
    }
    pub fn previous_line(&self) -> Option<Self> {
        self.manager.neighbor(self, false)
    }
}
