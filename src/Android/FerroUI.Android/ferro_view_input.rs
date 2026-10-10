//! The input of the view (the second part of the view class of the
//! reference).
//!
//! Touch, generic pointer, hover and key events, and the input connection
//! of the input method. What the input method gives the view to make the
//! connection with (`IInitEditorInfo` of the reference) is kept by the
//! top-level of the view, which exists before this object does. Stage 3 of
//! docs/porting/android-platform.md: the hover and key events of the
//! accessibility helper.

use crate::ferro_view::FerroView;
use crate::i_init_editor_info::EditorInfo;
use crate::interop::java::{
    call_static_void, new_object, new_string_array, JavaClass, JavaLocal, JavaObject, JavaRef, JavaValue,
};
use crate::interop::natives::{FERRO_INPUT_CONNECTION, PLATFORM_HELPER};
use crate::platform::input::text_edit_buffer::IInputConnectionTopLevel;
use std::rc::Rc;
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

    /// `onCreateInputConnection` of the view: fills `out_attrs` (an
    /// `EditorInfo`) and returns the input connection of the Java layer, or
    /// `None` when no text input has the focus.
    pub(crate) fn on_create_input_connection(&self, out_attrs: &JavaObject) -> Option<JavaLocal> {
        let init = self.top_level_impl().editor_info_init()?;
        let top_level: Rc<dyn IInputConnectionTopLevel> = self.top_level_impl().clone();
        let mut editor_info = EditorInfo::default();
        let input_connection = init(&top_level, &mut editor_info)?;

        let hint_locales = editor_info.hint_locales.as_deref().map(new_string_array);
        call_static_void(
            &JavaClass::find(PLATFORM_HELPER),
            "setEditorInfo",
            "(Landroid/view/inputmethod/EditorInfo;III[Ljava/lang/String;)V",
            &[
                JavaValue::Object(Some(out_attrs)),
                JavaValue::Int(editor_info.input_type),
                JavaValue::Int(editor_info.ime_options),
                JavaValue::Int(editor_info.initial_caps_mode),
                JavaValue::Object(hint_locales.as_ref().map(|locales| locales as &dyn JavaRef)),
            ],
        );

        Some(new_object(
            &JavaClass::find(FERRO_INPUT_CONNECTION),
            "(J)V",
            &[JavaValue::Long(input_connection.register())],
        ))
    }
}
