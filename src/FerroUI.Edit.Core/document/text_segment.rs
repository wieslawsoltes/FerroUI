use super::{DocumentSubscription, ISegment};
use crate::utils::Event;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// A range error or an invalid collection ownership transition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SegmentError {
    RangeOverflow,
    InvalidEndOffset,
    AlreadyConnected,
    NotInCollection,
    AutomaticUpdates,
    OutsideDocument {
        end_offset: usize,
        text_length: usize,
    },
}

impl std::fmt::Display for SegmentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RangeOverflow => f.write_str("Segment range overflows the offset type"),
            Self::InvalidEndOffset => f.write_str("Segment end precedes its start"),
            Self::AlreadyConnected => f.write_str("Segment already belongs to a collection"),
            Self::NotInCollection => f.write_str("Segment does not belong to this collection"),
            Self::AutomaticUpdates => {
                f.write_str("Connected collections update offsets automatically")
            }
            Self::OutsideDocument {
                end_offset,
                text_length,
            } => write!(
                f,
                "Segment end {end_offset} exceeds document length {text_length}"
            ),
        }
    }
}
impl std::error::Error for SegmentError {}

pub(crate) trait ISegmentTree {
    fn offset(&self, node: usize) -> usize;
    fn set_start(&self, node: usize, offset: usize) -> Result<(), SegmentError>;
    fn set_length(&self, node: usize, length: usize) -> Result<(), SegmentError>;
}

pub(crate) struct SegmentInner {
    pub start: Cell<usize>,
    pub length: Cell<usize>,
    pub node: Cell<Option<usize>>,
    pub owner: RefCell<Option<Weak<dyn ISegmentTree>>>,
    changed: Event<()>,
}

/// A shared mutable interval, optionally owned by one tracked collection.
/// Start changes preserve length; end changes preserve start. Document changes
/// keep intervals as small as possible and never remove them automatically.
#[derive(Clone)]
pub struct TextSegment(pub(crate) Rc<SegmentInner>);

impl Default for TextSegment {
    fn default() -> Self {
        Self::new()
    }
}
impl PartialEq for TextSegment {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for TextSegment {}
impl std::fmt::Debug for TextSegment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextSegment")
            .field("start_offset", &self.start_offset())
            .field("length", &self.length())
            .finish()
    }
}
impl TextSegment {
    pub fn new() -> Self {
        Self(Rc::new(SegmentInner {
            start: Cell::new(0),
            length: Cell::new(0),
            node: Cell::new(None),
            owner: RefCell::new(None),
            changed: Event::default(),
        }))
    }
    pub fn from_range(start_offset: usize, length: usize) -> Result<Self, SegmentError> {
        start_offset
            .checked_add(length)
            .ok_or(SegmentError::RangeOverflow)?;
        let value = Self::new();
        value.0.start.set(start_offset);
        value.0.length.set(length);
        Ok(value)
    }
    pub(crate) fn owner(&self) -> Option<Rc<dyn ISegmentTree>> {
        self.0.owner.borrow().as_ref().and_then(Weak::upgrade)
    }
    pub fn is_connected_to_collection(&self) -> bool {
        self.owner().is_some()
    }
    pub fn start_offset(&self) -> usize {
        match (self.owner(), self.0.node.get()) {
            (Some(owner), Some(node)) => owner.offset(node),
            _ => self.0.start.get(),
        }
    }
    pub fn offset(&self) -> usize {
        self.start_offset()
    }
    pub fn length(&self) -> usize {
        self.0.length.get()
    }
    pub fn end_offset(&self) -> usize {
        self.start_offset() + self.length()
    }
    pub fn set_start_offset(&self, offset: usize) -> Result<(), SegmentError> {
        offset
            .checked_add(self.length())
            .ok_or(SegmentError::RangeOverflow)?;
        if offset == self.start_offset() {
            return Ok(());
        }
        if let (Some(owner), Some(node)) = (self.owner(), self.0.node.get()) {
            owner.set_start(node, offset)?;
        }
        self.0.start.set(offset);
        self.0.changed.raise(&());
        Ok(())
    }
    pub fn set_length(&self, length: usize) -> Result<(), SegmentError> {
        self.start_offset()
            .checked_add(length)
            .ok_or(SegmentError::RangeOverflow)?;
        if length == self.length() {
            return Ok(());
        }
        if let (Some(owner), Some(node)) = (self.owner(), self.0.node.get()) {
            owner.set_length(node, length)?;
        }
        self.0.length.set(length);
        self.0.changed.raise(&());
        Ok(())
    }
    pub fn set_end_offset(&self, offset: usize) -> Result<(), SegmentError> {
        self.set_length(
            offset
                .checked_sub(self.start_offset())
                .ok_or(SegmentError::InvalidEndOffset)?,
        )
    }
    /// Explicit property changes only; automatic document tracking is silent.
    pub fn segment_changed(&self, handler: impl Fn() + 'static) -> DocumentSubscription {
        self.0.changed.subscribe(move |_| handler())
    }
    pub(crate) fn detach(&self, start: usize) {
        self.0.start.set(start);
        self.0.node.set(None);
        self.0.owner.borrow_mut().take();
    }
}
impl ISegment for TextSegment {
    fn offset(&self) -> usize {
        self.start_offset()
    }
    fn length(&self) -> usize {
        self.length()
    }
}

/// Composition replaces inheriting from a mutable segment class. Implement this
/// on an extension's handle to store custom data in a typed segment collection.
pub trait ITextSegment: Clone + 'static {
    fn text_segment(&self) -> &TextSegment;
}
impl ITextSegment for TextSegment {
    fn text_segment(&self) -> &TextSegment {
        self
    }
}
