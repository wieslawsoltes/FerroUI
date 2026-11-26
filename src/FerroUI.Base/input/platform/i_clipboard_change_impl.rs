use super::IClipboardImpl;
use crate::reactive::Subscription;
use std::rc::Rc;

/// Represents a clipboard implementation that reports changes of the content of the system
/// clipboard, made by this application or by another one.
///
/// This is an implementation detail of the platform backends. A platform observes the clipboard
/// only while a subscription is alive, so an application that does not ask pays nothing.
pub trait IClipboardChangeImpl: IClipboardImpl {
    /// Calls `handler` on the UI thread after each change of the content of the clipboard, until
    /// the subscription is dropped.
    fn subscribe_content_changed(&self, handler: Rc<dyn Fn()>) -> Subscription;
}
