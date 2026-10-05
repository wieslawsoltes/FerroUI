use bitflags::bitflags;

bitflags! {
    /// The window actions the platform currently allows.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct PlatformAllowedWindowActions: i32 {
        const NONE = 0;
        const MAXIMIZE = 1 << 0;
        const FULLSCREEN = 1 << 1;
        const MINIMIZE = 1 << 2;
        const ALL = Self::MAXIMIZE.bits() | Self::FULLSCREEN.bits() | Self::MINIMIZE.bits();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_match_the_reference_values() {
        assert_eq!(PlatformAllowedWindowActions::NONE.bits(), 0);
        assert_eq!(PlatformAllowedWindowActions::MAXIMIZE.bits(), 1);
        assert_eq!(PlatformAllowedWindowActions::FULLSCREEN.bits(), 2);
        assert_eq!(PlatformAllowedWindowActions::MINIMIZE.bits(), 4);
        assert_eq!(PlatformAllowedWindowActions::ALL.bits(), 7);
        assert_eq!(PlatformAllowedWindowActions::all(), PlatformAllowedWindowActions::ALL);
    }
}
