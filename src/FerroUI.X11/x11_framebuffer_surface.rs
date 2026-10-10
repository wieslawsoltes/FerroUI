//! The software render surface of a window (the port of
//! `X11FramebufferSurface.cs`): frames are drawn into memory and sent to
//! the server with `XPutImage`.

use crate::xlib::{self, XDisplay, XID};
use ferroui_base::platform::surfaces::{
    FramebufferLockProperties, FuncFramebufferRenderTarget, IFramebufferPlatformSurface, IFramebufferRenderTarget,
    IPlatformRenderSurface,
};
use ferroui_base::platform::{AlphaFormat, ILockedFramebuffer, PixelFormat, RenderTargetSceneInfo, RetainedFramebuffer};
use ferroui_base::{PixelSize, Vector};
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

/// A window of the server as a target of software rendering.
///
/// The surface is shared with the thread that renders, so it holds only
/// the connection and the identifier of the window. The framebuffer the
/// reference keeps in the surface (`_fb`) is kept by the render target
/// here, which belongs to the thread that created it.
pub struct X11FramebufferSurface {
    display: XDisplay,
    xid: XID,
    depth: i32,
    retain: bool,
}

/// What a render target of the surface keeps between frames.
struct TargetState {
    display: XDisplay,
    xid: XID,
    depth: i32,
    retain: bool,
    fb: RefCell<Option<Rc<RetainedFramebuffer>>>,
}

impl X11FramebufferSurface {
    pub fn new(display: XDisplay, xid: XID, depth: i32, retain: bool) -> Self {
        Self { display, xid, depth, retain }
    }
}

impl TargetState {
    fn blit(&self, fb: &Rc<RetainedFramebuffer>) {
        let size = fb.size();
        xlib::x_lock_display(self.display);
        // SAFETY: the framebuffer holds `row_bytes * height` bytes at its
        // address until it is disposed, which happens after this call at
        // the earliest (below, or when the next frame has another size).
        unsafe {
            xlib::x_put_image_32(self.display, self.xid, self.depth, fb.address(), size.width, size.height, fb.row_bytes());
        }
        xlib::x_sync(self.display, true);
        xlib::x_unlock_display(self.display);
        if !self.retain {
            let fb = self.fb.borrow_mut().take();
            if let Some(fb) = fb {
                fb.dispose();
            }
        }
    }

    fn lock(self: &Rc<Self>) -> (Rc<dyn ILockedFramebuffer>, FramebufferLockProperties) {
        xlib::x_lock_display(self.display);
        let geometry = xlib::x_get_geometry(self.display, self.xid).unwrap_or_default();
        xlib::x_unlock_display(self.display);
        let (width, height) = (geometry.width, geometry.height);

        let framebuffer_valid = self
            .fb
            .borrow()
            .as_ref()
            .is_some_and(|fb| fb.size().width == width && fb.size().height == height);
        if !framebuffer_valid {
            let previous = self.fb.borrow_mut().take();
            if let Some(previous) = previous {
                previous.dispose();
            }
            *self.fb.borrow_mut() =
                Some(RetainedFramebuffer::new(PixelSize::new(width, height), PixelFormat::BGRA8888, AlphaFormat::Premul));
        }

        let properties = FramebufferLockProperties { previous_frame_is_retained: framebuffer_valid };
        let fb = self.fb.borrow().clone().expect("the framebuffer was just made");
        let this = self.clone();
        (fb.lock(Vector::new(96.0, 96.0), move |fb| this.blit(fb)), properties)
    }
}

impl IPlatformRenderSurface for X11FramebufferSurface {
    fn as_framebuffer_surface(&self) -> Option<&dyn IFramebufferPlatformSurface> {
        Some(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IFramebufferPlatformSurface for X11FramebufferSurface {
    fn create_framebuffer_render_target(&self) -> Rc<dyn IFramebufferRenderTarget> {
        let state = Rc::new(TargetState {
            display: self.display,
            xid: self.xid,
            depth: self.depth,
            retain: self.retain,
            fb: RefCell::new(None),
        });
        Rc::new(FuncFramebufferRenderTarget::with_scene_info(
            move |_: &RenderTargetSceneInfo| state.lock(),
            self.retain,
        ))
    }
}
