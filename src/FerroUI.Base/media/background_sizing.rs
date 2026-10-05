/// Defines how a background is drawn relative to its border.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum BackgroundSizing {
    /// The background is drawn up to the inside edge of the border.
    ///
    /// The background will never be drawn under the border itself and will not be visible
    /// underneath the border regardless of border transparency.
    #[default]
    InnerBorderEdge = 0,

    /// The background is drawn completely to the outside edge of the border.
    ///
    /// The background will be visible underneath the border if the border has transparency.
    OuterBorderEdge = 1,

    /// The background is drawn to the midpoint (center) of the border.
    ///
    /// The background will be visible underneath half of the border if the border has
    /// transparency. For this reason it is not recommended to use
    /// [`CenterBorder`](Self::CenterBorder) if transparency is involved.
    ///
    /// This value does not exist in other XAML frameworks and only exists for backwards
    /// compatibility with legacy code. Before [`BackgroundSizing`] was added, rendering always
    /// used this value (Skia's default).
    CenterBorder = 2,
}
