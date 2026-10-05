use super::PlatformColorValues;
use crate::input::platform::PlatformHotkeyConfiguration;
use crate::input::PointerType;
use crate::reactive::IDisposable;
use crate::Size;
use std::rc::Rc;
use std::time::Duration;

/// A platform-specific settings contract.
///
/// This interface is not meant to be implemented by applications.
pub trait IPlatformSettings {
    /// The size of the rectangle around the location of a pointer down that
    /// a pointer up must occur within in order to register a tap gesture, in
    /// device-independent pixels.
    fn get_tap_size(&self, type_: PointerType) -> Size;

    /// The size of the rectangle around the location of a pointer down that
    /// a pointer up must occur within in order to register a double-tap
    /// gesture, in device-independent pixels.
    fn get_double_tap_size(&self, type_: PointerType) -> Size;

    /// The maximum time that may occur between the first and second click
    /// of a double-tap gesture.
    fn get_double_tap_time(&self, type_: PointerType) -> Duration;

    /// The time a pointer must be held down for in order to register a
    /// holding gesture.
    fn hold_wait_duration(&self) -> Duration;

    /// The key gestures of the platform for common commands.
    fn hotkey_configuration(&self) -> Rc<PlatformHotkeyConfiguration>;

    /// The preferred application language of the user: the tag of the
    /// language (such as `en-US`) the platform reports for the application.
    fn preferred_application_language(&self) -> String;

    /// Gets current system color values including dark mode and accent
    /// colors.
    fn get_color_values(&self) -> PlatformColorValues;

    /// Raises when current system color values are changed, including
    /// changing of a dark mode and accent colors. Disposing the returned
    /// handle unsubscribes.
    fn color_values_changed(&self, handler: Rc<dyn Fn(&PlatformColorValues)>) -> Rc<dyn IDisposable>;

    /// Raised when the system language preferences change. Disposing the
    /// returned handle unsubscribes.
    fn preferred_application_language_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable>;
}
