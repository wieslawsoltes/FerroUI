/// Specifies the visibility of a scroll bar for scrollable content.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ScrollBarVisibility {
    /// No scrollbars and no scrolling in this dimension.
    #[default]
    Disabled = 0,
    /// The scrollbar should be visible only if there is more content than
    /// fits in the viewport.
    Auto = 1,
    /// The scrollbar should never be visible. No space should ever be
    /// reserved for the scrollbar.
    Hidden = 2,
    /// The scrollbar should always be visible. Space should always be
    /// reserved for the scrollbar.
    Visible = 3,
}
