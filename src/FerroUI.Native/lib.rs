//! ferroui-native
//!
//! macOS native windowing backend: the Objective-C++ library in
//! `native/FerroUI.Native` (compiled and linked by `build.rs`), the raw
//! COM bindings generated from `frn.idl` and the platform implementation on
//! top of them: dispatcher, render timer, windows, popups, screens, cursors,
//! platform settings and the Metal platform graphics.
//!
//! An application selects the backend with
//! [`FerroNativePlatformExtensions::use_ferro_native`] on its application
//! builder; a host without one calls [`FerroNativePlatform::initialize`].
//!
//! The crate is empty on every other target.

#[cfg(target_os = "macos")]
mod callback_base;
#[cfg(target_os = "macos")]
mod clipboard_data_format_helper;
#[cfg(target_os = "macos")]
mod clipboard_data_transfer;
#[cfg(target_os = "macos")]
mod clipboard_data_transfer_item;
#[cfg(target_os = "macos")]
mod clipboard_impl;
#[cfg(target_os = "macos")]
mod clipboard_read_session;
#[cfg(target_os = "macos")]
mod cursor;
#[cfg(target_os = "macos")]
mod data_transfer_item_to_frn_clipboard_data_item_wrapper;
#[cfg(target_os = "macos")]
mod data_transfer_to_frn_clipboard_data_source_wrapper;
#[cfg(target_os = "macos")]
mod deferred_framebuffer;
#[cfg(target_os = "macos")]
mod dispatcher_impl;
#[cfg(target_os = "macos")]
mod double_click_helper;
#[cfg(target_os = "macos")]
mod embeddable_top_level_impl;
#[cfg(target_os = "macos")]
mod extensions;
#[cfg(target_os = "macos")]
mod ferro_native_application_platform;
#[cfg(target_os = "macos")]
mod ferro_native_drag_source;
#[cfg(target_os = "macos")]
mod ferro_native_menu_exporter;
#[cfg(target_os = "macos")]
mod ferro_native_platform;
mod ferro_native_platform_extensions;
#[cfg(target_os = "macos")]
mod ferro_native_render_timer;
#[cfg(target_os = "macos")]
mod ferro_native_text_input_method;
#[cfg(target_os = "macos")]
mod frn_automation_peer;
#[cfg(target_os = "macos")]
mod frn_dispatcher;
#[cfg(target_os = "macos")]
mod frn_menu;
#[cfg(target_os = "macos")]
mod frn_menu_item;
#[cfg(target_os = "macos")]
mod frn_string;
#[cfg(target_os = "macos")]
mod helpers;
#[cfg(target_os = "macos")]
mod icon_loader;
#[cfg(target_os = "macos")]
mod mac_os_activatable_lifetime;

#[cfg(target_os = "macos")]
mod mac_os_mounted_volume_info_provider;
#[cfg(target_os = "macos")]
mod mac_os_native_menu_commands;
#[cfg(target_os = "macos")]
mod menu_action_callback;
#[cfg(target_os = "macos")]
mod metal;
#[cfg(target_os = "macos")]
mod native_control_host_impl;

#[cfg(target_os = "macos")]
mod native_platform_settings;
#[cfg(target_os = "macos")]
mod platform_behavior_inhibition;
#[cfg(target_os = "macos")]
mod popup_impl;
#[cfg(target_os = "macos")]
mod predicate_callback;
#[cfg(target_os = "macos")]
mod screen_impl;
#[cfg(target_os = "macos")]
mod storage_item;

#[cfg(target_os = "macos")]
mod storage_provider_api;

#[cfg(target_os = "macos")]
mod storage_provider_impl;

#[cfg(target_os = "macos")]
mod top_level_impl;
#[cfg(target_os = "macos")]
mod tray_icon_impl;
#[cfg(target_os = "macos")]
mod window_impl;
#[cfg(target_os = "macos")]
mod window_impl_base;

