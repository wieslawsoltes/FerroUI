use crate::{PreparedReload, ReloadError};
use ferroui_base::{FerroObject, Ref};
use ferroui_markup_xaml::reload_events::{EventGeneration, HandlerChange, ReloadCodeBehind};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone)]
pub(crate) struct OwnedHandlers(Rc<RefCell<Rc<EventGeneration>>>);
impl OwnedHandlers {
    pub(crate) fn capture(root: &Ref<FerroObject>) -> Self {
        Self(Rc::new(RefCell::new(EventGeneration::current(root))))
    }
    pub(crate) fn prepare(
        &self,
        code: &Rc<ReloadCodeBehind>,
    ) -> Result<Box<dyn PreparedReload>, ReloadError> {
        Ok(Box::new(Change {
            owner: self.clone(),
            change: code
                .prepare(self.0.borrow().clone())
                .map_err(ReloadError::Failed)?,
        }))
    }
}
struct Change {
    owner: OwnedHandlers,
    change: HandlerChange,
}
impl PreparedReload for Change {
    fn commit(&mut self) -> Result<(), ReloadError> {
        self.change.commit().map_err(ReloadError::Failed)
    }
    fn rollback(&mut self) {
        self.change.rollback();
    }
    fn finish(&mut self) {
        *self.owner.0.borrow_mut() = self.change.finish();
    }
}
