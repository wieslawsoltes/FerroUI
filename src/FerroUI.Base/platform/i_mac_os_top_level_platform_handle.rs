/// The native handles of a top level on macOS.
///
/// This contract is not stable: it follows the needs of the platform
/// backend. The pointers are raw native object pointers.
pub trait IMacOSTopLevelPlatformHandle {
    /// The `NSView` of the top level.
    fn ns_view(&self) -> isize;

    /// The `NSView` of the top level, retained: the caller releases it.
    fn get_ns_view_retained(&self) -> isize;

    /// The `NSWindow` of the top level.
    fn ns_window(&self) -> isize;

    /// The `NSWindow` of the top level, retained: the caller releases it.
    fn get_ns_window_retained(&self) -> isize;
}
