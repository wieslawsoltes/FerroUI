// `compiled_markup`: the table of the units of the WinUI theme the build wrote (`build.rs`).
ferroui_markup_xaml::include_compiled_xaml!();
#[cfg(any(target_os = "android", target_os = "ios"))]
mod app;
#[cfg(target_os = "ios")]
pub use app::AppDelegate;
#[cfg(target_os = "android")]
ferroui_android::android_application!(app::build);
