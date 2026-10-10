//! The input of the view (the second part of the view class of the
//! reference).
//!
//! Built in stage 1: touch, generic pointer and hover events. Stage 2 of
//! docs/porting/android-platform.md: key events (`dispatchKeyEvent`, with
//! the keyboard helper) and the input connection of the input method
//! (`onCreateInputConnection`, `IInitEditorInfo`); until then the Java view
//! leaves both to its base class. Stage 3: the hover and key events of the
//! accessibility helper.

use crate::ferro_view::FerroView;
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
}
