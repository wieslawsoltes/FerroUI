/// Specifies the position of the tab strip within a tabbed page layout.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum TabPlacement {
    /// Automatically determines the tab placement based on the target
    /// platform. Resolves to [`Bottom`](Self::Bottom) on iOS and Android,
    /// and [`Top`](Self::Top) on all other platforms.
    #[default]
    Auto = 0,

    /// Displays tabs at the top of the content area.
    Top = 1,

    /// Displays tabs at the bottom of the content area.
    Bottom = 2,

    /// Displays tabs along the left side of the content area.
    Left = 3,

    /// Displays tabs along the right side of the content area.
    Right = 4,
}
