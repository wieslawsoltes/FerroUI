//! The screens of the server: the screens implementation, the providers of
//! the raw screen information and the scale factors.

pub mod x11_screen_providers;
pub mod x11_screens;
pub mod x11_screens_scaling;

pub use x11_screen_providers::{
    FallbackScreensImpl, IX11RawScreenInfoProvider, IX11RawScreenInfoProviderWithRefreshRate, MonitorInfo,
    Randr15ScreensImpl, X11Screen,
};
pub use x11_screens::X11Screens;
pub use x11_screens_scaling::{
    IScalingProvider, NullScalingProvider, PhysicalDpiScalingProvider, PostMultiplyScalingProvider,
    UserConfiguredScalingProvider, UserScalingConfiguration, XrdbScalingProvider, GLOBAL_SCALE_FACTOR_VARIABLE,
    SCREEN_SCALE_FACTORS_VARIABLE, SCREEN_SCALE_IGNORE_QT_VARIABLE, USE_PHYSICAL_DPI_VARIABLE,
};
