use super::folding_events::FoldingEvents;
use crate::document::{AnchorMovementType, DocumentError, TextAnchor, TextDocument};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

struct SectionInner {
    start: TextAnchor,
    end: RefCell<TextAnchor>,
    folded: Cell<bool>,
    title: RefCell<Option<String>>,
    definition: Cell<bool>,
    events: Weak<FoldingEvents>,
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
    ) -> Result<Self, DocumentError> {
        let start = document.create_anchor(start)?;
        let end = document.create_anchor(end)?;
        start.set_movement_type(AnchorMovementType::AfterInsertion);
        start.set_survive_deletion(true);
        end.set_movement_type(AnchorMovementType::BeforeInsertion);
        end.set_survive_deletion(true);
        Ok(Self {
            inner: Rc::new(SectionInner {
                start,
                end: RefCell::new(end),
                folded: Cell::new(false),
                title: RefCell::new(None),
                definition: Cell::new(false),
                events: Rc::downgrade(events),
            }),
        })
    }
    pub fn start_offset(&self) -> usize {
        self.inner.start.offset().unwrap_or(0)
    }
    pub fn end_offset(&self) -> usize {
        self.inner
            .end
            .borrow()
            .offset()
            .unwrap_or(self.start_offset())
            .max(self.start_offset())
    }
    pub fn length(&self) -> usize {
        self.end_offset() - self.start_offset()
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
    pub(crate) fn set_end_offset(
        &self,
        document: &TextDocument,
        offset: usize,
    ) -> Result<(), DocumentError> {
        let end = document.create_anchor(offset)?;
        end.set_movement_type(AnchorMovementType::BeforeInsertion);
        end.set_survive_deletion(true);
        *self.inner.end.borrow_mut() = end;
        self.notify();
        Ok(())
    }
    pub(crate) fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
    fn notify(&self) {
        if let Some(events) = self.inner.events.upgrade() {
            events.notify();
        }
    }
}
