use super::Page;
use ferroui_base::Ref;

/// Provides data for the `ModalPushed` event of a navigation page.
#[derive(Clone, PartialEq)]
pub struct ModalPushedEventArgs {
    modal: Ref<Page>,
}

impl ModalPushedEventArgs {
    /// Creates the args for the modal page that was pushed.
    pub fn new(modal: Ref<Page>) -> Self {
        Self { modal }
    }

    /// The modal page that was pushed.
    pub fn modal(&self) -> Ref<Page> {
        self.modal.clone()
    }
}
