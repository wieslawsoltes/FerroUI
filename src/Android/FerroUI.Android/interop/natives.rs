//! The native methods of the Java layer, and the loading of the library.
//!
//! Every class of the Java layer declares the methods it forwards to as
//! `native`. They are registered here when the library is loaded, so the
//! library exports one symbol (`JNI_OnLoad`) and nothing depends on how a
//! linker treats the symbols of a dependency. Each native method converts
//! its arguments and calls the member of the Rust class that the member of
//! the reference class became.
//!
//! A panic must not unwind into Java frames. Each native method runs under
//! [`guard`], which catches it, writes it to the log of the system and
//! throws a `java.lang.RuntimeException` with its message: the application
//! stops as one with an uncaught exception does.

use super::java::{
    call_static_int, new_string, read_float_array, string_array_of, read_int_array, read_string, register_natives, throw_runtime_exception,
    JavaClass, JavaObject, NativeMethod,
};
use super::panic_message;
use crate::android_dispatcher_impl::AndroidDispatcherImpl;
use crate::ferro_activity::FerroActivity;
use crate::ferro_android_application::FerroAndroidApplication;
use crate::automation::NodeActionArguments;
use crate::explore_by_touch_helper::{IExploreByTouchCallbacks, INVALID_ID};
use crate::ferro_access_helper::{populate_host_node, write_node_info};
use crate::ferro_view::FerroView;
use crate::log::{self, LogPriority};
use crate::platform::skia_platform::{SurfaceProperties, TopLevelImpl};
use crate::platform::android_insets_manager::AnimationEasing;
use crate::platform::input::android_input_method::extracted_text_to_java;
use crate::platform::input::ferro_input_connection::FerroInputConnection;
use crate::platform::input::text_edit_buffer::KeyEventToDispatch;
use crate::platform::specific::helpers::android_keyboard_events_helper::{KeyEventData, KeyEventDevice};
use crate::platform::specific::helpers::android_motion_events_helper::MotionEventData;
use crate::platform::AndroidPlatformSettings;
use crate::platform::AndroidScreens;
use ferroui_base::PixelSize;
use ferroui_controls::AppBuilder;
use jni_sys::{jarray, jfloat, jint, jlong, jobject, JNIEnv, JNI_ERR, JNI_VERSION_1_6};
use ferroui_base::animation::easings::IEasing;
use std::ffi::c_void;
use std::ptr;
use std::rc::Rc;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::OnceLock;

/// The classes of the Java layer.
pub(crate) const FERRO_APPLICATION: &str = "org/ferroui/android/FerroApplication";
pub(crate) const FERRO_ACTIVITY: &str = "org/ferroui/android/FerroActivity";
pub(crate) const FERRO_VIEW: &str = "org/ferroui/android/FerroView";
pub(crate) const FERRO_ACCESS_HELPER: &str = "org/ferroui/android/FerroAccessHelper";
pub(crate) const FERRO_SURFACE_VIEW: &str = "org/ferroui/android/FerroSurfaceView";
pub(crate) const MAIN_LOOPER_BRIDGE: &str = "org/ferroui/android/MainLooperBridge";
pub(crate) const FERRO_INPUT_CONNECTION: &str = "org/ferroui/android/FerroInputConnection";
pub(crate) const NATIVE_CLICK_LISTENER: &str = "org/ferroui/android/NativeClickListener";
pub(crate) const PLATFORM_HELPER: &str = "org/ferroui/android/PlatformHelper";
pub(crate) const CONFIGURATION_CHANGED_RECEIVER: &str = "org/ferroui/android/ConfigurationChangedReceiver";

/// A `boolean` argument of a native method: one byte, zero for false.
type JBoolean = u8;

static NEXT_HANDLE: AtomicI64 = AtomicI64::new(1);

/// A number for an object of the backend that a Java object calls back
/// with. Never zero, which is what a Java object without a native side
/// holds.
pub(crate) fn next_handle() -> i64 {
    NEXT_HANDLE.fetch_add(1, Ordering::Relaxed)
}

/// `Build.VERSION.SDK_INT`: the API level of the system.
pub(crate) fn sdk_int() -> i32 {
    static SDK_INT: OnceLock<i32> = OnceLock::new();
    *SDK_INT.get_or_init(|| call_static_int(&JavaClass::find(PLATFORM_HELPER), "sdkInt", "()I", &[]))
}

/// Runs the body of a native method; a panic becomes a Java exception and
/// `default` is returned.
fn guard<T>(default: T, body: impl FnOnce() -> T) -> T {
    match catch_unwind(AssertUnwindSafe(body)) {
        Ok(value) => value,
        Err(panic) => {
            let message = panic_message(&*panic);
            log::write(LogPriority::Error, log::TAG, &format!("panic in a native method: {message}"));
            throw_runtime_exception(&message);
            default
        }
    }
}