pub use ferro_native_platform_extensions::{FerroNativePlatformOptions, FerroNativeRenderingMode, MacOSPlatformOptions};

#[cfg(target_os = "macos")]
pub use self::{
    clipboard_impl::ClipboardImpl,
    cursor::{CursorFactory, FerroNativeCursor},
    dispatcher_impl::DispatcherImpl,
    embeddable_top_level_impl::EmbeddableTopLevelImpl,
    ferro_native_application_platform::FerroNativeApplicationPlatform,
    ferro_native_drag_source::FerroNativeDragSource,
    ferro_native_menu_exporter::FerroNativeMenuExporter,
    ferro_native_platform::FerroNativePlatform,
    ferro_native_render_timer::FerroNativeRenderTimer,
    ferro_native_text_input_method::FerroNativeTextInputMethod,
    frn_string::{frn_string_array_to_vec, frn_string_bytes, frn_string_to_string, FrnString, FrnStringArray},
    icon_loader::IconLoader,
    ferro_native_platform_extensions::FerroNativePlatformExtensions,
    mac_os_activatable_lifetime::MacOSActivatableLifetime,
    mac_os_native_menu_commands::MacOSNativeMenuCommands,
    metal::{MetalDevice, MetalDrawingSession, MetalPlatformGraphics, MetalPlatformSurface, MetalRenderTarget},
    native_platform_settings::NativePlatformSettings,
    platform_behavior_inhibition::PlatformBehaviorInhibition,
    popup_impl::PopupImpl,
    screen_impl::{NativeScreen, ScreenImpl},
    top_level_impl::{MacOSTopLevelHandle, TopLevelFramebufferSurface, TopLevelImpl},
    tray_icon_impl::TrayIconImpl,
    window_impl::WindowImpl,
    window_impl_base::WindowBaseImpl,
};

/// Raw COM bindings generated from `frn.idl` by `microcom-codegen`.
#[cfg(target_os = "macos")]
#[allow(non_camel_case_types, non_snake_case, clippy::all)]
pub mod interop {
    include!(concat!(env!("OUT_DIR"), "/frn_interop.rs"));

    extern "C" {
        /// Entry point of the native library (`main.mm`). Returns a new
        /// factory with one reference owned by the caller.
        pub fn CreateFerroNative() -> *mut IFerroNativeFactory;
    }

    /// Creates the native factory, the root object of the backend.
    pub fn create_ferro_native() -> Option<ComPtr<IFerroNativeFactory>> {
        // SAFETY: the function returns null or an owned factory reference.
        unsafe { ComPtr::from_raw(CreateFerroNative()) }
    }
}

