//! The windowing platform contracts: what an OS backend implements to give
//! the toolkit windows, popups, screens, tray icons and related services.

pub(crate) mod default_menu_interaction_handler;
mod i_input_pane;
mod i_insets_manager;
mod i_menu_interaction_handler;
mod i_native_application_commands;
mod i_native_control_host_impl;
mod i_platform_feedback;
mod i_platform_handle;
mod i_platform_icon_loader;
mod i_platform_lifetime_events_impl;
mod i_platform_native_surface_handle;
mod i_popup_impl;
mod i_screen_impl;
mod i_top_level_impl;
mod i_top_level_native_menu_exporter;
mod i_tray_icon_impl;
mod mac_os_properties;
mod i_win32_options_top_level_impl;
mod i_window_base_impl;
mod i_window_icon_impl;
mod i_window_impl;
mod i_windowing_platform;
mod i_x11_options_toplevel_impl_feature;
mod platform_allowed_window_actions;
mod platform_feedback;
mod platform_handle;
mod platform_manager;
mod platform_requested_drawn_decoration;
mod screen;
mod screen_helper;
mod win32_properties;

// --- storage-misc ---
mod dialogs;
mod in_process_drag_source;
mod x11_properties;
pub use dialogs::{IMountedVolumeInfoProvider, IStorageProviderFactory, MountedVolumeInfo};
pub use in_process_drag_source::InProcessDragSource;
pub use x11_properties::X11Properties;

pub use default_menu_interaction_handler::{
    DefaultMenuInteractionHandler, DefaultMenuInteractionHandlerOverrides, MenuDelayRun,
};
pub use i_menu_interaction_handler::IMenuInteractionHandler;
pub use i_input_pane::{IInputPane, InputPaneBase, InputPaneState, InputPaneStateEventArgs};
pub use i_insets_manager::{IInsetsManager, InsetsManagerBase, SafeAreaChangedArgs, SystemBarTheme};
pub use i_native_application_commands::INativeApplicationCommands;
pub use i_native_control_host_impl::{
    INativeControlHostControlTopLevelAttachment, INativeControlHostDestroyableControlHandle, INativeControlHostImpl,
};
pub use i_platform_feedback::{FeedbackAction, FeedbackType, IPlatformFeedback};
pub use i_platform_handle::IPlatformHandle;
pub use i_platform_icon_loader::IPlatformIconLoader;
pub use i_platform_lifetime_events_impl::IPlatformLifetimeEventsImpl;
pub use i_platform_native_surface_handle::INativePlatformHandleSurface;
pub use i_popup_impl::IPopupImpl;
pub use i_screen_impl::{IScreenImpl, PlatformScreen, ScreensBase, ScreensBaseImpl, ScreensBaseImplExt};
pub use i_top_level_impl::ITopLevelImpl;
pub use i_top_level_native_menu_exporter::{
    as_native_menu_exporter_provider, register_native_menu_exporter_provider, INativeMenuExporter,
    INativeMenuExporterProvider, ITopLevelNativeMenuExporter,
};
pub use i_tray_icon_impl::{ITrayIconImpl, ITrayIconWithIsTemplateImpl};
pub use mac_os_properties::MacOSProperties;
pub use i_win32_options_top_level_impl::IWin32OptionsTopLevelImpl;
pub use i_window_base_impl::IWindowBaseImpl;
pub use i_window_icon_impl::IWindowIconImpl;
pub use i_window_impl::IWindowImpl;
pub use i_windowing_platform::IWindowingPlatform;
pub use i_x11_options_toplevel_impl_feature::{IX11OptionsToplevelImplFeature, X11NetWmWindowType};
pub use platform_allowed_window_actions::PlatformAllowedWindowActions;
pub use platform_feedback::{PlatformFeedback, PlatformFeedbackExtensions};
pub use ferroui_base::platform::{ColorContrastPreference, PlatformThemeVariant};
pub use platform_handle::PlatformHandle;
pub use platform_manager::PlatformManager;
pub use platform_requested_drawn_decoration::PlatformRequestedDrawnDecoration;
pub use screen::{Screen, ScreenOrientation};
pub use screen_helper::ScreenHelper;
pub use win32_properties::{
    CustomWindowStylesCallback, CustomWndProcHookCallback, Win32HitTestValue, Win32Properties, WindowCornerPreference,
};

#[cfg(test)]
mod default_menu_interaction_handler_tests;
#[cfg(test)]
mod i_screen_impl_tests;
#[cfg(test)]
mod platform_manager_tests;
#[cfg(test)]
mod input_pane_tests;
