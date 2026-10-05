use super::Page;
use ferroui_base::Ref;

/// Provides data for the `ModalPopped` event of a navigation page.
#[derive(Clone, PartialEq)]
pub struct ModalPoppedEventArgs {
    modal: Ref<Page>,
}

impl ModalPoppedEventArgs {
    /// Creates the args for the modal page that was popped.
    pub fn new(modal: Ref<Page>) -> Self {
        Self { modal }
    }

    /// The modal page that was popped.
    pub fn modal(&self) -> Ref<Page> {
        self.modal.clone()
    }
}
