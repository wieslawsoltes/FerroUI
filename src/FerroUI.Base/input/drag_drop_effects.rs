use bitflags::bitflags;

bitflags! {
    /// The effects of a drag-and-drop operation.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct DragDropEffects: i32 {
        /// The data is not accepted.
        const NONE = 0;
        /// The data is copied to the drop target.
        const COPY = 1;
        /// The data is moved to the drop target.
        const MOVE = 2;
        /// The data is linked to the drop target.
        const LINK = 4;
    }
}
