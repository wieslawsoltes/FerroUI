//! The input of the view (the second part of the view class of the
//! reference).
//!
//! Touch, generic pointer, hover and key events. Stage 2b of
//! docs/porting/android-platform.md: the input connection of the input
//! method (`onCreateInputConnection`, `IInitEditorInfo`); until then the
//! Java view leaves it to its base class. Stage 3: the hover and key events
//! of the accessibility helper.

use crate::ferro_view::FerroView;
use crate::platform::specific::helpers::android_keyboard_events_helper::KeyEventData;
use crate::platform::specific::helpers::android_motion_events_helper::MotionEventData;

/// The answer to the Java view: no result, and whether the base class
/// dispatches the event too.
const RESULT_NONE: i32 = 0;
const RESULT_FALSE: i32 = 1;
const RESULT_TRUE: i32 = 2;
const CALL_BASE: i32 = 4;

impl FerroView {
    /// A touch, generic pointer or hover event of the view. The answer is
    /// what the pointer helper returned and whether the base class is
    /// called, as the Java view reads it.
    pub(crate) fn dispatch_motion_event(&self, e: Option<&MotionEventData>) -> i32 {
        let mut call_base = true;
        let result = self.top_level_impl().pointer_helper().dispatch_motion_event(e, &mut call_base);

        let result = match result {
            None => RESULT_NONE,
            Some(false) => RESULT_FALSE,
            Some(true) => RESULT_TRUE,
        };
        result | if call_base { CALL_BASE } else { 0 }
    }

    /// A key event of the view, answered as a motion event is.
    pub(crate) fn dispatch_key_event(&self, e: Option<&KeyEventData>) -> i32 {
        let mut call_base = true;
        let res = self.top_level_impl().keyboard_helper().dispatch_key_event(e, &mut call_base);
        if res == Some(false) {
            // The reference lets the accessibility helper of the view decide here whether
            // the base class is called; without the helper (stage 3 of
            // docs/porting/android-platform.md) its expression is false.
            call_base = false;
        }

        let result = match res {
            None => RESULT_NONE,
            Some(false) => RESULT_FALSE,
            Some(true) => RESULT_TRUE,
        };
        result | if call_base { CALL_BASE } else { 0 }
    }
}
