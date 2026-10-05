use super::{IMainMenu, InputElement};
use crate::Ref;
use std::rc::Rc;

/// Defines the interface for classes that handle access keys for a window.
pub trait IAccessKeyHandler {
    /// The window's main menu.
    fn main_menu(&self) -> Option<Rc<dyn IMainMenu>>;

    /// Sets the window's main menu.
    fn set_main_menu(&self, value: Option<Rc<dyn IMainMenu>>);

    /// Sets the owner of the access key handler.
    ///
    /// This method can only be called once, typically by the owner itself
    /// on creation.
    fn set_owner(&self, owner: &Ref<InputElement>);

    /// Registers an input element to be associated with an access key.
    fn register(&self, access_key: &str, element: &Ref<InputElement>);

    /// Unregisters the access keys associated with the input element.
    fn unregister(&self, element: &InputElement);
}
