//! The input method of the X server for a window (the port of
//! `X11Window.Xim.cs`): focus of the input context, its reset, and the
//! spot the server places its pre-edit at.
//!
//! The text itself comes through the input context when a key is looked
//! up (`Xutf8LookupString` in `x11_window_ime.rs`), after the dispatcher
//! gave the event to the input method (`XFilterEvent`); this class filters
//! nothing by itself, commits nothing and forwards nothing.

use crate::x11_window::X11Window;
use crate::xlib;
use ferroui_base::input::raw::IRawInputEventArgs;
use ferroui_base::input::text_input::{ITextInputMethodImpl, TextInputMethodClient, TextInputOptions};
use ferroui_base::input::LocalBoxFuture;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{PixelPoint, Rect};
use ferroui_freedesktop::{Event, IX11InputMethodControl, X11InputMethodForwardedKey};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

pub(crate) struct XimInputMethod {
    weak_self: Weak<XimInputMethod>,
    parent: Weak<X11Window>,
    window_active: Cell<bool>,
    ime_active: Cell<bool>,
    queued_cursor_rect: Cell<Option<Rect>>,
    client: RefCell<Option<Rc<dyn TextInputMethodClient>>>,
    commit: Event<String>,
    forward_key: Event<X11InputMethodForwardedKey>,
}

/// The spot of the pre-edit for a cursor rectangle in logical units
/// (`SetCursorRect`): the bottom left corner of the rectangle in pixels of
/// the window, within the range of the protocol.
pub(crate) fn spot_location(rect: Rect, scaling: f64) -> (i16, i16) {
    let rect = rect * scaling;

    (
        rect.x.max(f64::from(i16::MIN)).min(f64::from(i16::MAX)) as i16,
        (rect.y + rect.height).max(f64::from(i16::MIN)).min(f64::from(i16::MAX)) as i16,
    )
}

impl XimInputMethod {
    pub(crate) fn new(parent: Weak<X11Window>) -> Rc<Self> {
        Rc::new_cyclic(|weak_self| Self {
            weak_self: weak_self.clone(),
            parent,
            window_active: Cell::new(false),
            ime_active: Cell::new(false),
            queued_cursor_rect: Cell::new(None),
            client: RefCell::new(None),
            commit: Event::new(),
            forward_key: Event::new(),
        })
    }

    pub(crate) fn is_active(&self) -> bool {
        self.client.borrow().is_some()
    }

    fn xic(&self) -> Option<xlib::XIC> {
        let parent = self.parent.upgrade()?;
        let xic = parent.xic.get();
        (!xic.is_null()).then_some(xic)
    }

    fn update_active(&self) {
        let active = self.window_active.get() && self.is_active();
        let Some(xic) = self.xic() else {
            return;
        };
        if active != self.ime_active.get() {
            self.ime_active.set(active);
            if active {
                ITextInputMethodImpl::reset(self);
                xlib::x_set_ic_focus(xic);
            } else {
                xlib::x_unset_ic_focus(xic);
            }
        }
    }
}

impl ITextInputMethodImpl for XimInputMethod {
    fn set_client(&self, client: Option<Rc<dyn TextInputMethodClient>>) {
        *self.client.borrow_mut() = client;
        self.update_active();
    }

    fn set_cursor_rect(&self, rect: Rect) {
        let need_enqueue = self.queued_cursor_rect.get().is_none();
        self.queued_cursor_rect.set(Some(rect));
        if need_enqueue {
            let this = self.weak_self.clone();
            Dispatcher::ui_thread().post_local(
                move || {
                    let Some(this) = this.upgrade() else {
                        return;
                    };
                    // The rectangle that was set last. The reference
                    // takes the queued one out and then places the spot
                    // for the rectangle of the call that queued the job,
                    // which is the first of a burst (docs/porting/DEVIATIONS.md).
                    let Some(rc) = this.queued_cursor_rect.take() else {
                        return;
                    };

                    let Some(xic) = this.xic() else {
                        return;
                    };
                    let scaling = this.parent.upgrade().map_or(1.0, |parent| parent.scaling());

                    let (x, y) = spot_location(rc, scaling);
                    xlib::x_set_ic_spot_location(xic, x, y);
                },
                DispatcherPriority::BACKGROUND,
            );
        }
    }

    fn set_options(&self, _options: &TextInputOptions) {
        // No-op
    }

    fn reset(&self) {
        let Some(xic) = self.xic() else {
            return;
        };

        xlib::xmb_reset_ic(xic);
    }
}

impl IX11InputMethodControl for XimInputMethod {
    fn set_window_active(&self, active: bool) {
        self.window_active.set(active);
        self.update_active();
    }

    fn is_enabled(&self) -> bool {
        false
    }

    fn handle_event_async(
        &self,
        _args: Rc<dyn IRawInputEventArgs>,
        _key_val: i32,
        _key_code: i32,
    ) -> LocalBoxFuture<bool> {
        Box::pin(async { false })
    }

    fn commit(&self) -> &Event<String> {
        &self.commit
    }

    fn forward_key(&self) -> &Event<X11InputMethodForwardedKey> {
        &self.forward_key
    }

    fn update_window_info(&self, _position: PixelPoint, _scaling: f64) {
        // No-op
    }

    fn dispose(&self) {
        // No-op
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of this class.
    use super::*;

    #[test]
    fn the_spot_is_the_bottom_left_of_the_cursor_in_pixels() {
        assert_eq!(spot_location(Rect::new(10.0, 20.0, 1.0, 16.0), 1.0), (10, 36));
        assert_eq!(spot_location(Rect::new(10.0, 20.0, 1.0, 16.0), 2.0), (20, 72));
        assert_eq!(spot_location(Rect::new(10.5, 20.25, 1.0, 16.0), 1.5), (15, 54));
    }

    #[test]
    fn the_spot_stays_in_the_range_of_the_protocol() {
        assert_eq!(spot_location(Rect::new(1e6, 1e6, 1.0, 1.0), 1.0), (i16::MAX, i16::MAX));
        assert_eq!(spot_location(Rect::new(-1e6, -1e6, 1.0, 1.0), 1.0), (i16::MIN, i16::MIN));
    }

    #[test]
    fn without_a_window_the_input_method_does_nothing() {
        // The window is gone: there is no input context to focus, reset
        // or place.
        let xim = XimInputMethod::new(Weak::new());
        assert!(!xim.is_enabled());
        assert!(!xim.is_active());
        xim.set_window_active(true);
        ITextInputMethodImpl::reset(&*xim);
        xim.update_window_info(PixelPoint::new(1, 2), 2.0);
        xim.set_options(&TextInputOptions::default_options());
        xim.dispose();
        assert!(!xim.ime_active.get());
    }
}
