use super::{
    AnchorMovementType, DocumentChangeEventArgs, DocumentError, DocumentLine, DocumentSubscription,
    ITextAnchor, ITextSource, LiveDocumentLine, OffsetChangeMappingType, SimpleSegment,
    TextDocument, TextLocation, UpdateGuard,
};
use std::any::Any;
use std::rc::Rc;

/// An owned undo group. Dropping the implementation closes its group; explicit
/// completion also returns any document-state error to the caller.
pub trait UndoableAction {
    fn finish(self: Box<Self>) -> Result<(), DocumentError>;
}

impl UndoableAction for UpdateGuard {
    fn finish(self: Box<Self>) -> Result<(), DocumentError> {
        (*self).finish()
    }
}

/// A live document line. Position access is fallible after deletion, unlike
/// the infallible `ISegment` value contract. `segment` produces a checked value.
pub trait IDocumentLine: Any {
    fn snapshot(&self) -> Result<DocumentLine, DocumentError>;
    fn total_length(&self) -> usize;
    fn delimiter_length(&self) -> usize;
    fn is_deleted(&self) -> bool;
    fn previous_line(&self) -> Option<Rc<dyn IDocumentLine>>;
    fn next_line(&self) -> Option<Rc<dyn IDocumentLine>>;
    fn is_same_line_as(&self, other: &dyn IDocumentLine) -> bool;

    fn offset(&self) -> Result<usize, DocumentError> {
        Ok(self.snapshot()?.offset)
    }
    fn line_number(&self) -> Result<usize, DocumentError> {
        Ok(self.snapshot()?.line_number)
    }
    fn length(&self) -> usize {
        self.total_length().saturating_sub(self.delimiter_length())
    }
    fn end_offset(&self) -> Result<usize, DocumentError> {
        Ok(self.offset()? + self.length())
    }
    fn segment(&self) -> Result<SimpleSegment, DocumentError> {
        Ok(SimpleSegment::new(self.offset()?, self.length()))
    }
}

/// Object-safe editor/refactoring document contract. Positions count UTF-16
/// code units and lines/columns are one-based. `text_changed` describes each
/// edit; `change_completed` coalesces an undo group. This distinction differs
/// from the concrete document's similarly named completion event.
///
/// Framework service discovery stays with the host; this trait does not add a
/// second service-provider abstraction to the renderer-independent core.
pub trait IDocument: ITextSource + Any {
    fn set_text(&self, text: &str) -> Result<(), DocumentError>;
    fn text_changing(&self, handler: Rc<dyn Fn(&DocumentChangeEventArgs)>) -> DocumentSubscription;
    fn text_changed(&self, handler: Rc<dyn Fn(&DocumentChangeEventArgs)>) -> DocumentSubscription;
    fn change_completed(&self, handler: Rc<dyn Fn()>) -> DocumentSubscription;
    fn line_count(&self) -> usize;
    fn get_line_by_number(&self, number: usize) -> Result<Rc<dyn IDocumentLine>, DocumentError>;
    fn get_line_by_offset(&self, offset: usize) -> Result<Rc<dyn IDocumentLine>, DocumentError>;
    fn get_offset(&self, location: TextLocation) -> Result<usize, DocumentError>;
    fn get_location(&self, offset: usize) -> Result<TextLocation, DocumentError>;
    fn insert(&self, offset: usize, text: &str) -> Result<(), DocumentError>;
    fn insert_source(&self, offset: usize, text: &dyn ITextSource) -> Result<(), DocumentError>;
    fn insert_with_movement(
        &self,
        offset: usize,
        text: &str,
        movement: AnchorMovementType,
    ) -> Result<(), DocumentError>;
    fn insert_source_with_movement(
        &self,
        offset: usize,
        text: &dyn ITextSource,
        movement: AnchorMovementType,
    ) -> Result<(), DocumentError>;
    fn remove(&self, offset: usize, length: usize) -> Result<(), DocumentError>;
    fn replace(&self, offset: usize, length: usize, text: &str) -> Result<(), DocumentError>;
    fn replace_source(
        &self,
        offset: usize,
        length: usize,
        text: &dyn ITextSource,
    ) -> Result<(), DocumentError>;
    fn start_undoable_action(&self) -> Result<(), DocumentError>;
    fn end_undoable_action(&self) -> Result<(), DocumentError>;
    fn open_undo_group(&self) -> Result<Box<dyn UndoableAction>, DocumentError>;
    fn create_anchor(&self, offset: usize) -> Result<Rc<dyn ITextAnchor>, DocumentError>;
    fn file_name(&self) -> Option<String>;
    fn file_name_changed(&self, handler: Rc<dyn Fn()>) -> DocumentSubscription;
    fn is_same_document_as(&self, other: &dyn IDocument) -> bool;

    fn get_offset_at(&self, line: usize, column: usize) -> Result<usize, DocumentError> {
        self.get_offset(TextLocation::new(line, column))
    }
}

impl LiveDocumentLine {
    pub fn as_document_line(&self) -> Rc<dyn IDocumentLine> {
        Rc::new(self.clone())
    }
}

