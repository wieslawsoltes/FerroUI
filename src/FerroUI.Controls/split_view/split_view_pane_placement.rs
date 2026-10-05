/// Defines constants for where the pane of a split view should appear.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum SplitViewPanePlacement {
    /// The pane is shown to the left of content.
    #[default]
    Left = 0,

    /// The pane is shown to the right of content.
    Right = 1,

    /// The pane is shown above the content.
    Top = 2,

    /// The pane is shown below the content.
    Bottom = 3,
}
