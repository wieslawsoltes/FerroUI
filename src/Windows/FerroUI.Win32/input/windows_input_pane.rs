//! The input pane of the system, the touch keyboard, as the input pane of a
//! window (the port of `Input/WindowsInputPane.cs`): the framework input
//! pane of the shell tells a handler when the pane is about to show over
//! the window and when it hides.

use ferroui_base::{PixelSize, Point, Rect};
use ferroui_controls::platform::InputPaneState;

/// The rectangle of the pane, which the shell gives in pixels of the
/// screen, in the coordinates of the client area of the window: its
/// position as the window maps a point of the screen, and its size divided
/// by the scaling of the window.
pub(crate) fn screen_rect_to_client(client_position: Point, size: PixelSize, desktop_scaling: f64) -> Rect {
    let size = size.to_size(desktop_scaling);
    Rect::new(client_position.x, client_position.y, size.width, size.height)
}

/// What a notification of the shell makes of the pane: the rectangle it
/// covers and its state, and whether that is a change to report.
pub(crate) fn state_after_notification(
    old: (Rect, InputPaneState),
    showing: bool,
    occluded_rect: Option<Rect>,
) -> ((Rect, InputPaneState), bool) {
    let new = (occluded_rect.unwrap_or_default(), if showing { InputPaneState::Open } else { InputPaneState::Closed });
    (new, old != new)
}

#[cfg(windows)]
pub(crate) use imp::WindowsInputPane;

#[cfg(windows)]
mod imp {
    use super::{screen_rect_to_client, state_after_notification};
    use crate::interop::unmanaged_methods::{co_create_instance, RECT};
    use crate::win32_com::{IFrameworkInputPane, IFrameworkInputPaneHandler, IFrameworkInputPaneHandlerImpl};
    use crate::win_rt::WinRTApiInformation;
    use crate::window_impl::WindowImpl;
    use ferroui_base::reactive::IDisposable;
    use ferroui_base::{PixelPoint, PixelSize, Rect};
    use ferroui_controls::platform::{IInputPane, InputPaneBase, InputPaneState, InputPaneStateEventArgs};
    use ferroui_microcom::{ComPtr, Guid, HResult};
    use std::cell::{Cell, RefCell};
    use std::rc::{Rc, Weak};

    // GUID: D5120AA3-46BA-44C5-822D-CA8092C1FC72
    const CLSID_FRAMEWORK_INPUT_PANE: Guid = Guid::from_u128(0xD5120AA3_46BA_44C5_822D_CA8092C1FC72);
    // GUID: 5752238B-24F0-495A-82F1-2FD593056796
    const SID_I_FRAMEWORK_INPUT_PANE: Guid = Guid::from_u128(0x5752238B_24F0_495A_82F1_2FD593056796);
    const CLSCTX_INPROC_SERVER: u32 = 1;

    thread_local! {
        static INPUT_PANE_SUPPORTED: bool = WinRTApiInformation::is_type_present("Windows.UI.ViewManagement.InputPane");
    }

    pub(crate) struct WindowsInputPane {
        base: InputPaneBase,
        window_impl: RefCell<Weak<WindowImpl>>,
        input_pane: RefCell<Option<ComPtr<IFrameworkInputPane>>>,
        cookie: Cell<u32>,
        disposed: Cell<bool>,
    }

    impl WindowsInputPane {
        fn new(window_impl: &Rc<WindowImpl>) -> Option<Rc<WindowsInputPane>> {
            let instance =
                co_create_instance(&CLSID_FRAMEWORK_INPUT_PANE, CLSCTX_INPROC_SERVER, &SID_I_FRAMEWORK_INPUT_PANE).ok()?;
            // SAFETY: the pointer is the interface that was asked for, with
            // the one reference the creation gave, which the pointer of
            // the port takes over.
            let input_pane: ComPtr<IFrameworkInputPane> = unsafe { ComPtr::from_raw(instance.cast()) }?;

            let pane = Rc::new(WindowsInputPane {
                base: InputPaneBase::new(),
                window_impl: RefCell::new(Rc::downgrade(window_impl)),
                input_pane: RefCell::new(None),
                cookie: Cell::new(0),
                disposed: Cell::new(false),
            });

            let handler = IFrameworkInputPaneHandler::from_impl(Handler { pane: Rc::downgrade(&pane) });
            let mut cookie = 0u32;
            // SAFETY: the handler is an object of this thread that the
            // shell keeps a reference to until it is unadvised; the cookie
            // is written to a number of this frame.
            unsafe { input_pane.advise_with_hwnd(window_impl.hwnd(), Some(&handler), &mut cookie) };
            pane.cookie.set(cookie);
            *pane.input_pane.borrow_mut() = Some(input_pane);
            Some(pane)
        }

