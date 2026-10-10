//! The native control host of a view: the views of an application become
//! subviews of the view of a top-level, where the layout of the framework
//! puts them. And the handle of a `UIView`.

use ferroui_base::{Rect, Size};

fn math_max(val1: f64, val2: f64) -> f64 {
    if val1.is_nan() || val2.is_nan() {
        f64::NAN
    } else {
        val1.max(val2)
    }
}

/// The frame of a hidden native control of a size: at the origin, at
/// least one point wide and high.
pub fn hidden_frame(size: Size) -> Rect {
    Rect::new(0.0, 0.0, math_max(1.0, size.width), math_max(1.0, size.height))
}

/// The frame of a native control shown in bounds: at least one point
/// wide and high.
pub fn shown_frame(bounds: Rect) -> Rect {
    Rect::new(bounds.x, bounds.y, math_max(1.0, bounds.width), math_max(1.0, bounds.height))
}

#[cfg(target_os = "ios")]
pub use uikit::UIViewControlHandle;
#[cfg(target_os = "ios")]
pub(crate) use uikit::NativeControlHostImpl;

#[cfg(target_os = "ios")]
mod uikit {
    use super::{hidden_frame, shown_frame};
    use crate::extensions::to_cg_rect;
    use crate::ferro_view::FerroView;
    use ferroui_base::reactive::IDisposable;
    use ferroui_base::{Rect, Size};
    use ferroui_controls::platform::{
        INativeControlHostControlTopLevelAttachment, INativeControlHostDestroyableControlHandle,
        INativeControlHostImpl, IPlatformHandle,
    };
    use objc2::rc::{Retained, Weak};
    use objc2::{MainThreadMarker, MainThreadOnly};
    use objc2_ui_kit::UIView;
    use std::any::Any;
    use std::cell::RefCell;
    use std::rc::{Rc, Weak as RcWeak};

    /// The native control host of a view.
    pub(crate) struct NativeControlHostImpl {
        this: RcWeak<NativeControlHostImpl>,
        ferro_view: Weak<FerroView>,
    }

    impl NativeControlHostImpl {
        pub(crate) fn new(ferro_view: Weak<FerroView>) -> Rc<Self> {
            Rc::new_cyclic(|this| Self { this: this.clone(), ferro_view })
        }

        fn rc(&self) -> Rc<NativeControlHostImpl> {
            match self.this.upgrade() {
                Some(this) => this,
                None => panic!("The native control host is used while it is released."),
            }
        }

        fn view(&self) -> Retained<FerroView> {
            match self.ferro_view.load() {
                Some(view) => view,
                None => panic!("The view of the native control host was released."),
            }
        }
    }

    impl INativeControlHostImpl for NativeControlHostImpl {
        fn as_any(&self) -> &dyn Any {
            self
        }

        fn create_default_child(
            &self,
            _parent: Rc<dyn IPlatformHandle>,
        ) -> Rc<dyn INativeControlHostDestroyableControlHandle> {
            let mtm = self.view().mtm();
            Rc::new(UIViewControlHandle::new(UIView::new(mtm)))
        }

        fn create_new_attachment_with(
            &self,
            create: Rc<dyn Fn(Rc<dyn IPlatformHandle>) -> Rc<dyn IPlatformHandle>>,
        ) -> Rc<dyn INativeControlHostControlTopLevelAttachment> {
            let parent = Rc::new(UIViewControlHandle::new(Retained::into_super(self.view())));
            // The reference disposes the attachment when attaching it
            // fails; attaching does not fail here with an error that
            // could be handed on.
            let child = create(parent);
            let attachment = Rc::new(NativeControlAttachment::new(child));
            attachment.set_attached_to(Some(self.rc()));
            attachment
        }

        fn create_new_attachment(
            &self,
            handle: Rc<dyn IPlatformHandle>,
        ) -> Rc<dyn INativeControlHostControlTopLevelAttachment> {
            let attachment = Rc::new(NativeControlAttachment::new(handle));
            attachment.set_attached_to(Some(self.rc()));
            attachment
        }

        fn is_compatible_with(&self, handle: &dyn IPlatformHandle) -> bool {
            handle.handle_descriptor() == Some(UIViewControlHandle::UI_VIEW_DESCRIPTOR)
        }
    }

