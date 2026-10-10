//! The services of the platform (the `Platform` directory of the
//! reference).

pub(crate) mod android_activatable_lifetime;
pub(crate) mod android_data_format_helper;
pub(crate) mod android_insets_manager;
#[cfg(target_os = "android")]
pub(crate) mod android_launcher;
#[cfg(target_os = "android")]
pub(crate) mod android_native_control_host_impl;
pub(crate) mod android_platform_feedback;
pub(crate) mod android_platform_settings;
pub(crate) mod android_screens;
pub(crate) mod android_system_navigation_manager;
#[cfg(target_os = "android")]
pub(crate) mod clip_data_item_to_data_transfer_item_wrapper;
#[cfg(target_os = "android")]
pub(crate) mod clip_data_to_data_transfer_wrapper;
#[cfg(target_os = "android")]
pub(crate) mod clipboard_impl;
pub(crate) mod input;
pub(crate) mod platform_support;
pub(crate) mod skia_platform;
pub(crate) mod specific;
pub(crate) mod storage;

pub use android_activatable_lifetime::AndroidActivatableLifetime;
pub use android_insets_manager::AndroidInsetsManager;
#[cfg(target_os = "android")]
pub use android_native_control_host_impl::AndroidNativeControlHostImpl;
pub use android_platform_settings::AndroidPlatformSettings;
pub use android_screens::{AndroidScreen, AndroidScreens};
pub use android_system_navigation_manager::AndroidSystemNavigationManagerImpl;
pub use input::android_keyboard_device::AndroidKeyboardDevice;
#[cfg(target_os = "android")]
pub use skia_platform::TopLevelImpl;
