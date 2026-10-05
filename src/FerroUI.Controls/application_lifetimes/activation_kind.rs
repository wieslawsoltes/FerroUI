/// The kind of an application activation or deactivation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ActivationKind {
    /// When the application is passed a URI to open one or more files (for
    /// example on iOS and macOS).
    File = 10,

    /// When the application is passed a URI to open (deep linking).
    OpenUri = 20,

    /// When the application is asked to reopen. An example of this is on
    /// macOS when all the windows are closed, the application continues to
    /// run in the background, and the user clicks the application's dock
    /// icon.
    Reopen = 30,

    /// When the application enters or leaves a background state. An example
    /// is when on macOS the user hides or shows an application (not a
    /// window), which is equivalent to the application moving into or out
    /// of the background on a mobile platform.
    Background = 40,
}
