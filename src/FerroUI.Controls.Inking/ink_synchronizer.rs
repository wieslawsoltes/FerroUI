use super::{InkError, InkPresenter, InkStroke};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// Transactional handoff from presenter-owned wet ink to application-owned dry ink.
pub struct InkSynchronizer {
    presenter: Weak<InkPresenter>,
    active_batch: RefCell<Option<Vec<u32>>>,
}
impl PartialEq for InkSynchronizer {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl InkSynchronizer {
    pub(crate) fn new(presenter: Weak<InkPresenter>) -> Self {
        Self {
            presenter,
            active_batch: RefCell::new(None),
        }
    }
    pub fn begin_dry(&self) -> Result<Vec<Rc<InkStroke>>, InkError> {
        if self.active_batch.borrow().is_some() {
            return Err(InkError::InvalidOperation(
                "a drying transaction is already active",
            ));
        }
        let presenter = self
            .presenter
            .upgrade()
            .ok_or(InkError::InvalidOperation("presenter was released"))?;
        let strokes = presenter.pending_dry_strokes();
        *self.active_batch.borrow_mut() = Some(strokes.iter().map(|s| s.id()).collect());
        Ok(strokes)
    }
    pub fn end_dry(&self) -> Result<(), InkError> {
        let ids = self
            .active_batch
            .borrow_mut()
            .take()
            .ok_or(InkError::InvalidOperation("begin_dry must precede end_dry"))?;
        let presenter = self
            .presenter
            .upgrade()
            .ok_or(InkError::InvalidOperation("presenter was released"))?;
        presenter.finish_drying(&ids);
        Ok(())
    }
}
