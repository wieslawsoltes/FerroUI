//! ferroui-winui
//!
//! The framework's content inside a WinUI 3 (Windows App SDK) application:
//! [`FerroSwapChainPanel`], a `SwapChainPanel` the content renders into
//! through a composition swap chain of the Windows backend, with pointer,
//! keyboard and text input, cursors, drag and drop and automation.
//!
//! The design, the bindings, the threads and how it is tested are in
//! `docs/porting/winui.md`. The crate compiles on every host; what calls the
//! system is compiled for Windows only (`cfg(windows)`), and the logic that
//! maps values (keys, cursors, control types, pointer events, text) is
//! compiled, and tested, everywhere.
//!
//! An application initialises the Windows App SDK ([`WindowsAppSdk`]), sets
//! the framework up with the Windows backend and its ANGLE rendering mode
//! (the swap chain of the panel is created on ANGLE's Direct3D 11 device),
//! runs the XAML application ([`XamlApplication::start`]) and, from its
//! launch, puts a panel into a window of WinUI with the content it shows.
//! Once `Application.Start` has returned and the bootstrap is shut down, it
//! ends the process with [`end_process`], as the runtime of the reference
//! ends it (docs/porting/winui.md, section 7.7).

#![cfg_attr(not(windows), allow(dead_code))]

#[macro_use]
pub mod interop;

mod automation;
mod ferro_swap_chain_panel;
mod process_end;
mod win_ui_cursor_factory;
mod win_ui_drag_source;
mod win_ui_key_interop;
mod win_ui_text_input_method;
mod core_text_input_options;
mod core_text_layout;
mod core_text_synchronization;
mod windows_app_sdk;
mod xaml_application;

#[cfg(windows)]
pub use ferro_swap_chain_panel::FerroSwapChainPanel;
pub use windows_app_sdk::{parse_major_minor, WindowsAppSdkOptions};
#[cfg(windows)]
pub use process_end::{end_process, process_threads};
#[cfg(windows)]
pub use windows_app_sdk::WindowsAppSdk;
#[cfg(windows)]
pub use xaml_application::{XamlApplication, XamlGrid, XamlWindow};
