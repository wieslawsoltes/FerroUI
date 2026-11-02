//! The display of EGL of the dmabuf swapchain (the port of
//! `WaylandEglDisplay.cs`): a display over a GBM device, with the device,
//! the descriptor of its render node, the format and modifier pairs both
//! sides support, and the `zwp_linux_dmabuf_v1` the buffers are made with.
//!
//! The reference derives from `EglDisplay`. The display of the OpenGL crate
//! is a value the contexts hold (`Rc<EglDisplay>`), so here the subclass
//! holds the display and adds its own members; a render target finds this
//! object through the platform graphics of the worker and checks that its
//! context belongs to the display it holds.

use super::wayland_dmabuf_feedback::DmabufFormatModifierPair;
use ferroui_opengl::egl::{EglDisplay, EglDisplayCreationOptions};
use ferroui_opengl::OpenGlError;
use std::cell::RefCell;
use std::ffi::c_void;
use std::rc::Rc;
use wayland_protocols::wp::linux_dmabuf::zv1::client::zwp_linux_dmabuf_v1::ZwpLinuxDmabufV1;

/// An object of the worker thread, like the display it holds.
pub struct WaylandEglDisplay {
    display: Rc<EglDisplay>,
    gbm_device: *mut c_void,
    drm_fd: i32,
    supported_formats: RefCell<Vec<DmabufFormatModifierPair>>,
    linux_dmabuf: ZwpLinuxDmabufV1,
}

impl WaylandEglDisplay {
    /// Creates the display of the options (`base(options)`); the dispose callback of the
    /// options releases the GBM device and the descriptor.
    pub fn new(
        options: EglDisplayCreationOptions,
        gbm_device: *mut c_void,
        drm_fd: i32,
        linux_dmabuf: ZwpLinuxDmabufV1,
    ) -> Result<Rc<Self>, OpenGlError> {
        let display = EglDisplay::new_with_creation_options(options)?;
        Ok(Rc::new(Self { display, gbm_device, drm_fd, supported_formats: RefCell::new(Vec::new()), linux_dmabuf }))
    }

    /// The display of EGL (the base class of the reference).
    pub fn display(&self) -> &Rc<EglDisplay> {
        &self.display
    }

    pub fn gbm_device(&self) -> *mut c_void {
        self.gbm_device
    }

    pub fn drm_fd(&self) -> i32 {
        self.drm_fd
    }

    /// The format and modifier pairs the compositor and EGL both support.
    pub fn supported_formats(&self) -> std::cell::Ref<'_, Vec<DmabufFormatModifierPair>> {
        self.supported_formats.borrow()
    }

    pub(crate) fn supported_formats_mut(&self) -> std::cell::RefMut<'_, Vec<DmabufFormatModifierPair>> {
        self.supported_formats.borrow_mut()
    }

    pub fn linux_dmabuf(&self) -> &ZwpLinuxDmabufV1 {
        &self.linux_dmabuf
    }
}
