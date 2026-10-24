use super::{
    AnchorMovementType, DocumentError, DocumentSubscription, SimpleSegment, TextDocument,
    TextLocation,
};
use crate::utils::Event;
use std::cell::Cell;
use std::rc::Rc;

pub(crate) struct AnchorInner {
    pub document: TextDocument,
    pub node: Cell<Option<usize>>,
    pub movement: Cell<AnchorMovementType>,
    pub survive: Cell<bool>,
    pub deleted: Cell<bool>,
    pub deleted_event: Event<()>,
}

impl Drop for AnchorInner {
    fn drop(&mut self) {
        if let Some(node) = self.node.take() {
            self.document.remove_anchor(node);
        }
    }
}

/// A position that follows document edits. The document holds weak anchor
/// references, so dropping the last handle releases the anchor automatically.
#[derive(Clone)]
pub struct TextAnchor(pub(crate) Rc<AnchorInner>);

impl std::fmt::Debug for TextAnchor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextAnchor")
            .field("offset", &self.offset())
            .field("movement", &self.movement_type())
            .finish()
    }
}
impl PartialEq for TextAnchor {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for TextAnchor {}

impl TextAnchor {
    pub(crate) fn new(document: TextDocument) -> Self {
        Self(Rc::new(AnchorInner {
            document,
            node: Cell::new(None),
            movement: Cell::new(AnchorMovementType::Default),
            survive: Cell::new(false),
            deleted: Cell::new(false),
            deleted_event: Event::default(),
        }))
    }
    pub fn document(&self) -> TextDocument {
        self.0.document.clone()
    }
    pub fn offset(&self) -> Result<usize, DocumentError> {
        self.0
            .node
            .get()
            .and_then(|node| self.0.document.anchor_offset(node))
            .ok_or(DocumentError::DeletedAnchor)
    }
    pub fn is_deleted(&self) -> bool {
        self.0.deleted.get()
    }
    pub fn movement_type(&self) -> AnchorMovementType {
        self.0.movement.get()
    }
    pub fn set_movement_type(&self, value: AnchorMovementType) {
        self.0.movement.set(value);
    }
    pub fn survive_deletion(&self) -> bool {
        self.0.survive.get()
    }
    pub fn set_survive_deletion(&self, value: bool) {
        self.0.survive.set(value);
    }
    pub fn location(&self) -> Result<TextLocation, DocumentError> {
        self.0.document.get_location(self.offset()?)
    }
    pub fn line(&self) -> Result<usize, DocumentError> {
        Ok(self.location()?.line)
    }
    pub fn column(&self) -> Result<usize, DocumentError> {
        Ok(self.location()?.column)
    }
    pub fn deleted(&self, handler: impl Fn() + 'static) -> DocumentSubscription {
        self.0.deleted_event.subscribe(move |_| handler())
    }
}

/// A range whose two surviving anchors follow insertions and deletions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnchorSegment {
    pub start: TextAnchor,
    pub end: TextAnchor,
}

impl AnchorSegment {
    pub fn new(
        document: &TextDocument,
        offset: usize,
        length: usize,
    ) -> Result<Self, DocumentError> {
        if offset > document.text_length() || length > document.text_length() - offset {
            return Err(DocumentError::InvalidRange {
                offset,
                length,
                text_length: document.text_length(),
            });
        }
        let start = document.create_anchor(offset)?;
        let end = document.create_anchor(offset + length)?;
        start.set_survive_deletion(true);
        end.set_survive_deletion(true);
        start.set_movement_type(AnchorMovementType::AfterInsertion);
        end.set_movement_type(AnchorMovementType::BeforeInsertion);
        Ok(Self { start, end })
    }
    pub fn offset(&self) -> Result<usize, DocumentError> {
        self.start.offset()
    }
    pub fn end_offset(&self) -> Result<usize, DocumentError> {
        Ok(self.end.offset()?.max(self.start.offset()?))
    }
    pub fn length(&self) -> Result<usize, DocumentError> {
        Ok(self.end_offset()? - self.offset()?)
    }
    pub fn segment(&self) -> Result<SimpleSegment, DocumentError> {
        Ok(SimpleSegment::new(self.offset()?, self.length()?))
    }
}
