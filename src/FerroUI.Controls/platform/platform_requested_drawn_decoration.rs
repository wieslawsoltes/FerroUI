use bitflags::bitflags;

bitflags! {
    /// The parts of the window decorations the platform asks the toolkit to
    /// draw itself.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct PlatformRequestedDrawnDecoration: i32 {
        const NONE = 0;
        const SHADOW = 1;
        const BORDER = 2;
        const RESIZE_GRIPS = 4;
        const TITLE_BAR = 8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_match_the_reference_values() {
        assert_eq!(PlatformRequestedDrawnDecoration::NONE.bits(), 0);
        assert_eq!(PlatformRequestedDrawnDecoration::SHADOW.bits(), 1);
        assert_eq!(PlatformRequestedDrawnDecoration::BORDER.bits(), 2);
        assert_eq!(PlatformRequestedDrawnDecoration::RESIZE_GRIPS.bits(), 4);
        assert_eq!(PlatformRequestedDrawnDecoration::TITLE_BAR.bits(), 8);
    }
}
