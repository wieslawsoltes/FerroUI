use super::IPlatformHandle;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{Rect, Size};
use std::rc::Rc;

/// Hosts native controls inside a top-level.
pub trait INativeControlHostImpl {
    /// Creates the default native child of `parent`.
    fn create_default_child(&self, parent: Rc<dyn IPlatformHandle>) -> Rc<dyn INativeControlHostDestroyableControlHandle>;

    /// Creates an attachment for a native control that `create` creates
    /// from its parent handle.
    fn create_new_attachment_with(
        &self,
        create: Rc<dyn Fn(Rc<dyn IPlatformHandle>) -> Rc<dyn IPlatformHandle>>,
    ) -> Rc<dyn INativeControlHostControlTopLevelAttachment>;

    /// Creates an attachment for an existing native control.
    fn create_new_attachment(&self, handle: Rc<dyn IPlatformHandle>) -> Rc<dyn INativeControlHostControlTopLevelAttachment>;

    /// Whether the host can host the native control behind `handle`.
    fn is_compatible_with(&self, handle: &dyn IPlatformHandle) -> bool;
}

/// A native control handle that can be destroyed.
pub trait INativeControlHostDestroyableControlHandle: IPlatformHandle {
    fn destroy(&self);
}

/// The attachment of a native control to a top-level.
pub trait INativeControlHostControlTopLevelAttachment: IDisposable {
    /// The host the control is attached to.
    fn attached_to(&self) -> Option<Rc<dyn INativeControlHostImpl>>;

    /// Attaches the control to a host, or detaches it.
    fn set_attached_to(&self, value: Option<Rc<dyn INativeControlHostImpl>>);

    /// Whether the attachment can be moved to `host`.
    fn is_compatible_with(&self, host: &dyn INativeControlHostImpl) -> bool;

    /// Hides the control, keeping it at the specified size.
    fn hide_with_size(&self, size: Size);

    /// Shows the control in the specified bounds.
    fn show_in_bounds(&self, rect: Rect);
}
