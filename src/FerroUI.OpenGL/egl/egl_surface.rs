use super::{EglDisplay, EglInterface};
use ferroui_base::reactive::IDisposable;
use std::cell::Cell;
use std::rc::Rc;

/// A surface of a display. The surface is destroyed when it is disposed, or when it is
/// dropped without having been disposed (the original is a safe handle).
pub struct EglSurface {
    display: Rc<EglDisplay>,
    egl: Rc<EglInterface>,
    handle: Cell<isize>,
    released: Cell<bool>,
}

impl EglSurface {
    pub fn new(display: &Rc<EglDisplay>, surface: isize) -> Rc<EglSurface> {
        Rc::new(Self {
            display: display.clone(),
            egl: display.egl_interface().clone(),
            handle: Cell::new(surface),
            released: Cell::new(false),
        })
    }

    /// The handle of the surface. It stays readable after the surface is released, as the
    /// handle of a safe handle does.
    pub fn dangerous_get_handle(&self) -> isize {
        self.handle.get()
    }

    fn release_handle(&self) {
        let lock = self.display.lock();
        self.egl.destroy_surface(self.display.handle(), self.handle.get());
        lock.dispose();
    }

    pub fn is_invalid(&self) -> bool {
        self.handle.get() == 0
    }

    pub fn swap_buffers(&self) {
        self.egl.swap_buffers(self.display.handle(), self.handle.get())
    }
}

impl IDisposable for EglSurface {
    fn dispose(&self) {
        if self.released.replace(true) {
            return;
        }
        if !self.is_invalid() {
            self.release_handle();
        }
    }
}

impl Drop for EglSurface {
    fn drop(&mut self) {
        self.dispose();
    }
}
