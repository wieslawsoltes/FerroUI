use super::folding_events::FoldingEvents;
use crate::document::{
    DocumentError, DocumentSubscription, ISegment, ITextSegment, SegmentError, TextDocument,
    TextSegment,
};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

struct SectionInner {
    _segment_subscription: DocumentSubscription,
    segment: TextSegment,
    document: TextDocument,
    folded: Cell<bool>,
    title: RefCell<Option<String>>,
    tag: RefCell<Option<Rc<dyn Any>>>,
    definition: Cell<bool>,
    events: RefCell<Weak<FoldingEvents>>,
}
#[derive(Clone)]
pub struct FoldingSection {
    inner: Rc<SectionInner>,
}
impl FoldingSection {
    pub(crate) fn new(
        document: &TextDocument,
        start: usize,
        end: usize,
        events: &Rc<FoldingEvents>,
    ) -> Result<Self, SegmentError> {
        let segment = TextSegment::from_range(
            start,
            end.checked_sub(start)
                .ok_or(SegmentError::InvalidEndOffset)?,
        )?;
        let inner = Rc::new_cyclic(|inner: &Weak<SectionInner>| {
            let weak = inner.clone();
            let subscription = segment.segment_changed(move || {
                if let Some(inner) = weak.upgrade() {
                    Self { inner }.notify();
                }
            });
            SectionInner {
                _segment_subscription: subscription,
                segment,
                document: document.clone(),
                folded: Cell::new(false),
                title: RefCell::new(None),
                tag: RefCell::new(None),
                definition: Cell::new(false),
                events: RefCell::new(Rc::downgrade(events)),
            }
        });
        Ok(Self { inner })
    }
    pub fn start_offset(&self) -> usize {
        self.inner.segment.start_offset()
    }
    pub fn end_offset(&self) -> usize {
        self.inner.segment.end_offset()
    }
    pub fn length(&self) -> usize {
        self.inner.segment.length()
    }
    /// Moves the range and retains its current UTF-16 length.
    pub fn set_start_offset(&self, offset: usize) -> Result<(), SegmentError> {
        self.check_range(offset, self.length())?;
        self.inner.segment.set_start_offset(offset)
    }
    /// Changes the end while retaining the start.
    pub fn set_end_offset(&self, offset: usize) -> Result<(), SegmentError> {
        self.set_length(
            offset
                .checked_sub(self.start_offset())
                .ok_or(SegmentError::InvalidEndOffset)?,
        )
    }
    pub fn set_length(&self, length: usize) -> Result<(), SegmentError> {
        self.check_range(self.start_offset(), length)?;
        self.inner.segment.set_length(length)
    }
    fn check_range(&self, start: usize, length: usize) -> Result<(), SegmentError> {
        let end_offset = start
            .checked_add(length)
            .ok_or(SegmentError::RangeOverflow)?;
        let text_length = self.inner.document.text_length();
        if end_offset > text_length {
            return Err(SegmentError::OutsideDocument {
                end_offset,
                text_length,
            });
        }
        Ok(())
    }
    /// Reads the current text in this UTF-16 range.
    pub fn text_content(&self) -> Result<String, DocumentError> {
        self.inner
            .document
            .get_text(self.start_offset(), self.length())
    }
    /// Retrieves typed metadata. Parser-created sections start with a `NewFolding` tag.
    pub fn tag<T: Any>(&self) -> Option<Rc<T>> {
        self.inner.tag.borrow().as_ref()?.clone().downcast().ok()
    }
    /// Replaces application data without changing the visible section.
    pub fn set_tag(&self, tag: Option<Rc<dyn Any>>) {
        drop(self.inner.tag.replace(tag));
    }
    pub fn is_folded(&self) -> bool {
        self.inner.folded.get()
    }
    pub fn set_is_folded(&self, folded: bool) {
        if self.inner.folded.replace(folded) != folded {
            self.notify();
        }
    }
    pub fn title(&self) -> Option<String> {
        self.inner.title.borrow().clone()
    }
    pub fn set_title(&self, title: Option<String>) {
        let changed = *self.inner.title.borrow() != title;
        *self.inner.title.borrow_mut() = title;
        if changed {
            self.notify();
        }
    }
    pub fn is_definition(&self) -> bool {
        self.inner.definition.get()
    }
    pub fn set_is_definition(&self, value: bool) {
        if self.inner.definition.replace(value) != value {
            self.notify();
        }
    }
    pub(super) fn detach(&self) {
        self.inner.folded.set(false);
        *self.inner.events.borrow_mut() = Weak::new();
    }
    fn notify(&self) {
        if self.inner.segment.is_connected_to_collection() {
            let events = self.inner.events.borrow().upgrade();
            if let Some(events) = events {
                events.notify();
            }
        }
    }
}

impl PartialEq for FoldingSection {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}
impl Eq for FoldingSection {}

impl ISegment for FoldingSection {
    fn offset(&self) -> usize {
        self.start_offset()
    }
    fn length(&self) -> usize {
        self.length()
    }
}

impl ITextSegment for FoldingSection {
    fn text_segment(&self) -> &TextSegment {
        &self.inner.segment
    }
}
