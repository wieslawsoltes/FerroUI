//! Port of `Win32NativeControlHost.cs`: windows of other toolkits inside a
//! window of the backend. A native control is a child of a holder window
//! (a child window of the top-level that does nothing of its own), which
//! is moved, shown and hidden with the control that hosts it.
//!
//! Where the reference throws, the port panics with the message of the
//! exception.

use ferroui_base::{PixelRect, Rect, Size};

/// The size a hidden child is given: the size of its host in pixels, at
/// least one pixel each way.
pub(crate) fn hidden_child_size(size: Size, render_scaling: f64) -> (i32, i32) {
    let size = size * render_scaling;
    (1.max(size.width as i32), 1.max(size.height as i32))
}

/// The bounds of a holder in the pixels of its top-level: the position
/// truncated, the size at least one pixel each way.
pub(crate) fn holder_pixel_rect(bounds: Rect, render_scaling: f64) -> PixelRect {
    let bounds = bounds * render_scaling;
    PixelRect::new(bounds.x as i32, bounds.y as i32, 1.max(bounds.width as i32), 1.max(bounds.height as i32))
}

#[cfg(windows)]
pub(crate) use imp::Win32NativeControlHost;

#[cfg(windows)]
mod imp {
    use super::*;
    use crate::interop::unmanaged_methods::{
        create_window_ex, default_wnd_proc, destroy_window, invalidate_window, move_window, register_class_ex, set_layered_window_alpha,
        set_parent, set_window_pos, show_window, unregister_class, SetWindowPosFlags, ShowWindowCommand, WindowStyles,
    };
    use crate::offscreen_parent_window::OffscreenParentWindow;
    use crate::window_impl::WindowImpl;
    use ferroui_base::reactive::IDisposable;
    use ferroui_controls::platform::{
        INativeControlHostControlTopLevelAttachment, INativeControlHostDestroyableControlHandle, INativeControlHostImpl,
        IPlatformHandle, ITopLevelImpl,
    };
    use std::any::Any;
    use std::cell::{Cell, RefCell};
    use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
    use std::rc::{Rc, Weak};
    use std::sync::atomic::{AtomicU64, Ordering};

    pub(crate) struct Win32NativeControlHost {
        this: Weak<Win32NativeControlHost>,
        use_layered_window: bool,
        window: Weak<WindowImpl>,
    }

    impl Win32NativeControlHost {
        pub(crate) fn new(window: Weak<WindowImpl>, use_layered_window: bool) -> Rc<Self> {
            Rc::new_cyclic(|this| Self { this: this.clone(), use_layered_window, window })
        }

        /// The handle of the window of the host; 0 once the window is
        /// gone.
        fn window_handle(&self) -> isize {
            self.window.upgrade().map_or(0, |window| window.hwnd())
        }

        fn window_render_scaling(&self) -> f64 {
            self.window.upgrade().map_or(1.0, |window| ITopLevelImpl::render_scaling(&*window))
        }

        fn assert_compatible(&self, handle: &dyn IPlatformHandle) {
            if !INativeControlHostImpl::is_compatible_with(self, handle) {
                panic!("Don't know what to do with {}", handle.handle_descriptor().unwrap_or(""));
            }
        }

        fn as_host(&self) -> Option<Rc<dyn INativeControlHostImpl>> {
            let this: Rc<dyn INativeControlHostImpl> = self.this.upgrade()?;
            Some(this)
        }
    }

    impl INativeControlHostImpl for Win32NativeControlHost {
        fn as_any(&self) -> &dyn Any {
            self
        }

        fn create_default_child(&self, parent: Rc<dyn IPlatformHandle>) -> Rc<dyn INativeControlHostDestroyableControlHandle> {
            self.assert_compatible(&*parent);
            Rc::new(DumbWindow::new(false, Some(parent.handle())))
        }

