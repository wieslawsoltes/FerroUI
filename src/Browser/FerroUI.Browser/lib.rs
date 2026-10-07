//! The browser platform.
//!
//! An application runs on the main thread of a web page, inside the
//! WebAssembly module the host page creates. There are no windows: content
//! is shown by a [`FerroView`] over an element of the page, set as the main
//! view of the single-view lifetime that
//! [`BrowserAppBuilder::start_browser_app`] installs. The browser drives
//! everything: dispatcher work is posted as tasks, rendering follows the
//! animation frames, and the start-up function returns to the page.
//!
//! The script side of the platform is the `ferroui.js` module built from
//! `webapp/`. [`interop`] is the only place that crosses the boundary: it
//! imports functions of that module and exports the callbacks the module
//! invokes.

pub mod interop;
pub mod rendering;
pub mod storage;

mod browser_activatable_lifetime;
mod browser_app_builder;
mod browser_clipboard_data_transfer;
mod browser_clipboard_data_transfer_item;
mod browser_data_format_helper;
mod browser_data_transfer_helper;
mod browser_drag_data_transfer;
mod browser_drag_data_transfer_item;
mod browser_input_handler;
mod browser_insets_manager;
mod browser_input_pane;
mod browser_mouse_device;
mod browser_native_control_host;
mod browser_platform_settings;
mod browser_runtime_platform;
mod browser_screens;
mod browser_single_threaded_dispatcher_impl;
mod browser_single_view_lifetime;
mod browser_system_navigation_manager;
mod browser_text_input_method;
mod browser_top_level_impl;
mod clipboard_impl;
mod cursor;
mod ferro_view;
mod js_object_control_handle;
mod key_interop;
mod win_stubs;
mod windowing_platform;

pub use browser_activatable_lifetime::BrowserActivatableLifetime;
pub use browser_app_builder::{BrowserAppBuilder, BrowserPlatformOptions, BrowserRenderingMode};
pub use browser_input_handler::BrowserInputHandler;
pub use browser_insets_manager::BrowserInsetsManager;
pub use browser_input_pane::BrowserInputPane;
pub use browser_platform_settings::BrowserPlatformSettings;
pub use browser_runtime_platform::{BrowserRuntimePlatform, BrowserRuntimePlatformServices};
pub use browser_screens::{BrowserScreen, BrowserScreenKey, BrowserScreens};
pub use browser_single_threaded_dispatcher_impl::BrowserSingleThreadedDispatcherImpl;
pub use browser_single_view_lifetime::BrowserSingleViewLifetime;
pub use browser_system_navigation_manager::BrowserSystemNavigationManagerImpl;
pub use browser_text_input_method::BrowserTextInputMethod;
pub use browser_top_level_impl::BrowserTopLevelImpl;
pub use clipboard_impl::ClipboardImpl;
pub use cursor::{CssCursor, CssCursorFactory};
pub use ferro_view::FerroView;
pub use js_object_control_handle::{JsObjectControlHandle, JsObjectPlatformHandle};
pub use win_stubs::IconLoaderStub;
pub use windowing_platform::BrowserWindowingPlatform;
