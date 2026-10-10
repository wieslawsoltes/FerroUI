//! ferroui-ios
//!
//! The platform of FerroUI for iOS: a `UIView` that hosts a top-level, the
//! application delegate with the single-view lifetime, the dispatcher on
//! the run loop of the main thread, the render timer on a display link,
//! Metal graphics over the layer of the view, touch, key and scroll input,
//! text input from the keyboard of the system, the settings of the
//! system, the safe area and the screens. `docs/porting/ios-platform.md` has the design, the file
//! table and the stages.
//!
//! The platform calls UIKit, Core Animation and Metal through the `objc2`
//! family of crates; nothing of it is written in Objective-C. Everything
//! that touches UIKit exists on iOS only. What is logic and not a call into
//! the system (the options, the translation of touches, the safe area, the
//! geometry of a screen, the insets manager, the lifetimes' contracts)
//! builds on every system, and the dispatcher builds on every Apple system
//! (it needs Core Foundation and libdispatch only): that is what the tests
//! of the crate run on the development machine.

pub mod activatable_lifetime;
pub mod clipboard;
pub mod combined_span3;
pub mod completion;
pub mod extensions;
pub mod ferro_app_delegate;
pub mod input_handler;
pub mod insets_manager;
pub mod ios_platform_feedback;
pub mod ios_screens;
pub mod platform;
pub mod platform_settings;
pub mod storage;
pub mod stubs;
pub mod text_input_responder;
pub mod ui_kit_input_pane;
pub mod view_controller;

#[cfg(target_vendor = "apple")]
pub mod dispatcher_impl;
#[cfg(target_vendor = "apple")]
pub mod interop;

#[cfg(target_os = "ios")]
pub mod display_link_timer;
#[cfg(target_os = "ios")]
pub mod ferro_scene_delegate;
#[cfg(target_os = "ios")]
pub mod ferro_view;
#[cfg(target_os = "ios")]
pub mod ios_launcher;
#[cfg(target_os = "ios")]
pub mod metal;
#[cfg(target_os = "ios")]
pub mod native_control_host_impl;
#[cfg(target_os = "ios")]
pub mod single_view_lifetime;

pub use ferro_app_delegate::{FerroApplicationDelegate, IFerroAppDelegate};
pub use platform::{IosPlatformOptions, IosRenderingMode};
pub use view_controller::{IFerroViewController, StatusBarStyle};

#[cfg(target_os = "ios")]
pub use ferro_app_delegate::{run_application, run_application_with, FerroAppDelegate};
#[cfg(target_os = "ios")]
pub use ferro_view::FerroView;
#[cfg(target_os = "ios")]
pub use native_control_host_impl::UIViewControlHandle;
#[cfg(target_os = "ios")]
pub use platform::{IosApplicationExtensions, Platform};
#[cfg(target_os = "ios")]
pub use view_controller::DefaultFerroViewController;