/// Loads the library: keeps the virtual machine, registers the native
/// methods of the Java layer and keeps the function that makes the
/// application builder. Returns the version of the interface the library
/// needs, or an error value that makes the load fail.
///
/// Called by the `JNI_OnLoad` that
/// [`android_application!`](crate::android_application) writes.
///
/// # Safety
/// `vm` is the pointer the virtual machine passed to `JNI_OnLoad`.
pub unsafe fn on_load(vm: *mut c_void, build: fn() -> AppBuilder) -> i32 {
    // SAFETY: the contract of this function.
    unsafe { super::java::initialize(vm) };

    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        log::write(LogPriority::Error, log::TAG, &info.to_string());
        default_hook(info);
    }));

    let result = catch_unwind(|| {
        FerroAndroidApplication::set_builder(build);
        // The classes of the application are found on this thread, now: a thread the
        // virtual machine did not start finds only the classes of the system.
        JavaClass::preload(&[
            FERRO_APPLICATION,
            FERRO_ACTIVITY,
            FERRO_VIEW,
            FERRO_ACCESS_HELPER,
            FERRO_SURFACE_VIEW,
            MAIN_LOOPER_BRIDGE,
            FERRO_INPUT_CONNECTION,
            NATIVE_CLICK_LISTENER,
            crate::platform::storage::android_storage_item::STORAGE_HELPER,
            PLATFORM_HELPER,
            CONFIGURATION_CHANGED_RECEIVER,
        ]);
        register_all();
    });
    match result {
        Ok(()) => JNI_VERSION_1_6,
        Err(panic) => {
            log::write(
                LogPriority::Fatal,
                log::TAG,
                &format!("the library could not be loaded: {}", panic_message(&*panic)),
            );
            JNI_ERR
        }
    }
}

macro_rules! native {
    ($name:literal, $signature:literal, $function:expr) => {
        NativeMethod { name: $name, signature: $signature, function: $function as *mut c_void }
    };
}

