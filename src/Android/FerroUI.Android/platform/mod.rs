//! The services of the platform (the `Platform` directory of the
//! reference).

pub(crate) mod android_activatable_lifetime;
pub(crate) mod android_insets_manager;
pub(crate) mod android_platform_settings;
pub(crate) mod android_screens;
pub(crate) mod input;
pub(crate) mod skia_platform;
pub(crate) mod specific;

pub use android_activatable_lifetime::AndroidActivatableLifetime;
pub use android_insets_manager::AndroidInsetsManager;
pub use android_platform_settings::AndroidPlatformSettings;
pub use android_screens::{AndroidScreen, AndroidScreens};
pub use input::android_keyboard_device::AndroidKeyboardDevice;
#[cfg(target_os = "android")]
pub use skia_platform::TopLevelImpl;
