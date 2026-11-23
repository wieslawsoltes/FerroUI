//! The `WebView2` control: a web engine of the platform as a native child of the window.
//!
//! The control and its [`CoreWebView2`] speak to one [`IWebViewEngine`]. An engine is made when
//! the control is attached to a window, by the [`IWebViewEngineFactory`] bound in the locator or
//! else by the engine of the platform. What is asked before the engine is ready waits for it.
//! No application links an engine unless it uses this crate.

mod core_web_view2;
mod engine;
pub mod engines;
mod event_args;
mod json;
mod web_view2;

#[cfg(test)]
mod tests;

pub use core_web_view2::CoreWebView2;
pub use engine::{
    CoreWebView2WebErrorStatus, IWebViewEngine, IWebViewEngineEvents, IWebViewEngineFactory,
    ScriptCompletion, WebViewError, WebViewNavigation,
};
pub use event_args::{
    CoreWebView2InitializedEventArgs, NavigationCompletedEventArgs, NavigationStartingEventArgs,
    WebMessageReceivedEventArgs,
};
pub use web_view2::WebView2;

// Shared by the registration at run time and the scanner of the build-time XAML metadata.
const NAMESPACES: &[(&str, &str)] = &[("ferroui_controls_webview", "FerroUI.Controls.WebView")];
const TYPES: &[&ferroui_base::TypeInfo] = &[WebView2::TYPE];

/// Registers the namespace and the table of types of this crate and of the crates it is built
/// on. Cheap and idempotent.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        ferroui_controls::register_types();
        ferroui_base::TypeInfo::register_namespaces(NAMESPACES);
        ferroui_base::TypeInfo::register_table(TYPES);
    });
}
