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
    call_static_int, read_float_array, read_int_array, register_natives, throw_runtime_exception, JavaClass,
    JavaObject, NativeMethod,
};
use super::panic_message;
use crate::android_dispatcher_impl::AndroidDispatcherImpl;
use crate::ferro_activity::FerroActivity;
use crate::ferro_android_application::FerroAndroidApplication;
use crate::ferro_view::FerroView;
use crate::log::{self, LogPriority};
use crate::platform::skia_platform::{SurfaceProperties, TopLevelImpl};
use crate::platform::specific::helpers::android_motion_events_helper::MotionEventData;
use crate::platform::AndroidScreens;
use ferroui_base::PixelSize;
use ferroui_controls::AppBuilder;
use jni_sys::{jarray, jfloat, jint, jlong, jobject, JNIEnv, JNI_ERR, JNI_VERSION_1_6};
use std::ffi::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::OnceLock;

/// The classes of the Java layer.
pub(crate) const FERRO_APPLICATION: &str = "org/ferroui/android/FerroApplication";
pub(crate) const FERRO_ACTIVITY: &str = "org/ferroui/android/FerroActivity";
pub(crate) const FERRO_VIEW: &str = "org/ferroui/android/FerroView";
pub(crate) const FERRO_SURFACE_VIEW: &str = "org/ferroui/android/FerroSurfaceView";
pub(crate) const MAIN_LOOPER_BRIDGE: &str = "org/ferroui/android/MainLooperBridge";
pub(crate) const PLATFORM_HELPER: &str = "org/ferroui/android/PlatformHelper";

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
            FERRO_SURFACE_VIEW,
            MAIN_LOOPER_BRIDGE,
            PLATFORM_HELPER,
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
                    c"nativeMotionEvent",
                    c"(JJIIIIII[I[FFF)I",
                    view_motion_event as unsafe extern "system" fn(_, _, _, _, _, _, _, _, _, _, _, _, _, _) -> _
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
        register_natives(
            &JavaClass::find(PLATFORM_HELPER),
            &[
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

unsafe extern "system" fn helper_displays_changed(_env: *mut JNIEnv, _class: jobject, handle: jlong) {
    guard((), || AndroidScreens::on_displays_changed(handle));
}