impl IDocumentLine for LiveDocumentLine {
    fn snapshot(&self) -> Result<DocumentLine, DocumentError> {
        LiveDocumentLine::snapshot(self)
    }
    fn total_length(&self) -> usize {
        LiveDocumentLine::total_length(self)
    }
    fn delimiter_length(&self) -> usize {
        LiveDocumentLine::delimiter_length(self)
    }
    fn is_deleted(&self) -> bool {
        LiveDocumentLine::is_deleted(self)
    }
    fn previous_line(&self) -> Option<Rc<dyn IDocumentLine>> {
        LiveDocumentLine::previous_line(self).map(|line| line.as_document_line())
    }
    fn next_line(&self) -> Option<Rc<dyn IDocumentLine>> {
        LiveDocumentLine::next_line(self).map(|line| line.as_document_line())
    }
    fn is_same_line_as(&self, other: &dyn IDocumentLine) -> bool {
        let other: &dyn Any = other;
        other
            .downcast_ref::<Self>()
            .is_some_and(|other| self == other)
    }
}

impl TextDocument {
    pub fn as_document(&self) -> Rc<dyn IDocument> {
        Rc::new(self.clone())
    }
}

impl IDocument for TextDocument {
    fn set_text(&self, text: &str) -> Result<(), DocumentError> {
        TextDocument::set_text(self, text)
    }
    fn text_changing(&self, handler: Rc<dyn Fn(&DocumentChangeEventArgs)>) -> DocumentSubscription {
        TextDocument::changing(self, move |change| handler(change))
    }
    fn text_changed(&self, handler: Rc<dyn Fn(&DocumentChangeEventArgs)>) -> DocumentSubscription {
        TextDocument::changed(self, move |change| handler(change))
    }
    fn change_completed(&self, handler: Rc<dyn Fn()>) -> DocumentSubscription {
        TextDocument::text_changed(self, move || handler())
    }
    fn line_count(&self) -> usize {
        TextDocument::line_count(self)
    }
    fn get_line_by_number(&self, number: usize) -> Result<Rc<dyn IDocumentLine>, DocumentError> {
        Ok(self.get_live_line_by_number(number)?.as_document_line())
    }
    fn get_line_by_offset(&self, offset: usize) -> Result<Rc<dyn IDocumentLine>, DocumentError> {
        Ok(self.get_live_line_by_offset(offset)?.as_document_line())
    }
    fn get_offset(&self, location: TextLocation) -> Result<usize, DocumentError> {
        TextDocument::get_offset(self, location)
    }
    fn get_location(&self, offset: usize) -> Result<TextLocation, DocumentError> {
        TextDocument::get_location(self, offset)
    }
    fn insert(&self, offset: usize, text: &str) -> Result<(), DocumentError> {
        TextDocument::insert(self, offset, text)
    }
    fn insert_source(&self, offset: usize, text: &dyn ITextSource) -> Result<(), DocumentError> {
        self.insert_source_with_movement(offset, text, AnchorMovementType::Default)
    }
    fn insert_with_movement(
        &self,
        offset: usize,
        text: &str,
        movement: AnchorMovementType,
    ) -> Result<(), DocumentError> {
        TextDocument::insert_with_movement(self, offset, text, movement)
    }
    fn insert_source_with_movement(
        &self,
        offset: usize,
        text: &dyn ITextSource,
        movement: AnchorMovementType,
    ) -> Result<(), DocumentError> {
        let mapping = if movement == AnchorMovementType::BeforeInsertion {
            OffsetChangeMappingType::KeepAnchorBeforeInsertion
        } else {
            OffsetChangeMappingType::Normal
        };
        self.replace_snapshot(offset, 0, text.create_snapshot(), mapping)
    }
    fn remove(&self, offset: usize, length: usize) -> Result<(), DocumentError> {
        TextDocument::remove(self, offset, length)
    }
    fn replace(&self, offset: usize, length: usize, text: &str) -> Result<(), DocumentError> {
        TextDocument::replace(self, offset, length, text)
    }
    fn replace_source(
        &self,
        offset: usize,
        length: usize,
        text: &dyn ITextSource,
    ) -> Result<(), DocumentError> {
        self.replace_snapshot(
            offset,
            length,
            text.create_snapshot(),
            OffsetChangeMappingType::Normal,
        )
    }
    fn start_undoable_action(&self) -> Result<(), DocumentError> {
        TextDocument::start_undoable_action(self)
    }
    fn end_undoable_action(&self) -> Result<(), DocumentError> {
        self.end_update()
    }
    fn open_undo_group(&self) -> Result<Box<dyn UndoableAction>, DocumentError> {
        Ok(Box::new(self.begin_update()?))
    }
    fn create_anchor(&self, offset: usize) -> Result<Rc<dyn ITextAnchor>, DocumentError> {
        Ok(TextDocument::create_anchor(self, offset)?.as_text_anchor())
    }
    fn file_name(&self) -> Option<String> {
        TextDocument::file_name(self)
    }
    fn file_name_changed(&self, handler: Rc<dyn Fn()>) -> DocumentSubscription {
        TextDocument::file_name_changed(self, move || handler())
    }
    fn is_same_document_as(&self, other: &dyn IDocument) -> bool {
        let other: &dyn Any = other;
        other
            .downcast_ref::<Self>()
            .is_some_and(|other| self == other)
    }
}

#[cfg(test)]
mod tests;
