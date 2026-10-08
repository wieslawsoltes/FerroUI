use super::EglDisplay;
use ferroui_base::reactive::IDisposable;
use std::cell::Cell;
use std::rc::Rc;

/// Wraps a native `EGLImageKHR` handle and destroys it via `eglDestroyImageKHR` on disposal.
pub struct EglImage {
    display: Rc<EglDisplay>,
    handle: Cell<isize>,
}

impl EglImage {
    /// Initializes a new [`EglImage`] wrapping an existing `EGLImageKHR` handle of the EGL
    /// display that owns the image.
    ///
    /// # Panics
    /// Panics when `handle` is zero.
    pub fn new(display: &Rc<EglDisplay>, handle: isize) -> EglImage {
        if handle == 0 {
            panic!("Invalid EGLImage handle (Parameter 'handle')");
        }
        Self { display: display.clone(), handle: Cell::new(handle) }
    }

    /// Gets the native `EGLImageKHR` handle, or zero once disposed.
    pub fn handle(&self) -> isize {
        self.handle.get()
    }
}

impl IDisposable for EglImage {
    /// Destroys the underlying `EGLImageKHR`. Safe to call more than once.
    fn dispose(&self) {
        if self.handle.get() != 0 {
            let lock = self.display.lock();
            self.display.egl_interface().destroy_image_khr(self.display.handle(), self.handle.get());
            lock.dispose();
            self.handle.set(0);
        }
    }
}
