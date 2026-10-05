/// A platform specific behavior that can be inhibited.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum PlatformInhibitionType {
    /// When inhibited, prevents the app from being put to sleep or being
    /// given a lower priority when not in focus.
    AppSleep = 0,
}
