#[cfg(any(target_os = "android", target_os = "ios"))]
mod app;
#[cfg(target_os = "ios")]
pub use app::Delegate;
#[cfg(target_os = "android")]
ferroui_android::android_application!(app::build);
