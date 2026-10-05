use bitflags::bitflags;

bitflags! {
    /// Defines the selection mode for a control which can select multiple
    /// items.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct SelectionMode: i32 {
        /// One item can be selected.
        const SINGLE = 0x00;

        /// Multiple items can be selected.
        const MULTIPLE = 0x01;

        /// Item selection can be toggled by tapping/spacebar.
        const TOGGLE = 0x02;

        /// An item will always be selected as long as there are items to
        /// select.
        const ALWAYS_SELECTED = 0x04;
    }
}
