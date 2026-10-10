//! The drop source of a drag and drop operation: says when the operation
//! ends, and leaves the cursors to the system.

use crate::interop::unmanaged_methods::{ModifierKeys, HRESULT};
use crate::win32_com::{DropEffect, IDropSourceImpl};

const DRAGDROP_S_USEDEFAULTCURSORS: i32 = 0x0004_0102;
const DRAGDROP_S_DROP: i32 = 0x0004_0100;
const DRAGDROP_S_CANCEL: i32 = 0x0004_0101;

const MOUSE_BUTTONS: [i32; 3] =
    [ModifierKeys::MK_LBUTTON.bits(), ModifierKeys::MK_MBUTTON.bits(), ModifierKeys::MK_RBUTTON.bits()];

/// The drop source of a drag and drop operation.
pub(crate) struct OleDragSource;

impl OleDragSource {
    /// Whether a drag goes on: it is cancelled by the escape key and by a
    /// second mouse button, and ends in a drop when no button is held.
    pub(crate) fn query_continue_drag(f_escape_pressed: i32, grf_key_state: i32) -> i32 {
        if f_escape_pressed != 0 {
            return DRAGDROP_S_CANCEL;
        }

        let pressed_mouse_buttons = MOUSE_BUTTONS.iter().filter(|&&mb| (grf_key_state & mb) == mb).count();

        if pressed_mouse_buttons >= 2 {
            return DRAGDROP_S_CANCEL;
        }
        if pressed_mouse_buttons == 0 {
            return DRAGDROP_S_DROP;
        }

        HRESULT::S_OK as i32
    }
}


impl IDropSourceImpl for OleDragSource {
    fn query_continue_drag(&self, f_escape_pressed: i32, grf_key_state: i32) -> i32 {
        OleDragSource::query_continue_drag(f_escape_pressed, grf_key_state)
    }

    fn give_feedback(&self, _dw_effect: DropEffect) -> i32 {
        DRAGDROP_S_USEDEFAULTCURSORS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEFT: i32 = 0x0001;
    const RIGHT: i32 = 0x0002;
    const SHIFT: i32 = 0x0004;
    const CONTROL: i32 = 0x0008;
    const MIDDLE: i32 = 0x0010;

    #[test]
    fn a_drag_goes_on_while_one_button_is_held() {
        for button in [LEFT, RIGHT, MIDDLE] {
            assert_eq!(OleDragSource::query_continue_drag(0, button), 0);
            assert_eq!(OleDragSource::query_continue_drag(0, button | SHIFT | CONTROL), 0);
        }
    }

    #[test]
    fn releasing_the_button_drops() {
        assert_eq!(OleDragSource::query_continue_drag(0, 0), DRAGDROP_S_DROP);
        assert_eq!(OleDragSource::query_continue_drag(0, SHIFT | CONTROL), DRAGDROP_S_DROP);
    }

    #[test]
    fn the_escape_key_and_a_second_button_cancel() {
        assert_eq!(OleDragSource::query_continue_drag(1, LEFT), DRAGDROP_S_CANCEL);
        assert_eq!(OleDragSource::query_continue_drag(1, 0), DRAGDROP_S_CANCEL);
        assert_eq!(OleDragSource::query_continue_drag(0, LEFT | RIGHT), DRAGDROP_S_CANCEL);
        assert_eq!(OleDragSource::query_continue_drag(0, LEFT | MIDDLE | RIGHT), DRAGDROP_S_CANCEL);
    }

    #[test]
    fn the_cursors_are_the_ones_of_the_system() {
        assert_eq!(OleDragSource.give_feedback(DropEffect::Copy), 0x0004_0102);
    }
}