fn register_all() {
    // SAFETY (for every block below): each function is declared in this file with the
    // parameters of the signature it is registered with, after the environment and the
    // object or the class: `Z` is one byte, `I` an `i32`, `J` an `i64`, `F` an `f32`, and a
    // class or an array a reference.
    unsafe {
        register_natives(
            &JavaClass::find(FERRO_APPLICATION),
            &[native!(c"nativeOnCreate", c"()V", application_on_create as unsafe extern "system" fn(_, _))],
        );
    }
    unsafe {
        register_natives(
            &JavaClass::find(MAIN_LOOPER_BRIDGE),
            &[
                native!(c"nativeSignaled", c"()V", looper_signaled as unsafe extern "system" fn(_, _)),
                native!(c"nativeTimer", c"()V", looper_timer as unsafe extern "system" fn(_, _)),
                native!(c"nativeIdle", c"()V", looper_idle as unsafe extern "system" fn(_, _)),
            ],
        );
    }
    unsafe {
        register_natives(
            &JavaClass::find(FERRO_ACTIVITY),
            &[
                native!(c"nativeOnCreate", c"(Z)J", activity_on_create as unsafe extern "system" fn(_, _, _) -> _),
                native!(c"nativeOnCreated", c"(J)V", activity_on_created as unsafe extern "system" fn(_, _, _)),
                native!(
                    c"nativeHandleIntent",
                    c"(JLandroid/net/Uri;)V",
                    activity_handle_intent as unsafe extern "system" fn(_, _, _, _)
                ),
                native!(
                    c"nativeOnActivityResult",
                    c"(JIILandroid/content/Intent;)V",
                    activity_on_activity_result as unsafe extern "system" fn(_, _, _, _, _, _)
                ),
                native!(
                    c"nativeOnRequestPermissionsResult",
                    c"(JI[Ljava/lang/String;[I)V",
                    activity_on_request_permissions_result as unsafe extern "system" fn(_, _, _, _, _, _)
                ),
                native!(
                    c"nativeOnBackPressed",
                    c"(J)Z",
                    activity_on_back_pressed as unsafe extern "system" fn(_, _, _) -> _
                ),
                native!(
                    c"nativeHandleOnBackPressed",
                    c"(J)Z",
                    activity_handle_on_back_pressed as unsafe extern "system" fn(_, _, _) -> _
                ),
                native!(c"nativeOnStart", c"(J)V", activity_on_start as unsafe extern "system" fn(_, _, _)),
                native!(c"nativeOnStop", c"(J)V", activity_on_stop as unsafe extern "system" fn(_, _, _)),
                native!(c"nativeOnResume", c"(J)V", activity_on_resume as unsafe extern "system" fn(_, _, _)),
                native!(c"nativeOnDestroy", c"(J)V", activity_on_destroy as unsafe extern "system" fn(_, _, _)),
            ],
        );
    }
    unsafe {
        register_natives(
            &JavaClass::find(FERRO_VIEW),
            &[
                native!(
                    c"nativeCreate",
                    c"(Landroid/content/Context;)J",
                    view_create as unsafe extern "system" fn(_, _, _) -> _
                ),
                native!(c"nativeDispose", c"(J)V", view_dispose as unsafe extern "system" fn(_, _, _)),
                native!(c"nativeGlobalLayout", c"(J)V", view_global_layout as unsafe extern "system" fn(_, _, _)),
                native!(
                    c"nativeVisibilityChanged",
                    c"(JZ)V",
                    view_visibility_changed as unsafe extern "system" fn(_, _, _, _)
                ),
                native!(
                    c"nativeConfigurationChanged",
                    c"(JZZ)V",
                    view_configuration_changed as unsafe extern "system" fn(_, _, _, _, _)
                ),
                native!(
                    c"nativeCreateInputConnection",
                    c"(JLandroid/view/inputmethod/EditorInfo;)Landroid/view/inputmethod/InputConnection;",
                    view_create_input_connection as unsafe extern "system" fn(_, _, _, _) -> _
                ),
                native!(
                    c"nativeKeyEvent",
                    c"(JJIIIIIZZLjava/lang/String;ZII)I",
                    view_key_event as unsafe extern "system" fn(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _) -> _
                ),
                native!(
                    c"nativeMotionEvent",
                    c"(JJIIIIII[I[FFF)I",
                    view_motion_event as unsafe extern "system" fn(_, _, _, _, _, _, _, _, _, _, _, _, _, _) -> _
                ),
            ],
        );
    }
    unsafe {
        register_natives(
            &JavaClass::find(FERRO_ACCESS_HELPER),
            &[
                native!(
                    c"nativePopulateHost",
                    c"(JLandroid/view/accessibility/AccessibilityNodeInfo;Landroid/view/View;)V",
                    access_populate_host as unsafe extern "system" fn(_, _, _, _, _)
                ),
                native!(
                    c"nativePopulateNode",
                    c"(JILandroid/view/accessibility/AccessibilityNodeInfo;Landroid/view/View;)V",
                    access_populate_node as unsafe extern "system" fn(_, _, _, _, _, _)
                ),
                native!(
                    c"nativePerformAction",
                    c"(JIIZLjava/lang/String;)Z",
                    access_perform_action as unsafe extern "system" fn(_, _, _, _, _, _, _) -> _
                ),
                native!(
                    c"nativeFindFocus",
                    c"(JI)I",
                    access_find_focus as unsafe extern "system" fn(_, _, _, _) -> _
                ),
                native!(
                    c"nativeDispatchHoverEvent",
                    c"(JIFF)Z",
                    access_dispatch_hover_event as unsafe extern "system" fn(_, _, _, _, _, _) -> _
                ),
                native!(
                    c"nativeFocusChanged",
                    c"(JZ)V",
                    access_focus_changed as unsafe extern "system" fn(_, _, _, _)
                ),
            ],
        );
    }
    unsafe {
        register_natives(
            &JavaClass::find(FERRO_SURFACE_VIEW),
            &[
                native!(
                    c"nativeSurfaceCreated",
                    c"(JLandroid/view/Surface;ZIIF)V",
                    surface_created as unsafe extern "system" fn(_, _, _, _, _, _, _, _)
                ),
                native!(
                    c"nativeSurfaceChanged",
                    c"(JLandroid/view/Surface;ZIIFIII)V",
                    surface_changed as unsafe extern "system" fn(_, _, _, _, _, _, _, _, _, _, _)
                ),
                native!(c"nativeSurfaceDestroyed", c"(J)V", surface_destroyed as unsafe extern "system" fn(_, _, _)),
                native!(
                    c"nativeSurfaceRedrawNeeded",
                    c"(J)V",
                    surface_redraw_needed as unsafe extern "system" fn(_, _, _)
                ),
                native!(
                    c"nativeSurfaceRedrawNeededAsync",
                    c"(JLjava/lang/Runnable;)V",
                    surface_redraw_needed_async as unsafe extern "system" fn(_, _, _, _)
                ),
                native!(
                    c"nativeFocusChanged",
                    c"(JZ)V",
                    surface_focus_changed as unsafe extern "system" fn(_, _, _, _)
                ),
            ],
        );
    }
    unsafe {
        type Bool3 = unsafe extern "system" fn(*mut JNIEnv, jobject, jlong, jint, jint) -> JBoolean;
        type Bool1 = unsafe extern "system" fn(*mut JNIEnv, jobject, jlong) -> JBoolean;
        type BoolInt = unsafe extern "system" fn(*mut JNIEnv, jobject, jlong, jint) -> JBoolean;
        type BoolText = unsafe extern "system" fn(*mut JNIEnv, jobject, jlong, jobject, jint) -> JBoolean;
        register_natives(
            &JavaClass::find(FERRO_INPUT_CONNECTION),
            &[
                native!(c"nativeSetComposingRegion", c"(JII)Z", connection_set_composing_region as Bool3),
                native!(
                    c"nativeSetComposingText",
                    c"(JLjava/lang/String;I)Z",
                    connection_set_composing_text as BoolText
                ),
                native!(c"nativeSetSelection", c"(JII)Z", connection_set_selection as Bool3),
                native!(c"nativeBeginBatchEdit", c"(J)Z", connection_begin_batch_edit as Bool1),
                native!(c"nativeEndBatchEdit", c"(J)Z", connection_end_batch_edit as Bool1),
                native!(c"nativeCommitText", c"(JLjava/lang/String;I)Z", connection_commit_text as BoolText),
                native!(c"nativeDeleteSurroundingText", c"(JII)Z", connection_delete_surrounding_text as Bool3),
                native!(c"nativePerformEditorAction", c"(JI)Z", connection_perform_editor_action as BoolInt),
                native!(
                    c"nativeGetExtractedText",
                    c"(JZII)Landroid/view/inputmethod/ExtractedText;",
                    connection_get_extracted_text as unsafe extern "system" fn(_, _, _, _, _, _) -> _
                ),
                native!(
                    c"nativePerformContextMenuAction",
                    c"(JI)Z",
                    connection_perform_context_menu_action as BoolInt
                ),
                native!(
                    c"nativeCloseConnection",
                    c"(J)V",
                    connection_close_connection as unsafe extern "system" fn(_, _, _)
                ),
                native!(
                    c"nativeDeleteSurroundingTextInCodePoints",
                    c"(JII)Z",
                    connection_delete_surrounding_text_in_code_points as Bool3
                ),
                native!(c"nativeFinishComposingText", c"(J)Z", connection_finish_composing_text as Bool1),
                native!(
                    c"nativeGetCursorCapsMode",
                    c"(JI)I",
                    connection_get_cursor_caps_mode as unsafe extern "system" fn(_, _, _, _) -> _
                ),
                native!(
                    c"nativeGetSelectedText",
                    c"(JI)Ljava/lang/String;",
                    connection_get_selected_text as unsafe extern "system" fn(_, _, _, _) -> _
                ),
                native!(
                    c"nativeGetTextAfterCursor",
                    c"(JII)Ljava/lang/String;",
                    connection_get_text_after_cursor as unsafe extern "system" fn(_, _, _, _, _) -> _
                ),
                native!(
                    c"nativeGetTextBeforeCursor",
                    c"(JII)Ljava/lang/String;",
                    connection_get_text_before_cursor as unsafe extern "system" fn(_, _, _, _, _) -> _
                ),
                native!(
                    c"nativeSendKeyEvent",
                    c"(JLandroid/view/KeyEvent;)Z",
                    connection_send_key_event as unsafe extern "system" fn(_, _, _, _) -> _
                ),
            ],
        );
    }
    unsafe {
        register_natives(
            &JavaClass::find(NATIVE_CLICK_LISTENER),
            &[native!(c"nativeOnClick", c"(J)V", click_listener_on_click as unsafe extern "system" fn(_, _, _))],
        );
    }
    unsafe {
        register_natives(
            &JavaClass::find(CONFIGURATION_CHANGED_RECEIVER),
            &[native!(c"nativeOnReceive", c"()V", configuration_changed_on_receive as unsafe extern "system" fn(_, _))],
        );
    }
    unsafe {
        register_natives(
            &JavaClass::find(PLATFORM_HELPER),
            &[
                native!(
                    c"nativeInsetsAnimationStart",
                    c"(JIIJLandroid/view/animation/Interpolator;)V",
                    helper_insets_animation_start as unsafe extern "system" fn(_, _, _, _, _, _, _)
                ),
                native!(
                    c"nativeInsetsGlobalLayout",
                    c"(J)V",
                    helper_insets_global_layout as unsafe extern "system" fn(_, _, _)
                ),
                native!(
                    c"nativeApplyWindowInsets",
                    c"(JZZI)V",
                    helper_apply_window_insets as unsafe extern "system" fn(_, _, _, _, _, _)
                ),
                native!(
                    c"nativeDisplaysChanged",
                    c"(J)V",
                    helper_displays_changed as unsafe extern "system" fn(_, _, _)
                ),
            ],
        );
    }
}

