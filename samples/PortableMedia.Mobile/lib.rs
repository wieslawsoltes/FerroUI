#[cfg(target_os = "android")]
mod app;
#[cfg(target_os = "android")]
ferroui_android::android_application!(app::build);
