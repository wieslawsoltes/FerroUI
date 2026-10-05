/// Background sources for acrylic.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum AcrylicBackgroundSource {
    /// The acrylic has no background.
    #[default]
    None = 0,
    /// Cuts through all render layers to reveal the window background. This
    /// means if your window is transparent or blurred it can be blended with
    /// the material.
    Digger = 1,
}
