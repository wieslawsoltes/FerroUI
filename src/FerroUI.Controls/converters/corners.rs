use bitflags::bitflags;

bitflags! {
    /// Identifies the corners of a rectangle.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct Corners: i32 {
        /// No corner.
        const NONE = 0;

        /// The top left corner.
        const TOP_LEFT = 1;

        /// The top right corner.
        const TOP_RIGHT = 2;

        /// The bottom left corner.
        const BOTTOM_LEFT = 4;

        /// The bottom right corner.
        const BOTTOM_RIGHT = 8;
    }
}
