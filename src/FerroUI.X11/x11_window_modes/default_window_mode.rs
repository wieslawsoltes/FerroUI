//! A window as a top-level of the window manager (the port of
//! `X11WindowModes/DefaultWindowMode.cs`).

use super::X11WindowMode;
use crate::x11_structs::{EventMask, XEventName};
use crate::x11_window::X11Window;
use crate::xlib::{self, PropertyMode};
use ferroui_base::{PixelPoint, Point};

/// The mode of an ordinary window and of a popup.
#[derive(Default)]
pub struct DefaultTopLevelWindowMode;

impl DefaultTopLevelWindowMode {
    /// `Activate`, with what a derived mode does after the window was
    /// raised by hand (`OnManualXRaiseWindow`).
    pub(crate) fn activate_with(window: &X11Window, on_manual_x_raise_window: impl FnOnce()) {
        let x11 = window.x11();
        let supports_active_window = window
            .platform()
            .globals()
            .net_supported()
            .is_some_and(|supported| supported.contains(&x11.atoms()._NET_ACTIVE_WINDOW));
        if supports_active_window {
            window.send_net_wm_message(
                x11.atoms()._NET_ACTIVE_WINDOW,
                1,
                Some(x11.last_activity_timestamp() as i64),
                Some(0),
                None,
                None,
            );
        } else {
            xlib::x_raise_window(x11.display(), window.xid());
            on_manual_x_raise_window();
        }
    }

    pub(crate) fn show_default(window: &X11Window, activate: bool) {
        window.set_shown(true);

        window.set_was_mapped_at_least_once(true);

        let x11 = window.x11();
        if !activate {
            xlib::x_change_property_longs(
                x11.display(),
                window.xid(),
                x11.atoms()._NET_WM_USER_TIME,
                x11.atoms().CARDINAL,
                PropertyMode::Replace,
                &[0],
            );
        }

        xlib::x_map_window(x11.display(), window.xid());
        xlib::x_flush(x11.display());
    }

    pub(crate) fn hide_default(window: &X11Window) {
        let x11 = window.x11();
        xlib::x_unmap_window(x11.display(), window.xid());

        if !window.override_redirect() {
            Self::send_synthetic_unmap_notify(window);
        }

        window.set_shown(false);
    }

    // ICCCM 4.1.4: withdrawing also requires a synthetic UnmapNotify, since XUnmapWindow
    // generates no event when the window manager has already unmapped the window.
    fn send_synthetic_unmap_notify(window: &X11Window) {
        let x11 = window.x11();
        let mut ev = xlib::new_event();
        {
            let unmap = xlib::unmap_event_mut(&mut ev);
            unmap.type_ = XEventName::UnmapNotify as i32;
            unmap.send_event = 1;
            unmap.display = x11.display().as_ptr().cast();
            unmap.event = x11.root_window();
            unmap.window = window.xid();
            unmap.from_configure = 0;
        }
        xlib::x_send_event(
            x11.display(),
            x11.root_window(),
            false,
            (EventMask::SUBSTRUCTURE_REDIRECT_MASK | EventMask::SUBSTRUCTURE_NOTIFY_MASK).bits() as _,
            &mut ev,
        );
    }

    pub(crate) fn point_to_client_default(window: &X11Window, point: PixelPoint) -> Point {
        let position = window.position_or_default();
        Point::new(
            (point.x - position.x) as f64 / window.scaling(),
            (point.y - position.y) as f64 / window.scaling(),
        )
    }

    pub(crate) fn point_to_screen_default(window: &X11Window, point: Point) -> PixelPoint {
        let position = window.position_or_default();
        PixelPoint::new(
            (point.x * window.scaling() + position.x as f64) as i32,
            (point.y * window.scaling() + position.y as f64) as i32,
        )
    }
}

impl X11WindowMode for DefaultTopLevelWindowMode {
    fn activate(&self, window: &X11Window) {
        Self::activate_with(window, || {});
    }

    fn show(&self, window: &X11Window, activate: bool, _is_dialog: bool) {
        Self::show_default(window, activate);
    }

    fn hide(&self, window: &X11Window) {
        Self::hide_default(window);
    }

    fn point_to_client(&self, window: &X11Window, point: PixelPoint) -> Point {
        Self::point_to_client_default(window, point)
    }

    fn point_to_screen(&self, window: &X11Window, point: Point) -> PixelPoint {
        Self::point_to_screen_default(window, point)
    }
}