// ---- FerroApplication ---------------------------------------------------------------------

unsafe extern "system" fn application_on_create(_env: *mut JNIEnv, this: jobject) {
    guard((), || {
        // SAFETY: `this` is the object the method was called on.
        if let Some(application) = unsafe { JavaObject::from_raw(this) } {
            FerroAndroidApplication::on_create(application);
        }
    });
}

// ---- MainLooperBridge ---------------------------------------------------------------------

unsafe extern "system" fn looper_signaled(_env: *mut JNIEnv, _class: jobject) {
    guard((), AndroidDispatcherImpl::on_signaled);
}

unsafe extern "system" fn looper_timer(_env: *mut JNIEnv, _class: jobject) {
    guard((), AndroidDispatcherImpl::on_timer);
}

unsafe extern "system" fn looper_idle(_env: *mut JNIEnv, _class: jobject) {
    guard((), AndroidDispatcherImpl::on_idle);
}

// ---- FerroActivity ------------------------------------------------------------------------

unsafe extern "system" fn activity_on_create(_env: *mut JNIEnv, this: jobject, is_main_activity: JBoolean) -> jlong {
    guard(0, || {
        // SAFETY: `this` is the object the method was called on.
        match unsafe { JavaObject::from_raw(this) } {
            Some(activity) => FerroActivity::on_create(activity, is_main_activity != 0),
            None => 0,
        }
    })
}

unsafe extern "system" fn activity_on_created(_env: *mut JNIEnv, _class: jobject, handle: jlong) {
    guard((), || {
        if let Some(activity) = FerroActivity::from_handle(handle) {
            activity.on_created();
        }
    });
}

unsafe extern "system" fn activity_handle_intent(_env: *mut JNIEnv, _class: jobject, handle: jlong, data: jobject) {
    guard((), || {
        // SAFETY: `data` is the argument of the method, a reference or null.
        let data = unsafe { JavaObject::from_raw(data) };
        if let Some(activity) = FerroActivity::from_handle(handle) {
            activity.handle_intent(data);
        }
    });
}

unsafe extern "system" fn activity_on_activity_result(
    _env: *mut JNIEnv,
    _class: jobject,
    handle: jlong,
    request_code: jint,
    result_code: jint,
    data: jobject,
) {
    guard((), || {
        // SAFETY: `data` is the argument of the method, a reference or null.
        let data = unsafe { JavaObject::from_raw(data) };
        if let Some(activity) = FerroActivity::from_handle(handle) {
            activity.on_activity_result(request_code, result_code, data);
        }
    });
}

unsafe extern "system" fn activity_on_request_permissions_result(
    _env: *mut JNIEnv,
    _class: jobject,
    handle: jlong,
    request_code: jint,
    permissions: jobject,
    grant_results: jarray,
) {
    guard((), || {
        // SAFETY: the two references are the `String[]` and the `int[]` of the signature,
        // or null.
        let (permissions, grant_results) =
            unsafe { (JavaObject::from_raw(permissions), read_int_array(grant_results)) };
        let permissions = permissions.map(|permissions| string_array_of(&permissions)).unwrap_or_default();
        if let Some(activity) = FerroActivity::from_handle(handle) {
            activity.on_request_permissions_result(request_code, &permissions, &grant_results);
        }
    });
}

