//! What a window of the X11 platform asks of an input method (the port of
//! `IX11InputMethod.cs`).

use crate::event::Event;
use ferroui_base::input::raw::{IRawInputEventArgs, RawKeyEventType};
use ferroui_base::input::text_input::ITextInputMethodImpl;
use ferroui_base::input::{KeyModifiers, LocalBoxFuture};
use ferroui_base::PixelPoint;
use std::rc::Rc;

/// Creates the input method of a window.
pub trait IX11InputMethodFactory {
    /// The input method of the window `xid`, as the two interfaces the
    /// window uses it through.
    fn create_client(&self, xid: usize) -> (Rc<dyn ITextInputMethodImpl>, Rc<dyn IX11InputMethodControl>);
}

/// A key an input method hands back to the application.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X11InputMethodForwardedKey {
    pub key_val: i32,
    pub modifiers: KeyModifiers,
    pub type_: RawKeyEventType,
    pub with_text: bool,
}

/// The side of an input method the window drives.
pub trait IX11InputMethodControl {
    fn set_window_active(&self, active: bool);

    fn is_enabled(&self) -> bool;

    /// Offers a key event (raw key input, with or without text) to the
    /// input method. The future resolves to whether the input method
    /// consumed it.
    fn handle_event_async(
        &self,
        args: Rc<dyn IRawInputEventArgs>,
        key_val: i32,
        key_code: i32,
    ) -> LocalBoxFuture<bool>;

    /// Text the input method commits (`Commit`).
    fn commit(&self) -> &Event<String>;

    /// Keys the input method forwards (`ForwardKey`).
    fn forward_key(&self) -> &Event<X11InputMethodForwardedKey>;

    fn update_window_info(&self, position: PixelPoint, scaling: f64);

    /// Releases the input method (`IDisposable.Dispose`).
    fn dispose(&self);
}
