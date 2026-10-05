use bitflags::bitflags;

bitflags! {
    /// Flags specifying which elements are displayed in a drawn window title
    /// bar of [`WindowDrawnDecorations`](super::WindowDrawnDecorations).
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct TitleBarDecorations: i32 {
        /// No title bar element is displayed.
        const NONE = 0;

        /// The window title is displayed.
        const TITLE = 1 << 0;

        /// The minimize button is displayed.
        const MINIMIZE_BUTTON = 1 << 1;

        /// The maximize button is displayed.
        const MAXIMIZE_BUTTON = 1 << 2;

        /// The close button is displayed.
        const CLOSE_BUTTON = 1 << 3;

        /// The full screen button is displayed.
        const FULL_SCREEN_BUTTON = 1 << 4;

        /// All title bar elements are displayed.
        const ALL = Self::TITLE.bits()
            | Self::MINIMIZE_BUTTON.bits()
            | Self::MAXIMIZE_BUTTON.bits()
            | Self::CLOSE_BUTTON.bits()
            | Self::FULL_SCREEN_BUTTON.bits();
    }
}
