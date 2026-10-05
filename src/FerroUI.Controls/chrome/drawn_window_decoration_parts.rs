use bitflags::bitflags;

bitflags! {
    /// Flags controlling which parts of drawn window decorations are active.
    /// Set by the window based on platform capabilities and user
    /// preferences.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub(crate) struct DrawnWindowDecorationParts: i32 {
        /// No decoration parts are active.
        const NONE = 0;

        /// Shadow/outer area is active.
        const SHADOW = 1;

        /// Frame border is active.
        const BORDER = 2;

        /// Titlebar is active.
        const TITLE_BAR = 4;

        /// Resize grips are active.
        const RESIZE_GRIPS = 8;

        /// All decoration parts are active.
        const ALL = Self::SHADOW.bits() | Self::BORDER.bits() | Self::TITLE_BAR.bits() | Self::RESIZE_GRIPS.bits();
    }
}
