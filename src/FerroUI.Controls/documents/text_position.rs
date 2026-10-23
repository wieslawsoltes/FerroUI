use super::{
    DocumentChangedEventArgs, DocumentError, RichTextDocument, TextElement, WeakRichTextDocument,
};
use crate::{Control, RichTextBlock};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{Rect, Ref};
use std::cell::{Cell, RefCell};
use std::ops::Range;
use std::rc::Rc;

/// Affinity at an insertion boundary. Backward stays before inserted content;
/// Forward moves after it. Offsets are UTF-16 code units, never UTF-8 bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum TextGravity {
    Backward = 0,
    #[default]
    Forward = 1,
}

/// Direction in the logical text stream, used by document pointer APIs.
pub type LogicalDirection = TextGravity;

ferroui_base::ferro_markup_enum!(TextGravity { Backward, Forward }, { namespace: "FerroUI.Controls.Documents" });

/// A value position. Use [`TextPointer`] when the position must track subsequent edits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TextPosition {
    pub offset: usize,
    pub gravity: TextGravity,
}

impl TextPosition {
    pub const fn new(offset: usize, gravity: TextGravity) -> Self {
        Self { offset, gravity }
    }

    pub fn map(&self, change: &DocumentChangedEventArgs) -> Self {
        change.map_position(*self)
    }
}

/// A directional range. `start` and `end` may be reversed; the normalized range
/// is half-open and does not split a UTF-16 surrogate pair after validation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TextRange {
    pub start: TextPosition,
    pub end: TextPosition,
}

impl TextRange {
    pub const fn new(start: TextPosition, end: TextPosition) -> Self {
        Self { start, end }
    }

    pub fn range(&self) -> Range<usize> {
        self.start.offset.min(self.end.offset)..self.start.offset.max(self.end.offset)
    }
    pub fn is_empty(&self) -> bool {
        self.start.offset == self.end.offset
    }
    pub fn length(&self) -> usize {
        self.start.offset.abs_diff(self.end.offset)
    }
    pub fn map(&self, change: &DocumentChangedEventArgs) -> Self {
        Self {
            start: change.map_position(self.start),
            end: change.map_position(self.end),
        }
    }
}

/// A shared, live position whose document back pointer is weak. Holding a
/// pointer cannot keep a document, control or visual tree alive.
#[derive(Clone)]
pub struct TextPointer(Rc<TextPointerData>);

struct TextPointerData {
    document: WeakRichTextDocument,
    position: Cell<TextPosition>,
    subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

impl Drop for TextPointerData {
    fn drop(&mut self) {
        if let Some(subscription) = self.subscription.get_mut().take() {
            subscription.dispose();
        }
    }
}

impl std::fmt::Debug for TextPointer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextPointer")
            .field("position", &self.position())
            .field("is_valid", &self.is_valid())
            .finish()
    }
}

impl TextPointer {
    pub fn new(
        document: &RichTextDocument,
        offset: usize,
        gravity: TextGravity,
    ) -> Result<Self, DocumentError> {
        document.validate_range(offset..offset)?;
        let pointer = Self(Rc::new(TextPointerData {
            document: document.downgrade(),
            position: Cell::new(TextPosition::new(offset, gravity)),
            subscription: RefCell::new(None),
        }));
        let weak = Rc::downgrade(&pointer.0);
        let subscription = document.position_changed(move |change| {
            if let Some(pointer) = weak.upgrade() {
                pointer
                    .position
                    .set(change.map_position(pointer.position.get()));
            }
        });
        *pointer.0.subscription.borrow_mut() = Some(subscription);
        Ok(pointer)
    }

    pub fn document(&self) -> Option<RichTextDocument> {
        self.0.document.upgrade()
    }
    pub fn position(&self) -> TextPosition {
        self.0.position.get()
    }
    pub fn offset(&self) -> usize {
        self.position().offset
    }
    pub fn gravity(&self) -> TextGravity {
        self.position().gravity
    }
    pub fn logical_direction(&self) -> LogicalDirection {
        self.gravity()
    }
    pub fn is_valid(&self) -> bool {
        self.document().is_some()
    }

    pub fn parent(&self) -> Option<Ref<TextElement>> {
        self.document()?.element_at(self.offset(), self.gravity())
    }

    pub fn visual_parent(&self) -> Option<Ref<Control>> {
        let document = self.document()?;
        let owner = document
            .blocks()
            .logical_parent()?
            .cast::<RichTextBlock>()?;
        owner.text_view_for_position(self.offset()).map(Ref::upcast)
    }

    /// A caret rectangle in the owning text view. A detached document has no
    /// geometry. Direction chooses the character affinity at this same insertion
    /// position. Like the source TextPointer contract, the rectangle has zero width.
    pub fn get_character_rect(&self, direction: LogicalDirection) -> Option<Rect> {
        let view = self.visual_parent()?.cast::<RichTextBlock>()?;
        let mut rect =
            view.get_rect_from_text_position(TextPosition::new(self.offset(), direction));
        rect.width = 0.0;
        Some(rect)
    }

    pub fn is_at_insertion_position(&self) -> bool {
        self.document()
            .is_some_and(|document| document.is_grapheme_boundary(self.offset()))
    }

    pub fn get_position_at_offset(
        &self,
        offset: i32,
        gravity: TextGravity,
    ) -> Result<Self, DocumentError> {
        let document = self.document().ok_or(DocumentError::DocumentDropped)?;
        let next = if offset >= 0 {
            self.offset().checked_add(offset as usize)
        } else {
            self.offset().checked_sub(offset.unsigned_abs() as usize)
        }
        .ok_or(DocumentError::OffsetOutOfRange {
            offset: self.offset(),
            length: document.text_length(),
        })?;
        Self::new(&document, next, gravity)
    }

    pub fn next_insertion_position(&self) -> Option<Self> {
        let document = self.document()?;
        let next = document.next_grapheme(self.offset());
        (next != self.offset())
            .then(|| Self::new(&document, next, self.gravity()).ok())
            .flatten()
    }

    pub fn previous_insertion_position(&self) -> Option<Self> {
        let document = self.document()?;
        let previous = document.previous_grapheme(self.offset());
        (previous != self.offset())
            .then(|| Self::new(&document, previous, self.gravity()).ok())
            .flatten()
    }
}

#[cfg(test)]
#[path = "rich_text_pointer_tests.rs"]
mod tests;
