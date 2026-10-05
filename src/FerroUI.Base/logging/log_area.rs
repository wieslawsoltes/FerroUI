/// Specifies the area in which a log event occurred.
pub struct LogArea;

impl LogArea {
    /// The log event comes from the property system.
    pub const PROPERTY: &'static str = "Property";

    /// The log event comes from the binding system.
    pub const BINDING: &'static str = "Binding";

    /// The log event comes from the animations system.
    pub const ANIMATIONS: &'static str = "Animations";

    /// The log event comes from the fonts system.
    pub const FONTS: &'static str = "Fonts";

    /// The log event comes from the visual system.
    pub const VISUAL: &'static str = "Visual";

    /// The log event comes from the layout system.
    pub const LAYOUT: &'static str = "Layout";

    /// The log event comes from the control system.
    pub const CONTROL: &'static str = "Control";

    /// The log event comes from the platform abstraction layer.
    pub const PLATFORM: &'static str = "Platform";

    /// The log event comes from Win32 Platform.
    pub const WIN32_PLATFORM: &'static str = "Win32Platform";

    /// The log event comes from WinUI system.
    pub const WINUI_PLATFORM: &'static str = "WinUIPlatform";

    /// The log event comes from X11 Platform.
    pub const X11_PLATFORM: &'static str = "X11Platform";

    /// The log event comes from Android Platform.
    pub const ANDROID_PLATFORM: &'static str = "AndroidPlatform";

    /// The log event comes from iOS Platform.
    pub const IOS_PLATFORM: &'static str = "IOSPlatform";

    /// The log event comes from LinuxFramebuffer Platform.
    pub const LINUX_FRAMEBUFFER_PLATFORM: &'static str = "LinuxFramebufferPlatform";

    /// The log event comes from FreeDesktop Platform.
    pub const FREE_DESKTOP_PLATFORM: &'static str = "FreeDesktopPlatform";

    /// The log event comes from macOS Platform.
    pub const MACOS_PLATFORM: &'static str = "macOSPlatform";

    /// The log event comes from Browser Platform.
    pub const BROWSER_PLATFORM: &'static str = "BrowserPlatform";

    /// The log event comes from VNC Platform.
    pub const VNC_PLATFORM: &'static str = "VncPlatform";
}