unsafe extern "system" fn activity_on_back_pressed(_env: *mut JNIEnv, _class: jobject, handle: jlong) -> JBoolean {
    guard(0, || match FerroActivity::from_handle(handle) {
        Some(activity) => activity.on_back_pressed() as JBoolean,
        None => 0,
    })
}

unsafe extern "system" fn activity_handle_on_back_pressed(
    _env: *mut JNIEnv,
    _class: jobject,
    handle: jlong,
) -> JBoolean {
    guard(1, || match FerroActivity::from_handle(handle) {
        Some(activity) => activity.handle_on_back_pressed() as JBoolean,
        None => 1,
    })
}

unsafe extern "system" fn activity_on_start(_env: *mut JNIEnv, _class: jobject, handle: jlong) {
    guard((), || {
        if let Some(activity) = FerroActivity::from_handle(handle) {
            activity.on_start();
        }
    });
}

unsafe extern "system" fn activity_on_stop(_env: *mut JNIEnv, _class: jobject, handle: jlong) {
    guard((), || {
        if let Some(activity) = FerroActivity::from_handle(handle) {
            activity.on_stop();
        }
    });
}

unsafe extern "system" fn activity_on_resume(_env: *mut JNIEnv, _class: jobject, handle: jlong) {
    guard((), || {
        if let Some(activity) = FerroActivity::from_handle(handle) {
            activity.on_resume();
        }
    });
}

unsafe extern "system" fn activity_on_destroy(_env: *mut JNIEnv, _class: jobject, handle: jlong) {
    guard((), || {
        if let Some(activity) = FerroActivity::from_handle(handle) {
            activity.on_destroy();
        }
    });
}

// ---- FerroView ----------------------------------------------------------------------------

unsafe extern "system" fn view_create(_env: *mut JNIEnv, this: jobject, context: jobject) -> jlong {
    guard(0, || {
        // SAFETY: `this` is the object the method was called on and `context` its
        // argument, a reference or null.
        let (view, context) = unsafe { (JavaObject::from_raw(this), JavaObject::from_raw(context)) };
        match (view, context) {
            (Some(view), Some(context)) => FerroView::native_create(view, context),
            _ => panic!("The context of a view must not be null"),
        }
    })
}

unsafe extern "system" fn view_dispose(_env: *mut JNIEnv, _class: jobject, handle: jlong) {
    guard((), || {
        if let Some(view) = FerroView::from_handle(handle) {
            view.dispose();
        }
    });
}

unsafe extern "system" fn view_global_layout(_env: *mut JNIEnv, _class: jobject, handle: jlong) {
    guard((), || {
        if let Some(view) = FerroView::from_handle(handle) {
            view.on_global_layout();
        }
    });
}

unsafe extern "system" fn view_visibility_changed(
    _env: *mut JNIEnv,
    _class: jobject,
    handle: jlong,
    is_visible: JBoolean,
) {
    guard((), || {
        if let Some(view) = FerroView::from_handle(handle) {
            view.on_visibility_changed(is_visible != 0);
        }
    });
}

unsafe extern "system" fn view_configuration_changed(
    _env: *mut JNIEnv,
    _class: jobject,
    handle: jlong,
    has_configuration: JBoolean,
    night: JBoolean,
) {
    guard((), || {
        if let Some(view) = FerroView::from_handle(handle) {
            view.send_configuration_changed(has_configuration != 0, night != 0);
        }
    });
}

#[allow(clippy::too_many_arguments)]
unsafe extern "system" fn view_motion_event(
    _env: *mut JNIEnv,
    _class: jobject,
    handle: jlong,
    event_time: jlong,
    action_masked: jint,
    action_index: jint,
    meta_state: jint,
    button_state: jint,
    action_button: jint,
    history_size: jint,
    pointers: jarray,
    values: jarray,
    hscroll: jfloat,
    vscroll: jfloat,
) -> jint {
    guard(0, || {
        // SAFETY: the two references are the `int[]` and the `float[]` of the signature,
        // or null.
        let (pointers, values) = unsafe { (read_int_array(pointers), read_float_array(values)) };
        let Some(view) = FerroView::from_handle(handle) else {
            // No view: no result, and the base class dispatches the event.
            return 4;
        };
        let event = MotionEventData::from_arrays(
            event_time,
            action_masked,
            action_index,
            meta_state,
            button_state,
            action_button,
            history_size,
            &pointers,
            &values,
            hscroll,
            vscroll,
        );
        view.dispatch_motion_event(event.as_ref())
    })
}

#[allow(clippy::too_many_arguments)]
unsafe extern "system" fn view_key_event(
    _env: *mut JNIEnv,
    _class: jobject,
    handle: jlong,
    event_time: jlong,
    action: jint,
    key_code: jint,
    scan_code: jint,
    unicode_char: jint,
    repeat_count: jint,
    is_ctrl_pressed: JBoolean,
    is_shift_pressed: JBoolean,
    characters: jobject,
    has_device: JBoolean,
    device_sources: jint,
    device_keyboard_type: jint,
) -> jint {
    guard(0, || {
        // SAFETY: `characters` is the `String` of the signature, or null.
        let characters = unsafe { read_string(characters) };
        let Some(view) = FerroView::from_handle(handle) else {
            // No view: no result, and the base class dispatches the event.
            return 4;
        };
        let event = KeyEventData {
            event_time,
            action,
            key_code,
            scan_code,
            unicode_char,
            repeat_count,
            is_ctrl_pressed: is_ctrl_pressed != 0,
            is_shift_pressed: is_shift_pressed != 0,
            characters,
            device: (has_device != 0)
                .then_some(KeyEventDevice { sources: device_sources, keyboard_type: device_keyboard_type }),
        };
        view.dispatch_key_event(Some(&event))
    })
}

