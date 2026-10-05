use bitflags::bitflags;

bitflags! {
    /// Determines how a window will size itself to fit its content.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct SizeToContent: i32 {
        /// The window will not automatically size itself to fit its content.
        const MANUAL = 0;

        /// The window will size itself horizontally to fit its content.
        const WIDTH = 1;

        /// The window will size itself vertically to fit its content.
        const HEIGHT = 2;

        /// The window will size itself horizontally and vertically to fit
        /// its content.
        const WIDTH_AND_HEIGHT = 3;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_match_the_reference_values() {
        assert_eq!(SizeToContent::MANUAL.bits(), 0);
        assert_eq!(SizeToContent::WIDTH.bits(), 1);
        assert_eq!(SizeToContent::HEIGHT.bits(), 2);
        assert_eq!(SizeToContent::WIDTH_AND_HEIGHT.bits(), 3);
        assert_eq!(SizeToContent::WIDTH | SizeToContent::HEIGHT, SizeToContent::WIDTH_AND_HEIGHT);
        assert_eq!(SizeToContent::default(), SizeToContent::MANUAL);
    }
}