        fn create_new_attachment_with(
            &self,
            create: Rc<dyn Fn(Rc<dyn IPlatformHandle>) -> Rc<dyn IPlatformHandle>>,
        ) -> Rc<dyn INativeControlHostControlTopLevelAttachment> {
            let holder = Rc::new(DumbWindow::new(self.use_layered_window, Some(self.window_handle())));
            // It has to be disposed when the creation or the attachment
            // fails.
            let attachment: RefCell<Option<Rc<Win32NativeControlAttachment>>> = RefCell::new(None);
            let result = catch_unwind(AssertUnwindSafe(|| {
                let holder_handle: Rc<dyn IPlatformHandle> = holder.clone();
                let child = create(holder_handle);
                let created = Rc::new(Win32NativeControlAttachment::new(holder.clone(), child));
                *attachment.borrow_mut() = Some(created.clone());
                created.set_attached_to(self.as_host());
                created
            }));
            match result {
                Ok(attachment) => attachment,
                Err(panic) => {
                    if let Some(attachment) = attachment.borrow_mut().take() {
                        attachment.dispose();
                    }
                    holder.destroy();
                    resume_unwind(panic)
                }
            }
        }

        fn create_new_attachment(&self, handle: Rc<dyn IPlatformHandle>) -> Rc<dyn INativeControlHostControlTopLevelAttachment> {
            self.assert_compatible(&*handle);
            let attachment = Rc::new(Win32NativeControlAttachment::new(
                Rc::new(DumbWindow::new(self.use_layered_window, Some(self.window_handle()))),
                handle,
            ));
            attachment.set_attached_to(self.as_host());
            attachment
        }

        fn is_compatible_with(&self, handle: &dyn IPlatformHandle) -> bool {
            handle.handle_descriptor() == Some("HWND")
        }
    }

    /// The number of the next class of a holder window of the process.
    static NEXT_CLASS: AtomicU64 = AtomicU64::new(0);

    struct DumbWindow {
        handle: isize,
        class_name: String,
        released: Cell<bool>,
    }

    impl DumbWindow {
        fn new(layered: bool, parent: Option<isize>) -> Self {
            // Unique in the process, which is what the system requires;
            // the reference makes the name unique with a new identifier.
            let class_name =
                format!("FerroDumbWindow-{}-{}", std::process::id(), NEXT_CLASS.fetch_add(1, Ordering::Relaxed));

            let atom = register_class_ex(&class_name, 0, default_wnd_proc, 0);
            let handle = create_window_ex(
                if layered { WindowStyles::WS_EX_LAYERED.bits() } else { 0 },
                atom,
                WindowStyles::WS_CHILD.bits(),
                0,
                0,
                640,
                480,
                parent.unwrap_or_else(OffscreenParentWindow::handle),
            );

            if handle == 0 {
                unregister_class(&class_name);
                panic!("Unable to create child window for native control host. Application manifest with supported OS list might be required.");
            }

            if layered {
                set_layered_window_alpha(handle, 255);
            }

            Self { handle, class_name, released: Cell::new(false) }
        }

        fn release_unmanaged_resources(&self) {
            if !self.released.replace(true) {
                destroy_window(self.handle);
                unregister_class(&self.class_name);
            }
        }

        fn dispose(&self) {
            self.release_unmanaged_resources();
        }
    }

    impl Drop for DumbWindow {
        fn drop(&mut self) {
            self.release_unmanaged_resources();
        }
    }

    impl IPlatformHandle for DumbWindow {
        fn handle(&self) -> isize {
            self.handle
        }