unsafe extern "system" fn view_create_input_connection(
    _env: *mut JNIEnv,
    _class: jobject,
    handle: jlong,
    out_attrs: jobject,
) -> jobject {
    guard(ptr::null_mut(), || {
        // SAFETY: `out_attrs` is the argument of the method, a reference or null.
        let out_attrs = unsafe { JavaObject::from_raw(out_attrs) };
        let (Some(view), Some(out_attrs)) = (FerroView::from_handle(handle), out_attrs) else {
            return ptr::null_mut();
        };
        view.on_create_input_connection(&out_attrs).map_or(ptr::null_mut(), |connection| connection.into_raw())
    })
}

// ---- FerroAccessHelper ----------------------------------------------------------------------

unsafe extern "system" fn access_populate_host(
    _env: *mut JNIEnv,
    _class: jobject,
    handle: jlong,
    info: jobject,
    host: jobject,
) {
    guard((), || {
        // SAFETY: the two references are the arguments of the method, references or null.
        let (info, host) = unsafe { (JavaObject::from_raw(info), JavaObject::from_raw(host)) };
        let (Some(view), Some(info), Some(host)) = (FerroView::from_handle(handle), info, host) else {
            return;
        };
        let children = view.access_helper().get_visible_virtual_views();
        populate_host_node(&children, &info, &host);
    });
}

unsafe extern "system" fn access_populate_node(
    _env: *mut JNIEnv,
    _class: jobject,
    handle: jlong,
    virtual_view_id: jint,
    info: jobject,
    host: jobject,
) {
    guard((), || {
        // SAFETY: the two references are the arguments of the method, references or null.
        let (info, host) = unsafe { (JavaObject::from_raw(info), JavaObject::from_raw(host)) };
        let (Some(view), Some(info), Some(host)) = (FerroView::from_handle(handle), info, host) else {
            return;
        };
        let node = view.access_helper().create_node_for_virtual_view(virtual_view_id);
        write_node_info(&node, &info, &host);
    });
}

unsafe extern "system" fn access_perform_action(
    _env: *mut JNIEnv,
    _class: jobject,
    handle: jlong,
    virtual_view_id: jint,
    action: jint,
    has_arguments: JBoolean,
    set_text: jobject,
) -> JBoolean {
    guard(0, || {
        // SAFETY: `set_text` is the `String` of the signature, or null.
        let set_text = unsafe { read_string(set_text) };
        let Some(view) = FerroView::from_handle(handle) else {
            return 0;
        };
        let arguments = (has_arguments != 0).then(|| NodeActionArguments { set_text_char_sequence: set_text });
        view.access_helper().perform_action(virtual_view_id, action, arguments.as_ref()) as JBoolean
    })
}

unsafe extern "system" fn access_find_focus(_env: *mut JNIEnv, _class: jobject, handle: jlong, focus_type: jint) -> jint {
    guard(INVALID_ID, || match FerroView::from_handle(handle) {
        Some(view) => view.access_helper().find_focus(focus_type),
        None => INVALID_ID,
    })
}

unsafe extern "system" fn access_dispatch_hover_event(
    _env: *mut JNIEnv,
    _class: jobject,
    handle: jlong,
    action: jint,
    x: jfloat,
    y: jfloat,
) -> JBoolean {
    guard(0, || match FerroView::from_handle(handle) {
        Some(view) => view.dispatch_access_hover_event(action, x, y) as JBoolean,
        None => 0,
    })
}

unsafe extern "system" fn access_focus_changed(_env: *mut JNIEnv, _class: jobject, handle: jlong, gain_focus: JBoolean) {
    guard((), || {
        if let Some(view) = FerroView::from_handle(handle) {
            view.on_focus_changed(gain_focus != 0);
        }
    });
}

// ---- FerroInputConnection -----------------------------------------------------------------

/// Runs a member of the connection registered under `handle`; a connection
/// that is gone answers `default`.
fn with_connection<T: Copy>(handle: jlong, default: T, body: impl FnOnce(&FerroInputConnection) -> T) -> T {
    guard(default, || match FerroInputConnection::from_handle(handle) {
        Some(connection) => body(&connection),
        None => default,
    })
}

fn string_or_null(text: Option<String>) -> jobject {
    text.map_or(ptr::null_mut(), |text| new_string(&text).into_raw())
}

unsafe extern "system" fn connection_set_composing_region(
    _env: *mut JNIEnv,
    _class: jobject,
    handle: jlong,
    start: jint,
    end: jint,
) -> JBoolean {
    with_connection(handle, 0, |connection| connection.set_composing_region(start, end) as JBoolean)
}

unsafe extern "system" fn connection_set_composing_text(
    _env: *mut JNIEnv,
    _class: jobject,
    handle: jlong,
    text: jobject,
    new_cursor_position: jint,
) -> JBoolean {
    with_connection(handle, 0, |connection| {
        // SAFETY: `text` is the `String` of the signature, or null.
        let text = unsafe { read_string(text) };
        connection.set_composing_text(text.as_deref(), new_cursor_position) as JBoolean
    })
}

