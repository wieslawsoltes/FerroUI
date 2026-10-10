//! Windows of other clients and toolkits inside a window of the platform
//! (the port of `X11NativeControlHost.cs`). A native control is a child of
//! a holder window (a child window of the top-level that does nothing of
//! its own), which is moved, shown and hidden with the control that hosts
//! it.
//!
//! As the reference notes, this is not XEmbed: the window is reparented
//! with `XReparentWindow`.
//!
//! Where the reference throws, the port panics with the message of the
//! exception.

use crate::x11_info::X11Info;
use crate::x11_platform::FerroX11Platform;
use crate::x11_structs::{CreateWindowArgs, Gravity, SetWindowValuemask};
use crate::x11_window::X11Window;
use crate::xlib::{self, XDisplay, XID};

use ferroui_base::reactive::IDisposable;
use ferroui_base::{PixelRect, Rect, Size};
use ferroui_controls::platform::{
    INativeControlHostControlTopLevelAttachment, INativeControlHostDestroyableControlHandle, INativeControlHostImpl,
    IPlatformHandle,
};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::rc::{Rc, Weak};

/// The descriptor of a handle that is a window of the X server.
const XID_DESCRIPTOR: &str = "XID";

/// The size a hidden child is given: the size of its host in pixels, at
/// least one pixel each way.
pub(crate) fn hidden_child_size(size: Size, render_scaling: f64) -> (u32, u32) {
    let size = size * render_scaling;
    (1.max(size.width as i32) as u32, 1.max(size.height as i32) as u32)
}

/// The bounds of a holder in the pixels of its top-level: the position
/// truncated, the size at least one pixel each way.
pub(crate) fn holder_pixel_rect(bounds: Rect, render_scaling: f64) -> PixelRect {
    let bounds = bounds * render_scaling;
    PixelRect::new(bounds.x as i32, bounds.y as i32, 1.max(bounds.width as i32), 1.max(bounds.height as i32))
}

/// The native control host of a window.
pub struct X11NativeControlHost {
    platform: Rc<FerroX11Platform>,
    window: Weak<X11Window>,
    this: Weak<X11NativeControlHost>,
}

impl X11NativeControlHost {
    pub fn new(platform: &Rc<FerroX11Platform>, window: Weak<X11Window>) -> Rc<Self> {
        Rc::new_cyclic(|this| Self { platform: platform.clone(), window, this: this.clone() })
    }

    /// The window of the host on the server; 0 once the window is gone.
    fn window_handle(&self) -> XID {
        self.window.upgrade().map_or(0, |window| window.xid())
    }

    fn window_render_scaling(&self) -> f64 {
        self.window.upgrade().map_or(1.0, |window| window.scaling())
    }

    fn as_host(&self) -> Option<Rc<dyn INativeControlHostImpl>> {
        let this: Rc<dyn INativeControlHostImpl> = self.this.upgrade()?;
        Some(this)
    }
}

impl INativeControlHostImpl for X11NativeControlHost {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn create_default_child(&self, _parent: Rc<dyn IPlatformHandle>) -> Rc<dyn INativeControlHostDestroyableControlHandle> {
        Rc::new(DumbWindow::new(self.platform.info(), true, None))
    }

