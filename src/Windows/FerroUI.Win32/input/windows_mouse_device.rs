//! The mouse of the Windows backend: a mouse device whose pointer captures
//! the mouse in the system.

use crate::interop::unmanaged_methods::{release_capture, set_capture};
use crate::window_impl::WindowImpl;
use ferroui_base::input::{IPointer, InputElement, MouseDevice, Pointer, PointerType};
use ferroui_base::Ref;
use ferroui_controls::TopLevel;
use std::rc::Rc;

/// The mouse device of the Windows backend.
///
/// In the reference this is a class that derives from the mouse device and
/// becomes the primary mouse device of the toolkit. Here it holds a mouse
/// device of the base library that is created with the pointer of this
/// backend; the windows of the backend deliver their mouse input through
/// it.
pub struct WindowsMouseDevice {
    device: Rc<MouseDevice>,
}

impl WindowsMouseDevice {
    /// The mouse device of the calling thread.
    pub fn instance() -> Rc<WindowsMouseDevice> {
        thread_local! {
            static INSTANCE: Rc<WindowsMouseDevice> = Rc::new(WindowsMouseDevice::new());
        }
        INSTANCE.with(Rc::clone)
    }

    fn new() -> Self {
        Self { device: MouseDevice::with_pointer(WindowsMousePointer::create_pointer()) }
    }

    /// The mouse device of the toolkit.
    pub fn device(&self) -> &Rc<MouseDevice> {
        &self.device
    }

    /// Captures the mouse for an element.
    ///
    /// Normally user should use the capture of a pointer instead of the
    /// capture of the mouse device, but on Windows we need to handle the
    /// mouse capture manually without having access to the pointer.
    #[allow(dead_code)] // The drop target of stage 2 releases the capture through it.
    pub(crate) fn capture(&self, control: Option<&Ref<InputElement>>) {
        self.device.pointer().capture(control);
    }
}

/// The pointer of the mouse: capturing an element captures the mouse for
/// the window the element is in.
pub struct WindowsMousePointer;

impl WindowsMousePointer {
    /// Creates the pointer of the mouse.
    pub fn create_pointer() -> Rc<Pointer> {
        Pointer::with_platform_capture(Pointer::get_next_free_id(), PointerType::Mouse, true, Self::platform_capture)
    }

    fn platform_capture(element: Option<&Ref<InputElement>>) {
        let hwnd = element
            .and_then(|element| TopLevel::get_top_level(Some(element)))
            .and_then(|top_level| top_level.platform_impl())
            .and_then(|platform_impl| WindowImpl::from_top_level(&*platform_impl).map(|window| window.hwnd()));

        match hwnd {
            Some(hwnd) if hwnd != 0 => {
                set_capture(hwnd);
            }
            _ => {
                release_capture();
            }
        }
    }
}