unsafe extern "system" fn connection_set_selection(
    _env: *mut JNIEnv,
    _class: jobject,
    handle: jlong,
    start: jint,
    end: jint,
) -> JBoolean {
    with_connection(handle, 0, |connection| connection.set_selection(start, end) as JBoolean)
}

unsafe extern "system" fn connection_begin_batch_edit(_env: *mut JNIEnv, _class: jobject, handle: jlong) -> JBoolean {
    with_connection(handle, 0, |connection| connection.begin_batch_edit() as JBoolean)
}

unsafe extern "system" fn connection_end_batch_edit(_env: *mut JNIEnv, _class: jobject, handle: jlong) -> JBoolean {
    with_connection(handle, 0, |connection| connection.end_batch_edit() as JBoolean)
}

unsafe extern "system" fn connection_commit_text(
    _env: *mut JNIEnv,
    _class: jobject,
    handle: jlong,
    text: jobject,
    new_cursor_position: jint,
) -> JBoolean {
    with_connection(handle, 0, |connection| {
        // SAFETY: `text` is the `String` of the signature, or null.
        let text = unsafe { read_string(text) };
        connection.commit_text(text.as_deref(), new_cursor_position) as JBoolean
    })
}

unsafe extern "system" fn connection_delete_surrounding_text(
    _env: *mut JNIEnv,
    _class: jobject,
    handle: jlong,
    before_length: jint,
    after_length: jint,
) -> JBoolean {
    with_connection(handle, 0, |connection| {
        connection.delete_surrounding_text(before_length, after_length) as JBoolean
    })
}

unsafe extern "system" fn connection_perform_editor_action(
    _env: *mut JNIEnv,
    _class: jobject,
    handle: jlong,
    action_code: jint,
) -> JBoolean {
    with_connection(handle, 0, |connection| connection.perform_editor_action(action_code) as JBoolean)
}

unsafe extern "system" fn connection_get_extracted_text(
    _env: *mut JNIEnv,
    _class: jobject,
    handle: jlong,
    has_request: JBoolean,
    token: jint,
    flags: jint,
) -> jobject {
    with_connection(handle, ptr::null_mut(), |connection| {
        connection
            .get_extracted_text((has_request != 0).then_some(token), flags)
            .and_then(|text| extracted_text_to_java(&text))
            .map_or(ptr::null_mut(), |text| text.into_raw())
    })
}

unsafe extern "system" fn connection_perform_context_menu_action(
    _env: *mut JNIEnv,
    _class: jobject,
    handle: jlong,
    id: jint,
) -> JBoolean {
    with_connection(handle, 0, |connection| connection.perform_context_menu_action(id) as JBoolean)
}

unsafe extern "system" fn connection_close_connection(_env: *mut JNIEnv, _class: jobject, handle: jlong) {
    with_connection(handle, (), |connection| connection.close_connection());
}

unsafe extern "system" fn connection_delete_surrounding_text_in_code_points(
    _env: *mut JNIEnv,
    _class: jobject,
    handle: jlong,
    before_length: jint,
    after_length: jint,
) -> JBoolean {
    with_connection(handle, 0, |connection| {
        connection.delete_surrounding_text_in_code_points(before_length, after_length) as JBoolean
    })
}

unsafe extern "system" fn connection_finish_composing_text(
    _env: *mut JNIEnv,
    _class: jobject,
    handle: jlong,
) -> JBoolean {
    with_connection(handle, 0, |connection| connection.finish_composing_text() as JBoolean)
}

unsafe extern "system" fn connection_get_cursor_caps_mode(
    _env: *mut JNIEnv,
    _class: jobject,
    handle: jlong,
    req_modes: jint,
) -> jint {
    with_connection(handle, 0, |connection| connection.get_cursor_caps_mode(req_modes))
}

unsafe extern "system" fn connection_get_selected_text(
    _env: *mut JNIEnv,
    _class: jobject,
    handle: jlong,
    flags: jint,
) -> jobject {
    with_connection(handle, ptr::null_mut(), |connection| {
        string_or_null(connection.get_selected_text_formatted(flags))
    })
}

unsafe extern "system" fn connection_get_text_after_cursor(
    _env: *mut JNIEnv,
    _class: jobject,
    handle: jlong,
    n: jint,
    flags: jint,
) -> jobject {
    with_connection(handle, ptr::null_mut(), |connection| {
        string_or_null(connection.get_text_after_cursor_formatted(n, flags))
    })
}

unsafe extern "system" fn connection_get_text_before_cursor(
    _env: *mut JNIEnv,
    _class: jobject,
    handle: jlong,
    n: jint,
    flags: jint,
) -> jobject {
    with_connection(handle, ptr::null_mut(), |connection| {
        string_or_null(connection.get_text_before_cursor_formatted(n, flags))
    })
}

unsafe extern "system" fn connection_send_key_event(
    _env: *mut JNIEnv,
    _class: jobject,
    handle: jlong,
    event: jobject,
) -> JBoolean {
    with_connection(handle, 0, |connection| {
        // SAFETY: `event` is the argument of the method, a reference or null.
        let event = unsafe { JavaObject::from_raw(event) };
        connection.send_key_event(event.map(|event| KeyEventToDispatch::System(Rc::new(event)))) as JBoolean
    })
}

// ---- NativeClickListener ------------------------------------------------------------------

unsafe extern "system" fn click_listener_on_click(_env: *mut JNIEnv, _class: jobject, handle: jlong) {
    guard((), || super::listeners::on_click(handle));
}

// ---- ConfigurationChangedReceiver ---------------------------------------------------------