        fn handle_descriptor(&self) -> Option<&str> {
            Some("HWND")
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    impl INativeControlHostDestroyableControlHandle for DumbWindow {
        fn destroy(&self) {
            self.dispose();
        }
    }

    struct Win32NativeControlAttachment {
        holder: RefCell<Option<Rc<DumbWindow>>>,
        child: RefCell<Option<Rc<dyn IPlatformHandle>>>,
        attached_to: RefCell<Option<Rc<dyn INativeControlHostImpl>>>,
    }

    impl Win32NativeControlAttachment {
        fn new(holder: Rc<DumbWindow>, child: Rc<dyn IPlatformHandle>) -> Self {
            set_parent(child.handle(), holder.handle);
            show_window(child.handle(), ShowWindowCommand::SHOW_NO_ACTIVATE);
            Self { holder: RefCell::new(Some(holder)), child: RefCell::new(Some(child)), attached_to: RefCell::new(None) }
        }

        fn check_disposed(&self) -> Rc<DumbWindow> {
            match self.holder.borrow().clone() {
                Some(holder) => holder,
                None => panic!("Cannot access a disposed object: Win32NativeControlAttachment"),
            }
        }

        /// The host the attachment is attached to, as the host of this
        /// backend it is.
        fn with_attached_to<T>(&self, f: impl FnOnce(&Win32NativeControlHost) -> T) -> Option<T> {
            let attached_to = self.attached_to.borrow().clone()?;
            attached_to.as_any().downcast_ref::<Win32NativeControlHost>().map(f)
        }
    }

    impl IDisposable for Win32NativeControlAttachment {
        fn dispose(&self) {
            if let Some(child) = self.child.borrow().as_ref() {
                set_parent(child.handle(), OffscreenParentWindow::handle());
            }
            if let Some(holder) = self.holder.borrow_mut().take() {
                holder.dispose();
            }
            *self.child.borrow_mut() = None;
            *self.attached_to.borrow_mut() = None;
        }
    }

    impl INativeControlHostControlTopLevelAttachment for Win32NativeControlAttachment {
        fn attached_to(&self) -> Option<Rc<dyn INativeControlHostImpl>> {
            self.attached_to.borrow().clone()
        }

        fn set_attached_to(&self, value: Option<Rc<dyn INativeControlHostImpl>>) {
            let holder = self.check_disposed();
            // A host of another backend is no host of this attachment.
            let value = value.filter(|value| value.as_any().is::<Win32NativeControlHost>());
            *self.attached_to.borrow_mut() = value;
            match self.with_attached_to(Win32NativeControlHost::window_handle) {
                None => {
                    show_window(holder.handle, ShowWindowCommand::HIDE);
                    set_parent(holder.handle, OffscreenParentWindow::handle());
                }
                Some(window) => {
                    set_parent(holder.handle, window);
                }
            }
        }

        fn is_compatible_with(&self, host: &dyn INativeControlHostImpl) -> bool {
            host.as_any().is::<Win32NativeControlHost>()
        }

        fn hide_with_size(&self, size: Size) {
            let holder = self.check_disposed();
            set_window_pos(
                holder.handle,
                0,
                -100,
                -100,
                1,
                1,
                SetWindowPosFlags::SWP_HIDEWINDOW | SetWindowPosFlags::SWP_NOACTIVATE,
            );
            let Some(render_scaling) = self.with_attached_to(Win32NativeControlHost::window_render_scaling) else {
                return;
            };
            let Some(child) = self.child.borrow().clone() else {
                return;
            };
            let (width, height) = hidden_child_size(size, render_scaling);
            move_window(child.handle(), 0, 0, width, height, false);
        }

        fn show_in_bounds(&self, bounds: Rect) {
            let holder = self.check_disposed();
            let Some((render_scaling, window)) =
                self.with_attached_to(|host| (host.window_render_scaling(), host.window_handle()))
            else {
                panic!("The control isn't currently attached to a toplevel");
            };
            let pixel_rect = holder_pixel_rect(bounds, render_scaling);

            if let Some(child) = self.child.borrow().as_ref() {
                move_window(child.handle(), 0, 0, pixel_rect.width, pixel_rect.height, true);
            }

            set_window_pos(
                holder.handle,
                0,
                pixel_rect.x,
                pixel_rect.y,
                pixel_rect.width,
                pixel_rect.height,
                SetWindowPosFlags::SWP_SHOWWINDOW | SetWindowPosFlags::SWP_NOZORDER | SetWindowPosFlags::SWP_NOACTIVATE,
            );

            invalidate_window(window, false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hidden_child_keeps_the_size_of_its_host_in_pixels() {
        assert_eq!(hidden_child_size(Size::new(100.0, 50.0), 1.0), (100, 50));
        assert_eq!(hidden_child_size(Size::new(100.5, 50.4), 2.0), (201, 100));
        assert_eq!(hidden_child_size(Size::new(0.0, 0.2), 1.5), (1, 1));
    }

    #[test]
    fn the_bounds_of_a_holder_are_pixels_of_at_least_one_each_way() {
        assert_eq!(holder_pixel_rect(Rect::new(10.0, 20.0, 100.0, 50.0), 1.0), PixelRect::new(10, 20, 100, 50));
        assert_eq!(holder_pixel_rect(Rect::new(10.3, 20.6, 100.5, 50.4), 2.0), PixelRect::new(20, 41, 201, 100));
        assert_eq!(holder_pixel_rect(Rect::new(-4.5, 0.0, 0.0, 0.3), 1.0), PixelRect::new(-4, 0, 1, 1));
    }
}
