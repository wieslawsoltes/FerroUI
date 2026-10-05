use crate::interactivity::RoutedEventArgs;
use crate::reactive::IDisposable;
use std::rc::Rc;

/// Defines the interface for a top-level menu.
pub trait IMainMenu {
    /// Whether the menu is open.
    fn is_open(&self) -> bool;

    /// Closes the menu.
    fn close(&self);

    /// Opens the menu in response to the Alt/F10 key.
    fn open(&self);

    /// Occurs when the main menu closes. Disposing the returned handle
    /// unsubscribes.
    fn closed(&self, handler: Rc<dyn Fn(&RoutedEventArgs)>) -> Rc<dyn IDisposable>;
}