unsafe extern "system" fn configuration_changed_on_receive(_env: *mut JNIEnv, _class: jobject) {
    guard((), AndroidPlatformSettings::on_receive);
}

// ---- FerroSurfaceView ---------------------------------------------------------------------

/// # Safety
/// `surface` is null or a valid reference.
unsafe fn surface_properties(
    surface: jobject,
    has_frame: JBoolean,
    frame_width: jint,
    frame_height: jint,
    density: jfloat,
) -> SurfaceProperties {
    SurfaceProperties {
        // SAFETY: the contract of this function.
        surface: unsafe { JavaObject::from_raw(surface) },
        frame: (has_frame != 0).then_some(PixelSize::new(frame_width, frame_height)),
        density,
    }
}

#[allow(clippy::too_many_arguments)]
unsafe extern "system" fn surface_created(
    _env: *mut JNIEnv,
    _class: jobject,
    handle: jlong,
    surface: jobject,
    has_frame: JBoolean,
    frame_width: jint,
    frame_height: jint,
    density: jfloat,
) {
    guard((), || {
        // SAFETY: `surface` is the argument of the method, a reference or null.
        let properties = unsafe { surface_properties(surface, has_frame, frame_width, frame_height, density) };
        if let Some(top_level) = TopLevelImpl::from_handle(handle) {
            top_level.on_surface_created(&properties);
        }
    });
}

#[allow(clippy::too_many_arguments)]
unsafe extern "system" fn surface_changed(
    _env: *mut JNIEnv,
    _class: jobject,
    handle: jlong,
    surface: jobject,
    has_frame: JBoolean,
    frame_width: jint,
    frame_height: jint,
    density: jfloat,
    format: jint,
    width: jint,
    height: jint,
) {
    guard((), || {
        // SAFETY: `surface` is the argument of the method, a reference or null.
        let properties = unsafe { surface_properties(surface, has_frame, frame_width, frame_height, density) };
        if let Some(top_level) = TopLevelImpl::from_handle(handle) {
            top_level.on_surface_changed(&properties, format, width, height);
        }
    });
}

unsafe extern "system" fn surface_destroyed(_env: *mut JNIEnv, _class: jobject, handle: jlong) {
    guard((), || {
        if let Some(top_level) = TopLevelImpl::from_handle(handle) {
            top_level.on_surface_destroyed();
        }
    });
}

unsafe extern "system" fn surface_redraw_needed(_env: *mut JNIEnv, _class: jobject, handle: jlong) {
    guard((), || {
        if let Some(top_level) = TopLevelImpl::from_handle(handle) {
            top_level.on_surface_redraw_needed();
        }
    });
}

unsafe extern "system" fn surface_redraw_needed_async(
    _env: *mut JNIEnv,
    _class: jobject,
    handle: jlong,
    drawing_finished: jobject,
) {
    guard((), || {
        // SAFETY: `drawing_finished` is the argument of the method, a reference or null.
        let Some(drawing_finished) = (unsafe { JavaObject::from_raw(drawing_finished) }) else {
            return;
        };
        match TopLevelImpl::from_handle(handle) {
            Some(top_level) => top_level.on_surface_redraw_needed_async(drawing_finished),
            // Nobody draws: the system must not wait.
            None => super::java::call_void(&drawing_finished, "run", "()V", &[]),
        }
    });
}

unsafe extern "system" fn surface_focus_changed(
    _env: *mut JNIEnv,
    _class: jobject,
    handle: jlong,
    has_focus: JBoolean,
) {
    guard((), || {
        if let Some(top_level) = TopLevelImpl::from_handle(handle) {
            top_level.on_view_focus_changed(has_focus != 0);
        }
    });
}

// ---- PlatformHelper -----------------------------------------------------------------------

unsafe extern "system" fn helper_apply_window_insets(
    _env: *mut JNIEnv,
    _class: jobject,
    handle: jlong,
    has_insets: JBoolean,
    ime_visible: JBoolean,
    ime_bottom: jint,
) {
    guard((), || {
        if let Some(insets_manager) = TopLevelImpl::insets_manager_from_handle(handle) {
            insets_manager.on_apply_window_insets(has_insets != 0, ime_visible != 0, ime_bottom);
        }
    });
}

unsafe extern "system" fn helper_insets_animation_start(
    _env: *mut JNIEnv,
    _class: jobject,
    handle: jlong,
    lower_bound_bottom: jint,
    upper_bound_bottom: jint,
    duration_millis: jlong,
    interpolator: jobject,
) {
    guard((), || {
        // SAFETY: `interpolator` is the argument of the method, a reference or null.
        let interpolator = unsafe { JavaObject::from_raw(interpolator) };
        if let Some(insets_manager) = TopLevelImpl::insets_manager_from_handle(handle) {
            let easing = interpolator.map(|interpolator| Rc::new(AnimationEasing::new(interpolator)) as Rc<dyn IEasing>);
            insets_manager.on_insets_animation_start(lower_bound_bottom, upper_bound_bottom, duration_millis, easing);
        }
    });
}

unsafe extern "system" fn helper_insets_global_layout(_env: *mut JNIEnv, _class: jobject, handle: jlong) {
    guard((), || {
        if let Some(insets_manager) = TopLevelImpl::insets_manager_from_handle(handle) {
            insets_manager.on_global_layout();
        }
    });
}

unsafe extern "system" fn helper_displays_changed(_env: *mut JNIEnv, _class: jobject, handle: jlong) {
    guard((), || AndroidScreens::on_displays_changed(handle));
}