        /// The input pane of a window, on a system that has the input pane
        /// of the Windows Runtime; `None` on an older system, and when the
        /// shell does not create its object (the reference throws then).
        pub fn try_create(window_impl: &Rc<WindowImpl>) -> Option<Rc<WindowsInputPane>> {
            if INPUT_PANE_SUPPORTED.with(|supported| *supported) {
                return Self::new(window_impl);
            }

            None
        }

        fn on_state_changed(&self, showing: bool, prc_input_pane_screen_location: Option<RECT>) {
            // Unadvise can deliver last notification while it unwinds, and the shell can call back after teardown.
            // Either would dereference a disposed window, crashing the process.
            if self.disposed.get() {
                return;
            }
            let Some(window_impl) = self.window_impl.borrow().upgrade() else {
                return;
            };

            let old_state = (self.base.occluded_rect(), self.base.state());
            let occluded_rect = prc_input_pane_screen_location.map(|rect| Self::screen_rect_to_client(&window_impl, rect));
            let ((occluded_rect, state), changed) = state_after_notification(old_state, showing, occluded_rect);
            self.base.set_occluded_rect(occluded_rect);
            self.base.set_state(state);

            if changed {
                self.base.on_state_changed(InputPaneStateEventArgs::new(state, None, occluded_rect));
            }
        }

        fn screen_rect_to_client(window_impl: &WindowImpl, screen_rect: RECT) -> Rect {
            let position = PixelPoint::new(screen_rect.left, screen_rect.top);
            let size = PixelSize::new(screen_rect.width(), screen_rect.height());
            screen_rect_to_client(window_impl.point_to_client_impl(position), size, window_impl.scaling())
        }

        pub fn dispose(&self) {
            if self.disposed.replace(true) {
                return;
            }
            let input_pane = self.input_pane.borrow_mut().take();
            if let Some(input_pane) = input_pane {
                if self.cookie.get() != 0 {
                    input_pane.unadvise(self.cookie.get());
                }
            }

            // Released only once Unadvise has returned, so the field stays valid for the whole of the
            // teardown it is read during.
            *self.window_impl.borrow_mut() = Weak::new();
        }
    }

    impl IInputPane for WindowsInputPane {
        fn state(&self) -> InputPaneState {
            self.base.state()
        }

        fn occluded_rect(&self) -> Rect {
            self.base.occluded_rect()
        }

        fn state_changed(&self, handler: Rc<dyn Fn(&InputPaneStateEventArgs)>) -> Rc<dyn IDisposable> {
            self.base.state_changed(handler)
        }
    }

    impl Drop for WindowsInputPane {
        fn drop(&mut self) {
            self.dispose();
        }
    }

    struct Handler {
        pane: Weak<WindowsInputPane>,
    }

    impl IFrameworkInputPaneHandlerImpl for Handler {
        fn showing(&self, prc_input_pane_screen_location: *mut RECT, _f_ensure_focused_element_in_view: i32) -> Result<(), HResult> {
            // SAFETY: the shell passes the rectangle of the pane, valid
            // during the call; a null pointer is no rectangle.
            let rect = (!prc_input_pane_screen_location.is_null()).then(|| unsafe { *prc_input_pane_screen_location });
            if let Some(pane) = self.pane.upgrade() {
                pane.on_state_changed(true, rect);
            }
            Ok(())
        }

        fn hiding(&self, _f_ensure_focused_element_in_view: i32) -> Result<(), HResult> {
            if let Some(pane) = self.pane.upgrade() {
                pane.on_state_changed(false, None);
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the reference has no tests of the input pane.
    use super::*;

    #[test]
    fn the_rectangle_of_the_pane_is_in_the_units_of_the_window() {
        // A pane 2000 by 600 pixels of the screen, over a window whose
        // client area starts 100 pixels above it, at a scaling of two.
        let rect = screen_rect_to_client(Point::new(-20.0, 350.0), PixelSize::new(2000, 600), 2.0);

        assert_eq!(Rect::new(-20.0, 350.0, 1000.0, 300.0), rect);
    }

    #[test]
    fn a_notification_is_a_change_when_the_rectangle_or_the_state_differs() {
        let closed = (Rect::default(), InputPaneState::Closed);
        let covered = Rect::new(0.0, 300.0, 800.0, 200.0);

        let (open, changed) = state_after_notification(closed, true, Some(covered));
        assert_eq!((covered, InputPaneState::Open), open);
        assert!(changed);

        // The same notification again changes nothing.
        let (again, changed) = state_after_notification(open, true, Some(covered));
        assert_eq!(open, again);
        assert!(!changed);

        // The pane moves while it is open.
        let moved = Rect::new(0.0, 250.0, 800.0, 250.0);
        let (state, changed) = state_after_notification(open, true, Some(moved));
        assert_eq!((moved, InputPaneState::Open), state);
        assert!(changed);

        // Hiding covers nothing.
        let (state, changed) = state_after_notification(state, false, None);
        assert_eq!(closed, state);
        assert!(changed);
        assert!(!state_after_notification(closed, false, None).1);
    }
}
