//! The handle of a `UIView`.
//!
//! Stage 2 of `docs/porting/ios-platform.md` adds the rest of the file:
//! the native control host of a view (`NativeControlHostImpl`), which
//! attaches the views of an application to the view of a top-level, and
//! with it the destruction of a handle the host created.

use ferroui_controls::platform::IPlatformHandle;
use objc2::rc::Retained;
use objc2_ui_kit::UIView;
use std::any::Any;

/// The platform handle of a `UIView`.
pub struct UIViewControlHandle {
    view: Retained<UIView>,
}

impl UIViewControlHandle {
    /// The descriptor of the handle.
    pub const UI_VIEW_DESCRIPTOR: &'static str = "UIView";

    /// Creates the handle of `view`.
    pub fn new(view: Retained<UIView>) -> Self {
        Self { view }
    }

    /// The view.
    pub fn view(&self) -> &Retained<UIView> {
        &self.view
    }
}

impl IPlatformHandle for UIViewControlHandle {
    fn handle(&self) -> isize {
        Retained::as_ptr(&self.view) as isize
    }

    fn handle_descriptor(&self) -> Option<&str> {
        Some(Self::UI_VIEW_DESCRIPTOR)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