    /// The view of a handle that is not a handle of this crate: the
    /// pointer of the handle, which its descriptor says is a `UIView`.
    fn view_holder(handle: &dyn IPlatformHandle) -> Option<Retained<UIView>> {
        MainThreadMarker::new()?;
        if handle.handle_descriptor() != Some(UIViewControlHandle::UI_VIEW_DESCRIPTOR) {
            return None;
        }
        // SAFETY: a handle with the descriptor of a `UIView` holds the
        // pointer of a live `UIView` (the contract of the descriptor);
        // the view is retained for as long as the attachment has it. The
        // attachment is used on the main thread, which was just checked.
        unsafe { Retained::retain(handle.handle() as *mut UIView) }
    }

    struct NativeControlAttachment {
        // Keeps the handle, and with it the view, alive.
        child: RefCell<Option<Rc<dyn IPlatformHandle>>>,
        view: RefCell<Option<Retained<UIView>>>,
        attached_to: RefCell<Option<Rc<NativeControlHostImpl>>>,
    }

    impl NativeControlAttachment {
        fn new(child: Rc<dyn IPlatformHandle>) -> Self {
            let view = match child.as_any().downcast_ref::<UIViewControlHandle>() {
                Some(handle) => Some(handle.view().clone()),
                None => view_holder(&*child),
            };
            let Some(view) = view else {
                panic!("The handle of a native control is not the handle of a UIView.");
            };

            Self { child: RefCell::new(Some(child)), view: RefCell::new(Some(view)), attached_to: RefCell::new(None) }
        }

        fn check_disposed(&self) -> Retained<UIView> {
            match self.view.borrow().clone() {
                Some(view) => view,
                None => panic!("Cannot access a disposed object. Object name: 'NativeControlAttachment'."),
            }
        }
    }

    impl IDisposable for NativeControlAttachment {
        fn dispose(&self) {
            let view = self.view.borrow_mut().take();
            if let Some(view) = &view {
                view.removeFromSuperview();
            }
            *self.child.borrow_mut() = None;
            *self.attached_to.borrow_mut() = None;
        }
    }

    impl INativeControlHostControlTopLevelAttachment for NativeControlAttachment {
        fn attached_to(&self) -> Option<Rc<dyn INativeControlHostImpl>> {
            self.attached_to.borrow().clone().map(|host| host as Rc<dyn INativeControlHostImpl>)
        }

        fn set_attached_to(&self, value: Option<Rc<dyn INativeControlHostImpl>>) {
            let view = self.check_disposed();

            let host = value.map(|value| match value.as_any().downcast_ref::<NativeControlHostImpl>() {
                Some(host) => host.rc(),
                None => panic!("Unable to cast the native control host to NativeControlHostImpl."),
            });
            match &host {
                None => view.removeFromSuperview(),
                Some(host) => host.view().addSubview(&view),
            }
            *self.attached_to.borrow_mut() = host;
        }

        fn is_compatible_with(&self, host: &dyn INativeControlHostImpl) -> bool {
            host.as_any().is::<NativeControlHostImpl>()
        }

        fn hide_with_size(&self, size: Size) {
            let view = self.check_disposed();
            if self.attached_to.borrow().is_none() {
                return;
            }

            view.setHidden(true);
            view.setFrame(to_cg_rect(hidden_frame(size)));
        }

        fn show_in_bounds(&self, bounds: Rect) {
            let view = self.check_disposed();
            if self.attached_to.borrow().is_none() {
                panic!("The control isn't currently attached to a toplevel");
            }

            view.setFrame(to_cg_rect(shown_frame(bounds)));
            view.setHidden(false);
        }
    }

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

    impl INativeControlHostDestroyableControlHandle for UIViewControlHandle {
        fn destroy(&self) {
            // The reference disposes its wrapper of the view, which
            // releases its reference; the reference of the port is
            // released with the handle. What the application sees is
            // that the view leaves the view tree.
            self.view.removeFromSuperview();
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of this file.
    use super::*;

    #[test]
    fn a_hidden_control_keeps_a_size_at_the_origin() {
        assert_eq!(Rect::new(0.0, 0.0, 120.0, 40.0), hidden_frame(Size::new(120.0, 40.0)));
        assert_eq!(Rect::new(0.0, 0.0, 1.0, 1.0), hidden_frame(Size::new(0.0, 0.5)));
    }

    #[test]
    fn a_shown_control_is_at_least_a_point_wide_and_high() {
        assert_eq!(Rect::new(10.0, 20.0, 120.0, 40.0), shown_frame(Rect::new(10.0, 20.0, 120.0, 40.0)));
        assert_eq!(Rect::new(10.0, 20.0, 1.0, 1.0), shown_frame(Rect::new(10.0, 20.0, 0.0, -3.0)));
    }
}