    fn create_new_attachment_with(
        &self,
        create: Rc<dyn Fn(Rc<dyn IPlatformHandle>) -> Rc<dyn IPlatformHandle>>,
    ) -> Rc<dyn INativeControlHostControlTopLevelAttachment> {
        let holder = Rc::new(DumbWindow::new(self.platform.info(), true, Some(self.window_handle())));
        // It has to be disposed when the creation or the attachment fails.
        let attachment: RefCell<Option<Rc<Attachment>>> = RefCell::new(None);
        let result = catch_unwind(AssertUnwindSafe(|| {
            let holder_handle: Rc<dyn IPlatformHandle> = holder.clone();
            let child = create(holder_handle);
            let created = Rc::new(Attachment::new(
                self.platform.display(),
                holder.clone(),
                self.platform.orphaned_window(),
                child,
            ));
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
        if !self.is_compatible_with(&*handle) {
            panic!("{} is not compatible with the current window", handle.handle_descriptor().unwrap_or(""));
        }
        let attachment = Rc::new(Attachment::new(
            self.platform.display(),
            Rc::new(DumbWindow::new(self.platform.info(), false, Some(self.window_handle()))),
            self.platform.orphaned_window(),
            handle,
        ));
        attachment.set_attached_to(self.as_host());
        attachment
    }

    fn is_compatible_with(&self, handle: &dyn IPlatformHandle) -> bool {
        handle.handle_descriptor() == Some(XID_DESCRIPTOR)
    }
}

/// A window that does nothing of its own.
struct DumbWindow {
    display: XDisplay,
    handle: Cell<XID>,
}

impl DumbWindow {
    fn new(x11: &X11Info, sync: bool, parent: Option<XID>) -> Self {
        let display = x11.display();
        let mut attr = xlib::new_set_window_attributes();
        attr.backing_store = 1;
        attr.bit_gravity = Gravity::NorthWestGravity as i32;
        attr.win_gravity = Gravity::NorthWestGravity as i32;

        let parent = parent.unwrap_or_else(|| xlib::x_default_root_window(display));

        let handle = xlib::x_create_window(
            display,
            parent,
            0,
            0,
            1,
            1,
            0,
            0,
            CreateWindowArgs::InputOutput.0,
            std::ptr::null_mut(),
            (SetWindowValuemask::BORDER_PIXEL
                | SetWindowValuemask::BIT_GRAVITY
                | SetWindowValuemask::BACK_PIXEL
                | SetWindowValuemask::WIN_GRAVITY
                | SetWindowValuemask::BACKING_STORE)
                .bits() as u32 as _,
            &mut attr,
        );
        if sync {
            xlib::x_sync(display, false);
        }
        Self { display, handle: Cell::new(handle) }
    }
}

impl IPlatformHandle for DumbWindow {
    fn handle(&self) -> isize {
        self.handle.get() as isize
    }

    fn handle_descriptor(&self) -> Option<&str> {
        Some(XID_DESCRIPTOR)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_native_control_host_destroyable_control_handle(&self) -> Option<&dyn INativeControlHostDestroyableControlHandle> {
        Some(self)
    }
}

impl INativeControlHostDestroyableControlHandle for DumbWindow {
    fn destroy(&self) {
        let handle = self.handle.replace(0);
        if handle != 0 {
            xlib::x_destroy_window(self.display, handle);
        }
    }
}

struct Attachment {
    display: XDisplay,
    orphaned_window: XID,
    holder: RefCell<Option<Rc<DumbWindow>>>,
    child: RefCell<Option<Rc<dyn IPlatformHandle>>>,
    attached_to: RefCell<Option<Rc<dyn INativeControlHostImpl>>>,
    mapped: Cell<bool>,
}

impl Attachment {
    fn new(display: XDisplay, holder: Rc<DumbWindow>, orphaned_window: XID, child: Rc<dyn IPlatformHandle>) -> Self {
        xlib::x_reparent_window(display, child.handle() as XID, holder.handle.get(), 0, 0);
        xlib::x_map_window(display, child.handle() as XID);
        Self {
            display,
            orphaned_window,
            holder: RefCell::new(Some(holder)),
            child: RefCell::new(Some(child)),
            attached_to: RefCell::new(None),
            mapped: Cell::new(false),
        }
    }

    /// The holder and the child, which exist until the attachment is
    /// disposed.
    fn check_disposed(&self) -> (Rc<DumbWindow>, Rc<dyn IPlatformHandle>) {
        match (self.holder.borrow().clone(), self.child.borrow().clone()) {
            (Some(holder), Some(child)) => (holder, child),
            _ => panic!("Cannot access a disposed object: X11 INativeControlHostControlTopLevelAttachment"),
        }
    }

    /// The host the attachment is attached to, as the host of this
    /// platform it is.
    fn with_attached_to<T>(&self, f: impl FnOnce(&X11NativeControlHost) -> T) -> Option<T> {
        let attached_to = self.attached_to.borrow().clone()?;
        attached_to.as_any().downcast_ref::<X11NativeControlHost>().map(f)
    }
}

impl IDisposable for Attachment {
    fn dispose(&self) {
        if let Some(child) = self.child.borrow_mut().take() {
            xlib::x_reparent_window(self.display, child.handle() as XID, self.orphaned_window, 0, 0);
        }

        if let Some(holder) = self.holder.borrow_mut().take() {
            holder.destroy();
        }
        *self.attached_to.borrow_mut() = None;
    }
}

impl INativeControlHostControlTopLevelAttachment for Attachment {
    fn attached_to(&self) -> Option<Rc<dyn INativeControlHostImpl>> {
        self.attached_to.borrow().clone()
    }

    fn set_attached_to(&self, value: Option<Rc<dyn INativeControlHostImpl>>) {
        let (holder, _) = self.check_disposed();
        // A host of another platform is no host of this attachment (the cast of the
        // reference).
        let value = value.filter(|value| value.as_any().is::<X11NativeControlHost>());
        *self.attached_to.borrow_mut() = value;
        match self.with_attached_to(X11NativeControlHost::window_handle) {
            None => {
                self.mapped.set(false);
                xlib::x_unmap_window(self.display, holder.handle.get());
                xlib::x_reparent_window(self.display, holder.handle.get(), self.orphaned_window, 0, 0);
            }
            Some(window) => {
                xlib::x_reparent_window(self.display, holder.handle.get(), window, 0, 0);
            }
        }
    }

    fn is_compatible_with(&self, host: &dyn INativeControlHostImpl) -> bool {
        host.as_any().is::<X11NativeControlHost>()
    }

    fn hide_with_size(&self, size: Size) {
        let Some(render_scaling) = self.with_attached_to(X11NativeControlHost::window_render_scaling) else {
            return;
        };
        let Some(child) = self.child.borrow().clone() else {
            return;
        };
        if self.mapped.get() {
            self.mapped.set(false);
            if let Some(holder) = self.holder.borrow().as_ref() {
                xlib::x_unmap_window(self.display, holder.handle.get());
            }
        }

        let (width, height) = hidden_child_size(size, render_scaling);
        xlib::x_resize_window(self.display, child.handle() as XID, width, height);
    }

    fn show_in_bounds(&self, bounds: Rect) {
        let (holder, child) = self.check_disposed();
        let Some(render_scaling) = self.with_attached_to(X11NativeControlHost::window_render_scaling) else {
            panic!("The control isn't currently attached to a toplevel");
        };

        let pixel_rect = holder_pixel_rect(bounds, render_scaling);
        xlib::x_move_resize_window(
            self.display,
            child.handle() as XID,
            0,
            0,
            pixel_rect.width as u32,
            pixel_rect.height as u32,
        );
        xlib::x_move_resize_window(
            self.display,
            holder.handle.get(),
            pixel_rect.x,
            pixel_rect.y,
            pixel_rect.width as u32,
            pixel_rect.height as u32,
        );
        if !self.mapped.get() {
            xlib::x_map_window(self.display, holder.handle.get());
            xlib::x_raise_window(self.display, holder.handle.get());
            self.mapped.set(true);
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file. What needs a server is
    // checked by the smoke mode of the example.
    use super::*;

    #[test]
    fn a_hidden_child_keeps_the_size_of_its_host_in_pixels() {
        assert_eq!(hidden_child_size(Size::new(100.4, 50.9), 1.0), (100, 50));
        assert_eq!(hidden_child_size(Size::new(100.0, 50.0), 1.5), (150, 75));
        assert_eq!(hidden_child_size(Size::new(0.0, 0.2), 2.0), (1, 1));
    }

    #[test]
    fn the_bounds_of_a_holder_are_scaled_and_never_empty() {
        assert_eq!(holder_pixel_rect(Rect::new(10.6, 20.4, 30.0, 40.0), 2.0), PixelRect::new(21, 40, 60, 80));
        assert_eq!(holder_pixel_rect(Rect::new(1.0, 2.0, 0.0, 0.3), 1.0), PixelRect::new(1, 2, 1, 1));
    }
}