#[cfg(all(test, target_os = "macos"))]
mod menu_tests;

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::interop::*;
    use ferroui_microcom::{ComPtr, HResult, IUnknown, Interface};
    use std::cell::Cell;
    use std::ffi::CStr;
    use std::mem::size_of;
    use std::rc::Rc;

    const PTR: usize = size_of::<usize>();

    /// Slots are laid out linearly: IUnknown (3) + every interface in the
    /// chain, in declaration order (also for `cpp-virtual-inherits`).
    #[test]
    fn vtable_sizes() {
        assert_eq!(size_of::<IFrnStringVtbl>(), (3 + 2) * PTR);
        assert_eq!(size_of::<IFrnTopLevelVtbl>(), (3 + 16) * PTR);
        assert_eq!(size_of::<IFrnWindowBaseVtbl>(), (3 + 16 + 17) * PTR);
        assert_eq!(size_of::<IFrnPopupVtbl>(), (3 + 16 + 17 + 1) * PTR);
        assert_eq!(size_of::<IFrnWindowVtbl>(), (3 + 16 + 17 + 14) * PTR);
        assert_eq!(size_of::<IFrnWindowEventsVtbl>(), (3 + 11 + 3 + 3) * PTR);
        assert_eq!(size_of::<IFrnWindow>(), PTR);
    }

    #[test]
    fn struct_layouts_match_cpp() {
        assert_eq!(size_of::<FrnSize>(), 16);
        assert_eq!(size_of::<FrnRect>(), 32);
        assert_eq!(size_of::<FrnColor>(), 4);
        assert_eq!(size_of::<FrnScreen>(), 32 + 32 + 4 + 4 + 4 + 4);
        assert_eq!(size_of::<FrnFramebuffer>(), 8 + 4 + 4 + 4 + 4 + 16 + 4 + 4);
        assert_eq!(size_of::<FrnKey>(), 4);
        assert_eq!(FrnLandmarkType::LandmarkNone.0, -1);
        assert_eq!(FrnLandmarkType::LandmarkBanner.0, 0);
        assert_eq!(FrnKey::FrnKeyEnter, FrnKey::FrnKeyReturn);
        assert_eq!(format!("{:?}", FrnWindowState(99)), "FrnWindowState(99)");
    }

    struct Source {
        calls: Rc<Cell<u32>>,
    }
    impl IFrnClipboardDataSourceImpl for Source {
        fn get_item_count(&self) -> i32 {
            self.calls.set(self.calls.get() + 1);
            7
        }
        fn get_item(&self, index: i32) -> Result<Option<ComPtr<IFrnClipboardDataItem>>, HResult> {
            if index == 0 {
                Ok(None)
            } else {
                Err(HResult::INVALIDARG)
            }
        }
    }

    /// Rust implementation called back through its own COM vtable.
    #[test]
    fn rust_callback_roundtrip() {
        let calls = Rc::new(Cell::new(0));
        let source = IFrnClipboardDataSource::from_impl(Source { calls: calls.clone() });
        assert_eq!(source.get_item_count(), 7);
        assert_eq!(calls.get(), 1);
        assert_eq!(source.get_item(0), Ok(None));
        assert_eq!(source.get_item(3), Err(HResult::INVALIDARG));

        // QueryInterface: own IID (both spellings) and IUnknown, nothing else.
        assert!(source.cast::<IFrnClipboardDataSource>().is_ok());
        assert!(source.cast::<IUnknown>().is_ok());
        assert_eq!(source.cast::<IFrnClipboard>().unwrap_err(), HResult::NOINTERFACE);
        assert!(IFrnWindowEvents::matches_iid(&IFrnTopLevelEvents::IID));
        assert!(!IFrnTopLevelEvents::matches_iid(&IFrnWindowEvents::IID));
        assert_eq!(Rc::strong_count(&calls), 2);
        drop(source);
        assert_eq!(Rc::strong_count(&calls), 1, "implementation dropped with the last reference");
    }

    struct Peer;
    impl IFrnTextInputMethodClientImpl for Peer {
        fn set_preedit_text(&self, preedit_text: Option<&CStr>) {
            assert_eq!(preedit_text, Some(c"abc"));
        }
        fn select_in_surrounding_text(&self, start: i32, end: i32) {
            assert_eq!((start, end), (1, 2));
        }
    }

    #[test]
    fn rc_and_string_arguments() {
        let client = IFrnTextInputMethodClient::from_impl(Rc::new(Peer));
        client.set_preedit_text(Some(c"abc"));
        client.select_in_surrounding_text(1, 2);
    }

    /// Talks to the real native library without needing a window server
    /// connection: `GetMacOptions` only allocates a C++ object.
    #[test]
    fn native_factory_basic_calls() {
        let factory = create_ferro_native().expect("factory");
        let options = factory.get_mac_options().expect("mac options");
        options.set_disable_set_process_name(1).unwrap();
        // Native QueryInterface with the IID as materialized by com.h.
        assert!(factory.cast::<IFerroNativeFactory>().is_ok());
        assert!(factory.cast::<IUnknown>().is_ok());
        assert_eq!(factory.cast::<IFrnWindow>().unwrap_err(), HResult::NOINTERFACE);
        let helper = factory.create_memory_management_helper().unwrap().expect("helper");
        drop(helper);
    }
}
