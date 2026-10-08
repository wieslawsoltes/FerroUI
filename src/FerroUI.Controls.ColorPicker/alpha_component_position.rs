/// Defines the position of a color's alpha component relative to all other components.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum AlphaComponentPosition {
    /// The alpha component occurs before all other components.
    ///
    /// For example, this may indicate the #AARRGGBB or ARGB format which
    /// is the default format for XAML itself and the Color struct.
    Leading = 0,

    /// The alpha component occurs after all other components.
    ///
    /// For example, this may indicate the #RRGGBBAA or RGBA format which
    /// is the default format for CSS.
    Trailing = 1,
}
