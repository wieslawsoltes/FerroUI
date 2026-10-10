//! The Android platform.
//!
//! An application is a shared library that the application class of the
//! Java layer of this crate loads (`java/org/ferroui/android`). There are
//! no windows: content is shown by a [`FerroView`] in an activity
//! ([`FerroActivity`]; the main activity shows the main view of the
//! [`ApplicationLifetime`]). The system drives everything: the dispatcher
//! is the main looper, rendering follows the choreographer on a render
//! thread, and the surface of the view is drawn through EGL or into the
//! buffer of its native window.
//!
//! [`interop`] is the only place that crosses to Java and to the NDK. The
//! parts that call the system exist on Android only; on another system the
//! crate is the logic of the backend, which its tests run there. The design
//! is in `docs/porting/android-platform.md`.

// On another system than Android the logic is compiled for its tests, and nothing of the crate
// calls it there.
#![cfg_attr(not(target_os = "android"), allow(dead_code))]

pub mod interop;
pub mod log;
pub mod platform;

mod android_platform;
mod android_runtime_platform;
mod application_lifetime;
mod cursor_factory;
mod i_activity_result_handler;
mod i_android_navigation_service;
mod i_ferro_activity;
mod i_init_editor_info;
mod platform_icon_loader;
mod stubs;

#[cfg(target_os = "android")]
mod android_dispatcher_impl;
#[cfg(target_os = "android")]
mod android_egl;
#[cfg(target_os = "android")]
mod android_view_control_handle;
#[cfg(target_os = "android")]
mod back_pressed_callback;
#[cfg(target_os = "android")]
mod choreographer_timer;
#[cfg(target_os = "android")]
mod ferro_activity;
#[cfg(target_os = "android")]
mod ferro_android_application;
#[cfg(target_os = "android")]
mod ferro_main_activity;
#[cfg(target_os = "android")]
mod ferro_view;
#[cfg(target_os = "android")]
mod ferro_view_input;

pub use android_platform::{AndroidPlatformOptions, AndroidRenderingMode};
pub use application_lifetime::ApplicationLifetime;
pub use cursor_factory::CursorFactory;
pub use i_activity_result_handler::{
    ActivityResultHandler, IActivityResultHandler, Intent, RequestPermissionsResultHandler, PERMISSION_DENIED,
    PERMISSION_GRANTED, RESULT_CANCELED, RESULT_FIRST_USER, RESULT_OK,
};
pub use i_android_navigation_service::{AndroidBackRequestedEventArgs, IActivityNavigationService};
pub use i_ferro_activity::IFerroActivity;
pub use platform_icon_loader::PlatformIconLoader;
pub use stubs::{PlatformIconLoaderStub, WindowingPlatformStub};

#[cfg(target_os = "android")]
pub use android_dispatcher_impl::AndroidDispatcherImpl;
#[cfg(target_os = "android")]
pub use android_platform::{AndroidApplicationExtensions, AndroidPlatform};
#[cfg(target_os = "android")]
pub use android_runtime_platform::{AndroidRuntimePlatform, AndroidRuntimePlatformServices};
#[cfg(target_os = "android")]
pub use android_view_control_handle::AndroidViewControlHandle;
#[cfg(target_os = "android")]
pub use choreographer_timer::ChoreographerTimer;
#[cfg(target_os = "android")]
pub use ferro_activity::FerroActivity;
#[cfg(target_os = "android")]
pub use ferro_android_application::FerroAndroidApplication;
#[cfg(target_os = "android")]
pub use ferro_main_activity::FerroMainActivity;
#[cfg(target_os = "android")]
pub use ferro_view::FerroView;
#[cfg(target_os = "android")]
pub use interop::natives::on_load;

/// Makes a shared library the native side of an Android application.
///
/// `$build` is a function that returns the application builder
/// (`fn() -> AppBuilder`), usually
/// `AppBuilder::configure::<App>().use_android()`: what an application
/// overrides `CreateAppBuilder` and `CustomizeAppBuilder` for in the
/// reference. The macro writes the one symbol the library exports,
/// `JNI_OnLoad`, which the virtual machine calls when the application class
/// of the Java layer loads the library.
#[macro_export]
macro_rules! android_application {
    ($build:expr) => {
        /// Called by the Java virtual machine when the library is loaded.
        ///
        /// # Safety
        /// Called by the virtual machine only, with its own pointer.
        #[cfg(target_os = "android")]
        #[no_mangle]
        pub unsafe extern "system" fn JNI_OnLoad(
            vm: *mut ::std::ffi::c_void,
            _reserved: *mut ::std::ffi::c_void,
        ) -> i32 {
            // SAFETY: `vm` is the pointer the virtual machine passes to this function.
            unsafe { $crate::on_load(vm, $build) }
        }
    };
}
