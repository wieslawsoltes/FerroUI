//! Native Android composition root. The complete keyboard/view stays in shared crates.
#[cfg(target_os = "android")]
mod host;

/// Called by Android when it loads the application library.
///
/// # Safety
/// Android supplies its process-lifetime VM pointer; called once by System.loadLibrary.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub unsafe extern "system" fn JNI_OnLoad(
    vm: *mut std::ffi::c_void,
    _reserved: *mut std::ffi::c_void,
) -> i32 {
    // SAFETY: this entry point receives the actual process VM from Android.
    if unsafe { ferroboard_core::text::locale_text::install_android_java_vm(vm) }.is_err() {
        return jni_sys::JNI_ERR;
    }
    // SAFETY: forward the same VM to the selected framework's real application bootstrap.
    unsafe { ferroui_android::on_load(vm, host::build) }
}

/// Persist active sessions before Android backgrounds the activity.
///
/// # Safety
/// Called only by the matching static Java native method on the attached UI thread.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_org_ferroboard_android_MainActivity_nativeSave(
    env: *mut jni_sys::JNIEnv,
    _class: jni_sys::jclass,
) {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if let Some(app) = ferroui_controls::Application::current()
            .and_then(|app| app.cast::<ferroboard_sample::App>())
        {
            app.save_session();
        }
    }));
    if result.is_err() {
        // SAFETY: env is this JNI callback's live environment; terminated constant names.
        // An exception is explicit failure; a Rust unwind must never cross Java frames.
        unsafe {
            let class = ((**env).v1_6.FindClass)(env, c"java/lang/RuntimeException".as_ptr());
            if !class.is_null() {
                ((**env).v1_6.ThrowNew)(
                    env,
                    class,
                    c"FerroBoard session persistence failed".as_ptr(),
                );
                ((**env).v1_6.DeleteLocalRef)(env, class);
            }
        }
    }
}
