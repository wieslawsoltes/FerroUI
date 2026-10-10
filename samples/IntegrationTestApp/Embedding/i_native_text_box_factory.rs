//! Port of `Embedding/INativeTextBoxFactory.cs`.

use ferroui_base::reactive::IDisposable;
use ferroui_controls::platform::IPlatformHandle;
use std::rc::Rc;

/// The platform specific part of the native text box.
pub trait INativeTextBoxImpl {
    fn handle(&self) -> Rc<dyn IPlatformHandle>;
    fn text(&self) -> String;
    fn set_text(&self, value: &str);
    /// Occurs when the context menu of the text box is asked for. Disposing the returned handle
    /// unsubscribes.
    fn context_menu_requested(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable>;
    /// Occurs when the pointer rests on the text box.
    fn hovered(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable>;
    /// Occurs when the pointer leaves the text box.
    fn pointer_exited(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable>;
}

/// Creates the platform specific part of a native text box.
pub trait INativeTextBoxFactory {
    fn create_control(&self, parent: Rc<dyn IPlatformHandle>) -> Rc<dyn INativeTextBoxImpl>;
}
