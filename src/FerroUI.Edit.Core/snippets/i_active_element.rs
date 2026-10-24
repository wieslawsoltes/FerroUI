use super::{SnippetElementRef, SnippetError, SnippetEventArgs, SnippetSession};
use crate::document::{DocumentSubscription, SimpleSegment};
use std::rc::Rc;

/// An interactive element activated after the prepared expansion is accepted.
/// Callbacks may inspect or subscribe to the context without retained borrows.
pub trait IActiveElement {
    fn on_insertion_completed(&self, _context: &SnippetSession) -> Result<(), SnippetError> {
        Ok(())
    }
    fn deactivate(&self, _args: &SnippetEventArgs) {}
    fn is_editable(&self) -> bool;
    fn segment(&self) -> Option<SimpleSegment>;
}

pub trait IReplaceableActiveElement: IActiveElement {
    fn text(&self) -> Result<String, SnippetError>;
    fn text_changed(&self, handler: Rc<dyn Fn()>) -> DocumentSubscription;
}

pub(crate) type ActiveElementFactory =
    dyn Fn(&SnippetSession) -> Result<Rc<dyn IActiveElement>, SnippetError>;

pub(crate) struct ActiveElementRegistration {
    pub owner: SnippetElementRef,
    pub factory: Rc<ActiveElementFactory>,
}
